//! Independent closure and interface inventory reconstruction from neutral canonical records.
//! Does not call transport admission, production closure resolution, interface projection, or linking.

use super::source::PackageContainer;
use super::{PackageRevision, PackageTransport};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::*;
use crate::platform::package_interface::{PackageInterfaceOwner, decode_package_interface_binding};
use crate::platform::persistent_map::{MapRoot, MapWork, PersistentMap};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StageOutcome, StoreError, StoreErrorClass,
    StoreWork,
};
use crate::platform::storage::page_store::ObjectPageReader;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

type EncodedEntries = Vec<(Vec<u8>, Vec<u8>)>;

#[derive(Clone, Debug)]
pub(crate) struct OracleClosure {
    pub snapshots: BTreeMap<PackageId, KernelSnapshot>,
    pub revisions: BTreeMap<PackageId, PackageRevision>,
    pub interfaces: BTreeMap<PackageId, BTreeMap<OwnerKey, PackageInterfaceRecord>>,
    pub objects: usize,
    pub edges: usize,
    pub validation_visits: u64,
    pub validation_read_bytes: u64,
}

fn failure(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, "package_closure_oracle", message)
}

struct Reader<'a> {
    objects: &'a BTreeMap<ObjectKey, Vec<u8>>,
    seen: RefCell<BTreeSet<ObjectKey>>,
    visits: Cell<u64>,
    bytes: Cell<u64>,
}

impl Reader<'_> {
    fn charge(&self, visits: usize) -> Result<(), Diagnostic> {
        let visits = self
            .visits
            .get()
            .checked_add(u64::try_from(visits).map_err(|_| failure("oracle visit conversion"))?)
            .filter(|visits| *visits <= 16_000_000)
            .ok_or_else(|| failure("oracle exhausted 16000000 aggregate validation visits"))?;
        self.visits.set(visits);
        Ok(())
    }
    fn object(&self, domain: ObjectDomain, digest: [u8; 32]) -> Result<Vec<u8>, Diagnostic> {
        self.read(
            ObjectKey::from_digest(domain, digest),
            domain.maximum_bytes(),
            &mut StoreWork::default(),
        )
        .map_err(|error| failure(error.message))?
        .ok_or_else(|| {
            failure(format!(
                "missing canonical {} {}",
                domain.name(),
                crate::platform::semantic_id::encode_hex(&digest)
            ))
        })
    }

    fn map(&self, root: MapRoot) -> Result<EncodedEntries, Diagnostic> {
        if root.entries() > 1_000_000 {
            return Err(failure("oracle map entry bound"));
        }
        let reader = ObjectPageReader::new(self);
        let mut result = Vec::new();
        PersistentMap::from_root(root)
            .for_each(&reader, &mut MapWork::default(), |key, value| {
                result.push((key.to_vec(), value.to_vec()));
                Ok(())
            })
            .map_err(|error| failure(error.message))?;
        if result.len() as u64 != root.entries() {
            return Err(failure("map inventory count disagreement"));
        }
        Ok(result)
    }
}

impl ImmutableObjectStore for Reader<'_> {
    fn read(
        &self,
        key: ObjectKey,
        maximum: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.charge(1).map_err(|error| {
            StoreError::new(
                StoreErrorClass::Resource,
                "package_oracle_visits",
                error.message,
            )
        })?;
        let additional = self.objects.get(&key).map_or(0, Vec::len) as u64;
        let bytes = self
            .bytes
            .get()
            .checked_add(additional)
            .filter(|bytes| *bytes <= 4_294_967_296)
            .ok_or_else(|| {
                StoreError::new(
                    StoreErrorClass::Resource,
                    "package_oracle_read_bytes",
                    "oracle exhausted 4294967296 aggregate validation-read bytes",
                )
            })?;
        self.bytes.set(bytes);
        let bytes = self.objects.read(key, maximum, work)?;
        if bytes.is_some() {
            self.seen.borrow_mut().insert(key);
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
            "package_oracle_read_only",
            "oracle cannot stage objects",
        ))
    }
}

