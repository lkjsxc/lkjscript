//! Independent closure and interface inventory reconstruction from neutral canonical records.
//! Does not call transport admission, production closure resolution, interface projection, or linking.

use super::source::{PackageContainer, SourceObservation};
use super::{PackageRevision, PackageTransport};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::*;
use crate::platform::package_interface::{PackageInterfaceOwner, decode_package_interface_binding};
use crate::platform::persistent_map::{MapError, MapErrorClass, MapRoot, MapWork, PersistentMap};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StageOutcome, StoreError, StoreErrorClass,
    StoreWork,
};
use crate::platform::storage::page_store::ObjectPageReader;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[path = "oracle_read_views.rs"]
mod read_views;

type EncodedEntries = Vec<(Vec<u8>, Vec<u8>)>;

#[derive(Clone, Debug)]
pub(crate) struct OracleClosure {
    pub snapshots: BTreeMap<PackageId, KernelSnapshot>,
    pub revisions: BTreeMap<PackageId, PackageRevision>,
    pub interfaces: BTreeMap<PackageId, BTreeMap<OwnerKey, PackageInterfaceRecord>>,
    interface_types: BTreeMap<PackageId, BTreeMap<TypeObjectDigest, TypeObject>>,
    pub objects: usize,
    pub edges: usize,
    pub validation_visits: u64,
    pub validation_visits_saturated: bool,
    pub validation_read_bytes: u64,
    pub validation_read_bytes_saturated: bool,
}

impl OracleClosure {
    /// Passive modeled copy work, not allocation permission or allocator/RSS accounting.
    /// Overflow marks a lower bound and cannot reject ordinary independent reconstruction.
    pub(crate) fn package_copy_material(
        &self,
        package: PackageId,
        encoded_source_bytes: SourceObservation,
    ) -> Option<(SourceObservation, SourceObservation)> {
        let snapshot = self.snapshots.get(&package)?;
        let interfaces = self.interfaces.get(&package)?;
        let types = self.interface_types.get(&package)?;
        let records = [
            1,
            snapshot.owners.len(),
            snapshot.types.len(),
            snapshot.dependency_interfaces.len(),
            snapshot.dependency_types.len(),
            snapshot.representation_dependencies.len(),
            snapshot.blobs.len(),
            snapshot.dependencies.len(),
            snapshot.retirements.len(),
            interfaces.len(),
            types.len(),
        ]
        .into_iter()
        .fold(SourceObservation::default(), |mut total, records| {
            total.observe(observed_count(records));
            total
        });
        let mut bytes = observed_product(encoded_source_bytes, 4);
        bytes.observe(observed_product(records, 256));
        Some((bytes, records))
    }
}

#[cfg(test)]
std::thread_local! {
    static FULL_PACKAGE_VALIDATIONS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn full_package_validations() -> usize {
    FULL_PACKAGE_VALIDATIONS.with(Cell::get)
}

fn failure(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, "package_closure_oracle", message)
}

fn observed_count(count: usize) -> SourceObservation {
    match u64::try_from(count) {
        Ok(consumed) => SourceObservation::exact(consumed),
        Err(_) => SourceObservation {
            consumed: u64::MAX,
            saturated: true,
        },
    }
}

fn observed_product(value: SourceObservation, factor: u64) -> SourceObservation {
    let mut result = SourceObservation::product(&[value.consumed, factor]);
    result.saturated |= value.saturated && factor != 0;
    result
}

fn observe(counter: &Cell<SourceObservation>, additional: SourceObservation) {
    let mut next = counter.get();
    next.observe(additional);
    counter.set(next);
}

#[cfg(test)]
#[path = "oracle_observation_tests.rs"]
mod observation_tests;

struct Reader<'a> {
    objects: &'a BTreeMap<ObjectKey, Vec<u8>>,
    seen: RefCell<BTreeSet<ObjectKey>>,
    visits: Cell<SourceObservation>,
    bytes: Cell<SourceObservation>,
    copy_bytes: Cell<SourceObservation>,
}

