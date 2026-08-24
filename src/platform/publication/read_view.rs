//! Revision-pinned, bounded reads over accepted Graph 5 authority and its committed witness.

use super::{
    AuthoredChangeRequest, AuthoredChangeResponse, CurrentPublication, PreparedPublication,
    PublicationOptions, prepare_change_publication,
};
use crate::platform::change::{
    AuthoredChangeSet, AuthoredLoweringWork, BoundOwnerSummary, CanonicalBaseRead, CanonicalDelta,
    CanonicalRead, CanonicalReadWork, ChangeBudget, DerivedDelta, PrimitiveEdit, SummaryDelta,
    TestDependencyDelta, WitnessBaseRead, WitnessMapBase, WitnessMapUpdate, WitnessRead,
    WitnessReadWork, WitnessRelationRead, WitnessTestDependencyRead, lower_authored_changes,
    prepare_change_analysis_with_budget, update_witness_maps_from,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    BlobObjectDigest, DependencyRecord, EncodedOwnerKey, KernelSnapshot, OwnerKey, OwnerRecord,
    PackageId, PackageInterfaceRecord, RelationEdge, RelationEndpoint, RelationKind,
    RetirementRecord, TypeObject, TypeObjectDigest, decode_dependency, decode_dependency_binding,
    decode_owner, decode_owner_binding, decode_retirement, decode_retirement_binding,
    decode_type_object, dependency_map_key, owner_map_key, retirement_map_key,
};
use crate::platform::package_interface::{
    PackageInterfaceOwner, PackageInterfaceSelection, PackageInterfaceValidation,
    build_package_interface, decode_package_interface_binding,
};
use crate::platform::package_object::{
    MAXIMUM_PACKAGE_OBJECT_DEPENDENCIES, PackageObject, validate_package_object_closure,
    validate_package_object_closure_with_interface,
};
use crate::platform::persistent_map::{MapError, MapErrorClass, MapRoot, MapWork, PersistentMap};
use crate::platform::semantic_id::RevisionId;
use crate::platform::storage::contract::TARGET_PACK_BYTES;
use crate::platform::storage::directory::PackDirectoryStore;
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, ObjectStage, StoreError, StoreErrorClass,
    StoreWork,
};
use crate::platform::storage::pack::PackBuilder;
use crate::platform::storage::page_store::ObjectPageReader;
use crate::platform::witness::{
    MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS, NamespaceKey, OwnerSummary, OwnershipEntry,
    SummaryBinding, TestDependency, decode_forward_relation_key, decode_owner_summary,
    decode_ownership, decode_reverse_relation_key, decode_test_dependency_forward_key,
    forward_relation_prefix, owner_key_bytes, reverse_relation_package_owner_prefix,
    reverse_relation_prefix, test_dependency_forward_prefix,
};
use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

pub const MAXIMUM_RELATION_READ_ITEMS: usize =
    crate::platform::witness::MAXIMUM_RELATION_PREFIX_ITEMS;
pub const MAXIMUM_TEST_DEPENDENCY_READ_ITEMS: usize = MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS;
const RELATION_LIMIT_STOP: &str = "publication_relation_limit_stop";
const TEST_DEPENDENCY_LIMIT_STOP: &str = "publication_test_dependency_limit_stop";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RepositoryReadWork {
    pub map: MapWork,
    pub store: StoreWork,
    pub canonical_records_decoded: u64,
    pub witness_records_decoded: u64,
    pub items_returned: u64,
}

impl RepositoryReadWork {
    fn add(&mut self, other: Self) {
        self.add_map(other.map);
        self.store.add(other.store);
        self.canonical_records_decoded = self
            .canonical_records_decoded
            .saturating_add(other.canonical_records_decoded);
        self.witness_records_decoded = self
            .witness_records_decoded
            .saturating_add(other.witness_records_decoded);
        self.items_returned = self.items_returned.saturating_add(other.items_returned);
    }

