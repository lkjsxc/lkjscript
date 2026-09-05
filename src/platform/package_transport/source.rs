//! Code-complete immutable package containers. No repository or executable authority is imported.

use super::{
    PackageRevision, PackageTransport, PackageTransportBinding, package_error, read_revision,
    store_diagnostic, validate_package_transport_closure_metered,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::*;
use crate::platform::package_interface::{
    PackageInterfaceOwner, PackageInterfaceSelection, build_package_interface,
    package_interface_digest,
};
use crate::platform::persistent_map::{MapRoot, MapWork, PersistentMap};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StageOutcome, StoreError, StoreErrorClass,
    StoreReadAdmission, StoreReadLimits, StoreWork,
};
use crate::platform::storage::page_store::ObjectPageReader;
use crate::platform::witness::{FullWitness, rebuild_full_witness_with_limit};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

type EncodedEntries = Vec<(Vec<u8>, Vec<u8>)>;
type InterfaceProjection = (
    BTreeMap<OwnerKey, PackageInterfaceOwner>,
    BTreeMap<TypeObjectDigest, Vec<u8>>,
);

pub const CONTAINER_MAGIC: [u8; 8] = *b"LKJPKC01";
pub const CONTAINER_CONTRACT_IDENTITY: &str = "lkjscript-package-container-1";
pub const CONTAINER_CONTRACT_VERSION: u16 = 1;
pub const READINESS_CONTRACT_IDENTITY: &str = "lkjscript-package-transport-selection-2";
pub const READINESS_CONTRACT_VERSION: u16 = 2;
pub const MAXIMUM_CONTAINER_BYTES: usize = 268_435_456;
pub const MAXIMUM_CONTAINER_OBJECTS: usize = 1_000_000;
pub const MAXIMUM_VALIDATION_VISITS: u64 = 16_000_000;
pub const MAXIMUM_VALIDATION_READ_BYTES: u64 = 4_294_967_296;
pub const READINESS_MAGIC: [u8; 8] = *b"LKJPTS02";
pub const MAXIMUM_READINESS_BYTES: usize = 12 + 64 * super::MAXIMUM_PACKAGE_TRANSPORT_CANDIDATES;

/// One atomic operational inventory. Exact source bindings, never mutable package names.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageReadiness {
    pub bindings: BTreeMap<PackageRevisionDigest, PackageTransportDigest>,
}

impl PackageReadiness {
    pub fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        if self.bindings.len() > super::MAXIMUM_PACKAGE_TRANSPORT_CANDIDATES {
            return Err(limit("ready package revisions"));
        }
        let mut bytes = Vec::with_capacity(12 + self.bindings.len() * 64);
        bytes.extend_from_slice(&READINESS_MAGIC);
        bytes.extend_from_slice(&(self.bindings.len() as u32).to_be_bytes());
        for (revision, transport) in &self.bindings {
            bytes.extend_from_slice(&revision.bytes());
            bytes.extend_from_slice(&transport.bytes());
        }
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        if bytes.len() > MAXIMUM_READINESS_BYTES {
            return Err(limit("ready selection bytes"));
        }
        let mut cursor = Cursor(bytes);
        if cursor.take(8)? != READINESS_MAGIC {
            return Err(corrupt(
                "package_readiness_contract",
                "predecessor package selections are not code-complete readiness; restage exact source",
            ));
        }
        let count = u32::from_be_bytes(cursor.array()?) as usize;
        if count > super::MAXIMUM_PACKAGE_TRANSPORT_CANDIDATES {
            return Err(limit("ready package revisions"));
        }
        if cursor.0.len() != count * 64 {
            return Err(corrupt(
                "package_readiness_length",
                "ready inventory length disagrees with its exact count; restage exact source",
            ));
        }
        let mut bindings = BTreeMap::new();
        let mut previous = None;
        for _ in 0..count {
            let binding = cursor.binding()?;
            if previous.is_some_and(|previous| previous >= binding.package_revision) {
                return Err(corrupt(
                    "package_readiness_order",
                    "ready inventory is not unique and ordered",
                ));
            }
            previous = Some(binding.package_revision);
            bindings.insert(binding.package_revision, binding.transport);
        }
        Ok(Self { bindings })
    }
}

#[derive(Clone, Debug)]
pub struct PackageContainer {
    pub root: PackageTransportBinding,
    pub selections: Vec<PackageTransportBinding>,
    pub objects: BTreeMap<ObjectKey, Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct ImmutablePackage {
    pub binding: PackageTransportBinding,
    pub revision: PackageRevision,
    pub transport: PackageTransport,
    pub snapshot: KernelSnapshot,
    pub witness: FullWitness,
    pub interface_owners: BTreeMap<OwnerKey, PackageInterfaceOwner>,
    pub interface_types: BTreeMap<TypeObjectDigest, Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct AdmittedClosure {
    pub container: PackageContainer,
    pub packages: BTreeMap<PackageRevisionDigest, ImmutablePackage>,
    pub dependency_order: Vec<PackageRevisionDigest>,
    pub validation_visits: u64,
    pub validation_read_bytes: u64,
    pub dependency_edges: usize,
}

fn corrupt(code: &'static str, message: impl Into<String>) -> Diagnostic {
    package_error(DiagnosticClass::Corrupt, code, message)
}

fn limit(dimension: &str) -> Diagnostic {
    package_error(
        DiagnosticClass::Resource,
        "package_source_budget",
        format!(
            "offline package admission exhausted {dimension}; export a smaller exact closure and restage"
        ),
    )
}

fn source_domain(domain: ObjectDomain) -> bool {
    matches!(
        domain,
        ObjectDomain::Owner
            | ObjectDomain::Type
            | ObjectDomain::Blob
            | ObjectDomain::MapPage
            | ObjectDomain::SemanticRoot
            | ObjectDomain::Dependency
            | ObjectDomain::Retirement
            | ObjectDomain::PackageRevision
            | ObjectDomain::PackageTransport
            | ObjectDomain::PackageInterface
    )
}

fn admit_container_counts(bytes: usize, packages: usize, objects: usize) -> Result<(), Diagnostic> {
    if bytes > MAXIMUM_CONTAINER_BYTES {
        return Err(limit("container bytes"));
    }
    if packages == 0 || packages > super::MAXIMUM_PACKAGE_CLOSURE {
        return Err(limit("packages"));
    }
    if objects > MAXIMUM_CONTAINER_OBJECTS {
        return Err(limit("distinct objects"));
    }
    Ok(())
}

impl PackageContainer {
    pub(crate) fn encoded_size(&self) -> Result<usize, Diagnostic> {
        self.validate_inventory()?;
        let size = self
            .objects
            .values()
            .try_fold(84_usize + self.selections.len() * 64, |size, bytes| {
                size.checked_add(41)?.checked_add(bytes.len())
            })
            .ok_or_else(|| limit("container bytes"))?;
        admit_container_counts(size, self.selections.len(), self.objects.len())?;
        Ok(size)
    }

    pub fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        let size = self.encoded_size()?;
        let mut bytes = Vec::with_capacity(size);
        bytes.extend_from_slice(&CONTAINER_MAGIC);
        bytes.extend_from_slice(&self.root.package_revision.bytes());
        bytes.extend_from_slice(&self.root.transport.bytes());
        bytes.extend_from_slice(&(self.selections.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&(self.objects.len() as u64).to_be_bytes());
        for selection in &self.selections {
            bytes.extend_from_slice(&selection.package_revision.bytes());
            bytes.extend_from_slice(&selection.transport.bytes());
        }
        // ObjectDomain enum order is not a contract; order explicitly by its stable wire tag.
        let mut ordered = self.objects.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|(key, _)| (key.domain.tag(), key.digest.bytes()));
        for (key, object) in ordered {
            key.verify(object).map_err(store_diagnostic)?;
            bytes.push(key.domain.tag());
            bytes.extend_from_slice(&key.digest.bytes());
            bytes.extend_from_slice(&(object.len() as u64).to_be_bytes());
            bytes.extend_from_slice(object);
        }
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8], expected: PackageTransportDigest) -> Result<Self, Diagnostic> {
        Self::decode_bounded(
            bytes,
            expected,
            MAXIMUM_CONTAINER_BYTES,
            super::MAXIMUM_PACKAGE_CLOSURE,
            MAXIMUM_CONTAINER_OBJECTS,
        )
    }