pub(crate) fn reconstruct(container: &PackageContainer) -> Result<OracleClosure, Diagnostic> {
    let reader = Reader {
        objects: &container.objects,
        seen: RefCell::new(BTreeSet::new()),
        visits: Cell::new(0),
        bytes: Cell::new(0),
    };
    let mut snapshots = BTreeMap::new();
    let mut revisions = BTreeMap::new();
    let mut interfaces = BTreeMap::new();
    let mut digests = BTreeMap::new();
    let mut interface_types = BTreeMap::new();
    let choices = container
        .selections
        .iter()
        .map(|binding| (binding.package_revision, binding.transport))
        .collect::<BTreeMap<_, _>>();
    if choices.len() != container.selections.len() {
        return Err(failure("duplicate selections"));
    }
    if choices.get(&container.root.package_revision) != Some(&container.root.transport) {
        return Err(failure("root selection disagreement"));
    }
    let mut pending = VecDeque::from([(None, container.root.package_revision)]);
    let mut observed = BTreeSet::new();
    let mut edges = 0_usize;
    while let Some((expected, digest)) = pending.pop_front() {
        let revision = PackageRevision::decode(
            &reader.object(ObjectDomain::PackageRevision, digest.bytes())?,
            digest,
        )?;
        let semantic = revision.revision.revision_id()?;
        if let Some((package, semantic_revision)) = expected
            && (revision.package != package || semantic_revision != semantic)
        {
            return Err(failure("logical edge identity disagreement"));
        }
        if let Some(previous) = digests.insert(revision.package, digest)
            && previous != digest
        {
            return Err(failure(format!(
                "conflicting exact revisions for {}",
                revision.package
            )));
        }
        if !observed.insert(digest) {
            continue;
        }
        if observed.len() > 10_000 {
            return Err(failure("oracle package bound"));
        }
        edges = edges
            .checked_add(revision.dependencies.len())
            .ok_or_else(|| failure("edge count overflow"))?;
        if edges > 100_000 {
            return Err(failure("oracle edge bound"));
        }
        for dependency in &revision.dependencies {
            if dependency.package == revision.package {
                return Err(failure("self dependency"));
            }
            pending.push_back((
                Some((dependency.package, dependency.semantic_revision)),
                dependency.package_revision,
            ));
        }
        let selected = choices
            .get(&digest)
            .ok_or_else(|| failure("missing transitive selection"))?;
        let transport = PackageTransport::decode(
            &reader.object(ObjectDomain::PackageTransport, selected.bytes())?,
            *selected,
        )?;
        if transport.package_revision != digest {
            return Err(failure("transport revision mismatch"));
        }
        let root = decode_root(
            &reader.object(ObjectDomain::SemanticRoot, transport.semantic_root.bytes())?,
            transport.semantic_root,
        )?;
        if root.package_id != revision.package
            || root.repository_id != revision.revision.repository_id
        {
            return Err(failure("foreign semantic root"));
        }
        let mut snapshot = KernelSnapshot {
            root,
            owners: BTreeMap::new(),
            types: BTreeMap::new(),
            blobs: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            retirements: BTreeMap::new(),
            dependency_interfaces: BTreeMap::new(),
            dependency_types: BTreeMap::new(),
        };
        for (key, value) in reader.map(snapshot.root.owners)? {
            let owner = EncodedOwnerKey::decode(&key)?;
            let binding = decode_owner_binding(&value, owner)?;
            let bytes = reader.object(ObjectDomain::Owner, binding.object.bytes())?;
            if snapshot
                .owners
                .insert(
                    owner,
                    decode_owner(&bytes, owner, binding.kind, binding.object)?,
                )
                .is_some()
            {
                return Err(failure("duplicate canonical owner"));
            }
        }
        for (key, value) in reader.map(snapshot.root.dependencies)? {
            let package = PackageId::from_bytes(
                key.as_slice()
                    .try_into()
                    .map_err(|_| failure("dependency key length"))?,
            )
            .ok_or_else(|| failure("zero package identity"))?;
            let binding = decode_dependency_binding(&value)?;
            let record = decode_dependency(
                &reader.object(ObjectDomain::Dependency, binding.object.bytes())?,
                &package,
                binding.object,
            )?;
            snapshot.dependencies.insert(package, record);
        }
        if snapshot.dependencies.values().cloned().collect::<Vec<_>>() != revision.dependencies {
            return Err(failure("omitted or changed canonical package edge"));
        }
        for (key, value) in reader.map(snapshot.root.retirements)? {
            let owner = EncodedOwnerKey::decode(&key)?;
            let binding = decode_retirement_binding(&value)?;
            snapshot.retirements.insert(
                owner,
                decode_retirement(
                    &reader.object(ObjectDomain::Retirement, binding.object.bytes())?,
                    owner,
                    binding.object,
                )?,
            );
        }
        let mut type_queue = VecDeque::new();
        for owner in snapshot.owners.values() {
            type_queue.extend(owner.type_roots());
            for (digest, size) in owner.blob_roots() {
                let bytes = reader.object(ObjectDomain::Blob, digest.bytes())?;
                if bytes.len() as u64 != size {
                    return Err(failure("blob length disagreement"));
                }
                snapshot.blobs.insert(digest, size);
            }
        }
        while let Some(digest) = type_queue.pop_front() {
            if snapshot.types.contains_key(&digest) {
                continue;
            }
            let object =
                decode_type_object(&reader.object(ObjectDomain::Type, digest.bytes())?, digest)?;
            type_queue.extend(object.child_types());
            snapshot.types.insert(digest, object);
        }
        if semantic_state_digest(&snapshot)? != revision.revision.semantic_state {
            return Err(failure("canonical body or state identity disagreement"));
        }
        let expected = public_inventory(&snapshot, &reader)?;
        let mut transported = BTreeMap::new();
        let mut public_types = BTreeMap::new();
        for (key, value) in reader.map(transport.interface_owners)? {
            let owner = EncodedOwnerKey::decode(&key)?;
            let binding = decode_package_interface_binding(&value)?;
            let projected = PackageInterfaceOwner::decode(
                &reader.object(ObjectDomain::PackageInterface, binding.bytes())?,
                owner,
                binding,
            )?;
            type_queue.extend(projected.type_roots());
            transported.insert(owner, projected.record);
        }
        while let Some(digest) = type_queue.pop_front() {
            if public_types.contains_key(&digest) {
                continue;
            }
            let object =
                decode_type_object(&reader.object(ObjectDomain::Type, digest.bytes())?, digest)?;
            type_queue.extend(object.child_types());
            public_types.insert(digest, object);
        }
        if expected != transported {
            return Err(failure(
                "independent public-interface inventory disagrees with canonical owners",
            ));
        }
        let interface_entries = expected
            .iter()
            .map(|(owner, record)| {
                let value = PackageInterfaceOwner {
                    contract_version:
                        crate::platform::package_interface::PACKAGE_INTERFACE_CONTRACT_VERSION,
                    record: record.clone(),
                };
                let (digest, _) = value.encode()?;
                Ok((
                    EncodedOwnerKey::new(*owner).bytes().to_vec(),
                    crate::platform::package_interface::encode_package_interface_binding(digest),
                ))
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        let content =
            crate::platform::persistent_map::MapContentRoot::from_sorted(interface_entries)
                .map_err(|error| failure(error.message))?;
        if crate::platform::package_interface::package_interface_digest(revision.package, content)?
            != revision.interface
        {
            return Err(failure(
                "independently reconstructed interface commitment disagrees with logical revision",
            ));
        }
        interface_types.insert(revision.package, public_types);
        interfaces.insert(revision.package, expected);
        snapshots.insert(revision.package, snapshot);
        revisions.insert(revision.package, revision);
    }
    if observed != choices.keys().copied().collect() {
        return Err(failure("unreachable transport selection"));
    }
    let mut children_remaining = BTreeMap::new();
    let mut parents = BTreeMap::<PackageId, Vec<PackageId>>::new();
    for (package, revision) in &revisions {
        children_remaining.insert(*package, revision.dependencies.len());
        for dependency in &revision.dependencies {
            parents
                .entry(dependency.package)
                .or_default()
                .push(*package);
        }
    }
    // Independent degree elimination over reconstructed package IDs. Each edge is visited once;
    // no production traversal, revision-unification result, or dependency order is consulted.
    let mut leaves = children_remaining
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(*id))
        .collect::<VecDeque<_>>();
    let mut eliminated = 0;
    while let Some(leaf) = leaves.pop_front() {
        reader.charge(1)?;
        eliminated += 1;
        for parent in parents.remove(&leaf).unwrap_or_default() {
            reader.charge(1)?;
            let count = children_remaining
                .get_mut(&parent)
                .ok_or_else(|| failure("unknown cycle parent"))?;
            *count = count
                .checked_sub(1)
                .ok_or_else(|| failure("duplicate cycle edge"))?;
            if *count == 0 {
                leaves.push_back(parent);
            }
        }
    }
    if eliminated != revisions.len() {
        return Err(failure("cyclic exact closure"));
    }
    for snapshot in snapshots.values_mut() {
        for dependency in snapshot.dependencies.values() {
            reader.charge(
                interfaces.get(&dependency.package).map_or(0, BTreeMap::len)
                    + interface_types
                        .get(&dependency.package)
                        .map_or(0, BTreeMap::len),
            )?;
            snapshot.dependency_interfaces.insert(
                dependency.package_revision,
                interfaces
                    .get(&dependency.package)
                    .cloned()
                    .ok_or_else(|| failure("unresolved public dependency"))?,
            );
            snapshot.dependency_types.extend(
                interface_types
                    .get(&dependency.package)
                    .cloned()
                    .ok_or_else(|| failure("unresolved dependency types"))?,
            );
        }
        let validation = crate::platform::kernel::validate_full_with_limit(
            snapshot,
            (16_000_000 - reader.visits.get()) as usize,
        )
        .map_err(|errors| {
            failure(format!(
                "canonical type validation rejected independent inventory: {}",
                errors
                    .first()
                    .map_or("missing diagnostic", |error| error.code.as_str())
            ))
        })?;
        reader.charge(validation.work_consumed as usize)?;
        let mut intrinsic_work = 0;
        for record in snapshot.owners.values() {
            if let OwnerRecord::Declaration(DeclarationRecord {
                payload: DeclarationPayload::External(external),
                ..
            }) = record
            {
                crate::platform::intrinsic_contract::validate_kernel_intrinsic(
                    snapshot,
                    external,
                    &mut intrinsic_work,
                    (16_000_000 - reader.visits.get()) as usize,
                )?;
            }
        }
        reader.charge(intrinsic_work)?;
    }
    if *reader.seen.borrow() != container.objects.keys().copied().collect() {
        return Err(failure("missing or extra complete source object"));
    }
    Ok(OracleClosure {
        snapshots,
        revisions,
        interfaces,
        objects: container.objects.len(),
        edges,
        validation_visits: reader.visits.get(),
        validation_read_bytes: reader.bytes.get(),
    })
}

fn public_inventory(
    snapshot: &KernelSnapshot,
    reader: &Reader<'_>,
) -> Result<BTreeMap<OwnerKey, PackageInterfaceRecord>, Diagnostic> {
    let mut selected = BTreeSet::new();
    let mut result = BTreeMap::new();
    for (owner, record) in &snapshot.owners {
        reader.charge(1)?;
        let OwnerRecord::Declaration(declaration) = record else {
            continue;
        };
        if declaration.visibility != DeclarationVisibility::Public {
            continue;
        }
        let payload = match &declaration.payload {
            DeclarationPayload::Record { fields } => {
                reader.charge(fields.len())?;
                selected.extend(fields.iter().copied().map(OwnerKey::Field));
                PackageInterfaceDeclarationPayload::Record {
                    fields: fields.clone(),
                }
            }
            DeclarationPayload::Variant { cases } => {
                reader.charge(cases.len())?;
                selected.extend(cases.iter().copied().map(OwnerKey::Case));
                PackageInterfaceDeclarationPayload::Variant {
                    cases: cases.clone(),
                }
            }
            DeclarationPayload::Interface { operations } => {
                reader.charge(operations.len())?;
                selected.extend(operations.iter().copied().map(OwnerKey::Operation));
                PackageInterfaceDeclarationPayload::Interface {
                    operations: operations.clone(),
                }
            }
            DeclarationPayload::Function(function) => {
                reader.charge(function.parameters.len() + function.type_parameters.len())?;
                selected.extend(function.parameters.iter().copied().map(OwnerKey::Parameter));
                selected.extend(
                    function
                        .type_parameters
                        .iter()
                        .copied()
                        .map(OwnerKey::TypeParameter),
                );
                if let FunctionEffect::Task { requirements } = &function.effect {
                    reader.charge(requirements.len())?;
                    selected.extend(
                        requirements
                            .iter()
                            .filter(|requirement| requirement.package == snapshot.root.package_id)
                            .map(|requirement| OwnerKey::Requirement(requirement.requirement)),
                    );
                }
                PackageInterfaceDeclarationPayload::Function(PackageFunctionSignature {
                    type_parameters: function.type_parameters.clone(),
                    parameters: function.parameters.clone(),
                    result: function.result,
                    effect: function.effect.clone(),
                })
            }
            DeclarationPayload::External(function) => {
                reader.charge(function.parameters.len() + function.type_parameters.len())?;
                selected.extend(function.parameters.iter().copied().map(OwnerKey::Parameter));
                selected.extend(
                    function
                        .type_parameters
                        .iter()
                        .copied()
                        .map(OwnerKey::TypeParameter),
                );
                PackageInterfaceDeclarationPayload::External(PackageExternalSignature {
                    type_parameters: function.type_parameters.clone(),
                    parameters: function.parameters.clone(),
                    result: function.result,
                })
            }
            DeclarationPayload::Constant { ty, .. } => {
                PackageInterfaceDeclarationPayload::Constant { ty: *ty }
            }
            DeclarationPayload::Component {
                requirements,
                ports,
            } => {
                reader.charge(requirements.len() + ports.len())?;
                selected.extend(requirements.iter().copied().map(OwnerKey::Requirement));
                selected.extend(ports.iter().copied().map(OwnerKey::Port));
                PackageInterfaceDeclarationPayload::Component {
                    requirements: requirements.clone(),
                    ports: ports.clone(),
                }
            }
            DeclarationPayload::Test { .. } => {
                return Err(failure("public test is not an interface"));
            }
        };
        result.insert(
            *owner,
            PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                header: declaration.header,
                name: declaration.name.clone(),
                payload,
            }),
        );
    }
    while let Some(owner) = selected.pop_first() {
        reader.charge(1)?;
        if result.contains_key(&owner) {
            continue;
        }
        let record = snapshot
            .owners
            .get(&owner)
            .ok_or_else(|| failure("public declaration names absent member"))?;
        let projected = match record {
            OwnerRecord::TypeParameter(value) => {
                PackageInterfaceRecord::TypeParameter(value.clone())
            }
            OwnerRecord::Field(value) => PackageInterfaceRecord::Field(value.clone()),
            OwnerRecord::Case(value) => PackageInterfaceRecord::Case(value.clone()),
            OwnerRecord::Operation(value) => {
                reader.charge(value.parameters.len())?;
                selected.extend(value.parameters.iter().copied().map(OwnerKey::Parameter));
                PackageInterfaceRecord::Operation(value.clone())
            }
            OwnerRecord::Parameter(value) => PackageInterfaceRecord::Parameter(value.clone()),
            OwnerRecord::Requirement(value) => PackageInterfaceRecord::Requirement(value.clone()),
            OwnerRecord::Port(value) => PackageInterfaceRecord::Port(PackageInterfacePort {
                header: value.header,
                declaration: value.declaration,
                name: value.name.clone(),
                function_type: value.function_type,
            }),
            _ => {
                return Err(failure(
                    "non-interface member selected by public declaration",
                ));
            }
        };
        result.insert(owner, projected);
    }
    Ok(result)
}