    fn add_map(&mut self, other: MapWork) {
        self.map.pages_read = self.map.pages_read.saturating_add(other.pages_read);
        self.map.pages_decoded = self.map.pages_decoded.saturating_add(other.pages_decoded);
        self.map.pages_encoded = self.map.pages_encoded.saturating_add(other.pages_encoded);
        self.map.pages_written = self.map.pages_written.saturating_add(other.pages_written);
        self.map.pages_reused = self.map.pages_reused.saturating_add(other.pages_reused);
        self.map.bytes_read = self.map.bytes_read.saturating_add(other.bytes_read);
        self.map.bytes_encoded = self.map.bytes_encoded.saturating_add(other.bytes_encoded);
        self.map.bytes_written = self.map.bytes_written.saturating_add(other.bytes_written);
        self.map.key_comparisons = self
            .map
            .key_comparisons
            .saturating_add(other.key_comparisons);
        self.map.entries_visited = self
            .map
            .entries_visited
            .saturating_add(other.entries_visited);
        self.map.differences_emitted = self
            .map
            .differences_emitted
            .saturating_add(other.differences_emitted);
        self.map.subtrees_skipped = self
            .map
            .subtrees_skipped
            .saturating_add(other.subtrees_skipped);
        self.map.entries_skipped = self
            .map
            .entries_skipped
            .saturating_add(other.entries_skipped);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionRead<T> {
    pub revision: RevisionId,
    pub value: T,
    pub work: RepositoryReadWork,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationRead {
    pub edges: Vec<RelationEdge>,
    pub truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestDependencyRead {
    pub dependencies: Vec<TestDependency>,
    pub truncated: bool,
}

#[derive(Clone, Debug)]
pub struct RevisionWitnessMapUpdate {
    pub revision: RevisionId,
    pub update: WitnessMapUpdate,
    pub store_work: StoreWork,
}

#[derive(Clone, Debug)]
pub struct ExportedPackageObject {
    pub object: PackageObject,
    pub digest: crate::platform::kernel::PackageObjectDigest,
    pub packs: Vec<Vec<u8>>,
    pub interface_owner_count: u64,
    pub interface_type_count: u64,
    pub interface_map_work: MapWork,
    pub read_work: RepositoryReadWork,
    pub closure_work: StoreWork,
}

/// One fully validated authored request plus its exact request-local identity allocation.
#[derive(Clone, Debug)]
pub struct PreparedAuthoredPublication {
    pub publication: PreparedPublication,
    pub allocated: BTreeMap<String, OwnerKey>,
    pub lowering_work: AuthoredLoweringWork,
}

/// One immutable catalog snapshot plus the exact accepted revision it was opened against.
///
/// Packs are append-only in the current Graph 5 store, so a later HEAD publication cannot alter
/// any object visible through this view. Future physical deletion must add an explicit lease
/// before it may coexist with these views.
#[derive(Debug)]
pub struct RepositoryView {
    current: CurrentPublication,
    store: PackDirectoryStore,
}

impl RepositoryView {
    pub(super) const fn new(current: CurrentPublication, store: PackDirectoryStore) -> Self {
        Self { current, store }
    }

    pub const fn revision(&self) -> RevisionId {
        self.current.head.revision
    }

    pub const fn package(&self) -> PackageId {
        self.current.semantic_root.package_id
    }

    pub const fn current(&self) -> &CurrentPublication {
        &self.current
    }

    /// Normalizes, validates, and stages one change against this view's exact immutable revision.
    /// Publication still rechecks the expected HEAD under the repository lock.
    pub fn prepare_change(
        &self,
        edits: Vec<PrimitiveEdit>,
        options: PublicationOptions,
    ) -> Result<PreparedPublication, Vec<Diagnostic>> {
        self.prepare_change_with_prior_work(
            edits,
            options,
            AuthoredLoweringWork::default(),
            ChangeBudget::default(),
        )
    }

    /// Resolves one strict authored request at this view's exact revision and prepares its one
    /// generic canonical publication. Request-local allocations are returned separately from
    /// accepted authority so callers can render a compact protocol receipt.
    pub fn prepare_authored_change(
        &self,
        request: &AuthoredChangeSet,
        options: PublicationOptions,
    ) -> Result<PreparedAuthoredPublication, Vec<Diagnostic>> {
        let lowering =
            lower_authored_changes(self, self, request, options.idempotency_key.as_deref())
                .map_err(|diagnostic| vec![diagnostic])?;
        let publication = self.prepare_change_with_prior_work(
            lowering.edits,
            options,
            lowering.work,
            request.budget,
        )?;
        Ok(PreparedAuthoredPublication {
            publication,
            allocated: lowering.allocated,
            lowering_work: lowering.work,
        })
    }

    /// Decodes operational options from the strict Change Contract 5 envelope and returns the
    /// compact response projection derived from the same prepared publication.
    pub fn prepare_authored_protocol_change(
        &self,
        request: &AuthoredChangeRequest,
    ) -> Result<(PreparedAuthoredPublication, AuthoredChangeResponse), Vec<Diagnostic>> {
        request.validate().map_err(|diagnostic| vec![diagnostic])?;
        let prepared =
            self.prepare_authored_change(&request.semantic, request.publication_options())?;
        let response =
            AuthoredChangeResponse::prepared(&prepared).map_err(|diagnostic| vec![diagnostic])?;
        Ok((prepared, response))
    }

    fn prepare_change_with_prior_work(
        &self,
        edits: Vec<PrimitiveEdit>,
        options: PublicationOptions,
        prior_work: AuthoredLoweringWork,
        budget: ChangeBudget,
    ) -> Result<PreparedPublication, Vec<Diagnostic>> {
        let normalization =
            CanonicalDelta::normalize_from(self, edits).map_err(|diagnostic| vec![diagnostic])?;
        if normalization.base_revision != Some(self.revision()) {
            return Err(vec![read_error(
                DiagnosticClass::Corrupt,
                "publication_prepare_revision_binding",
                "canonical normalization did not retain the pinned repository revision",
            )]);
        }
        let mut budget_reads = prior_work;
        budget_reads.canonical.add(normalization.work);
        let initial_budget_work = budget_reads.budget_work();
        let mut analysis = prepare_change_analysis_with_budget(
            self,
            self,
            normalization.canonical,
            budget,
            initial_budget_work,
        )?;
        analysis.canonical_read_work.add(prior_work.canonical);
        analysis.canonical_read_work.add(normalization.work);
        analysis.witness_read_work.add(prior_work.witness);
        prepare_change_publication(
            self.current.accepted,
            self,
            self,
            &analysis,
            &self.store,
            options,
        )
    }

    pub fn owner(&self, owner: OwnerKey) -> Result<RevisionRead<Option<OwnerRecord>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(binding_bytes) = self.lookup_map(
            self.current.semantic_root.owners,
            &owner_map_key(owner),
            &mut work,
        )?
        else {
            return Ok(self.read(None, work));
        };
        let binding = decode_owner_binding(&binding_bytes, owner)?;
        let bytes = self.read_required_object(
            ObjectDomain::Owner,
            binding.object.bytes(),
            "accepted owner binding references a missing owner object",
            &mut work,
        )?;
        let record = decode_owner(&bytes, owner, binding.kind, binding.object)?;
        work.canonical_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(Some(record), work))
    }

    pub fn type_object(
        &self,
        digest: TypeObjectDigest,
    ) -> Result<RevisionRead<Option<TypeObject>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(bytes) =
            self.read_optional_object(ObjectDomain::Type, digest.bytes(), &mut work)?
        else {
            return Ok(self.read(None, work));
        };
        let object = decode_type_object(&bytes, digest)?;
        work.canonical_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(Some(object), work))
    }