    // Internal lowering of hard decoder ceilings supports finite exact-fit fixtures. No public
    // input selects these bounds, and no caller can raise the normative ceilings.
    fn decode_bounded(
        bytes: &[u8],
        expected: PackageTransportDigest,
        maximum_bytes: usize,
        maximum_packages: usize,
        maximum_objects: usize,
    ) -> Result<Self, Diagnostic> {
        if maximum_bytes > MAXIMUM_CONTAINER_BYTES
            || maximum_packages > super::MAXIMUM_PACKAGE_CLOSURE
            || maximum_objects > MAXIMUM_CONTAINER_OBJECTS
        {
            return Err(limit("non-overridable decoder ceilings"));
        }
        if bytes.len() > maximum_bytes {
            return Err(limit("container bytes"));
        }
        let mut cursor = Cursor(bytes);
        if cursor.take(8)? != CONTAINER_MAGIC {
            return Err(corrupt(
                "package_container_contract",
                "expected a code-complete package container; predecessor bare packs and executable artifacts cannot be staged",
            ));
        }
        let root = cursor.binding()?;
        if root.transport != expected {
            return Err(corrupt(
                "package_container_root",
                format!(
                    "container does not bind requested transport {expected}; export and restage its exact source"
                ),
            ));
        }
        let selection_count = u32::from_be_bytes(cursor.array()?) as usize;
        let object_count = usize::try_from(u64::from_be_bytes(cursor.array()?))
            .map_err(|_| limit("distinct objects"))?;
        admit_container_counts(bytes.len(), selection_count, object_count)?;
        if selection_count > maximum_packages || object_count > maximum_objects {
            return Err(limit("package or object inventory"));
        }
        let minimum = selection_count
            .checked_mul(64)
            .and_then(|count| object_count.checked_mul(41)?.checked_add(count))
            .ok_or_else(|| limit("container lengths"))?;
        if minimum > cursor.0.len() {
            return Err(corrupt(
                "package_container_truncated",
                "container inventory is truncated before allocation",
            ));
        }
        let mut selections = Vec::with_capacity(selection_count);
        for _ in 0..selection_count {
            selections.push(cursor.binding()?);
        }
        let mut objects = BTreeMap::new();
        let mut previous = None;
        for _ in 0..object_count {
            let tag = cursor.take(1)?[0];
            let domain = ObjectDomain::from_tag(tag).map_err(store_diagnostic)?;
            if !source_domain(domain) {
                return Err(corrupt(
                    "package_container_object_domain",
                    "container contains non-source material",
                ));
            }
            let digest = cursor.array()?;
            let order = (tag, digest);
            if previous.is_some_and(|previous| previous >= order) {
                return Err(corrupt(
                    "package_container_object_order",
                    "object keys must be unique and strictly ordered by wire tag and digest",
                ));
            }
            previous = Some(order);
            let length = usize::try_from(u64::from_be_bytes(cursor.array()?))
                .map_err(|_| limit("object bytes"))?;
            if length > domain.maximum_bytes() {
                return Err(limit("per-object bytes"));
            }
            let object = cursor.take(length)?;
            let key = ObjectKey::from_digest(domain, digest);
            key.verify(object).map_err(store_diagnostic)?;
            objects.insert(key, object.to_vec());
        }
        if !cursor.0.is_empty() {
            return Err(corrupt(
                "package_container_trailing",
                "container has trailing material",
            ));
        }
        let container = Self {
            root,
            selections,
            objects,
        };
        container.validate_inventory()?;
        Ok(container)
    }

    fn validate_inventory(&self) -> Result<(), Diagnostic> {
        admit_container_counts(0, self.selections.len(), self.objects.len())?;
        if self
            .selections
            .windows(2)
            .any(|pair| pair[0].package_revision >= pair[1].package_revision)
            || !self.selections.contains(&self.root)
        {
            return Err(corrupt(
                "package_container_selection",
                "container requires one unique ordered exact selection including its root",
            ));
        }
        if self.objects.keys().any(|key| !source_domain(key.domain)) {
            return Err(corrupt(
                "package_container_object_domain",
                "container contains non-source material",
            ));
        }
        Ok(())
    }

    pub fn admit(&self) -> Result<AdmittedClosure, Diagnostic> {
        self.admit_with_budget(MAXIMUM_VALIDATION_VISITS, MAXIMUM_VALIDATION_READ_BYTES)
    }

    pub(crate) fn admit_with_budget(
        &self,
        maximum_visits: u64,
        maximum_read_bytes: u64,
    ) -> Result<AdmittedClosure, Diagnostic> {
        if maximum_visits > MAXIMUM_VALIDATION_VISITS
            || maximum_read_bytes > MAXIMUM_VALIDATION_READ_BYTES
        {
            return Err(limit("non-overridable admission ceilings"));
        }
        // Include the decoder's one complete container read and object-integrity visits.
        let input_bytes = self.encoded_size()? as u64;
        let input_visits = self.objects.len() as u64;
        let visits = maximum_visits
            .checked_sub(input_visits)
            .ok_or_else(|| limit("validation visits"))?;
        let reads = maximum_read_bytes
            .checked_sub(input_bytes)
            .ok_or_else(|| limit("validation-read bytes"))?;
        let mut admitted =
            collect_with_budget(&self.objects, self.root, &self.selections, visits, reads)?;
        if admitted.container.objects != self.objects {
            return Err(corrupt(
                "package_container_completeness",
                "container contains missing or extra canonical objects; export the complete current closure and restage",
            ));
        }
        admitted.validation_visits += input_visits;
        admitted.validation_read_bytes += input_bytes;
        Ok(admitted)
    }
}