impl<'a> Reader<'a> {
    fn new(objects: &'a BTreeMap<ObjectKey, Vec<u8>>) -> Self {
        Self {
            objects,
            seen: RefCell::new(BTreeSet::new()),
            visits: Cell::new(SourceObservation::default()),
            bytes: Cell::new(SourceObservation::default()),
            copy_bytes: Cell::new(SourceObservation::default()),
        }
    }

    fn observe_copy(&self, bytes: SourceObservation, mut records: SourceObservation) {
        records.observe(SourceObservation {
            consumed: bytes.consumed.div_ceil(64),
            saturated: bytes.saturated,
        });
        observe(&self.visits, records);
        observe(&self.copy_bytes, bytes);
    }

    fn charge(&self, visits: usize) -> Result<(), Diagnostic> {
        // Keep the callback result shape used by independent semantic walkers. This
        // passive observation never supplies or removes execution permission.
        observe(&self.visits, observed_count(visits));
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
                let exhausted = || MapError {
                    class: MapErrorClass::Resource,
                    code: "package_closure_oracle_allocation",
                    message: "independent source map inventory cannot reserve its buffer".into(),
                };
                result.try_reserve(1).map_err(|_| exhausted())?;
                let mut key_copy = Vec::new();
                key_copy
                    .try_reserve_exact(key.len())
                    .map_err(|_| exhausted())?;
                let mut value_copy = Vec::new();
                value_copy
                    .try_reserve_exact(value.len())
                    .map_err(|_| exhausted())?;
                key_copy.extend_from_slice(key);
                value_copy.extend_from_slice(value);
                result.push((key_copy, value_copy));
                Ok(())
            })
            .map_err(|error| {
                if error.class == MapErrorClass::Resource {
                    Diagnostic::new(DiagnosticClass::Resource, error.code, error.message)
                } else {
                    failure(error.message)
                }
            })?;
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
        maximum: Option<usize>,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        observe(&self.visits, SourceObservation::exact(1));
        observe(
            &self.bytes,
            observed_count(self.objects.get(&key).map_or(0, Vec::len)),
        );
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
    let reader = Reader::new(&container.objects);
    let mut snapshots = BTreeMap::new();
    let mut revisions = BTreeMap::new();
    let mut interfaces = BTreeMap::new();
    let mut digests = BTreeMap::new();
    let mut interface_types = BTreeMap::new();
    let mut shared_packages = BTreeSet::new();
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
        if let Some(standard) = crate::platform::builtin_standard::BuiltinStandard::initialized_for(
            super::PackageTransportBinding {
                package_revision: digest,
                transport: *selected,
            },
        ) {
            // The cached oracle was independently reconstructed from these same original
            // bytes. Neither a producer witness nor the production snapshot is its proof.
            // Missing/corrupt physical bytes remain failures even when the local proof exists.
            for (key, expected) in &standard.admitted_source().container.objects {
                let actual = reader.object(key.domain, key.digest.bytes())?;
                if &actual != expected {
                    return Err(failure("retained builtin oracle source bytes changed"));
                }
            }
            let cached = standard.independent_source();
            if cached.revisions.get(&revision.package) != Some(&revision) {
                return Err(failure("retained builtin oracle revision changed"));
            }
            let package = revision.package;
            let source_bytes = standard.admitted_source().container.objects.values().fold(
                SourceObservation::default(),
                |mut total, bytes| {
                    total.observe(observed_count(bytes.len()));
                    total
                },
            );
            let (copy_bytes, records) = cached
                .package_copy_material(package, source_bytes)
                .ok_or_else(|| failure("retained builtin oracle copy inventory is absent"))?;
            // Observe all three deep copies independently of the physical reads above.
            // No historical validation work or producer proof becomes current permission.
            reader.observe_copy(copy_bytes, records);
            snapshots.insert(package, cached.snapshots[&package].clone());
            interfaces.insert(package, cached.interfaces[&package].clone());
            interface_types.insert(package, cached.interface_types[&package].clone());
            shared_packages.insert(package);
            revisions.insert(package, revision);
            continue;
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
            representation_dependencies: BTreeMap::new(),
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
        if crate::platform::package_interface::package_interface_digest_for_graph(
            revision.package,
            content,
            revision.graph_contract_version,
        )? != revision.interface
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
    // Reconstruct static-witness visibility directly from the independently
    // selected public inventories, never from canonical private owners.
    for (package, inventory) in &interfaces {
        let require = |package_id: PackageId,
                       key: OwnerKey,
                       kinds: &[OwnerKind]|
         -> Result<(), Diagnostic> {
            reader.charge(1)?;
            let mut declared = package_id == *package;
            for dependency in &revisions[package].dependencies {
                reader.charge(1)?;
                declared |= dependency.package == package_id;
            }
            if !declared {
                return Err(failure("static witness names an undeclared dependency"));
            }
            let owner = interfaces
                .get(&package_id)
                .and_then(|i| i.get(&key))
                .ok_or_else(|| failure("static witness signature is not public"))?;
            let matches = kinds.iter().any(|kind| match (kind, owner) {
                (OwnerKind::OwnedContract, PackageInterfaceRecord::Declaration(d)) => matches!(
                    d.payload,
                    PackageInterfaceDeclarationPayload::OwnedContract(_)
                ),
                (OwnerKind::OwnedImplementation, PackageInterfaceRecord::Declaration(d)) => matches!(
                    d.payload,
                    PackageInterfaceDeclarationPayload::OwnedImplementation(_)
                ),
                (OwnerKind::PureFunction, PackageInterfaceRecord::Declaration(d)) => {
                    matches!(&d.payload, PackageInterfaceDeclarationPayload::Function(f) if matches!(f.effect, FunctionEffect::Pure))
                }
                (OwnerKind::TaskFunction, PackageInterfaceRecord::Declaration(d)) => {
                    matches!(&d.payload, PackageInterfaceDeclarationPayload::Function(f) if matches!(f.effect, FunctionEffect::Task { .. }))
                }
                (OwnerKind::Requirement, PackageInterfaceRecord::Requirement(_)) => true,
                _ => false,
            });
            if !matches {
                return Err(failure(
                    "static witness public signature has the wrong kind",
                ));
            }
            Ok(())
        };
        for value in inventory.values() {
            reader.charge(1)?;
            let PackageInterfaceRecord::Declaration(d) = value else {
                continue;
            };
            match &d.payload {
                PackageInterfaceDeclarationPayload::OwnedContract(c) => {
                    for method in &c.methods {
                        reader.charge(1)?;
                        for requirement in method.effect.row().requirements {
                            require(
                                requirement.package(),
                                requirement.owner(),
                                &[OwnerKind::Requirement],
                            )?;
                        }
                    }
                }
                PackageInterfaceDeclarationPayload::OwnedImplementation(i) => {
                    require(
                        i.contract.package,
                        OwnerKey::Declaration(i.contract.declaration),
                        &[OwnerKind::OwnedContract],
                    )?;
                    for p in &i.implementation_parameters {
                        require(
                            p.contract.package,
                            OwnerKey::Declaration(p.contract.declaration),
                            &[OwnerKind::OwnedContract],
                        )?;
                    }
                    for m in &i.methods {
                        let omitted_local = m.function.package == *package
                            && !inventory
                                .contains_key(&OwnerKey::Declaration(m.function.declaration));
                        if omitted_local {
                            reader.charge(1)?;
                            // Complete canonical source was independently reconstructed above.
                            // Its mandatory full validation below proves this private target;
                            // public inventory absence supplies no function authority.
                        } else {
                            require(
                                m.function.package,
                                OwnerKey::Declaration(m.function.declaration),
                                &[OwnerKind::PureFunction, OwnerKind::TaskFunction],
                            )?;
                        }
                        for operand in &m.implementations {
                            operand.validate_local()?;
                            for operand in operand.walk() {
                                reader.charge(1)?;
                                if let ImplementationOperand::Concrete { implementation, .. } =
                                    operand
                                {
                                    require(
                                        implementation.package,
                                        OwnerKey::Declaration(implementation.declaration),
                                        &[OwnerKind::OwnedImplementation],
                                    )?;
                                }
                            }
                        }
                    }
                }
                PackageInterfaceDeclarationPayload::Function(f) => {
                    for p in &f.implementation_parameters {
                        require(
                            p.contract.package,
                            OwnerKey::Declaration(p.contract.declaration),
                            &[OwnerKind::OwnedContract],
                        )?;
                    }
                }
                _ => {}
            }
        }
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
    for (package, snapshot) in &mut snapshots {
        if shared_packages.contains(package) {
            continue;
        }
        reader.charge(snapshot.dependencies.len())?;
        let mut pending = snapshot
            .dependencies
            .values()
            .cloned()
            .collect::<VecDeque<_>>();
        while let Some(mut dependency) = pending.pop_front() {
            reader.charge(1)?;
            dependency.graph_contract_version =
                crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
            if let Some(previous) = snapshot
                .representation_dependencies
                .get(&dependency.package)
            {
                if previous != &dependency {
                    return Err(failure(
                        "conflicting opaque representation package revisions",
                    ));
                }
                continue;
            }
            let revision = revisions
                .get(&dependency.package)
                .ok_or_else(|| failure("unresolved representation revision"))?;
            revision.matches_dependency(dependency.package_revision, &dependency)?;
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
                    .ok_or_else(|| failure("unresolved public dependency"))?
                    .into(),
            );
            snapshot.dependency_types.extend(
                interface_types
                    .get(&dependency.package)
                    .cloned()
                    .ok_or_else(|| failure("unresolved dependency types"))?,
            );
            snapshot
                .representation_dependencies
                .insert(dependency.package, dependency);
            reader.charge(revision.dependencies.len())?;
            pending.extend(revision.dependencies.iter().cloned());
        }
        #[cfg(test)]
        FULL_PACKAGE_VALIDATIONS.with(|count| count.set(count.get().saturating_add(1)));
        let validation = crate::platform::kernel::validate_full_with_limit(snapshot, None)
            .map_err(|errors| {
                failure(format!(
                    "canonical type validation rejected independent inventory: {}",
                    errors
                        .first()
                        .map_or("missing diagnostic", |error| error.code.as_str())
                ))
            })?;
        observe(
            &reader.visits,
            SourceObservation {
                consumed: validation.work_consumed,
                saturated: validation.work_saturated,
            },
        );
        let mut intrinsic_work = crate::platform::kernel::ValidationWork::default();
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
                    None,
                )?;
            }
        }
        let mut intrinsic_observation = observed_count(intrinsic_work.consumed);
        intrinsic_observation.saturated |= intrinsic_work.saturated;
        observe(&reader.visits, intrinsic_observation);
    }
    // A package dependency DAG cannot establish finite generic instantiation:
    // a supplied witness may dispatch from an upstream body back downstream.
    // Reconstruct the proof from this independently decoded complete source.
    read_views::admit(&snapshots, &interfaces, &revisions, &reader)?;
    reader.charge(snapshots.len())?;
    let complete_sources: Vec<_> = snapshots.values().collect();
    let mut composition_work = crate::platform::kernel::ValidationWork::default();
    crate::platform::kernel::callable_flow::validate_snapshot_callable_closure(
        &complete_sources,
        &|| Ok(()),
        &mut composition_work,
        None,
    )?;
    let mut composition_observation = observed_count(composition_work.consumed);
    composition_observation.saturated |= composition_work.saturated;
    observe(&reader.visits, composition_observation);
    if *reader.seen.borrow() != container.objects.keys().copied().collect() {
        return Err(failure("missing or extra complete source object"));
    }
    Ok(OracleClosure {
        snapshots,
        revisions,
        interfaces,
        interface_types,
        objects: container.objects.len(),
        edges,
        validation_visits: reader.visits.get().consumed,
        validation_visits_saturated: reader.visits.get().saturated,
        validation_read_bytes: reader.bytes.get().consumed,
        validation_read_bytes_saturated: reader.bytes.get().saturated,
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
            DeclarationPayload::OwnedContract(c) => {
                reader.charge(1 + c.type_parameters.len() + c.associated_types.len())?;
                selected.insert(OwnerKey::TypeParameter(c.self_parameter));
                selected.extend(
                    c.type_parameters
                        .iter()
                        .copied()
                        .map(OwnerKey::TypeParameter),
                );
                for method in &c.methods {
                    reader.charge(
                        1 + method.parameters.len()
                            + method.read_scope_parameters.len()
                            + method.read_scope_sources.len(),
                    )?;
                    if let FunctionEffect::Task {
                        requirements,
                        effect_parameters,
                    } = &method.effect
                    {
                        reader.charge(requirements.len() + effect_parameters.len())?;
                        selected.extend(
                            requirements
                                .iter()
                                .filter(|r| r.package() == snapshot.root.package_id)
                                .map(|r| r.owner()),
                        );
                    }
                }
                PackageInterfaceDeclarationPayload::OwnedContract(c.clone())
            }
            DeclarationPayload::OwnedImplementation(i) => {
                reader.charge(
                    i.type_parameters.len()
                        + i.type_arguments.len()
                        + i.implementation_parameters.len()
                        + i.methods.len()
                        + i.associated_types.len(),
                )?;
                for parameter in &i.implementation_parameters {
                    reader.charge(1 + parameter.type_arguments.len())?;
                }
                for method in &i.methods {
                    reader
                        .charge(method.type_arguments.len() + method.read_scope_arguments.len())?;
                    for operand in &method.implementations {
                        operand.validate_local()?;
                        for operand in operand.walk() {
                            reader.charge(1 + operand.type_arguments().len())?;
                        }
                    }
                }
                selected.extend(
                    i.type_parameters
                        .iter()
                        .copied()
                        .map(OwnerKey::TypeParameter),
                );
                PackageInterfaceDeclarationPayload::OwnedImplementation(i.clone())
            }
            DeclarationPayload::Record {
                fields,
                type_parameters,
            } => {
                reader.charge(type_parameters.len())?;
                selected.extend(type_parameters.iter().copied().map(OwnerKey::TypeParameter));
                reader.charge(fields.len())?;
                selected.extend(fields.iter().copied().map(OwnerKey::Field));
                PackageInterfaceDeclarationPayload::Record {
                    type_parameters: type_parameters.clone(),
                    fields: fields.clone(),
                }
            }
            DeclarationPayload::OwnedRecord {
                fields,
                type_parameters,
            } => {
                reader.charge(type_parameters.len() + fields.len())?;
                selected.extend(type_parameters.iter().copied().map(OwnerKey::TypeParameter));
                selected.extend(fields.iter().copied().map(OwnerKey::Field));
                PackageInterfaceDeclarationPayload::OwnedRecord {
                    type_parameters: type_parameters.clone(),
                    fields: fields.clone(),
                }
            }
            DeclarationPayload::Variant {
                cases,
                type_parameters,
            } => {
                reader.charge(type_parameters.len())?;
                selected.extend(type_parameters.iter().copied().map(OwnerKey::TypeParameter));
                reader.charge(cases.len())?;
                selected.extend(cases.iter().copied().map(OwnerKey::Case));
                PackageInterfaceDeclarationPayload::Variant {
                    type_parameters: type_parameters.clone(),
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
                reader.charge(function.implementation_parameters.len())?;
                for parameter in &function.implementation_parameters {
                    reader.charge(1 + parameter.type_arguments.len())?;
                }
                reader.charge(function.requirement_parameters.len())?;
                selected.extend(
                    function
                        .requirement_parameters
                        .iter()
                        .copied()
                        .map(OwnerKey::RequirementParameter),
                );
                reader.charge(function.effect_parameters.len())?;
                selected.extend(
                    function
                        .effect_parameters
                        .iter()
                        .copied()
                        .map(OwnerKey::EffectParameter),
                );
                reader.charge(function.parameters.len() + function.type_parameters.len())?;
                reader.charge(
                    function.read_scope_parameters.len() + function.read_scope_sources.len(),
                )?;
                selected.extend(function.parameters.iter().copied().map(OwnerKey::Parameter));
                selected.extend(
                    function
                        .type_parameters
                        .iter()
                        .copied()
                        .map(OwnerKey::TypeParameter),
                );
                if let FunctionEffect::Task {
                    effect_parameters: _,
                    requirements,
                } = &function.effect
                {
                    reader.charge(requirements.len())?;
                    selected.extend(
                        requirements
                            .iter()
                            .filter(|requirement| requirement.package() == snapshot.root.package_id)
                            .map(|requirement| requirement.owner()),
                    );
                }
                PackageInterfaceDeclarationPayload::Function(PackageFunctionSignature {
                    implementation_parameters: function.implementation_parameters.clone(),
                    requirement_parameters: function.requirement_parameters.clone(),
                    effect_parameters: function.effect_parameters.clone(),
                    type_parameters: function.type_parameters.clone(),
                    parameters: function.parameters.clone(),
                    result: function.result,
                    read_scope_parameters: function.read_scope_parameters.clone(),
                    read_scope_sources: function.read_scope_sources.clone(),
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
            DeclarationPayload::ReadView(view) => {
                reader.charge(1 + view.type_parameters.len() + view.fields.len())?;
                selected.extend(
                    view.type_parameters
                        .iter()
                        .copied()
                        .map(OwnerKey::TypeParameter),
                );
                PackageInterfaceDeclarationPayload::ReadView(view.clone())
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
            OwnerRecord::RequirementParameter(value) => {
                PackageInterfaceRecord::RequirementParameter(value.clone())
            }
            OwnerRecord::EffectParameter(value) => {
                PackageInterfaceRecord::EffectParameter(value.clone())
            }
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
    extend_representation_inventory(snapshot, reader, &mut result)?;
    let mut types = result
        .values()
        .flat_map(PackageInterfaceRecord::type_roots)
        .collect::<Vec<_>>();
    let mut visited = BTreeSet::new();
    while let Some(digest) = types.pop() {
        reader.charge(1)?;
        if !visited.insert(digest) {
            continue;
        }
        let object = snapshot
            .types
            .get(&digest)
            .or_else(|| snapshot.dependency_types.get(&digest))
            .ok_or_else(|| failure("public signature type is absent from canonical inventory"))?;
        let children = object.child_types();
        reader.charge(children.len())?;
        types.extend(children);
        if let TypeForm::TaskFunction { effect, .. } = &object.form {
            reader.charge(effect.requirements.len())?;
            for reference in &effect.requirements {
                if reference.package() != snapshot.root.package_id {
                    continue;
                }
                let owner = reference.owner();
                let record = match snapshot.owners.get(&owner) {
                    Some(OwnerRecord::Requirement(record)) => {
                        PackageInterfaceRecord::Requirement(record.clone())
                    }
                    Some(OwnerRecord::RequirementParameter(record)) => {
                        PackageInterfaceRecord::RequirementParameter(record.clone())
                    }
                    _ => {
                        return Err(failure(
                            "public callable requirement is absent from canonical inventory",
                        ));
                    }
                };
                result.insert(owner, record);
            }
        }
    }
    Ok(result)
}

/// Reconstruct private representation evidence from canonical layouts independently of the
/// producer's selection machinery. A private declaration never becomes a public declaration.
fn extend_representation_inventory(
    snapshot: &KernelSnapshot,
    reader: &Reader<'_>,
    result: &mut BTreeMap<OwnerKey, PackageInterfaceRecord>,
) -> Result<(), Diagnostic> {
    let mut pending = Vec::new();
    for value in result.values() {
        reader.charge(1)?;
        let roots = value.type_roots();
        reader.charge(roots.len())?;
        pending.extend(roots);
        if let PackageInterfaceRecord::Declaration(record) = value
            && let PackageInterfaceDeclarationPayload::OwnedRecord { fields, .. } = &record.payload
        {
            for field in fields {
                reader.charge(1)?;
                match snapshot.owners.get(&OwnerKey::Field(*field)) {
                    Some(OwnerRecord::Field(field)) => pending.push(field.ty),
                    _ => return Err(failure("owned representation field is absent")),
                }
            }
        }
    }
    let mut visited_types = BTreeSet::new();
    let mut visited_declarations = BTreeSet::new();
    while let Some(digest) = pending.pop() {
        reader.charge(1)?;
        if !visited_types.insert(digest) {
            continue;
        }
        let object = snapshot
            .types
            .get(&digest)
            .or_else(|| snapshot.dependency_types.get(&digest))
            .ok_or_else(|| failure("owned representation type is absent"))?;
        reader.charge(object.child_type_count())?;
        pending.extend(object.child_types());
        let reference = match object.form {
            TypeForm::Named { declaration }
            | TypeForm::Applied { declaration, .. }
            | TypeForm::ReadViewApplied { declaration, .. }
                if declaration.package == snapshot.root.package_id =>
            {
                declaration
            }
            _ => continue,
        };
        if !visited_declarations.insert(reference.declaration) {
            continue;
        }
        let Some(OwnerRecord::Declaration(declaration)) = snapshot
            .owners
            .get(&OwnerKey::Declaration(reference.declaration))
        else {
            return Err(failure(
                "owned representation nominal declaration is absent",
            ));
        };
        reader.charge(declaration.payload.type_parameters().iter().count())?;
        let representation_work = match &declaration.payload {
            DeclarationPayload::OwnedRecord { fields, .. }
            | DeclarationPayload::Record { fields, .. } => {
                observed_product(observed_count(fields.len()), 3)
            }
            DeclarationPayload::Variant { cases, .. } => {
                observed_product(observed_count(cases.len()), 3)
            }
            DeclarationPayload::ReadView(view) => {
                let mut work = observed_product(observed_count(view.fields.len()), 3);
                work.observe(SourceObservation::exact(1));
                work
            }
            _ => SourceObservation::default(),
        };
        observe(&reader.visits, representation_work);
        let (payload, members) = match &declaration.payload {
            DeclarationPayload::OwnedRecord {
                type_parameters,
                fields,
            } => (
                PackageInterfaceDeclarationPayload::OwnedRecord {
                    type_parameters: type_parameters.clone(),
                    fields: fields.clone(),
                },
                fields
                    .iter()
                    .copied()
                    .map(OwnerKey::Field)
                    .collect::<Vec<_>>(),
            ),
            DeclarationPayload::Record {
                type_parameters,
                fields,
            } => (
                PackageInterfaceDeclarationPayload::Record {
                    type_parameters: type_parameters.clone(),
                    fields: fields.clone(),
                },
                fields
                    .iter()
                    .copied()
                    .map(OwnerKey::Field)
                    .collect::<Vec<_>>(),
            ),
            DeclarationPayload::Variant {
                type_parameters,
                cases,
            } => (
                PackageInterfaceDeclarationPayload::Variant {
                    type_parameters: type_parameters.clone(),
                    cases: cases.clone(),
                },
                cases
                    .iter()
                    .copied()
                    .map(OwnerKey::Case)
                    .collect::<Vec<_>>(),
            ),
            DeclarationPayload::ReadView(view) => {
                pending.extend(view.fields.iter().map(|field| field.ty));
                (
                    PackageInterfaceDeclarationPayload::ReadView(view.clone()),
                    Vec::new(),
                )
            }
            _ => {
                return Err(failure(
                    "owned representation names a non-layout declaration",
                ));
            }
        };
        let projected = PackageInterfaceDeclaration {
            header: declaration.header,
            name: declaration.name.clone(),
            payload,
        };
        let value = if declaration.visibility == DeclarationVisibility::Public {
            PackageInterfaceRecord::Declaration(projected)
        } else {
            PackageInterfaceRecord::RepresentationDeclaration(projected)
        };
        result.insert(OwnerKey::Declaration(reference.declaration), value);
        for id in declaration.payload.type_parameters().iter() {
            reader.charge(1)?;
            match snapshot.owners.get(&OwnerKey::TypeParameter(*id)) {
                Some(OwnerRecord::TypeParameter(parameter)) => {
                    result.insert(
                        OwnerKey::TypeParameter(*id),
                        PackageInterfaceRecord::TypeParameter(parameter.clone()),
                    );
                }
                _ => return Err(failure("owned representation parameter is absent")),
            }
        }
        for key in members {
            reader.charge(1)?;
            let value = match snapshot.owners.get(&key) {
                Some(OwnerRecord::Field(field)) => {
                    pending.push(field.ty);
                    PackageInterfaceRecord::Field(field.clone())
                }
                Some(OwnerRecord::Case(case)) => {
                    pending.extend(case.payload);
                    PackageInterfaceRecord::Case(case.clone())
                }
                _ => return Err(failure("owned representation member is absent")),
            };
            result.insert(key, value);
        }
    }
    Ok(())
}