    pub fn dependency(
        &self,
        package: PackageId,
    ) -> Result<RevisionRead<Option<DependencyRecord>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(binding_bytes) = self.lookup_map(
            self.current.semantic_root.dependencies,
            &dependency_map_key(package),
            &mut work,
        )?
        else {
            return Ok(self.read(None, work));
        };
        let binding = decode_dependency_binding(&binding_bytes)?;
        let bytes = self.read_required_object(
            ObjectDomain::Dependency,
            binding.object.bytes(),
            "accepted dependency binding references a missing dependency object",
            &mut work,
        )?;
        let record = decode_dependency(&bytes, &package, binding.object)?;
        work.canonical_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(Some(record), work))
    }

    /// Resolves one implementation-free owner from the exact package object selected by the
    /// pinned local dependency binding. Work follows one dependency-map path, one package object,
    /// one interface-map path, and one interface owner object.
    pub fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<RevisionRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        let dependency = self.dependency(package)?;
        let mut work = dependency.work;
        let Some(dependency) = dependency.value else {
            return Ok(self.read(None, work));
        };
        let exact = self.package_interface_owner_from_dependency(&dependency, owner)?;
        work.add(exact.work);
        Ok(self.read(exact.value, work))
    }

    /// Resolves one implementation-free owner through an explicit exact dependency binding.
    /// This path admits a dependency newly added by the current candidate overlay while retaining
    /// the same package-object, revision, map, and owner integrity checks as accepted reads.
    fn package_interface_owner_from_dependency(
        &self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
    ) -> Result<RevisionRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let package_bytes = self.read_required_object(
            ObjectDomain::PackageObject,
            dependency.package_object.bytes(),
            "dependency binding references a missing package object",
            &mut work,
        )?;
        let package_object = PackageObject::decode(&package_bytes, dependency.package_object)?;
        package_object.matches_dependency(dependency)?;
        let Some(binding_bytes) = self.lookup_map(
            package_object.interface_owners,
            &crate::platform::kernel::owner_map_key(owner),
            &mut work,
        )?
        else {
            return Ok(self.read(None, work));
        };
        let digest = decode_package_interface_binding(&binding_bytes)?;
        let bytes = self.read_required_object(
            ObjectDomain::PackageInterface,
            digest.bytes(),
            "package-interface map references a missing owner object",
            &mut work,
        )?;
        let value = PackageInterfaceOwner::decode(&bytes, owner, digest)?;
        work.canonical_records_decoded = work.canonical_records_decoded.saturating_add(1);
        work.items_returned = 1;
        Ok(self.read(Some(value.record), work))
    }

    /// Reconstructs the complete logical Graph 5 view for independent full validation and
    /// witness comparison. This is an explicitly broad oracle operation: ordinary reads and
    /// changes continue to use exact point and prefix lookups through this revision-pinned view.
    pub fn reconstruct_full_oracle(&self) -> Result<RevisionRead<KernelSnapshot>, Diagnostic> {
        let declared_records = self
            .current
            .semantic_root
            .owners
            .entries()
            .checked_add(self.current.semantic_root.dependencies.entries())
            .and_then(|count| count.checked_add(self.current.semantic_root.retirements.entries()))
            .ok_or_else(|| {
                read_error(
                    DiagnosticClass::Resource,
                    "publication_full_oracle_record_count",
                    "full-oracle canonical record count overflowed",
                )
            })?;
        if declared_records > crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK as u64 {
            return Err(full_oracle_work_error());
        }
        let mut consumed = usize::try_from(declared_records).map_err(|_| {
            read_error(
                DiagnosticClass::Resource,
                "publication_full_oracle_record_count",
                "full-oracle canonical record count does not fit this platform",
            )
        })?;
        let mut work = RepositoryReadWork::default();

        let mut owners = BTreeMap::new();
        work.add(self.for_each_owner_record(|owner, record| {
            if owners.insert(owner, record.clone()).is_some() {
                return Err(read_error(
                    DiagnosticClass::Corrupt,
                    "publication_full_oracle_owner_duplicate",
                    "owner map reconstructed one stable identity more than once",
                ));
            }
            Ok(())
        })?);
        if owners.len() as u64 != self.current.semantic_root.owners.entries() {
            return Err(read_error(
                DiagnosticClass::Corrupt,
                "publication_full_oracle_owner_count",
                "reconstructed owner count disagrees with the accepted semantic root",
            ));
        }

        let dependency_read = self.package_dependencies()?;
        work.add(dependency_read.work);
        let mut dependencies = BTreeMap::new();
        for dependency in dependency_read.value {
            let package = dependency.package;
            if dependencies.insert(package, dependency).is_some() {
                return Err(read_error(
                    DiagnosticClass::Corrupt,
                    "publication_full_oracle_dependency_duplicate",
                    "dependency map reconstructed one package identity more than once",
                ));
            }
        }
        if dependencies.len() as u64 != self.current.semantic_root.dependencies.entries() {
            return Err(read_error(
                DiagnosticClass::Corrupt,
                "publication_full_oracle_dependency_count",
                "reconstructed dependency count disagrees with the accepted semantic root",
            ));
        }

        let retirement_read = self.all_retirements()?;
        work.add(retirement_read.work);
        let retirements = retirement_read.value;
        if retirements.len() as u64 != self.current.semantic_root.retirements.entries() {
            return Err(read_error(
                DiagnosticClass::Corrupt,
                "publication_full_oracle_retirement_count",
                "reconstructed retirement count disagrees with the accepted semantic root",
            ));
        }

        let mut types = BTreeMap::new();
        let mut pending_types = owners
            .values()
            .flat_map(OwnerRecord::type_roots)
            .collect::<BTreeSet<_>>();
        while let Some(digest) = pending_types.pop_first() {
            if types.contains_key(&digest) {
                continue;
            }
            consume_full_oracle_work(&mut consumed, 1)?;
            let read = self.type_object(digest)?;
            work.add(read.work);
            let object = read.value.ok_or_else(|| {
                read_error(
                    DiagnosticClass::Corrupt,
                    "publication_full_oracle_type_missing",
                    format!("accepted owner authority references missing type object {digest}"),
                )
            })?;
            pending_types.extend(object.child_types());
            types.insert(digest, object);
        }

        let mut declared_blobs = BTreeMap::<BlobObjectDigest, u64>::new();
        for (digest, bytes) in owners.values().flat_map(OwnerRecord::blob_roots) {
            if let Some(previous) = declared_blobs.insert(digest, bytes)
                && previous != bytes
            {
                return Err(read_error(
                    DiagnosticClass::Corrupt,
                    "publication_full_oracle_blob_binding",
                    format!(
                        "blob {digest} is referenced with conflicting lengths {previous} and {bytes}"
                    ),
                ));
            }
        }
        let mut blobs = BTreeMap::new();
        for (digest, expected_bytes) in declared_blobs {
            consume_full_oracle_work(&mut consumed, 1)?;
            let bytes = self.read_required_object(
                ObjectDomain::Blob,
                digest.bytes(),
                "accepted owner authority references a missing blob object",
                &mut work,
            )?;
            let actual_bytes = u64::try_from(bytes.len()).map_err(|_| {
                read_error(
                    DiagnosticClass::Resource,
                    "publication_full_oracle_blob_length",
                    "blob byte length does not fit the canonical length domain",
                )
            })?;
            if actual_bytes != expected_bytes {
                return Err(read_error(
                    DiagnosticClass::Corrupt,
                    "publication_full_oracle_blob_length",
                    format!(
                        "blob {digest} is bound as {expected_bytes} bytes but contains {actual_bytes}"
                    ),
                ));
            }
            blobs.insert(digest, actual_bytes);
        }

        let mut dependency_interfaces = BTreeMap::new();
        let mut dependency_types = BTreeMap::new();
        for dependency in dependencies.values() {
            let mut closure_work = StoreWork::default();
            let validated = validate_package_object_closure_with_interface(
                &self.store,
                dependency.package_object,
                Some(dependency),
                &mut closure_work,
            )?;
            work.store.add(closure_work);
            let PackageInterfaceValidation {
                owners: interface_owners,
                type_objects: interface_types,
                reachable_objects: _,
                map_work,
            } = validated.root_interface;
            work.add_map(map_work);
            let interface_items = interface_owners
                .len()
                .checked_add(interface_types.len())
                .ok_or_else(full_oracle_work_error)?;
            consume_full_oracle_work(&mut consumed, interface_items)?;
            work.canonical_records_decoded = work
                .canonical_records_decoded
                .saturating_add(interface_owners.len() as u64)
                .saturating_add(interface_types.len() as u64);
            let records = interface_owners
                .into_iter()
                .map(|(owner, value)| (owner, value.record))
                .collect::<BTreeMap<_, _>>();
            match dependency_interfaces.entry(dependency.package_object) {
                Entry::Vacant(entry) => {
                    entry.insert(records);
                }
                Entry::Occupied(entry) if entry.get() != &records => {
                    return Err(read_error(
                        DiagnosticClass::Corrupt,
                        "publication_full_oracle_interface_conflict",
                        "one exact package object reconstructed two different interfaces",
                    ));
                }
                Entry::Occupied(_) => {}
            }
            for (digest, object) in interface_types {
                match dependency_types.entry(digest) {
                    Entry::Vacant(entry) => {
                        entry.insert(object);
                    }
                    Entry::Occupied(entry) if entry.get() != &object => {
                        return Err(read_error(
                            DiagnosticClass::Corrupt,
                            "publication_full_oracle_dependency_type_conflict",
                            "one dependency type digest reconstructed two different objects",
                        ));
                    }
                    Entry::Occupied(_) => {}
                }
            }
        }

        let returned_items = [
            owners.len(),
            types.len(),
            blobs.len(),
            dependencies.len(),
            retirements.len(),
            dependency_types.len(),
        ]
        .into_iter()
        .try_fold(0_usize, usize::checked_add)
        .and_then(|count| {
            dependency_interfaces
                .values()
                .try_fold(count, |count, records| count.checked_add(records.len()))
        })
        .ok_or_else(|| {
            read_error(
                DiagnosticClass::Resource,
                "publication_full_oracle_result_count",
                "full-oracle result item count overflowed",
            )
        })?;
        work.items_returned = u64::try_from(returned_items).map_err(|_| {
            read_error(
                DiagnosticClass::Resource,
                "publication_full_oracle_result_count",
                "full-oracle result item count does not fit its observation domain",
            )
        })?;
        Ok(self.read(
            KernelSnapshot {
                root: self.current.semantic_root.clone(),
                owners,
                types,
                dependency_interfaces,
                dependency_types,
                blobs,
                dependencies,
                retirements,
            },
            work,
        ))
    }

    /// Builds one exact package descriptor from the pinned accepted revision. This explicit
    /// export may enumerate direct dependencies, but it does not reconstruct owner authority.
    pub fn export_package_object(&self) -> Result<ExportedPackageObject, Diagnostic> {
        let dependencies = self.package_dependencies()?;
        let mut selection = PackageInterfaceSelection::new(self.package());
        let mut read_work = dependencies.work;
        read_work
            .add(self.for_each_owner_record(|_, record| selection.observe_declaration(record))?);
        read_work.add(self.for_each_owner_record(|_, record| selection.observe_operation(record))?);

        let mut interface_owners = BTreeMap::new();
        let mut type_roots = std::collections::BTreeSet::new();
        for owner in selection.owners() {
            let canonical = self.owner(owner)?;
            read_work.add(canonical.work);
            let canonical = canonical.value.ok_or_else(|| {
                read_error(
                    DiagnosticClass::Corrupt,
                    "publication_package_interface_owner_missing",
                    "public interface selection names a missing canonical owner",
                )
            })?;
            let summary = self.bound_owner_summary(owner)?;
            read_work.add(summary.work);
            let summary = summary.value.ok_or_else(|| {
                read_error(
                    DiagnosticClass::Corrupt,
                    "publication_package_interface_summary_missing",
                    "public interface selection names an owner without a committed summary",
                )
            })?;
            let interface =
                PackageInterfaceOwner::project(&canonical, &summary.summary, &selection)?
                    .ok_or_else(|| {
                        read_error(
                            DiagnosticClass::Corrupt,
                            "publication_package_interface_projection",
                            "selected public owner produced no package-interface record",
                        )
                    })?;
            type_roots.extend(interface.type_roots());
            interface_owners.insert(owner, interface);
        }
        let mut interface_types = BTreeMap::new();
        let mut pending = type_roots.into_iter().collect::<Vec<_>>();
        while let Some(digest) = pending.pop() {
            if interface_types.contains_key(&digest) {
                continue;
            }
            let read = self.type_object(digest)?;
            read_work.add(read.work);
            let object = read.value.ok_or_else(|| {
                read_error(
                    DiagnosticClass::Corrupt,
                    "publication_package_interface_type_missing",
                    "public interface references a missing canonical type object",
                )
            })?;
            pending.extend(object.child_types());
            let (encoded_digest, bytes) = crate::platform::kernel::encode_type_object(&object)?;
            if encoded_digest != digest {
                return Err(read_error(
                    DiagnosticClass::Corrupt,
                    "publication_package_interface_type_digest",
                    "public interface type changed during canonical re-encoding",
                ));
            }
            interface_types.insert(digest, bytes);
        }
        let interface = build_package_interface(&interface_owners, &interface_types)?;
        let object = PackageObject {
            contract_version: crate::platform::package_object::PACKAGE_OBJECT_CONTRACT_VERSION,
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: self.current.semantic_root.repository_id,
            package: self.current.semantic_root.package_id,
            semantic_revision: self.current.head.revision,
            semantic_root: self.current.accepted.semantic_root,
            validation_witness: self.current.accepted.validation_witness,
            witness: self.current.witness.clone(),
            interface_owners: interface.root,
            dependencies: dependencies.value,
        };
        let (digest, bytes) = object.encode()?;
        let mut stage = ObjectStage::new(&self.store);
        let mut closure_work = StoreWork::default();
        for (key, value) in &interface.objects {
            stage
                .stage(*key, value, &mut closure_work)
                .map_err(store_diagnostic)?;
        }
        stage
            .stage(
                ObjectKey::from_digest(ObjectDomain::PackageObject, digest.bytes()),
                &bytes,
                &mut closure_work,
            )
            .map_err(store_diagnostic)?;
        let validated = validate_package_object_closure(&stage, digest, None, &mut closure_work)?;
        if validated != object {
            return Err(read_error(
                DiagnosticClass::Corrupt,
                "publication_package_object_export",
                "exported package object changed during exact closure validation",
            ));
        }
        let mut builder = PackBuilder::default();
        for (key, value) in &interface.objects {
            builder.insert(*key, value).map_err(store_diagnostic)?;
        }
        builder
            .insert(
                ObjectKey::from_digest(ObjectDomain::PackageObject, digest.bytes()),
                &bytes,
            )
            .map_err(store_diagnostic)?;
        let packs = builder
            .seal_targeted(TARGET_PACK_BYTES)
            .map_err(store_diagnostic)?
            .into_iter()
            .map(|pack| pack.bytes)
            .collect();
        Ok(ExportedPackageObject {
            object,
            digest,
            packs,
            interface_owner_count: interface.owner_count,
            interface_type_count: interface.type_count,
            interface_map_work: interface.map_work,
            read_work,
            closure_work,
        })
    }

    fn for_each_owner_record(
        &self,
        mut visitor: impl FnMut(OwnerKey, &OwnerRecord) -> Result<(), Diagnostic>,
    ) -> Result<RepositoryReadWork, Diagnostic> {
        let reader = ObjectPageReader::new(&self.store);
        let mut map_work = MapWork::default();
        let mut object_work = StoreWork::default();
        let mut captured = None;
        let result = PersistentMap::from_root(self.current.semantic_root.owners).for_each(
            &reader,
            &mut map_work,
            |key, value| {
                let operation = (|| {
                    let owner = EncodedOwnerKey::decode(key)?;
                    let binding = decode_owner_binding(value, owner)?;
                    let object_key =
                        ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
                    let bytes = self
                        .store
                        .read(
                            object_key,
                            ObjectDomain::Owner.maximum_bytes(),
                            &mut object_work,
                        )
                        .map_err(store_diagnostic)?
                        .ok_or_else(|| {
                            read_error(
                                DiagnosticClass::Corrupt,
                                "publication_package_owner_missing",
                                "package export found a missing canonical owner object",
                            )
                        })?;
                    let record = decode_owner(&bytes, owner, binding.kind, binding.object)?;
                    visitor(owner, &record)
                })();
                match operation {
                    Ok(()) => Ok(()),
                    Err(diagnostic) => {
                        captured = Some(diagnostic);
                        Err(MapError {
                            class: MapErrorClass::Corrupt,
                            code: "publication_package_owner_stop",
                            message: "owner scan stopped after an exact diagnostic".to_owned(),
                        })
                    }
                }
            },
        );
        let mut work = RepositoryReadWork {
            map: map_work,
            store: reader.work(),
            canonical_records_decoded: self.current.semantic_root.owners.entries(),
            items_returned: self.current.semantic_root.owners.entries(),
            ..RepositoryReadWork::default()
        };
        work.store.add(object_work);
        if let Some(diagnostic) = captured {
            return Err(diagnostic);
        }
        result.map_err(map_diagnostic)?;
        Ok(work)
    }

    fn package_dependencies(&self) -> Result<RevisionRead<Vec<DependencyRecord>>, Diagnostic> {
        let count = self.current.semantic_root.dependencies.entries();
        if count > MAXIMUM_PACKAGE_OBJECT_DEPENDENCIES as u64 {
            return Err(read_error(
                DiagnosticClass::Resource,
                "publication_package_dependency_count",
                format!(
                    "package has {count} dependencies; current package-object export supports at most {MAXIMUM_PACKAGE_OBJECT_DEPENDENCIES}"
                ),
            ));
        }
        let reader = ObjectPageReader::new(&self.store);
        let mut map_work = MapWork::default();
        let mut bindings = Vec::with_capacity(count as usize);
        PersistentMap::from_root(self.current.semantic_root.dependencies)
            .for_each(&reader, &mut map_work, |key, value| {
                bindings.push((key.to_vec(), value.to_vec()));
                Ok(())
            })
            .map_err(map_diagnostic)?;
        let mut work = RepositoryReadWork {
            map: map_work,
            store: reader.work(),
            ..RepositoryReadWork::default()
        };
        let mut dependencies = Vec::with_capacity(bindings.len());
        for (key, binding_bytes) in bindings {
            let package_bytes: [u8; 16] = key.try_into().map_err(|_| {
                read_error(
                    DiagnosticClass::Corrupt,
                    "publication_package_dependency_key",
                    "dependency map contains a noncanonical package key",
                )
            })?;
            let package = PackageId::from_bytes(package_bytes).ok_or_else(|| {
                read_error(
                    DiagnosticClass::Corrupt,
                    "publication_package_dependency_key",
                    "dependency map contains the reserved all-zero package identity",
                )
            })?;
            let binding = decode_dependency_binding(&binding_bytes)?;
            let bytes = self.read_required_object(
                ObjectDomain::Dependency,
                binding.object.bytes(),
                "package export found a missing dependency object",
                &mut work,
            )?;
            dependencies.push(decode_dependency(&bytes, &package, binding.object)?);
        }
        work.canonical_records_decoded = dependencies.len() as u64;
        work.items_returned = dependencies.len() as u64;
        Ok(self.read(dependencies, work))
    }

    fn all_retirements(
        &self,
    ) -> Result<RevisionRead<BTreeMap<OwnerKey, RetirementRecord>>, Diagnostic> {
        let reader = ObjectPageReader::new(&self.store);
        let mut map_work = MapWork::default();
        let mut bindings = Vec::new();
        PersistentMap::from_root(self.current.semantic_root.retirements)
            .for_each(&reader, &mut map_work, |key, value| {
                bindings.push((key.to_vec(), value.to_vec()));
                Ok(())
            })
            .map_err(map_diagnostic)?;
        let mut work = RepositoryReadWork {
            map: map_work,
            store: reader.work(),
            ..RepositoryReadWork::default()
        };
        let mut retirements = BTreeMap::new();
        for (key_bytes, binding_bytes) in bindings {
            let owner = EncodedOwnerKey::decode(&key_bytes)?;
            let binding = decode_retirement_binding(&binding_bytes)?;
            let bytes = self.read_required_object(
                ObjectDomain::Retirement,
                binding.object.bytes(),
                "full-oracle reconstruction found a missing retirement object",
                &mut work,
            )?;
            let record = decode_retirement(&bytes, owner, binding.object)?;
            if retirements.insert(owner, record).is_some() {
                return Err(read_error(
                    DiagnosticClass::Corrupt,
                    "publication_full_oracle_retirement_duplicate",
                    "retirement map reconstructed one owner identity more than once",
                ));
            }
        }
        work.canonical_records_decoded = retirements.len() as u64;
        work.items_returned = retirements.len() as u64;
        Ok(self.read(retirements, work))
    }

    pub fn retirement(
        &self,
        owner: OwnerKey,
    ) -> Result<RevisionRead<Option<RetirementRecord>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(binding_bytes) = self.lookup_map(
            self.current.semantic_root.retirements,
            &retirement_map_key(owner),
            &mut work,
        )?
        else {
            return Ok(self.read(None, work));
        };
        let binding = decode_retirement_binding(&binding_bytes)?;
        let bytes = self.read_required_object(
            ObjectDomain::Retirement,
            binding.object.bytes(),
            "accepted retirement binding references a missing retirement object",
            &mut work,
        )?;
        let record = decode_retirement(&bytes, owner, binding.object)?;
        work.canonical_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(Some(record), work))
    }

    pub fn namespace(
        &self,
        key: &NamespaceKey,
    ) -> Result<RevisionRead<Option<OwnerKey>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let value = self.lookup_map(
            self.current.witness.roots.namespaces,
            &key.encode(),
            &mut work,
        )?;
        let value = value.as_deref().map(EncodedOwnerKey::decode).transpose()?;
        work.witness_records_decoded = u64::from(value.is_some());
        work.items_returned = u64::from(value.is_some());
        Ok(self.read(value, work))
    }

    pub fn ownership(
        &self,
        owner: OwnerKey,
    ) -> Result<RevisionRead<Option<OwnershipEntry>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let value = self.lookup_map(
            self.current.witness.roots.ownership,
            &owner_key_bytes(owner),
            &mut work,
        )?;
        let value = value.as_deref().map(decode_ownership).transpose()?;
        work.witness_records_decoded = u64::from(value.is_some());
        work.items_returned = u64::from(value.is_some());
        Ok(self.read(value, work))
    }

    pub fn owner_summary(
        &self,
        owner: OwnerKey,
    ) -> Result<RevisionRead<Option<OwnerSummary>>, Diagnostic> {
        let read = self.bound_owner_summary(owner)?;
        Ok(RevisionRead {
            revision: read.revision,
            value: read.value.map(|bound| bound.summary),
            work: read.work,
        })
    }

    pub fn bound_owner_summary(
        &self,
        owner: OwnerKey,
    ) -> Result<RevisionRead<Option<BoundOwnerSummary>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(binding_bytes) = self.lookup_map(
            self.current.witness.roots.owner_summaries,
            &owner_key_bytes(owner),
            &mut work,
        )?
        else {
            return Ok(self.read(None, work));
        };
        let binding = SummaryBinding::decode(&binding_bytes, owner)?;
        let bytes = self.read_required_object(
            ObjectDomain::OwnerSummary,
            binding.summary.bytes(),
            "accepted witness binding references a missing owner-summary object",
            &mut work,
        )?;
        let summary = decode_owner_summary(&bytes, binding.summary)?;
        if summary.owner != owner || summary.kind != binding.kind {
            return Err(read_error(
                DiagnosticClass::Corrupt,
                "publication_summary_binding",
                "owner-summary object disagrees with its exact witness binding",
            ));
        }
        work.witness_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(
            Some(BoundOwnerSummary {
                digest: binding.summary,
                summary,
            }),
            work,
        ))
    }

    pub fn contains_forward_relation(
        &self,
        edge: RelationEdge,
    ) -> Result<RevisionRead<bool>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let value = self.lookup_map(
            self.current.witness.roots.forward_relations,
            &crate::platform::witness::forward_relation_key(edge),
            &mut work,
        )?;
        if value.as_ref().is_some_and(|bytes| !bytes.is_empty()) {
            return Err(read_error(
                DiagnosticClass::Corrupt,
                "publication_relation_value",
                "relation witness entry has a nonempty value",
            ));
        }
        work.witness_records_decoded = u64::from(value.is_some());
        work.items_returned = u64::from(value.is_some());
        Ok(self.read(value.is_some(), work))
    }

    pub fn update_witness_maps(
        &self,
        derived: &DerivedDelta,
        summaries: &SummaryDelta,
        tests: &TestDependencyDelta,
    ) -> Result<RevisionWitnessMapUpdate, Diagnostic> {
        let reader = ObjectPageReader::new(&self.store);
        let update =
            update_witness_maps_from(&self.current.witness, &reader, derived, summaries, tests)?;
        Ok(RevisionWitnessMapUpdate {
            revision: self.revision(),
            update,
            store_work: reader.work(),
        })
    }

    pub fn outgoing_relations(
        &self,
        source: RelationEndpoint,
        kind: Option<RelationKind>,
        maximum_items: usize,
    ) -> Result<RevisionRead<RelationRead>, Diagnostic> {
        self.relations(source, kind, maximum_items, false)
    }

    pub fn incoming_relations(
        &self,
        target: RelationEndpoint,
        kind: Option<RelationKind>,
        maximum_items: usize,
    ) -> Result<RevisionRead<RelationRead>, Diagnostic> {
        self.relations(target, kind, maximum_items, true)
    }

    pub fn incoming_package_relations(
        &self,
        package: PackageId,
        maximum_items: usize,
    ) -> Result<RevisionRead<RelationRead>, Diagnostic> {
        if maximum_items == 0 || maximum_items > MAXIMUM_RELATION_READ_ITEMS {
            return Err(read_error(
                DiagnosticClass::Resource,
                "publication_relation_item_budget",
                format!("relation item budget must be 1 through {MAXIMUM_RELATION_READ_ITEMS}"),
            ));
        }
        let prefix = reverse_relation_package_owner_prefix(package);
        let mut work = RepositoryReadWork::default();
        let mut keys = Vec::with_capacity(maximum_items.min(64));
        let mut truncated = false;
        let reader = ObjectPageReader::new(&self.store);
        let mut map_work = MapWork::default();
        let result = PersistentMap::from_root(self.current.witness.roots.reverse_relations)
            .for_each_prefix(&reader, &prefix, &mut map_work, |key, value| {
                if !value.is_empty() {
                    return Err(MapError {
                        class: MapErrorClass::Corrupt,
                        code: "publication_relation_value",
                        message: "relation witness entry has a nonempty value".to_owned(),
                    });
                }
                if keys.len() == maximum_items {
                    truncated = true;
                    return Err(MapError {
                        class: MapErrorClass::Resource,
                        code: RELATION_LIMIT_STOP,
                        message: "bounded package-relation prefix has more matching entries"
                            .to_owned(),
                    });
                }
                keys.push(key.to_vec());
                Ok(())
            });
        work.map = map_work;
        work.store = reader.work();
        if let Err(error) = result
            && error.code != RELATION_LIMIT_STOP
        {
            return Err(map_diagnostic(error));
        }
        let mut edges = Vec::with_capacity(keys.len());
        for key in keys {
            let edge = decode_reverse_relation_key(&key)?;
            if !matches!(
                edge.target,
                RelationEndpoint::Owner(crate::platform::kernel::ExactOwnerKey {
                    package: target_package,
                    ..
                }) if target_package == package
            ) {
                return Err(read_error(
                    DiagnosticClass::Corrupt,
                    "publication_relation_package_prefix",
                    "decoded relation does not agree with its selected package prefix",
                ));
            }
            edges.push(edge);
        }
        work.items_returned = edges.len() as u64;
        work.witness_records_decoded = edges.len() as u64;
        Ok(self.read(RelationRead { edges, truncated }, work))
    }

    pub fn test_dependencies(
        &self,
        test: OwnerKey,
        maximum_items: usize,
    ) -> Result<RevisionRead<TestDependencyRead>, Diagnostic> {
        if maximum_items == 0 || maximum_items > MAXIMUM_TEST_DEPENDENCY_READ_ITEMS {
            return Err(read_error(
                DiagnosticClass::Resource,
                "publication_test_dependency_item_budget",
                format!(
                    "test-dependency item budget must be 1 through {MAXIMUM_TEST_DEPENDENCY_READ_ITEMS}"
                ),
            ));
        }
        if !matches!(test, OwnerKey::Declaration(_)) {
            return Err(read_error(
                DiagnosticClass::Source,
                "publication_test_dependency_owner",
                "test-dependency lookup requires a declaration owner",
            ));
        }
        let prefix = test_dependency_forward_prefix(test);
        let mut work = RepositoryReadWork::default();
        let mut keys = Vec::with_capacity(maximum_items.min(64));
        let mut truncated = false;
        let reader = ObjectPageReader::new(&self.store);
        let mut map_work = MapWork::default();
        let result = PersistentMap::from_root(self.current.witness.roots.test_dependencies)
            .for_each_prefix(&reader, &prefix, &mut map_work, |key, value| {
                if !value.is_empty() {
                    return Err(MapError {
                        class: MapErrorClass::Corrupt,
                        code: "publication_test_dependency_value",
                        message: "test-dependency witness entry has a nonempty value".to_owned(),
                    });
                }
                if keys.len() == maximum_items {
                    truncated = true;
                    return Err(MapError {
                        class: MapErrorClass::Resource,
                        code: TEST_DEPENDENCY_LIMIT_STOP,
                        message: "bounded test-dependency prefix has more matching entries"
                            .to_owned(),
                    });
                }
                keys.push(key.to_vec());
                Ok(())
            });
        work.map = map_work;
        work.store = reader.work();
        if let Err(error) = result
            && error.code != TEST_DEPENDENCY_LIMIT_STOP
        {
            return Err(map_diagnostic(error));
        }
        let mut dependencies = Vec::with_capacity(keys.len());
        for key_bytes in keys {
            let dependency = decode_test_dependency_forward_key(&key_bytes)?;
            if dependency.test != test {
                return Err(read_error(
                    DiagnosticClass::Corrupt,
                    "publication_test_dependency_prefix",
                    "decoded test dependency does not agree with its selected witness prefix",
                ));
            }
            dependencies.push(dependency);
        }
        work.items_returned = dependencies.len() as u64;
        work.witness_records_decoded = dependencies.len() as u64;
        Ok(self.read(
            TestDependencyRead {
                dependencies,
                truncated,
            },
            work,
        ))
    }

    fn relations(
        &self,
        endpoint: RelationEndpoint,
        kind: Option<RelationKind>,
        maximum_items: usize,
        reverse: bool,
    ) -> Result<RevisionRead<RelationRead>, Diagnostic> {
        if maximum_items == 0 || maximum_items > MAXIMUM_RELATION_READ_ITEMS {
            return Err(read_error(
                DiagnosticClass::Resource,
                "publication_relation_item_budget",
                format!("relation item budget must be 1 through {MAXIMUM_RELATION_READ_ITEMS}"),
            ));
        }
        let prefix = if reverse {
            reverse_relation_prefix(endpoint, kind)
        } else {
            forward_relation_prefix(endpoint, kind)
        };
        let root = if reverse {
            self.current.witness.roots.reverse_relations
        } else {
            self.current.witness.roots.forward_relations
        };
        let mut work = RepositoryReadWork::default();
        let mut keys = Vec::with_capacity(maximum_items.min(64));
        let mut truncated = false;
        let reader = ObjectPageReader::new(&self.store);
        let mut map_work = MapWork::default();
        let result = PersistentMap::from_root(root).for_each_prefix(
            &reader,
            &prefix,
            &mut map_work,
            |key, value| {
                if !value.is_empty() {
                    return Err(MapError {
                        class: MapErrorClass::Corrupt,
                        code: "publication_relation_value",
                        message: "relation witness entry has a nonempty value".to_owned(),
                    });
                }
                if keys.len() == maximum_items {
                    truncated = true;
                    return Err(MapError {
                        class: MapErrorClass::Resource,
                        code: RELATION_LIMIT_STOP,
                        message: "bounded relation prefix has more matching entries".to_owned(),
                    });
                }
                keys.push(key.to_vec());
                Ok(())
            },
        );
        work.map = map_work;
        work.store = reader.work();
        if let Err(error) = result
            && error.code != RELATION_LIMIT_STOP
        {
            return Err(map_diagnostic(error));
        }
        let mut edges = Vec::with_capacity(keys.len());
        for key_bytes in keys {
            let edge = if reverse {
                decode_reverse_relation_key(&key_bytes)?
            } else {
                decode_forward_relation_key(&key_bytes)?
            };
            let observed = if reverse { edge.target } else { edge.source };
            if observed != endpoint || kind.is_some_and(|expected| edge.kind != expected) {
                return Err(read_error(
                    DiagnosticClass::Corrupt,
                    "publication_relation_prefix",
                    "decoded relation does not agree with its selected witness prefix",
                ));
            }
            edges.push(edge);
        }
        work.items_returned = edges.len() as u64;
        work.witness_records_decoded = edges.len() as u64;
        Ok(self.read(RelationRead { edges, truncated }, work))
    }

    fn lookup_map(
        &self,
        root: MapRoot,
        key: &[u8],
        work: &mut RepositoryReadWork,
    ) -> Result<Option<Vec<u8>>, Diagnostic> {
        let reader = ObjectPageReader::new(&self.store);
        let result = PersistentMap::from_root(root).lookup(&reader, key, &mut work.map);
        work.store = reader.work();
        result.map_err(map_diagnostic)
    }

    fn read_optional_object(
        &self,
        domain: ObjectDomain,
        digest: [u8; 32],
        work: &mut RepositoryReadWork,
    ) -> Result<Option<Vec<u8>>, Diagnostic> {
        self.store
            .read(
                ObjectKey::from_digest(domain, digest),
                domain.maximum_bytes(),
                &mut work.store,
            )
            .map_err(store_diagnostic)
    }

    fn read_required_object(
        &self,
        domain: ObjectDomain,
        digest: [u8; 32],
        missing: &'static str,
        work: &mut RepositoryReadWork,
    ) -> Result<Vec<u8>, Diagnostic> {
        self.read_optional_object(domain, digest, work)?
            .ok_or_else(|| {
                read_error(
                    DiagnosticClass::Corrupt,
                    "publication_read_object_missing",
                    missing,
                )
            })
    }

    fn read<T>(&self, value: T, work: RepositoryReadWork) -> RevisionRead<T> {
        RevisionRead {
            revision: self.revision(),
            value,
            work,
        }
    }
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    let class = match error.class {
        MapErrorClass::Input => DiagnosticClass::Source,
        MapErrorClass::Resource => DiagnosticClass::Resource,
        MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
        MapErrorClass::Store => DiagnosticClass::Infrastructure,
    };
    read_error(class, error.code, error.message)
}