struct Cursor<'a>(&'a [u8]);
impl<'a> Cursor<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], Diagnostic> {
        if length > self.0.len() {
            return Err(corrupt(
                "package_container_truncated",
                "package container is truncated",
            ));
        }
        let (value, remaining) = self.0.split_at(length);
        self.0 = remaining;
        Ok(value)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], Diagnostic> {
        self.take(N)?.try_into().map_err(|_| {
            corrupt(
                "package_container_truncated",
                "fixed package field is truncated",
            )
        })
    }
    fn binding(&mut self) -> Result<PackageTransportBinding, Diagnostic> {
        Ok(PackageTransportBinding {
            package_revision: PackageRevisionDigest::from_bytes(self.array()?),
            transport: PackageTransportDigest::from_bytes(self.array()?),
        })
    }
}

// A source store has no stage path. The collector charges all reads and retains only visited keys.
impl ImmutableObjectStore for BTreeMap<ObjectKey, Vec<u8>> {
    fn read(
        &self,
        key: ObjectKey,
        maximum: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.read_admitted(key, maximum, &mut StoreReadAdmission::unbounded(), work)
    }
    fn read_admitted(
        &self,
        key: ObjectKey,
        maximum: usize,
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        admission.admit_catalog_lookup()?;
        work.catalog_lookups += 1;
        let Some(bytes) = self.get(&key) else {
            return Ok(None);
        };
        if bytes.len() > maximum {
            return Err(StoreError::new(
                StoreErrorClass::Resource,
                "package_source_object_bytes",
                "source object exceeds its read bound",
            ));
        }
        admission.admit_object(bytes.len())?;
        key.verify(bytes)?;
        work.objects_read += 1;
        work.bytes_read += bytes.len() as u64;
        Ok(Some(bytes.clone()))
    }
    fn contains(&self, key: ObjectKey, work: &mut StoreWork) -> Result<bool, StoreError> {
        work.catalog_lookups += 1;
        Ok(self.contains_key(&key))
    }
    fn stage(
        &mut self,
        _key: ObjectKey,
        _bytes: &[u8],
        _work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        Err(StoreError::new(
            StoreErrorClass::Input,
            "package_source_read_only",
            "immutable package source cannot be edited",
        ))
    }
}

pub(crate) struct CollectingStore<'a, S: ?Sized> {
    base: &'a S,
    admission: RefCell<StoreReadAdmission>,
    objects: RefCell<BTreeMap<ObjectKey, Vec<u8>>>,
    retained_bytes: std::cell::Cell<usize>,
    maximum_visits: u64,
    semantic_visits: std::cell::Cell<u64>,
}
impl<'a, S: ?Sized> CollectingStore<'a, S> {
    pub(crate) fn new(base: &'a S) -> Self {
        Self {
            base,
            admission: RefCell::new(StoreReadAdmission::new(StoreReadLimits {
                maximum_catalog_lookups: MAXIMUM_VALIDATION_VISITS,
                maximum_objects: MAXIMUM_VALIDATION_VISITS,
                maximum_bytes: MAXIMUM_VALIDATION_READ_BYTES,
            })),
            objects: RefCell::new(BTreeMap::new()),
            retained_bytes: std::cell::Cell::new(84),
            maximum_visits: MAXIMUM_VALIDATION_VISITS,
            semantic_visits: std::cell::Cell::new(0),
        }
    }
    pub(crate) fn visits(&self) -> u64 {
        self.maximum_visits - self.admission.borrow().remaining().maximum_objects
            + self.semantic_visits.get()
    }
    pub(crate) fn read_bytes(&self) -> u64 {
        MAXIMUM_VALIDATION_READ_BYTES - self.admission.borrow().remaining().maximum_bytes
    }
    fn charge_visits(&self, count: u64) -> Result<(), Diagnostic> {
        if self
            .visits()
            .checked_add(count)
            .is_none_or(|total| total > self.maximum_visits)
        {
            return Err(limit("aggregate validation visits"));
        }
        self.semantic_visits.set(self.semantic_visits.get() + count);
        Ok(())
    }
}
impl<S: ImmutableObjectStore + ?Sized> ImmutableObjectStore for CollectingStore<'_, S> {
    fn read(
        &self,
        key: ObjectKey,
        maximum: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.read_admitted(key, maximum, &mut StoreReadAdmission::unbounded(), work)
    }
    fn read_admitted(
        &self,
        key: ObjectKey,
        maximum: usize,
        external: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        if !self.objects.borrow().contains_key(&key)
            && self.objects.borrow().len() == MAXIMUM_CONTAINER_OBJECTS
        {
            return Err(StoreError::new(
                StoreErrorClass::Resource,
                "package_source_objects",
                "source admission exhausted distinct objects",
            ));
        }
        let maximum = if self.objects.borrow().contains_key(&key) {
            maximum
        } else {
            maximum.min(
                MAXIMUM_CONTAINER_BYTES
                    .saturating_sub(self.retained_bytes.get())
                    .saturating_sub(41),
            )
        };
        let internal = self.admission.borrow().remaining();
        let outer = external.remaining();
        let combined = StoreReadLimits {
            maximum_catalog_lookups: internal
                .maximum_catalog_lookups
                .min(outer.maximum_catalog_lookups),
            maximum_objects: internal
                .maximum_objects
                .saturating_sub(self.semantic_visits.get())
                .min(outer.maximum_objects),
            maximum_bytes: internal.maximum_bytes.min(outer.maximum_bytes),
        };
        let mut admission = StoreReadAdmission::new(combined);
        let result = self.base.read_admitted(key, maximum, &mut admission, work);
        let remaining = admission.remaining();
        let charged = |budget: StoreReadLimits| {
            StoreReadAdmission::new(StoreReadLimits {
                maximum_catalog_lookups: budget.maximum_catalog_lookups
                    - (combined.maximum_catalog_lookups - remaining.maximum_catalog_lookups),
                maximum_objects: budget.maximum_objects
                    - (combined.maximum_objects - remaining.maximum_objects),
                maximum_bytes: budget.maximum_bytes
                    - (combined.maximum_bytes - remaining.maximum_bytes),
            })
        };
        *self.admission.borrow_mut() = charged(internal);
        *external = charged(outer);
        let bytes = result?;
        if let Some(bytes) = &bytes
            && !self.objects.borrow().contains_key(&key)
        {
            let total = self
                .retained_bytes
                .get()
                .checked_add(41)
                .and_then(|total| total.checked_add(bytes.len()))
                .filter(|total| *total <= MAXIMUM_CONTAINER_BYTES)
                .ok_or_else(|| {
                    StoreError::new(
                        StoreErrorClass::Resource,
                        "package_source_container_bytes",
                        "source admission exhausted container bytes before retaining an object",
                    )
                })?;
            self.retained_bytes.set(total);
            self.objects.borrow_mut().insert(key, bytes.clone());
        }
        Ok(bytes)
    }
    fn contains(&self, key: ObjectKey, work: &mut StoreWork) -> Result<bool, StoreError> {
        Ok(self.read(key, key.domain.maximum_bytes(), work)?.is_some())
    }
    fn stage(
        &mut self,
        _key: ObjectKey,
        _bytes: &[u8],
        _work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        Err(StoreError::new(
            StoreErrorClass::Input,
            "package_source_read_only",
            "package admission cannot write canonical source",
        ))
    }
}

pub fn collect<S: ImmutableObjectStore + ?Sized>(
    base: &S,
    root: PackageTransportBinding,
    selections: &[PackageTransportBinding],
) -> Result<AdmittedClosure, Diagnostic> {
    collect_with_budget(
        base,
        root,
        selections,
        MAXIMUM_VALIDATION_VISITS,
        MAXIMUM_VALIDATION_READ_BYTES,
    )
}

pub(crate) fn collect_with_budget<S: ImmutableObjectStore + ?Sized>(
    base: &S,
    root: PackageTransportBinding,
    selections: &[PackageTransportBinding],
    maximum_visits: u64,
    maximum_read_bytes: u64,
) -> Result<AdmittedClosure, Diagnostic> {
    collect_admitted(base, root, selections, maximum_visits, maximum_read_bytes).map_err(|error| {
        Diagnostic::new(
            error.class,
            error.code,
            format!(
                "exact source {} ({}): {}; export, restage, and replan this exact closure",
                root.package_revision, root.transport, error.message
            ),
        )
    })
}

fn collect_admitted<S: ImmutableObjectStore + ?Sized>(
    base: &S,
    root: PackageTransportBinding,
    selections: &[PackageTransportBinding],
    maximum_visits: u64,
    maximum_read_bytes: u64,
) -> Result<AdmittedClosure, Diagnostic> {
    if maximum_visits > MAXIMUM_VALIDATION_VISITS
        || maximum_read_bytes > MAXIMUM_VALIDATION_READ_BYTES
    {
        return Err(limit("non-overridable admission ceilings"));
    }
    let store = CollectingStore {
        base,
        admission: RefCell::new(StoreReadAdmission::new(StoreReadLimits {
            maximum_catalog_lookups: maximum_visits,
            maximum_objects: maximum_visits,
            maximum_bytes: maximum_read_bytes,
        })),
        objects: RefCell::new(BTreeMap::new()),
        retained_bytes: std::cell::Cell::new(84 + selections.len() * 64),
        maximum_visits,
        semantic_visits: std::cell::Cell::new(0),
    };
    let mut work = StoreWork::default();
    let validated = validate_package_transport_closure_metered(
        &store,
        root.package_revision,
        selections,
        None,
        &mut work,
        &mut crate::platform::persistent_map::MapAdmission::unbounded(),
        &mut |count| store.charge_visits(count),
    )?;
    if validated.root_transport_digest != root.transport {
        return Err(corrupt(
            "package_container_root",
            "container root transport disagrees with its exact selection",
        ));
    }
    let mut snapshots = BTreeMap::new();
    let mut revisions = BTreeMap::new();
    let mut interfaces = BTreeMap::new();
    let mut transports = BTreeMap::new();
    for binding in selections {
        let revision = read_revision(&store, binding.package_revision, &mut work)?;
        let (transport, interface) = super::validate_package_transport_local_metered(
            &store,
            *binding,
            &revision,
            &mut work,
            &mut crate::platform::persistent_map::MapAdmission::unbounded(),
            &mut |count| store.charge_visits(count),
        )?;
        snapshots.insert(
            binding.package_revision,
            reconstruct(&store, &transport, &revision, &mut work)?,
        );
        interfaces.insert(binding.package_revision, interface);
        revisions.insert(binding.package_revision, revision);
        transports.insert(binding.package_revision, transport);
    }
    let mut packages = BTreeMap::new();
    // Every canonical read charges a decoding/validation visit, including repeated interface
    // validation and map traversal. Full type/expression/relation work shares the same remainder.
    for binding in selections {
        let mut snapshot = snapshots
            .remove(&binding.package_revision)
            .ok_or_else(|| corrupt("package_source_snapshot", "source snapshot disappeared"))?;
        for dependency in snapshot.dependencies.values() {
            let interface = interfaces
                .get(&dependency.package_revision)
                .ok_or_else(|| {
                    corrupt(
                        "package_source_dependency",
                        "direct dependency interface is unavailable",
                    )
                })?;
            snapshot.dependency_interfaces.insert(
                dependency.package_revision,
                interface
                    .owners
                    .iter()
                    .map(|(key, owner)| (*key, owner.record.clone()))
                    .collect(),
            );
            snapshot
                .dependency_types
                .extend(interface.type_objects.clone());
        }
        let projection_visits = snapshot
            .owners
            .len()
            .checked_add(snapshot.types.len())
            .and_then(|total| total.checked_add(snapshot.dependency_types.len()))
            .ok_or_else(|| limit("validation visits"))? as u64;
        store.charge_visits(projection_visits)?;
        let mut intrinsic_work = 0;
        for record in snapshot.owners.values() {
            if let OwnerRecord::Declaration(DeclarationRecord {
                payload: DeclarationPayload::External(external),
                ..
            }) = record
            {
                crate::platform::intrinsic_contract::validate_kernel_intrinsic(
                    &snapshot,
                    external,
                    &mut intrinsic_work,
                    (maximum_visits - store.visits()) as usize,
                )?;
            }
        }
        store.charge_visits(intrinsic_work as u64)?;
        let remaining = usize::try_from(maximum_visits - store.visits())
            .map_err(|_| limit("validation visits"))?;
        let witness = rebuild_full_witness_with_limit(&snapshot, remaining).map_err(|errors| {
            let first = errors.into_iter().next().unwrap_or_else(|| {
                corrupt("package_source_validation", "full source validation failed")
            });
            Diagnostic::new(
                first.class,
                first.code,
                format!(
                    "package {} at {}: {}; export valid source and restage",
                    snapshot.root.package_id, binding.package_revision, first.message
                ),
            )
        })?;
        store.charge_visits(witness.report.full_validation.work_consumed)?;
        let (interface_owners, interface_types) = project(&snapshot, &witness)?;
        let rebuilt = build_package_interface(&interface_owners, &interface_types)?;
        let revision = revisions
            .remove(&binding.package_revision)
            .ok_or_else(|| corrupt("package_source_revision", "source revision disappeared"))?;
        if package_interface_digest(revision.package, rebuilt.root.content_root())?
            != revision.interface
            || interfaces
                .get(&binding.package_revision)
                .is_none_or(|interface| interface.owners != interface_owners)
        {
            return Err(corrupt(
                "package_source_interface",
                format!(
                    "package {} at {} has inconsistent interface and body; export and restage",
                    revision.package, binding.package_revision
                ),
            ));
        }
        let transport = transports
            .remove(&binding.package_revision)
            .ok_or_else(|| corrupt("package_source_transport", "source transport disappeared"))?;
        packages.insert(
            binding.package_revision,
            ImmutablePackage {
                binding: *binding,
                revision,
                transport,
                snapshot,
                witness,
                interface_owners,
                interface_types,
            },
        );
    }
    let mut dependency_order = Vec::new();
    let mut counts = BTreeMap::new();
    let mut parents = BTreeMap::<PackageRevisionDigest, BTreeSet<PackageRevisionDigest>>::new();
    let mut ready = BTreeSet::new();
    for (digest, package) in &packages {
        counts.insert(*digest, package.revision.dependencies.len());
        if package.revision.dependencies.is_empty() {
            ready.insert(*digest);
        }
        for dependency in &package.revision.dependencies {
            parents
                .entry(dependency.package_revision)
                .or_default()
                .insert(*digest);
        }
    }
    while let Some(next) = ready.pop_first() {
        dependency_order.push(next);
        if let Some(parents) = parents.get(&next) {
            for parent in parents {
                let count = counts.get_mut(parent).ok_or_else(|| {
                    corrupt("package_source_edges", "dependency order lost its parent")
                })?;
                *count = count.checked_sub(1).ok_or_else(|| {
                    corrupt("package_source_edges", "duplicate dependency order edge")
                })?;
                if *count == 0 {
                    ready.insert(*parent);
                }
            }
        }
    }
    if dependency_order.len() != packages.len() {
        return Err(corrupt(
            "package_source_cycle",
            "source dependency closure is cyclic",
        ));
    }
    let validation_visits = store.visits();
    Ok(AdmittedClosure {
        validation_read_bytes: maximum_read_bytes
            - store.admission.borrow().remaining().maximum_bytes,
        container: PackageContainer {
            root,
            selections: selections.to_vec(),
            objects: store.objects.into_inner(),
        },
        packages,
        dependency_order,
        validation_visits,
        dependency_edges: validated.dependency_edges,
    })
}