fn consume_full_oracle_work(consumed: &mut usize, additional: usize) -> Result<(), Diagnostic> {
    *consumed = consumed
        .checked_add(additional)
        .ok_or_else(full_oracle_work_error)?;
    if *consumed > crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK {
        return Err(full_oracle_work_error());
    }
    Ok(())
}

fn full_oracle_work_error() -> Diagnostic {
    read_error(
        DiagnosticClass::Resource,
        "publication_full_oracle_work",
        format!(
            "full-oracle reconstruction exceeds its explicit {}-item work budget",
            crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK
        ),
    )
}

fn store_diagnostic(error: StoreError) -> Diagnostic {
    let class = match error.class {
        StoreErrorClass::Input => DiagnosticClass::Source,
        StoreErrorClass::Resource => DiagnosticClass::Resource,
        StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
        StoreErrorClass::Io => DiagnosticClass::Infrastructure,
    };
    read_error(class, error.code, error.message)
}

fn read_error(
    class: DiagnosticClass,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

impl CanonicalBaseRead for RepositoryView {
    fn semantic_root(&self) -> &crate::platform::kernel::SemanticRoot {
        &self.current.semantic_root
    }

    fn repository_id(&self) -> crate::platform::semantic_id::RepositoryId {
        self.current.semantic_root.repository_id
    }

    fn package_id(&self) -> PackageId {
        self.package()
    }

    fn exact_revision(&self) -> Option<RevisionId> {
        Some(self.revision())
    }

    fn owner_count(&self) -> u64 {
        self.current.semantic_root.owners.entries()
    }

    fn dependency_count(&self) -> u64 {
        self.current.semantic_root.dependencies.entries()
    }

    fn retirement_count(&self) -> u64 {
        self.current.semantic_root.retirements.entries()
    }

    fn read_owner(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
        RepositoryView::owner(self, owner).map(canonical_read)
    }

    fn read_type_object(
        &self,
        digest: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        RepositoryView::type_object(self, digest).map(canonical_read)
    }

    fn read_package_interface_owner(
        &self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        self.package_interface_owner_from_dependency(dependency, owner)
            .map(canonical_read)
    }

    fn read_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        RepositoryView::dependency(self, package).map(canonical_read)
    }

    fn read_retirement(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic> {
        RepositoryView::retirement(self, owner).map(canonical_read)
    }
}

impl WitnessBaseRead for RepositoryView {
    fn witness_manifest(&self) -> &crate::platform::witness::ValidationWitnessManifest {
        &self.current.witness
    }

    fn witness_repository_id(&self) -> crate::platform::semantic_id::RepositoryId {
        self.current.witness.repository_id
    }

    fn witness_package_id(&self) -> PackageId {
        self.current.witness.package_id
    }

    fn witness_contract_is_current(&self) -> bool {
        self.current.witness.contract_is_current()
    }

    fn owner_summary_count(&self) -> u64 {
        self.current.witness.roots.owner_summaries.entries()
    }

    fn read_namespace(
        &self,
        key: &NamespaceKey,
    ) -> Result<WitnessRead<Option<OwnerKey>>, Diagnostic> {
        RepositoryView::namespace(self, key).map(witness_read)
    }

    fn read_ownership(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<OwnershipEntry>>, Diagnostic> {
        RepositoryView::ownership(self, owner).map(witness_read)
    }

    fn contains_forward_relation(
        &self,
        edge: RelationEdge,
    ) -> Result<WitnessRead<bool>, Diagnostic> {
        RepositoryView::contains_forward_relation(self, edge).map(witness_read)
    }

    fn read_owner_summary(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<BoundOwnerSummary>>, Diagnostic> {
        RepositoryView::bound_owner_summary(self, owner).map(witness_read)
    }

    fn read_outgoing_relations(
        &self,
        owner: OwnerKey,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        let endpoint = RelationEndpoint::Owner(crate::platform::kernel::ExactOwnerKey {
            package: self.package(),
            owner,
        });
        RepositoryView::outgoing_relations(self, endpoint, None, maximum_items)
            .map(witness_relation_read)
    }

    fn read_incoming_relations(
        &self,
        owner: OwnerKey,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        let endpoint = RelationEndpoint::Owner(crate::platform::kernel::ExactOwnerKey {
            package: self.package(),
            owner,
        });
        RepositoryView::incoming_relations(self, endpoint, None, maximum_items)
            .map(witness_relation_read)
    }

    fn read_incoming_package_relations(
        &self,
        package: PackageId,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        RepositoryView::incoming_package_relations(self, package, maximum_items)
            .map(witness_relation_read)
    }

    fn read_test_dependencies(
        &self,
        test: OwnerKey,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessTestDependencyRead>, Diagnostic> {
        RepositoryView::test_dependencies(self, test, maximum_items)
            .map(witness_test_dependency_read)
    }
}

impl WitnessMapBase for RepositoryView {
    fn update_witness_maps(
        &self,
        derived: &DerivedDelta,
        summaries: &SummaryDelta,
        tests: &TestDependencyDelta,
    ) -> Result<WitnessMapUpdate, Diagnostic> {
        RepositoryView::update_witness_maps(self, derived, summaries, tests)
            .map(|updated| updated.update)
    }
}

fn canonical_read<T>(read: RevisionRead<T>) -> CanonicalRead<T> {
    CanonicalRead {
        value: read.value,
        work: CanonicalReadWork {
            point_reads: 1,
            map_pages_read: read.work.map.pages_read,
            map_entries_visited: read.work.map.entries_visited,
            catalog_lookups: read.work.store.catalog_lookups,
            objects_read: read.work.store.objects_read,
            bytes_read: read.work.store.bytes_read,
            canonical_records_decoded: read.work.canonical_records_decoded,
        },
    }
}

fn witness_read<T>(read: RevisionRead<T>) -> WitnessRead<T> {
    WitnessRead {
        value: read.value,
        work: WitnessReadWork {
            point_reads: 1,
            map_pages_read: read.work.map.pages_read,
            map_entries_visited: read.work.map.entries_visited,
            catalog_lookups: read.work.store.catalog_lookups,
            objects_read: read.work.store.objects_read,
            bytes_read: read.work.store.bytes_read,
            witness_records_decoded: read.work.witness_records_decoded,
        },
    }
}

fn witness_relation_read(read: RevisionRead<RelationRead>) -> WitnessRead<WitnessRelationRead> {
    WitnessRead {
        value: WitnessRelationRead {
            edges: read.value.edges,
            truncated: read.value.truncated,
        },
        work: WitnessReadWork {
            point_reads: 1,
            map_pages_read: read.work.map.pages_read,
            map_entries_visited: read.work.map.entries_visited,
            catalog_lookups: read.work.store.catalog_lookups,
            objects_read: read.work.store.objects_read,
            bytes_read: read.work.store.bytes_read,
            witness_records_decoded: read.work.witness_records_decoded,
        },
    }
}

fn witness_test_dependency_read(
    read: RevisionRead<TestDependencyRead>,
) -> WitnessRead<WitnessTestDependencyRead> {
    WitnessRead {
        value: WitnessTestDependencyRead {
            dependencies: read.value.dependencies,
            truncated: read.value.truncated,
        },
        work: WitnessReadWork {
            point_reads: 1,
            map_pages_read: read.work.map.pages_read,
            map_entries_visited: read.work.map.entries_visited,
            catalog_lookups: read.work.store.catalog_lookups,
            objects_read: read.work.store.objects_read,
            bytes_read: read.work.store.bytes_read,
            witness_records_decoded: read.work.witness_records_decoded,
        },
    }
}