pub(crate) fn required<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    domain: ObjectDomain,
    digest: [u8; 32],
    work: &mut StoreWork,
) -> Result<Vec<u8>, Diagnostic> {
    store.read(ObjectKey::from_digest(domain, digest), domain.maximum_bytes(), work).map_err(store_diagnostic)?
        .ok_or_else(|| corrupt("package_source_missing", format!("canonical {} object {} is missing; export the exact complete source and restage", domain.name(), crate::platform::semantic_id::encode_hex(&digest))))
}

pub(crate) fn entries<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    root: MapRoot,
) -> Result<EncodedEntries, Diagnostic> {
    if root.entries() > MAXIMUM_CONTAINER_OBJECTS as u64 {
        return Err(limit("map entries"));
    }
    let reader = ObjectPageReader::new(store);
    let mut entries = Vec::new();
    PersistentMap::from_root(root)
        .for_each(&reader, &mut MapWork::default(), |key, value| {
            entries.push((key.to_vec(), value.to_vec()));
            Ok(())
        })
        .map_err(|error| corrupt("package_source_map", error.message))?;
    if entries.len() as u64 != root.entries() {
        return Err(corrupt(
            "package_source_map_count",
            "source map count disagrees with reconstructed inventory",
        ));
    }
    Ok(entries)
}

fn reconstruct<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    transport: &PackageTransport,
    revision: &PackageRevision,
    work: &mut StoreWork,
) -> Result<KernelSnapshot, Diagnostic> {
    let root = decode_root(
        &required(
            store,
            ObjectDomain::SemanticRoot,
            transport.semantic_root.bytes(),
            work,
        )?,
        transport.semantic_root,
    )?;
    let mut snapshot = KernelSnapshot {
        root,
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    };
    for (key, value) in entries(store, snapshot.root.owners)? {
        let owner = EncodedOwnerKey::decode(&key)?;
        let binding = decode_owner_binding(&value, owner)?;
        let bytes = required(store, ObjectDomain::Owner, binding.object.bytes(), work)?;
        snapshot.owners.insert(
            owner,
            decode_owner(&bytes, owner, binding.kind, binding.object)?,
        );
    }
    for (key, value) in entries(store, snapshot.root.dependencies)? {
        let package = key
            .as_slice()
            .try_into()
            .ok()
            .and_then(PackageId::from_bytes)
            .ok_or_else(|| {
                corrupt(
                    "package_source_dependency_key",
                    "invalid canonical package key",
                )
            })?;
        let binding = decode_dependency_binding(&value)?;
        let bytes = required(
            store,
            ObjectDomain::Dependency,
            binding.object.bytes(),
            work,
        )?;
        snapshot.dependencies.insert(
            package,
            decode_dependency(&bytes, &package, binding.object)?,
        );
    }
    if snapshot.dependencies.values().cloned().collect::<Vec<_>>() != revision.dependencies {
        return Err(corrupt(
            "package_source_edges",
            "canonical dependency inventory disagrees with the logical package revision",
        ));
    }
    for (key, value) in entries(store, snapshot.root.retirements)? {
        let owner = EncodedOwnerKey::decode(&key)?;
        let binding = decode_retirement_binding(&value)?;
        let bytes = required(
            store,
            ObjectDomain::Retirement,
            binding.object.bytes(),
            work,
        )?;
        snapshot
            .retirements
            .insert(owner, decode_retirement(&bytes, owner, binding.object)?);
    }
    let mut pending = snapshot
        .owners
        .values()
        .flat_map(OwnerRecord::type_roots)
        .collect::<BTreeSet<_>>();
    while let Some(digest) = pending.pop_first() {
        if snapshot.types.contains_key(&digest) {
            continue;
        }
        let bytes = required(store, ObjectDomain::Type, digest.bytes(), work)?;
        let ty = decode_type_object(&bytes, digest)?;
        pending.extend(ty.child_types());
        snapshot.types.insert(digest, ty);
    }
    for (digest, length) in snapshot.owners.values().flat_map(OwnerRecord::blob_roots) {
        if snapshot
            .blobs
            .get(&digest)
            .is_some_and(|existing| *existing != length)
        {
            return Err(corrupt(
                "package_source_blob",
                "canonical blob has conflicting length bindings",
            ));
        }
        if snapshot.blobs.contains_key(&digest) {
            continue;
        }
        let bytes = required(store, ObjectDomain::Blob, digest.bytes(), work)?;
        if bytes.len() as u64 != length {
            return Err(corrupt(
                "package_source_blob",
                "canonical blob length disagrees with its owner",
            ));
        }
        snapshot.blobs.insert(digest, length);
    }
    if semantic_state_digest(&snapshot)? != revision.revision.semantic_state {
        return Err(corrupt(
            "package_source_state",
            "reconstructed canonical inventory disagrees with the logical semantic state",
        ));
    }
    Ok(snapshot)
}

fn project(
    snapshot: &KernelSnapshot,
    witness: &FullWitness,
) -> Result<InterfaceProjection, Diagnostic> {
    let selection =
        PackageInterfaceSelection::from_records(snapshot.root.package_id, &snapshot.owners)?;
    let mut owners = BTreeMap::new();
    for owner in selection.owners() {
        let canonical = snapshot
            .owners
            .get(&owner)
            .ok_or_else(|| corrupt("package_source_public_owner", "public owner is absent"))?;
        let summary = witness
            .summaries
            .get(&owner)
            .ok_or_else(|| corrupt("package_source_summary", "rebuilt owner summary is absent"))?;
        let value =
            PackageInterfaceOwner::project(canonical, summary, &selection)?.ok_or_else(|| {
                corrupt(
                    "package_source_public_owner",
                    "selected public owner cannot be projected",
                )
            })?;
        owners.insert(owner, value);
    }
    let mut pending = owners
        .values()
        .flat_map(PackageInterfaceOwner::type_roots)
        .collect::<BTreeSet<_>>();
    let mut types = BTreeMap::new();
    while let Some(digest) = pending.pop_first() {
        if types.contains_key(&digest) {
            continue;
        }
        let object = snapshot
            .types
            .get(&digest)
            .or_else(|| snapshot.dependency_types.get(&digest))
            .ok_or_else(|| {
                corrupt(
                    "package_source_public_type",
                    "public type is absent from canonical source",
                )
            })?;
        pending.extend(object.child_types());
        types.insert(digest, encode_type_object(object)?.1);
    }
    Ok((owners, types))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::publication::GraphRepository;
    fn standard_source() -> AdmittedClosure {
        let repository = GraphRepository::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard"),
        )
        .expect("maintained source");
        repository
            .export_package_container()
            .expect("full canonical source admission")
    }

    fn assert_not_ready(bytes: &[u8], transport: PackageTransportDigest) {
        let temporary = tempfile::tempdir().unwrap();
        let target = GraphRepository::create(
            &temporary.path().join("consumer"),
            &crate::platform::kernel::tests::transport_snapshot(),
            None,
        )
        .unwrap();
        let standard = standard_source();
        target
            .repository
            .stage_package_transport(
                standard.container.root.transport,
                &standard.container.encode().unwrap(),
            )
            .unwrap();
        let ready = target.repository.root().join("PACKAGE-TRANSPORTS/CURRENT");
        let before_ready = std::fs::read(&ready).unwrap();
        let before = target
            .repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        let pack_inventory = || {
            std::fs::read_dir(target.repository.root().join("packs"))
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect::<BTreeSet<_>>()
        };
        let before_packs = pack_inventory();
        assert!(
            target
                .repository
                .stage_package_transport(transport, bytes)
                .is_err()
        );
        assert_eq!(
            target.repository.current().unwrap().head,
            target.current.head
        );
        assert_eq!(std::fs::read(&ready).unwrap(), before_ready);
        let after = target
            .repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        assert_eq!(after.owners, before.owners);
        assert_eq!(after.types, before.types);
        assert_eq!(after.dependencies, before.dependencies);
        assert_eq!(after.retirements, before.retirements);
        assert_eq!(pack_inventory(), before_packs);
        assert!(
            target
                .repository
                .object_store()
                .unwrap()
                .staging_leftovers()
                .is_empty()
        );
        assert_eq!(
            std::fs::read_dir(ready.parent().unwrap()).unwrap().count(),
            1
        );
    }

    #[test]
    fn complete_source_reconstructs_maintained_inventory_and_strict_container() {
        let admitted = standard_source();
        let package = &admitted.packages[&admitted.container.root.package_revision];
        let repository = GraphRepository::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard"),
        )
        .expect("maintained source");
        let independent = repository
            .view_current()
            .expect("view")
            .reconstruct_full_oracle()
            .expect("independent complete reconstruction")
            .value;
        assert_eq!(package.snapshot.owners, independent.owners);
        assert_eq!(package.snapshot.types, independent.types);
        assert_eq!(package.snapshot.retirements, independent.retirements);
        assert_eq!(package.snapshot.dependencies, independent.dependencies);
        let bytes = admitted.container.encode().expect("encode");
        let decoded =
            PackageContainer::decode(&bytes, admitted.container.root.transport).expect("decode");
        assert_eq!(decoded.encode().expect("canonical encode"), bytes);
        let repeated = decoded.admit().expect("repeat full admission");
        assert_eq!(admitted.validation_visits, repeated.validation_visits);
        assert_eq!(
            admitted.validation_read_bytes,
            repeated.validation_read_bytes
        );
        assert!(bytes.len() < MAXIMUM_CONTAINER_BYTES);
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_not_ready(&trailing, admitted.container.root.transport);
        assert_eq!(
            PackageContainer::decode(&trailing, admitted.container.root.transport)
                .expect_err("trailing")
                .code,
            "package_container_trailing"
        );
        for length in [0, 7, 8, 71, 83, bytes.len() - 1] {
            assert!(
                PackageContainer::decode(&bytes[..length], admitted.container.root.transport)
                    .is_err()
            );
            assert_not_ready(&bytes[..length], admitted.container.root.transport);
        }
    }

    #[test]
    fn source_cannot_omit_private_meaning_or_smuggle_unreachable_objects() {
        let admitted = standard_source();
        let package = &admitted.packages[&admitted.container.root.package_revision];
        let owner = package
            .snapshot
            .owners
            .values()
            .find(|record| matches!(record, OwnerRecord::Expression(_)))
            .expect("private body");
        let mut omitted = admitted.container.clone();
        omitted.objects.remove(&ObjectKey::from_digest(
            ObjectDomain::Owner,
            encode_owner(owner).expect("owner").0.bytes(),
        ));
        assert_eq!(
            omitted.admit().expect_err("body omission").code,
            "package_source_missing"
        );
        assert_not_ready(&omitted.encode().unwrap(), omitted.root.transport);
        let mut extra = admitted.container.clone();
        let bytes = b"unreachable blob".to_vec();
        extra
            .objects
            .insert(ObjectKey::for_bytes(ObjectDomain::Blob, &bytes), bytes);
        assert_eq!(
            extra.admit().expect_err("extra object").code,
            "package_container_completeness"
        );
        assert_not_ready(&extra.encode().unwrap(), extra.root.transport);
        let errors = crate::platform::kernel::validate_full_with_limit(&package.snapshot, 1)
            .expect_err("bounded validation");
        assert!(errors.iter().any(|error| error.code == "kernel_full_work"));
    }

    #[test]
    fn aggregate_validation_exact_fit_and_one_over_and_independent_inventory() {
        let source = standard_source();
        let exact = source
            .container
            .admit_with_budget(source.validation_visits, source.validation_read_bytes)
            .unwrap();
        assert_eq!(exact.validation_visits, source.validation_visits);
        assert_eq!(exact.validation_read_bytes, source.validation_read_bytes);
        assert!(
            source
                .container
                .admit_with_budget(source.validation_visits - 1, source.validation_read_bytes)
                .is_err()
        );
        assert!(
            source
                .container
                .admit_with_budget(source.validation_visits, source.validation_read_bytes - 1)
                .is_err()
        );
        let oracle =
            crate::platform::package_transport::oracle::reconstruct(&source.container).unwrap();
        let package = &source.packages[&source.container.root.package_revision];
        assert_eq!(
            oracle.snapshots[&package.revision.package].owners,
            package.snapshot.owners
        );
        let mut missing = source.container.clone();
        let private = package
            .snapshot
            .owners
            .values()
            .find(|record| matches!(record, OwnerRecord::Expression(_)))
            .unwrap();
        missing.objects.remove(&ObjectKey::from_digest(
            ObjectDomain::Owner,
            encode_owner(private).unwrap().0.bytes(),
        ));
        assert!(crate::platform::package_transport::oracle::reconstruct(&missing).is_err());
        let mut forged = source.container.clone();
        forged.root.transport = PackageTransportDigest::from_bytes([0xff; 32]);
        assert!(crate::platform::package_transport::oracle::reconstruct(&forged).is_err());
    }

    #[test]
    fn omitted_transitive_selection_or_source_rejects_even_when_dependency_is_already_ready() {
        let source = GraphRepository::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal"),
        )
        .unwrap()
        .export_package_container()
        .unwrap();
        assert_eq!(source.packages.len(), 2);
        let dependency = source
            .container
            .selections
            .iter()
            .find(|selection| **selection != source.container.root)
            .copied()
            .unwrap();
        let mut omitted_edge = source.container.clone();
        omitted_edge
            .selections
            .retain(|selection| *selection != dependency);
        assert!(omitted_edge.admit().is_err());
        assert!(crate::platform::package_transport::oracle::reconstruct(&omitted_edge).is_err());
        assert_not_ready(&omitted_edge.encode().unwrap(), omitted_edge.root.transport);
        let mut omitted_source = source.container.clone();
        omitted_source.objects.remove(&ObjectKey::from_digest(
            ObjectDomain::PackageRevision,
            dependency.package_revision.bytes(),
        ));
        assert!(omitted_source.admit().is_err());
        assert!(crate::platform::package_transport::oracle::reconstruct(&omitted_source).is_err());
        assert_not_ready(
            &omitted_source.encode().unwrap(),
            omitted_source.root.transport,
        );
    }

    #[test]
    fn readiness_ceiling_and_predecessor_rejection_are_exact() {
        let mut ready = PackageReadiness::default();
        for index in 0..super::super::MAXIMUM_PACKAGE_TRANSPORT_CANDIDATES {
            let mut digest = [0_u8; 32];
            digest[..8].copy_from_slice(&(index as u64).to_be_bytes());
            ready.bindings.insert(
                PackageRevisionDigest::from_bytes(digest),
                PackageTransportDigest::from_bytes([1; 32]),
            );
        }
        let bytes = ready.encode().unwrap();
        assert_eq!(bytes.len(), MAXIMUM_READINESS_BYTES);
        assert_eq!(PackageReadiness::decode(&bytes).unwrap(), ready);
        ready.bindings.insert(
            PackageRevisionDigest::from_bytes([0xff; 32]),
            PackageTransportDigest::from_bytes([1; 32]),
        );
        assert!(ready.encode().is_err());
        assert!(PackageReadiness::decode(b"LKJPTS01").is_err());
        let source = standard_source();
        for predecessor in [b"LKJPACK1".as_slice(), b"LKJART01".as_slice()] {
            assert_not_ready(predecessor, source.container.root.transport);
            assert_eq!(
                PackageContainer::decode(predecessor, source.container.root.transport)
                    .unwrap_err()
                    .code,
                "package_container_contract"
            );
        }
        let mut bytes = source.container.encode().unwrap();
        bytes[72..76].copy_from_slice(&10_001_u32.to_be_bytes());
        assert_not_ready(&bytes, source.container.root.transport);
        assert_eq!(
            PackageContainer::decode(&bytes, source.container.root.transport)
                .unwrap_err()
                .code,
            "package_source_budget"
        );
        let mut bytes = source.container.encode().unwrap();
        bytes[76..84].copy_from_slice(&1_000_001_u64.to_be_bytes());
        assert_not_ready(&bytes, source.container.root.transport);
        assert_eq!(
            PackageContainer::decode(&bytes, source.container.root.transport)
                .unwrap_err()
                .code,
            "package_source_budget"
        );
    }

    #[test]
    fn decoder_header_ceiling_guards_are_exact_and_duplicate_keys_reject() {
        // Exercise the pre-allocation dimension guards at their actual integer ceilings. This is
        // not a maximum-capacity graph execution: complete source exact-fit work is tested with
        // the bounded semantic/read admission fixture above.
        admit_container_counts(268_435_456, 10_000, 1_000_000).unwrap();
        for counts in [
            (268_435_457, 10_000, 1_000_000),
            (268_435_456, 10_001, 1_000_000),
            (268_435_456, 10_000, 1_000_001),
        ] {
            assert_eq!(
                admit_container_counts(counts.0, counts.1, counts.2)
                    .unwrap_err()
                    .class,
                DiagnosticClass::Resource
            );
        }
        let source = standard_source();
        let exact_bytes = source.container.encode().unwrap();
        let counts = (
            exact_bytes.len(),
            source.container.selections.len(),
            source.container.objects.len(),
        );
        let exact = PackageContainer::decode_bounded(
            &exact_bytes,
            source.container.root.transport,
            counts.0,
            counts.1,
            counts.2,
        )
        .unwrap();
        assert!(exact.admit().is_ok());
        for bounds in [
            (counts.0 - 1, counts.1, counts.2),
            (counts.0, counts.1 - 1, counts.2),
            (counts.0, counts.1, counts.2 - 1),
        ] {
            assert_eq!(
                PackageContainer::decode_bounded(
                    &exact_bytes,
                    source.container.root.transport,
                    bounds.0,
                    bounds.1,
                    bounds.2,
                )
                .unwrap_err()
                .class,
                DiagnosticClass::Resource
            );
        }
        let mut bytes = source.container.encode().unwrap();
        let records = 84 + source.container.selections.len() * 64;
        let first_length =
            u64::from_be_bytes(bytes[records + 33..records + 41].try_into().unwrap()) as usize;
        let duplicate = bytes[records..records + 41 + first_length].to_vec();
        bytes.splice(records..records, duplicate);
        bytes[76..84].copy_from_slice(&(source.container.objects.len() as u64 + 1).to_be_bytes());
        assert_not_ready(&bytes, source.container.root.transport);
        assert_eq!(
            PackageContainer::decode(&bytes, source.container.root.transport)
                .unwrap_err()
                .code,
            "package_container_object_order"
        );
        let mut selections = source.container.clone();
        selections.selections.push(selections.root);
        assert_eq!(
            selections.encode().unwrap_err().code,
            "package_container_selection"
        );
    }

    /// Hostile encoder, not an authoring path: coherently rehash an unvalidated private owner
    /// and all enclosing source identities while retaining the same committed public interface.
    fn rehash_owner(original: &AdmittedClosure, replacement: OwnerRecord) -> PackageContainer {
        use crate::platform::persistent_map::MemoryPageStore;
        let package = &original.packages[&original.container.root.package_revision];
        assert!(package.revision.dependencies.is_empty());
        let mut objects = original.container.objects.clone();
        objects.retain(|key, _| {
            !matches!(
                key.domain,
                ObjectDomain::MapPage
                    | ObjectDomain::SemanticRoot
                    | ObjectDomain::PackageRevision
                    | ObjectDomain::PackageTransport
            )
        });
        let (old_digest, _) = encode_owner(&package.snapshot.owners[&replacement.owner()]).unwrap();
        objects.remove(&ObjectKey::from_digest(
            ObjectDomain::Owner,
            old_digest.bytes(),
        ));
        let (digest, bytes) = encode_owner(&replacement).unwrap();
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
            bytes,
        );
        let mut owner_entries =
            entries(&original.container.objects, package.snapshot.root.owners).unwrap();
        let key = EncodedOwnerKey::new(replacement.owner()).bytes().to_vec();
        let value = encode_owner_binding(&OwnerBinding {
            kind: replacement.kind(),
            object: digest,
        });
        owner_entries
            .iter_mut()
            .find(|(owner, _)| *owner == key)
            .unwrap()
            .1 = value;
        let mut pages = MemoryPageStore::default();
        let owners = PersistentMap::from_sorted(&mut pages, owner_entries, &mut MapWork::default())
            .unwrap()
            .root();
        let reader = ObjectPageReader::new(&original.container.objects);
        for root in [
            package.snapshot.root.dependencies,
            package.snapshot.root.retirements,
            package.transport.interface_owners,
        ] {
            PersistentMap::from_root(root)
                .copy_reachable(&reader, &mut pages, &mut MapWork::default())
                .unwrap();
        }
        for (digest, bytes) in pages.objects() {
            objects.insert(
                ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes()),
                bytes.to_vec(),
            );
        }
        let mut root = package.snapshot.root.clone();
        root.owners = owners;
        let (root_digest, root_bytes) = encode_root(&root).unwrap();
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::SemanticRoot, root_digest.bytes()),
            root_bytes,
        );
        let mut revision = package.revision.clone();
        revision.revision.semantic_state = semantic_state_digest_from_root(&root).unwrap();
        let (revision_digest, revision_bytes) = revision.encode().unwrap();
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::PackageRevision, revision_digest.bytes()),
            revision_bytes,
        );
        let mut transport = package.transport.clone();
        transport.semantic_root = root_digest;
        transport.package_revision = revision_digest;
        let (witness, digest, _) = crate::platform::witness::bind_witness_manifest(
            root.repository_id,
            root.package_id,
            root_digest,
            transport.witness.roots,
        )
        .unwrap();
        transport.witness = witness;
        transport.validation_witness = digest;
        let (transport_digest, bytes) = transport.encode().unwrap();
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::PackageTransport, transport_digest.bytes()),
            bytes,
        );
        let root = PackageTransportBinding {
            package_revision: revision_digest,
            transport: transport_digest,
        };
        PackageContainer {
            root,
            selections: vec![root],
            objects,
        }
    }

    #[test]
    fn coherently_rehashed_private_types_and_external_registration_cannot_bypass_admission() {
        let temporary = tempfile::tempdir().unwrap();
        let repository = GraphRepository::create(
            &temporary.path().join("source"),
            &crate::platform::kernel::tests::transport_snapshot(),
            None,
        )
        .unwrap();
        let original = repository.repository.export_package_container().unwrap();
        let package = &original.packages[&original.container.root.package_revision];
        let expression = package.snapshot.owners.values().find(|record| matches!(record, OwnerRecord::Expression(record) if record.operation == ExpressionOperation::Unit {})).unwrap().clone();
        let valid = rehash_owner(&original, expression.clone());
        assert!(valid.admit().is_ok());
        assert!(crate::platform::package_transport::oracle::reconstruct(&valid).is_ok());
        let OwnerRecord::Expression(mut expression) = expression else {
            unreachable!()
        };
        expression.operation = ExpressionOperation::Bool { value: true };
        let hostile = rehash_owner(&original, OwnerRecord::Expression(expression));
        let error = hostile.admit().unwrap_err();
        assert!(error.code.starts_with("kernel_"), "{error:?}");
        assert!(crate::platform::package_transport::oracle::reconstruct(&hostile).is_err());
        assert_not_ready(&hostile.encode().unwrap(), hostile.root.transport);
        let external = package
            .snapshot
            .owners
            .values()
            .find(|record| {
                matches!(
                    record,
                    OwnerRecord::Declaration(DeclarationRecord {
                        payload: DeclarationPayload::External(_),
                        ..
                    })
                )
            })
            .unwrap()
            .clone();
        for (implementation, code) in [
            ("unregistered.host", "intrinsic_unknown"),
            ("core.i64.add", "intrinsic_signature"),
        ] {
            let OwnerRecord::Declaration(mut declaration) = external.clone() else {
                unreachable!()
            };
            let DeclarationPayload::External(external) = &mut declaration.payload else {
                unreachable!()
            };
            external.implementation = ImplementationName::new(implementation).unwrap();
            let hostile = rehash_owner(&original, OwnerRecord::Declaration(declaration));
            assert_not_ready(&hostile.encode().unwrap(), hostile.root.transport);
            assert_eq!(hostile.admit().unwrap_err().code, code);
            assert_eq!(
                crate::platform::package_transport::oracle::reconstruct(&hostile)
                    .unwrap_err()
                    .code,
                code
            );
        }
    }
}
