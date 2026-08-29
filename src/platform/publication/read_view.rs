//! Revision-pinned, bounded reads over accepted Graph 5 authority and its committed witness.

use super::{
    CurrentPublication, PreparedPublication, PublicationOptions, prepare_change_publication,
};
use crate::platform::change::{
    AuthoredAllocation, AuthoredChangeSet, AuthoredLoweringWork, BoundOwnerSummary,
    BudgetedCanonicalBase, BudgetedWitnessBase, CanonicalBaseRead, CanonicalDelta, CanonicalRead,
    CanonicalReadAdmission, CanonicalReadWork, ChangeBudget, DerivedDelta,
    LogicalChangePlanEvidence, LogicalDependencyValues, LogicalRetirementValues,
    PreparedChangeAnalysis, PrimitiveEdit, SummaryDelta, TestDependencyDelta, WitnessBaseRead,
    WitnessMapAdmission, WitnessMapBase, WitnessMapUpdate, WitnessRead, WitnessReadAdmission,
    WitnessReadWork, WitnessRelationRead, WitnessTestDependencyRead, lower_authored_changes,
    prepare_change_analysis_with_budget, update_witness_maps_from,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    BlobObjectDigest, DependencyRecord, EncodedOwnerKey, KernelSnapshot, OwnerKey, OwnerKind,
    OwnerRecord, PackageId, PackageInterfaceRecord, PackageRevisionDigest, PackageTransportDigest,
    RelationEdge, RelationEndpoint, RelationKind, RetirementRecord, TypeObject, TypeObjectDigest,
    decode_dependency, decode_dependency_binding, decode_owner, decode_owner_binding,
    decode_retirement, decode_retirement_binding, decode_type_object, dependency_map_key,
    encode_dependency, encode_retirement, owner_map_key, retirement_map_key,
};
use crate::platform::package_interface::{
    PackageInterfaceBuild, PackageInterfaceOwner, PackageInterfaceSelection,
    PackageInterfaceValidation, build_package_interface, package_interface_digest,
};
use crate::platform::package_transport::{
    MAXIMUM_PACKAGE_CLOSURE, MAXIMUM_PACKAGE_DEPENDENCIES, PACKAGE_REVISION_CONTRACT_VERSION,
    PACKAGE_TRANSPORT_CONTRACT_VERSION, PackageRevision, PackageTransport,
    PackageTransportClosureValidation,
};
use crate::platform::persistent_map::{
    MapAdmission, MapError, MapErrorClass, MapRangeControl, MapRoot, MapWork, PersistentMap,
};
use crate::platform::semantic_id::RevisionId;
use crate::platform::storage::contract::TARGET_PACK_BYTES;
use crate::platform::storage::directory::PackDirectoryStore;
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, ObjectStage, StageOutcome, StoreError,
    StoreErrorClass, StoreReadAdmission, StoreReadLimits, StoreWork,
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
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

pub const MAXIMUM_RELATION_READ_ITEMS: usize =
    crate::platform::witness::MAXIMUM_RELATION_PREFIX_ITEMS;
pub const MAXIMUM_TEST_DEPENDENCY_READ_ITEMS: usize = MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS;
const RELATION_LIMIT_STOP: &str = "publication_relation_limit_stop";
const TEST_DEPENDENCY_LIMIT_STOP: &str = "publication_test_dependency_limit_stop";
const QUERY_VISITOR_ABORT: &str = "publication_query_visitor_abort";

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RepositoryReadAdmission {
    map: MapAdmission,
    store: StoreReadAdmission,
    maximum_canonical_records: u64,
    maximum_witness_records: u64,
    canonical_code: &'static str,
    witness_code: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RepositoryQueryAdmission {
    pub map_pages: u64,
    pub map_bytes: u64,
    pub map_entries: u64,
    pub catalog_lookups: u64,
    pub store_objects: u64,
    pub store_bytes: u64,
    pub canonical_records: u64,
    pub witness_records: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RepositoryRelationQueryRange<'a> {
    pub endpoint: RelationEndpoint,
    pub kind: Option<RelationKind>,
    pub incoming: bool,
    pub exclusive_lower_bound: Option<&'a [u8]>,
    pub maximum_scan: u64,
    pub maximum_items: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RepositoryQueryRangeRead {
    pub last_visited_key: Option<Vec<u8>>,
    pub visited: u64,
    pub returned: u64,
    pub has_more: bool,
}

impl RepositoryReadAdmission {
    const fn canonical(admission: CanonicalReadAdmission) -> Self {
        Self {
            map: MapAdmission {
                maximum_pages_read: admission.maximum_map_pages,
                maximum_bytes_read: admission.maximum_bytes,
                maximum_entries_visited: admission.maximum_map_entries,
                maximum_pages_encoded: 0,
                maximum_bytes_encoded: 0,
            },
            store: StoreReadAdmission::new(StoreReadLimits {
                maximum_catalog_lookups: admission.maximum_catalog_lookups,
                maximum_objects: admission.maximum_objects,
                maximum_bytes: admission.maximum_bytes,
            }),
            maximum_canonical_records: admission.maximum_decoded_records,
            maximum_witness_records: 0,
            canonical_code: "change_budget_canonical_decoded_records",
            witness_code: "change_budget_witness_decoded_records",
        }
    }

    const fn witness(admission: WitnessReadAdmission) -> Self {
        Self {
            map: MapAdmission {
                maximum_pages_read: admission.maximum_map_pages,
                maximum_bytes_read: admission.maximum_bytes,
                maximum_entries_visited: admission.maximum_map_entries,
                maximum_pages_encoded: 0,
                maximum_bytes_encoded: 0,
            },
            store: StoreReadAdmission::new(StoreReadLimits {
                maximum_catalog_lookups: admission.maximum_catalog_lookups,
                maximum_objects: admission.maximum_objects,
                maximum_bytes: admission.maximum_bytes,
            }),
            maximum_canonical_records: 0,
            maximum_witness_records: admission.maximum_decoded_records,
            canonical_code: "change_budget_canonical_decoded_records",
            witness_code: "change_budget_witness_decoded_records",
        }
    }

    const fn unbounded() -> Self {
        Self {
            map: MapAdmission::unbounded(),
            store: StoreReadAdmission::unbounded(),
            maximum_canonical_records: u64::MAX,
            maximum_witness_records: u64::MAX,
            canonical_code: "change_budget_canonical_decoded_records",
            witness_code: "change_budget_witness_decoded_records",
        }
    }

    const fn query(admission: RepositoryQueryAdmission) -> Self {
        Self {
            map: MapAdmission {
                maximum_pages_read: admission.map_pages,
                maximum_bytes_read: admission.map_bytes,
                maximum_entries_visited: admission.map_entries,
                maximum_pages_encoded: 0,
                maximum_bytes_encoded: 0,
            },
            store: StoreReadAdmission::new(StoreReadLimits {
                maximum_catalog_lookups: admission.catalog_lookups,
                maximum_objects: admission.store_objects,
                maximum_bytes: admission.store_bytes,
            }),
            maximum_canonical_records: admission.canonical_records,
            maximum_witness_records: admission.witness_records,
            canonical_code: "query_admission_canonical_records",
            witness_code: "query_admission_witness_records",
        }
    }

    fn admit_canonical_records(&mut self, records: u64) -> Result<(), Diagnostic> {
        admit_decoded_records(
            &mut self.maximum_canonical_records,
            records,
            self.canonical_code,
            "canonical records",
        )
    }

    fn admit_witness_records(&mut self, records: u64) -> Result<(), Diagnostic> {
        admit_decoded_records(
            &mut self.maximum_witness_records,
            records,
            self.witness_code,
            "witness records",
        )
    }
}

/// Read-only adapter that carries one package-closure object and decoder admission through APIs
/// whose transport-neutral store boundary predates change-budget enforcement.
struct AdmittedRepositoryStore<'a> {
    base: &'a PackDirectoryStore,
    store: Cell<StoreReadAdmission>,
    canonical_records: Cell<u64>,
}

impl<'a> AdmittedRepositoryStore<'a> {
    const fn new(
        base: &'a PackDirectoryStore,
        store: StoreReadAdmission,
        canonical_records: u64,
    ) -> Self {
        Self {
            base,
            store: Cell::new(store),
            canonical_records: Cell::new(canonical_records),
        }
    }

    fn remaining_store(&self) -> StoreReadAdmission {
        self.store.get()
    }

    fn remaining_canonical_records(&self) -> u64 {
        self.canonical_records.get()
    }

    fn admit_decoded_object(&self, domain: ObjectDomain) -> Result<(), StoreError> {
        if !matches!(
            domain,
            ObjectDomain::SemanticRoot
                | ObjectDomain::Type
                | ObjectDomain::PackageRevision
                | ObjectDomain::PackageTransport
                | ObjectDomain::PackageInterface
        ) {
            return Ok(());
        }
        let remaining = self.canonical_records.get();
        let Some(remaining) = remaining.checked_sub(1) else {
            return Err(StoreError::new(
                StoreErrorClass::Resource,
                "change_budget_canonical_decoded_records",
                "package closure exhausted its canonical decoded-record admission",
            ));
        };
        self.canonical_records.set(remaining);
        Ok(())
    }
}

impl ImmutableObjectStore for AdmittedRepositoryStore<'_> {
    fn read(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        let mut admission = self.store.get();
        let result = self
            .base
            .read_admitted(key, maximum_bytes, &mut admission, work);
        self.store.set(admission);
        let bytes = result?;
        if bytes.is_some() {
            self.admit_decoded_object(key.domain)?;
        }
        Ok(bytes)
    }

    fn contains(&self, key: ObjectKey, work: &mut StoreWork) -> Result<bool, StoreError> {
        let mut admission = self.store.get();
        let result = self.base.contains_admitted(key, &mut admission, work);
        self.store.set(admission);
        result
    }

    fn stage(
        &mut self,
        _key: ObjectKey,
        _bytes: &[u8],
        _work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        Err(StoreError::new(
            StoreErrorClass::Input,
            "publication_admitted_store_read_only",
            "package closure admission is a read-only object-store boundary",
        ))
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
pub struct ExportedPackageTransport {
    pub revision: PackageRevision,
    pub revision_digest: PackageRevisionDigest,
    pub revision_bytes: Vec<u8>,
    pub transport: PackageTransport,
    pub transport_digest: PackageTransportDigest,
    pub packs: Vec<Vec<u8>>,
    pub(crate) interface_owners: BTreeMap<OwnerKey, PackageInterfaceOwner>,
    pub(crate) interface_types: BTreeMap<TypeObjectDigest, Vec<u8>>,
    pub interface_owner_count: u64,
    pub interface_type_count: u64,
    pub interface_map_work: MapWork,
    pub read_work: RepositoryReadWork,
    pub closure_work: StoreWork,
}

#[derive(Clone, Debug)]
pub(crate) struct BuiltPackageRevision {
    pub revision: PackageRevision,
    pub revision_digest: PackageRevisionDigest,
    pub revision_bytes: Vec<u8>,
    pub interface: PackageInterfaceBuild,
    pub interface_owners: BTreeMap<OwnerKey, PackageInterfaceOwner>,
    pub interface_types: BTreeMap<TypeObjectDigest, Vec<u8>>,
    pub read_work: RepositoryReadWork,
}

/// One fully validated authored request plus its exact request-local identity allocation.
#[derive(Clone, Debug)]
pub struct PreparedAuthoredPublication {
    pub publication: PreparedPublication,
    pub allocated: BTreeMap<String, OwnerKey>,
    pub lowering_work: AuthoredLoweringWork,
    pub logical_plan: LogicalChangePlanEvidence,
}

struct PreparedChangeWithAnalysis {
    publication: PreparedPublication,
    analysis: PreparedChangeAnalysis,
}

fn logical_plan_evidence(
    mut analysis: PreparedChangeAnalysis,
    allocations: Vec<AuthoredAllocation>,
    mut dependency_befores: BTreeMap<PackageId, DependencyRecord>,
) -> Result<LogicalChangePlanEvidence, Vec<Diagnostic>> {
    if analysis.validation.structurally_checked != analysis.summaries.plan.structurally_checked
        || analysis.validation.semantically_checked != analysis.summaries.plan.semantically_checked
    {
        return Err(vec![read_error(
            DiagnosticClass::Corrupt,
            "change_logical_plan_validation",
            "prepared validation membership disagrees with its impact plan",
        )]);
    }
    if analysis.validation.tests_selected
        != u64::try_from(analysis.summaries.plan.tests.len()).unwrap_or(u64::MAX)
    {
        return Err(vec![read_error(
            DiagnosticClass::Corrupt,
            "change_logical_plan_tests",
            "prepared validation test count disagrees with its selected graph tests",
        )]);
    }
    if allocations.windows(2).any(|pair| {
        pair[0]
            .domain
            .tag()
            .cmp(&pair[1].domain.tag())
            .then_with(|| pair[0].ordinal.cmp(&pair[1].ordinal))
            .is_ge()
    }) || allocations
        .iter()
        .any(|allocation| allocation.domain != allocation.owner.identity_kind())
    {
        return Err(vec![read_error(
            DiagnosticClass::Corrupt,
            "change_logical_plan_allocations",
            "request-local allocations are not in canonical typed ordinal order",
        )]);
    }
    if analysis
        .summaries
        .plan
        .reasons
        .windows(2)
        .any(|pair| !pair[0].logical_cmp(&pair[1]).is_lt())
    {
        return Err(vec![read_error(
            DiagnosticClass::Corrupt,
            "change_logical_plan_reasons",
            "impact reasons are not unique in canonical logical order",
        )]);
    }

    let mut dependencies = BTreeMap::new();
    for (package, edit) in std::mem::take(&mut analysis.canonical.dependencies) {
        let before = dependency_befores.remove(&package);
        let before_digest = before
            .as_ref()
            .map(encode_dependency)
            .transpose()
            .map_err(|diagnostic| vec![diagnostic])?
            .map(|(digest, _)| digest);
        let canonical_after_digest = edit.after.as_ref().map(|(digest, _)| *digest);
        let after = edit.after.map(|(_, record)| record);
        let after_digest = after
            .as_ref()
            .map(encode_dependency)
            .transpose()
            .map_err(|diagnostic| vec![diagnostic])?
            .map(|(digest, _)| digest);
        if before_digest != edit.before
            || after_digest != canonical_after_digest
            || before
                .as_ref()
                .is_some_and(|record| record.package != package)
            || after
                .as_ref()
                .is_some_and(|record| record.package != package)
        {
            return Err(vec![read_error(
                DiagnosticClass::Corrupt,
                "change_logical_plan_dependency",
                "logical dependency values disagree with the canonical dependency delta",
            )]);
        }
        dependencies.insert(package, LogicalDependencyValues { before, after });
    }
    if !dependency_befores.is_empty() {
        return Err(vec![read_error(
            DiagnosticClass::Corrupt,
            "change_logical_plan_dependency_before",
            "authored lowering retained a dependency base value without a canonical edit",
        )]);
    }

    let mut retirements = BTreeMap::new();
    for (owner, edit) in std::mem::take(&mut analysis.canonical.retirements) {
        if edit.before.is_some() {
            return Err(vec![read_error(
                DiagnosticClass::Corrupt,
                "change_logical_plan_retirement_before",
                "authored retirement replacement lacks a retained logical base value",
            )]);
        }
        let canonical_after_digest = edit.after.as_ref().map(|(digest, _)| *digest);
        let after = edit.after.map(|(_, record)| record);
        let after_digest = after
            .as_ref()
            .map(encode_retirement)
            .transpose()
            .map_err(|diagnostic| vec![diagnostic])?
            .map(|(digest, _)| digest);
        if after_digest != canonical_after_digest
            || after.as_ref().is_some_and(|record| record.owner != owner)
        {
            return Err(vec![read_error(
                DiagnosticClass::Corrupt,
                "change_logical_plan_retirement",
                "logical retirement value disagrees with the canonical retirement delta",
            )]);
        }
        retirements.insert(
            owner,
            LogicalRetirementValues {
                before: None,
                after,
            },
        );
    }

    Ok(LogicalChangePlanEvidence {
        budget: analysis.budget,
        allocations,
        dependencies,
        retirements,
        relations: std::mem::take(&mut analysis.derived.relations),
        structurally_checked: std::mem::take(&mut analysis.validation.structurally_checked),
        semantically_checked: std::mem::take(&mut analysis.validation.semantically_checked),
        tests: std::mem::take(&mut analysis.summaries.plan.tests),
        reasons: std::mem::take(&mut analysis.summaries.plan.reasons),
    })
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
    hidden_type_objects: BTreeSet<TypeObjectDigest>,
}

impl RepositoryView {
    pub(super) const fn new(current: CurrentPublication, store: PackDirectoryStore) -> Self {
        Self {
            current,
            store,
            hidden_type_objects: BTreeSet::new(),
        }
    }

    pub(super) const fn new_idempotency_base(
        current: CurrentPublication,
        store: PackDirectoryStore,
        hidden_type_objects: BTreeSet<TypeObjectDigest>,
    ) -> Self {
        Self {
            current,
            store,
            hidden_type_objects,
        }
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
        .map(|prepared| prepared.publication)
    }

    /// Resolves one strict authored request at this view's exact revision and prepares its one
    /// generic canonical publication. Request-local allocations are returned separately from
    /// accepted authority so callers can render a compact protocol receipt.
    pub fn prepare_authored_change(
        &self,
        request: &AuthoredChangeSet,
        options: PublicationOptions,
    ) -> Result<PreparedAuthoredPublication, Vec<Diagnostic>> {
        let canonical = BudgetedCanonicalBase::new(
            self,
            request.budget.canonical_reads,
            CanonicalReadWork::default(),
        )
        .map_err(|diagnostic| vec![diagnostic])?;
        let witness = BudgetedWitnessBase::new(
            self,
            request.budget.witness_reads,
            WitnessReadWork::default(),
        )
        .map_err(|diagnostic| vec![diagnostic])?;
        let lowering = lower_authored_changes(&canonical, &witness, request)
            .map_err(|diagnostic| vec![diagnostic])?;
        let crate::platform::change::AuthoredLowering {
            edits,
            allocated,
            allocations,
            dependency_befores,
            work: lowering_work,
        } = lowering;
        let prepared =
            self.prepare_change_with_prior_work(edits, options, lowering_work, request.budget)?;
        let logical_plan =
            logical_plan_evidence(prepared.analysis, allocations, dependency_befores)?;
        Ok(PreparedAuthoredPublication {
            publication: prepared.publication,
            allocated,
            lowering_work,
            logical_plan,
        })
    }

    fn prepare_change_with_prior_work(
        &self,
        edits: Vec<PrimitiveEdit>,
        options: PublicationOptions,
        prior_work: AuthoredLoweringWork,
        budget: ChangeBudget,
    ) -> Result<PreparedChangeWithAnalysis, Vec<Diagnostic>> {
        let canonical =
            BudgetedCanonicalBase::new(self, budget.canonical_reads, prior_work.canonical)
                .map_err(|diagnostic| vec![diagnostic])?;
        let normalization = CanonicalDelta::normalize_from(&canonical, edits)
            .map_err(|diagnostic| vec![diagnostic])?;
        if normalization.base_revision != Some(self.revision()) {
            return Err(vec![read_error(
                DiagnosticClass::Corrupt,
                "publication_prepare_revision_binding",
                "canonical normalization did not retain the pinned repository revision",
            )]);
        }
        for edit in normalization.canonical.dependencies.values() {
            if let Some((_, dependency)) = &edit.after {
                canonical
                    .read_admitted(|read_admission| {
                        self.resolve_package_transport_admitted(
                            dependency.package_revision,
                            &mut RepositoryReadAdmission::canonical(read_admission),
                        )
                        .map(canonical_read)
                    })
                    .map_err(|diagnostic| vec![diagnostic])?;
            }
        }
        let mut budget_reads = prior_work;
        budget_reads.canonical = canonical.work();
        let initial_budget_work = budget_reads.budget_work();
        let mut analysis = prepare_change_analysis_with_budget(
            self,
            self,
            normalization.canonical,
            budget,
            initial_budget_work,
        )?;
        analysis.canonical_read_work.add(budget_reads.canonical);
        analysis.witness_read_work.add(prior_work.witness);
        let publication = prepare_change_publication(
            self.current.accepted,
            self,
            self,
            &analysis,
            &self.store,
            options,
        )?;
        Ok(PreparedChangeWithAnalysis {
            publication,
            analysis,
        })
    }

    pub fn owner(&self, owner: OwnerKey) -> Result<RevisionRead<Option<OwnerRecord>>, Diagnostic> {
        self.owner_admitted(owner, &mut RepositoryReadAdmission::unbounded())
    }

    pub(crate) fn query_owner(
        &self,
        owner: OwnerKey,
        admission: RepositoryQueryAdmission,
    ) -> Result<RevisionRead<Option<OwnerRecord>>, Diagnostic> {
        self.owner_admitted(owner, &mut RepositoryReadAdmission::query(admission))
    }

    fn owner_admitted(
        &self,
        owner: OwnerKey,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<RevisionRead<Option<OwnerRecord>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(binding_bytes) = self.lookup_map_admitted(
            self.current.semantic_root.owners,
            &owner_map_key(owner),
            &mut work,
            admission,
        )?
        else {
            return Ok(self.read(None, work));
        };
        let binding = decode_owner_binding(&binding_bytes, owner)?;
        let bytes = self.read_required_object_admitted(
            ObjectDomain::Owner,
            binding.object.bytes(),
            "accepted owner binding references a missing owner object",
            &mut work,
            admission,
        )?;
        admission.admit_canonical_records(1)?;
        let record = decode_owner(&bytes, owner, binding.kind, binding.object)?;
        work.canonical_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(Some(record), work))
    }

    pub fn type_object(
        &self,
        digest: TypeObjectDigest,
    ) -> Result<RevisionRead<Option<TypeObject>>, Diagnostic> {
        self.type_object_admitted(digest, &mut RepositoryReadAdmission::unbounded())
    }

    fn type_object_admitted(
        &self,
        digest: TypeObjectDigest,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<RevisionRead<Option<TypeObject>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        // Repreparing an accepted idempotent request must observe the same exact base that its
        // reviewed plan observed. Immutable content added by that accepted child can now exist in
        // the append-only physical store even though it was absent at the historical base. The
        // idempotency view hides exactly those child additions so physical store growth cannot
        // change normalized intent, transaction identity, or reviewed plan identity.
        if self.hidden_type_objects.contains(&digest) {
            return Ok(self.read(None, work));
        }
        let Some(bytes) = self.read_optional_object_admitted(
            ObjectDomain::Type,
            digest.bytes(),
            &mut work,
            admission,
        )?
        else {
            return Ok(self.read(None, work));
        };
        admission.admit_canonical_records(1)?;
        let object = decode_type_object(&bytes, digest)?;
        work.canonical_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(Some(object), work))
    }

    pub fn dependency(
        &self,
        package: PackageId,
    ) -> Result<RevisionRead<Option<DependencyRecord>>, Diagnostic> {
        self.dependency_admitted(package, &mut RepositoryReadAdmission::unbounded())
    }

    fn dependency_admitted(
        &self,
        package: PackageId,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<RevisionRead<Option<DependencyRecord>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(binding_bytes) = self.lookup_map_admitted(
            self.current.semantic_root.dependencies,
            &dependency_map_key(package),
            &mut work,
            admission,
        )?
        else {
            return Ok(self.read(None, work));
        };
        let binding = decode_dependency_binding(&binding_bytes)?;
        let bytes = self.read_required_object_admitted(
            ObjectDomain::Dependency,
            binding.object.bytes(),
            "accepted dependency binding references a missing dependency object",
            &mut work,
            admission,
        )?;
        admission.admit_canonical_records(1)?;
        let record = decode_dependency(&bytes, &package, binding.object)?;
        work.canonical_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(Some(record), work))
    }

    fn resolve_package_transport(
        &self,
        revision: PackageRevisionDigest,
    ) -> Result<RevisionRead<PackageTransportClosureValidation>, Diagnostic> {
        self.resolve_package_transport_admitted(revision, &mut RepositoryReadAdmission::unbounded())
    }

    fn resolve_package_transport_admitted(
        &self,
        revision: PackageRevisionDigest,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<RevisionRead<PackageTransportClosureValidation>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let admitted_store = AdmittedRepositoryStore::new(
            &self.store,
            admission.store,
            admission.maximum_canonical_records,
        );
        let initial_map_admission = admission.map;
        let validated = super::repository::resolve_package_transport_closure_admitted(
            &admitted_store,
            &self.store,
            revision,
            None,
            None,
            &mut work.store,
            &mut admission.map,
        )?;
        admission.store = admitted_store.remaining_store();
        let remaining_records = admitted_store.remaining_canonical_records();
        work.canonical_records_decoded = admission
            .maximum_canonical_records
            .saturating_sub(remaining_records);
        admission.maximum_canonical_records = remaining_records;
        let mut interface_map_work = validated.interface_map_work;
        interface_map_work.pages_read = initial_map_admission
            .maximum_pages_read
            .saturating_sub(admission.map.maximum_pages_read);
        interface_map_work.pages_decoded = interface_map_work.pages_read;
        interface_map_work.bytes_read = initial_map_admission
            .maximum_bytes_read
            .saturating_sub(admission.map.maximum_bytes_read);
        interface_map_work.entries_visited = initial_map_admission
            .maximum_entries_visited
            .saturating_sub(admission.map.maximum_entries_visited);
        work.add_map(interface_map_work);
        work.items_returned = 1;
        Ok(self.read(validated, work))
    }

    /// Resolves one implementation-free owner from the exact package revision selected by the
    /// pinned local dependency binding. Work follows one dependency-map path, one package transport,
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
    /// the same package-revision, transport, map, and owner integrity checks as accepted reads.
    fn package_interface_owner_from_dependency(
        &self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
    ) -> Result<RevisionRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        self.package_interface_owner_from_dependency_admitted(
            dependency,
            owner,
            &mut RepositoryReadAdmission::unbounded(),
        )
    }

    fn package_interface_owner_from_dependency_admitted(
        &self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<RevisionRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let resolved =
            self.resolve_package_transport_admitted(dependency.package_revision, admission)?;
        work.add(resolved.work);
        resolved
            .value
            .root_revision
            .matches_dependency(dependency.package_revision, dependency)?;
        let value = resolved
            .value
            .root_interface
            .owners
            .get(&owner)
            .map(|value| value.record.clone());
        work.items_returned = u64::from(value.is_some());
        Ok(self.read(value, work))
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
            let resolved = self.resolve_package_transport(dependency.package_revision)?;
            work.add(resolved.work);
            let validated = resolved.value;
            validated
                .root_revision
                .matches_dependency(dependency.package_revision, dependency)?;
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
            match dependency_interfaces.entry(dependency.package_revision) {
                Entry::Vacant(entry) => {
                    entry.insert(records);
                }
                Entry::Occupied(entry) if entry.get() != &records => {
                    return Err(read_error(
                        DiagnosticClass::Corrupt,
                        "publication_full_oracle_interface_conflict",
                        "one exact logical package revision reconstructed two different interfaces",
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

    pub(crate) fn build_package_revision(&self) -> Result<BuiltPackageRevision, Diagnostic> {
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
        let interface_digest =
            package_interface_digest(self.package(), interface.root.content_root())?;
        let revision = PackageRevision {
            contract_version: PACKAGE_REVISION_CONTRACT_VERSION,
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            package: self.current.semantic_root.package_id,
            revision: self.current.revision.core.clone(),
            interface: interface_digest,
            dependencies: dependencies.value,
        };
        let (revision_digest, revision_bytes) = revision.encode()?;
        Ok(BuiltPackageRevision {
            revision,
            revision_digest,
            revision_bytes,
            interface,
            interface_owners,
            interface_types,
            read_work,
        })
    }

    /// Builds one logical package revision and its separate physical acceptance transport.
    pub fn export_package_transport(&self) -> Result<ExportedPackageTransport, Diagnostic> {
        let BuiltPackageRevision {
            revision,
            revision_digest,
            revision_bytes,
            interface,
            interface_owners,
            interface_types,
            read_work,
        } = self.build_package_revision()?;
        let transport = PackageTransport {
            contract_version: PACKAGE_TRANSPORT_CONTRACT_VERSION,
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            package_revision: revision_digest,
            semantic_root: self.current.accepted.semantic_root,
            validation_witness: self.current.accepted.validation_witness,
            witness: self.current.witness.clone(),
            interface_owners: interface.root,
        };
        let (transport_digest, transport_bytes) = transport.encode()?;
        let (semantic_root_digest, semantic_root_bytes) =
            crate::platform::kernel::encode_root(&self.current.semantic_root)?;
        if semantic_root_digest != transport.semantic_root {
            return Err(read_error(
                DiagnosticClass::Corrupt,
                "publication_package_transport_semantic_root",
                "accepted semantic root changed during package transport export",
            ));
        }
        let mut stage = ObjectStage::new(&self.store);
        let mut closure_work = StoreWork::default();
        for (key, value) in &interface.objects {
            stage
                .stage(*key, value, &mut closure_work)
                .map_err(store_diagnostic)?;
        }
        stage
            .stage(
                ObjectKey::from_digest(ObjectDomain::PackageRevision, revision_digest.bytes()),
                &revision_bytes,
                &mut closure_work,
            )
            .map_err(store_diagnostic)?;
        stage
            .stage(
                ObjectKey::from_digest(ObjectDomain::SemanticRoot, semantic_root_digest.bytes()),
                &semantic_root_bytes,
                &mut closure_work,
            )
            .map_err(store_diagnostic)?;
        stage
            .stage(
                ObjectKey::from_digest(ObjectDomain::PackageTransport, transport_digest.bytes()),
                &transport_bytes,
                &mut closure_work,
            )
            .map_err(store_diagnostic)?;
        let validated = super::repository::resolve_package_transport_closure(
            &stage,
            &self.store,
            revision_digest,
            Some(
                crate::platform::package_transport::PackageTransportBinding {
                    package_revision: revision_digest,
                    transport: transport_digest,
                },
            ),
            None,
            &mut closure_work,
        )?;
        if validated.root_revision != revision || validated.root_transport != transport {
            return Err(read_error(
                DiagnosticClass::Corrupt,
                "publication_package_transport_export",
                "exported package revision or transport changed during closure validation",
            ));
        }
        let mut builder = PackBuilder::default();
        for (key, value) in &interface.objects {
            builder.insert(*key, value).map_err(store_diagnostic)?;
        }
        builder
            .insert(
                ObjectKey::from_digest(ObjectDomain::PackageRevision, revision_digest.bytes()),
                &revision_bytes,
            )
            .map_err(store_diagnostic)?;
        builder
            .insert(
                ObjectKey::from_digest(ObjectDomain::SemanticRoot, semantic_root_digest.bytes()),
                &semantic_root_bytes,
            )
            .map_err(store_diagnostic)?;
        builder
            .insert(
                ObjectKey::from_digest(ObjectDomain::PackageTransport, transport_digest.bytes()),
                &transport_bytes,
            )
            .map_err(store_diagnostic)?;
        let packs = builder
            .seal_targeted(TARGET_PACK_BYTES)
            .map_err(store_diagnostic)?
            .into_iter()
            .map(|pack| pack.bytes)
            .collect();
        Ok(ExportedPackageTransport {
            revision,
            revision_digest,
            revision_bytes,
            transport,
            transport_digest,
            packs,
            interface_owners,
            interface_types,
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
        if count > MAXIMUM_PACKAGE_DEPENDENCIES as u64 {
            return Err(read_error(
                DiagnosticClass::Resource,
                "publication_package_dependency_count",
                format!(
                    "package has {count} dependencies; current package transport supports at most {MAXIMUM_PACKAGE_DEPENDENCIES}"
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
        self.retirement_admitted(owner, &mut RepositoryReadAdmission::unbounded())
    }

    fn retirement_admitted(
        &self,
        owner: OwnerKey,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<RevisionRead<Option<RetirementRecord>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(binding_bytes) = self.lookup_map_admitted(
            self.current.semantic_root.retirements,
            &retirement_map_key(owner),
            &mut work,
            admission,
        )?
        else {
            return Ok(self.read(None, work));
        };
        let binding = decode_retirement_binding(&binding_bytes)?;
        let bytes = self.read_required_object_admitted(
            ObjectDomain::Retirement,
            binding.object.bytes(),
            "accepted retirement binding references a missing retirement object",
            &mut work,
            admission,
        )?;
        admission.admit_canonical_records(1)?;
        let record = decode_retirement(&bytes, owner, binding.object)?;
        work.canonical_records_decoded = 1;
        work.items_returned = 1;
        Ok(self.read(Some(record), work))
    }

    pub fn namespace(
        &self,
        key: &NamespaceKey,
    ) -> Result<RevisionRead<Option<OwnerKey>>, Diagnostic> {
        self.namespace_admitted(key, &mut RepositoryReadAdmission::unbounded())
    }

    pub(crate) fn query_namespace(
        &self,
        key: &NamespaceKey,
        admission: RepositoryQueryAdmission,
    ) -> Result<RevisionRead<Option<OwnerKey>>, Diagnostic> {
        self.namespace_admitted(key, &mut RepositoryReadAdmission::query(admission))
    }

    fn namespace_admitted(
        &self,
        key: &NamespaceKey,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<RevisionRead<Option<OwnerKey>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let value = self.lookup_map_admitted(
            self.current.witness.roots.namespaces,
            &key.encode(),
            &mut work,
            admission,
        )?;
        admission.admit_witness_records(u64::from(value.is_some()))?;
        let value = value.as_deref().map(EncodedOwnerKey::decode).transpose()?;
        work.witness_records_decoded = u64::from(value.is_some());
        work.items_returned = u64::from(value.is_some());
        Ok(self.read(value, work))
    }

    pub fn ownership(
        &self,
        owner: OwnerKey,
    ) -> Result<RevisionRead<Option<OwnershipEntry>>, Diagnostic> {
        self.ownership_admitted(owner, &mut RepositoryReadAdmission::unbounded())
    }

    fn ownership_admitted(
        &self,
        owner: OwnerKey,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<RevisionRead<Option<OwnershipEntry>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let value = self.lookup_map_admitted(
            self.current.witness.roots.ownership,
            &owner_key_bytes(owner),
            &mut work,
            admission,
        )?;
        admission.admit_witness_records(u64::from(value.is_some()))?;
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

    pub(crate) fn visit_query_owners<F>(
        &self,
        exclusive_lower_bound: Option<&[u8]>,
        kind: Option<OwnerKind>,
        maximum_scan: u64,
        maximum_items: u64,
        query_admission: RepositoryQueryAdmission,
        mut visitor: F,
    ) -> Result<RevisionRead<RepositoryQueryRangeRead>, Diagnostic>
    where
        F: FnMut(OwnerKey, &OwnerRecord) -> Result<MapRangeControl, Diagnostic>,
    {
        if maximum_scan == 0 || maximum_items == 0 {
            return Err(read_error(
                DiagnosticClass::Resource,
                "query_admission_logical_range",
                "query range scan and item admission must both be positive",
            ));
        }
        let mut admission = RepositoryReadAdmission::query(query_admission);
        let shared_store_admission = RefCell::new(admission.store);
        let reader = ObjectPageReader::new_shared_admission(&self.store, &shared_store_admission);
        let mut map_work = MapWork::with_admission(admission.map);
        let mut object_work = StoreWork::default();
        let mut value = RepositoryQueryRangeRead::default();
        let mut decoded_records = 0_u64;
        let mut captured = None;
        let range = PersistentMap::from_root(self.current.semantic_root.owners).for_each_range(
            &reader,
            &[],
            exclusive_lower_bound,
            &mut map_work,
            |key, binding_bytes| {
                let operation = (|| {
                    if value.visited >= maximum_scan {
                        return Ok(MapRangeControl::StopBefore);
                    }
                    let owner = EncodedOwnerKey::decode(key)?;
                    let binding = decode_owner_binding(binding_bytes, owner)?;
                    if kind.is_some_and(|expected| expected != binding.kind) {
                        value.visited = checked_query_count(value.visited, "visited owners")?;
                        value.last_visited_key = Some(key.to_vec());
                        return Ok(MapRangeControl::Continue);
                    }
                    if value.returned >= maximum_items {
                        return Ok(MapRangeControl::StopBefore);
                    }
                    let bytes = self
                        .store
                        .read_admitted(
                            ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes()),
                            ObjectDomain::Owner.maximum_bytes(),
                            &mut shared_store_admission.borrow_mut(),
                            &mut object_work,
                        )
                        .map_err(store_diagnostic)?
                        .ok_or_else(|| {
                            read_error(
                                DiagnosticClass::Corrupt,
                                "query_owner_object_missing",
                                "accepted owner binding references a missing owner object",
                            )
                        })?;
                    admission.admit_canonical_records(1)?;
                    let record = decode_owner(&bytes, owner, binding.kind, binding.object)?;
                    decoded_records = checked_query_count(decoded_records, "canonical records")?;
                    match visitor(owner, &record)? {
                        MapRangeControl::Continue => {
                            value.visited = checked_query_count(value.visited, "visited owners")?;
                            value.returned =
                                checked_query_count(value.returned, "returned owners")?;
                            value.last_visited_key = Some(key.to_vec());
                            Ok(MapRangeControl::Continue)
                        }
                        MapRangeControl::StopBefore => Ok(MapRangeControl::StopBefore),
                    }
                })();
                operation.map_err(|diagnostic| {
                    captured = Some(diagnostic);
                    query_visitor_abort()
                })
            },
        );
        let range = match range {
            Ok(range) => range,
            Err(error) if error.code == QUERY_VISITOR_ABORT => {
                return Err(match captured {
                    Some(diagnostic) => diagnostic,
                    None => read_error(
                        DiagnosticClass::Corrupt,
                        "query_visitor_diagnostic_missing",
                        "query range aborted without its owning diagnostic",
                    ),
                });
            }
            Err(error) => return Err(map_diagnostic(error)),
        };
        if range.entries_visited != value.visited {
            return Err(read_error(
                DiagnosticClass::Corrupt,
                "query_owner_range_count",
                "owner range traversal disagrees with its logical visit count",
            ));
        }
        value.has_more = range.has_more;
        let mut work = RepositoryReadWork {
            map: map_work,
            store: reader.work(),
            canonical_records_decoded: decoded_records,
            witness_records_decoded: 0,
            items_returned: value.returned,
        };
        add_query_store_work(&mut work.store, object_work)?;
        Ok(self.read(value, work))
    }

    /// Explicit broad inventory for a clean normalized compilation.
    ///
    /// This scans only committed summary-map keys and their inline kind bindings. It does not
    /// decode owner or summary objects, and ordinary incremental builds use their exact impact
    /// set instead.
    pub(crate) fn compilation_unit_owners(
        &self,
        maximum_items: u64,
    ) -> Result<RevisionRead<Vec<OwnerKey>>, Diagnostic> {
        let reader = ObjectPageReader::new(&self.store);
        let mut map_work = MapWork::default();
        let mut owners = Vec::new();
        let mut captured = None;
        let result = PersistentMap::from_root(self.current.witness.roots.owner_summaries).for_each(
            &reader,
            &mut map_work,
            |key, value| {
                let operation = (|| {
                    let owner = EncodedOwnerKey::decode(key)?;
                    let binding = SummaryBinding::decode(value, owner)?;
                    if binding.kind.has_compilation_unit() {
                        if owners.len() as u64 >= maximum_items {
                            return Err(read_error(
                                DiagnosticClass::Resource,
                                "publication_compilation_inventory_count",
                                "clean compiler inventory exceeds the current derived-unit implementation bound",
                            ));
                        }
                        owners.push(owner);
                    }
                    Ok::<(), Diagnostic>(())
                })();
                match operation {
                    Ok(()) => Ok(()),
                    Err(diagnostic) => {
                        captured = Some(diagnostic);
                        Err(MapError {
                            class: MapErrorClass::Corrupt,
                            code: "publication_compilation_inventory_stop",
                            message: "compiler inventory stopped after an exact diagnostic"
                                .to_owned(),
                        })
                    }
                }
            },
        );
        if let Some(diagnostic) = captured {
            return Err(diagnostic);
        }
        result.map_err(map_diagnostic)?;
        let items_returned = owners.len() as u64;
        Ok(self.read(
            owners,
            RepositoryReadWork {
                map: map_work,
                store: reader.work(),
                witness_records_decoded: self.current.witness.roots.owner_summaries.entries(),
                items_returned,
                ..RepositoryReadWork::default()
            },
        ))
    }

    pub fn bound_owner_summary(
        &self,
        owner: OwnerKey,
    ) -> Result<RevisionRead<Option<BoundOwnerSummary>>, Diagnostic> {
        self.bound_owner_summary_admitted(owner, &mut RepositoryReadAdmission::unbounded())
    }

    fn bound_owner_summary_admitted(
        &self,
        owner: OwnerKey,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<RevisionRead<Option<BoundOwnerSummary>>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let Some(binding_bytes) = self.lookup_map_admitted(
            self.current.witness.roots.owner_summaries,
            &owner_key_bytes(owner),
            &mut work,
            admission,
        )?
        else {
            return Ok(self.read(None, work));
        };
        let binding = SummaryBinding::decode(&binding_bytes, owner)?;
        let bytes = self.read_required_object_admitted(
            ObjectDomain::OwnerSummary,
            binding.summary.bytes(),
            "accepted witness binding references a missing owner-summary object",
            &mut work,
            admission,
        )?;
        admission.admit_witness_records(1)?;
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
        self.contains_forward_relation_admitted(edge, &mut RepositoryReadAdmission::unbounded())
    }

    fn contains_forward_relation_admitted(
        &self,
        edge: RelationEdge,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<RevisionRead<bool>, Diagnostic> {
        let mut work = RepositoryReadWork::default();
        let value = self.lookup_map_admitted(
            self.current.witness.roots.forward_relations,
            &crate::platform::witness::forward_relation_key(edge),
            &mut work,
            admission,
        )?;
        admission.admit_witness_records(u64::from(value.is_some()))?;
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
        admission: WitnessMapAdmission,
    ) -> Result<RevisionWitnessMapUpdate, Diagnostic> {
        let reader = ObjectPageReader::new_admitted(
            &self.store,
            StoreReadAdmission::new(StoreReadLimits {
                maximum_catalog_lookups: admission.maximum_catalog_lookups,
                maximum_objects: admission.maximum_objects,
                maximum_bytes: admission.maximum_bytes,
            }),
        );
        let mut update = update_witness_maps_from(
            &self.current.witness,
            &reader,
            derived,
            summaries,
            tests,
            admission,
        )?;
        let store_work = reader.work();
        update.read_work.catalog_lookups = store_work.catalog_lookups;
        update.read_work.objects_read = store_work.objects_read;
        Ok(RevisionWitnessMapUpdate {
            revision: self.revision(),
            update,
            store_work,
        })
    }

    pub fn outgoing_relations(
        &self,
        source: RelationEndpoint,
        kind: Option<RelationKind>,
        maximum_items: usize,
    ) -> Result<RevisionRead<RelationRead>, Diagnostic> {
        self.relations(
            source,
            kind,
            maximum_items,
            false,
            &mut RepositoryReadAdmission::unbounded(),
        )
    }

    pub(crate) fn visit_query_relations<F>(
        &self,
        query: RepositoryRelationQueryRange<'_>,
        query_admission: RepositoryQueryAdmission,
        mut visitor: F,
    ) -> Result<RevisionRead<RepositoryQueryRangeRead>, Diagnostic>
    where
        F: FnMut(RelationEdge) -> Result<MapRangeControl, Diagnostic>,
    {
        let RepositoryRelationQueryRange {
            endpoint,
            kind,
            incoming,
            exclusive_lower_bound,
            maximum_scan,
            maximum_items,
        } = query;
        if maximum_items == 0 {
            return Err(read_error(
                DiagnosticClass::Resource,
                "query_admission_logical_range",
                "query relation item admission must be positive",
            ));
        }
        let mut admission = RepositoryReadAdmission::query(query_admission);
        let prefix = if incoming {
            reverse_relation_prefix(endpoint, kind)
        } else {
            forward_relation_prefix(endpoint, kind)
        };
        let root = if incoming {
            self.current.witness.roots.reverse_relations
        } else {
            self.current.witness.roots.forward_relations
        };
        let reader = ObjectPageReader::new_admitted(&self.store, admission.store);
        let mut map_work = MapWork::with_admission(admission.map);
        let mut value = RepositoryQueryRangeRead::default();
        let mut decoded_records = 0_u64;
        let mut captured = None;
        let range = PersistentMap::from_root(root).for_each_range(
            &reader,
            &prefix,
            exclusive_lower_bound,
            &mut map_work,
            |key, relation_value| {
                let operation = (|| {
                    if value.visited >= maximum_scan || value.returned >= maximum_items {
                        return Ok(MapRangeControl::StopBefore);
                    }
                    admission.admit_witness_records(1)?;
                    let edge =
                        decode_query_relation_entry(key, relation_value, endpoint, kind, incoming)?;
                    decoded_records = checked_query_count(decoded_records, "witness records")?;
                    match visitor(edge)? {
                        MapRangeControl::Continue => {
                            value.visited =
                                checked_query_count(value.visited, "visited relations")?;
                            value.returned =
                                checked_query_count(value.returned, "returned relations")?;
                            value.last_visited_key = Some(key.to_vec());
                            Ok(MapRangeControl::Continue)
                        }
                        MapRangeControl::StopBefore => Ok(MapRangeControl::StopBefore),
                    }
                })();
                operation.map_err(|diagnostic| {
                    captured = Some(diagnostic);
                    query_visitor_abort()
                })
            },
        );
        let range = match range {
            Ok(range) => range,
            Err(error) if error.code == QUERY_VISITOR_ABORT => {
                return Err(match captured {
                    Some(diagnostic) => diagnostic,
                    None => read_error(
                        DiagnosticClass::Corrupt,
                        "query_visitor_diagnostic_missing",
                        "query relation range aborted without its owning diagnostic",
                    ),
                });
            }
            Err(error) => return Err(map_diagnostic(error)),
        };
        if range.entries_visited != value.visited {
            return Err(read_error(
                DiagnosticClass::Corrupt,
                "query_relation_range_count",
                "relation range traversal disagrees with its logical visit count",
            ));
        }
        value.has_more = range.has_more;
        let work = RepositoryReadWork {
            map: map_work,
            store: reader.work(),
            canonical_records_decoded: 0,
            witness_records_decoded: decoded_records,
            items_returned: value.returned,
        };
        Ok(self.read(value, work))
    }

    fn outgoing_relations_admitted(
        &self,
        source: RelationEndpoint,
        kind: Option<RelationKind>,
        maximum_items: usize,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<RevisionRead<RelationRead>, Diagnostic> {
        self.relations(source, kind, maximum_items, false, admission)
    }

    pub fn incoming_relations(
        &self,
        target: RelationEndpoint,
        kind: Option<RelationKind>,
        maximum_items: usize,
    ) -> Result<RevisionRead<RelationRead>, Diagnostic> {
        self.relations(
            target,
            kind,
            maximum_items,
            true,
            &mut RepositoryReadAdmission::unbounded(),
        )
    }

    fn incoming_relations_admitted(
        &self,
        target: RelationEndpoint,
        kind: Option<RelationKind>,
        maximum_items: usize,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<RevisionRead<RelationRead>, Diagnostic> {
        self.relations(target, kind, maximum_items, true, admission)
    }

    pub fn incoming_package_relations(
        &self,
        package: PackageId,
        maximum_items: usize,
    ) -> Result<RevisionRead<RelationRead>, Diagnostic> {
        self.incoming_package_relations_admitted(
            package,
            maximum_items,
            &mut RepositoryReadAdmission::unbounded(),
        )
    }

    fn incoming_package_relations_admitted(
        &self,
        package: PackageId,
        maximum_items: usize,
        admission: &mut RepositoryReadAdmission,
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
        let reader = ObjectPageReader::new_admitted(&self.store, admission.store);
        let mut map_work = MapWork::with_admission(admission.map);
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
        admission.map = map_work.remaining_admission();
        admission.store = StoreReadAdmission::new(reader.remaining_read_admission().unwrap_or(
            StoreReadLimits {
                maximum_catalog_lookups: 0,
                maximum_objects: 0,
                maximum_bytes: 0,
            },
        ));
        work.add_map(map_work);
        work.store.add(reader.work());
        if let Err(error) = result
            && error.code != RELATION_LIMIT_STOP
        {
            return Err(map_diagnostic(error));
        }
        admission.admit_witness_records(u64::try_from(keys.len()).unwrap_or(u64::MAX))?;
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
        self.test_dependencies_admitted(
            test,
            maximum_items,
            &mut RepositoryReadAdmission::unbounded(),
        )
    }

    fn test_dependencies_admitted(
        &self,
        test: OwnerKey,
        maximum_items: usize,
        admission: &mut RepositoryReadAdmission,
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
        let reader = ObjectPageReader::new_admitted(&self.store, admission.store);
        let mut map_work = MapWork::with_admission(admission.map);
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
        admission.map = map_work.remaining_admission();
        admission.store = StoreReadAdmission::new(reader.remaining_read_admission().unwrap_or(
            StoreReadLimits {
                maximum_catalog_lookups: 0,
                maximum_objects: 0,
                maximum_bytes: 0,
            },
        ));
        work.add_map(map_work);
        work.store.add(reader.work());
        if let Err(error) = result
            && error.code != TEST_DEPENDENCY_LIMIT_STOP
        {
            return Err(map_diagnostic(error));
        }
        admission.admit_witness_records(u64::try_from(keys.len()).unwrap_or(u64::MAX))?;
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
        admission: &mut RepositoryReadAdmission,
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
        let reader = ObjectPageReader::new_admitted(&self.store, admission.store);
        let mut map_work = MapWork::with_admission(admission.map);
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
        admission.map = map_work.remaining_admission();
        admission.store = StoreReadAdmission::new(reader.remaining_read_admission().unwrap_or(
            StoreReadLimits {
                maximum_catalog_lookups: 0,
                maximum_objects: 0,
                maximum_bytes: 0,
            },
        ));
        work.add_map(map_work);
        work.store.add(reader.work());
        if let Err(error) = result
            && error.code != RELATION_LIMIT_STOP
        {
            return Err(map_diagnostic(error));
        }
        admission.admit_witness_records(u64::try_from(keys.len()).unwrap_or(u64::MAX))?;
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
        self.lookup_map_admitted(root, key, work, &mut RepositoryReadAdmission::unbounded())
    }

    fn lookup_map_admitted(
        &self,
        root: MapRoot,
        key: &[u8],
        work: &mut RepositoryReadWork,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<Option<Vec<u8>>, Diagnostic> {
        let reader = ObjectPageReader::new_admitted(&self.store, admission.store);
        let mut map_work = MapWork::with_admission(admission.map);
        let result = PersistentMap::from_root(root).lookup(&reader, key, &mut map_work);
        admission.map = map_work.remaining_admission();
        admission.store = StoreReadAdmission::new(reader.remaining_read_admission().unwrap_or(
            StoreReadLimits {
                maximum_catalog_lookups: 0,
                maximum_objects: 0,
                maximum_bytes: 0,
            },
        ));
        work.add_map(map_work);
        work.store.add(reader.work());
        result.map_err(map_diagnostic)
    }

    fn read_optional_object(
        &self,
        domain: ObjectDomain,
        digest: [u8; 32],
        work: &mut RepositoryReadWork,
    ) -> Result<Option<Vec<u8>>, Diagnostic> {
        self.read_optional_object_admitted(
            domain,
            digest,
            work,
            &mut RepositoryReadAdmission::unbounded(),
        )
    }

    fn read_optional_object_admitted(
        &self,
        domain: ObjectDomain,
        digest: [u8; 32],
        work: &mut RepositoryReadWork,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<Option<Vec<u8>>, Diagnostic> {
        let mut store_work = StoreWork::default();
        let result = self
            .store
            .read_admitted(
                ObjectKey::from_digest(domain, digest),
                domain.maximum_bytes(),
                &mut admission.store,
                &mut store_work,
            )
            .map_err(store_diagnostic);
        work.store.add(store_work);
        result
    }

    fn read_required_object(
        &self,
        domain: ObjectDomain,
        digest: [u8; 32],
        missing: &'static str,
        work: &mut RepositoryReadWork,
    ) -> Result<Vec<u8>, Diagnostic> {
        self.read_required_object_admitted(
            domain,
            digest,
            missing,
            work,
            &mut RepositoryReadAdmission::unbounded(),
        )
    }

    fn read_required_object_admitted(
        &self,
        domain: ObjectDomain,
        digest: [u8; 32],
        missing: &'static str,
        work: &mut RepositoryReadWork,
        admission: &mut RepositoryReadAdmission,
    ) -> Result<Vec<u8>, Diagnostic> {
        self.read_optional_object_admitted(domain, digest, work, admission)?
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

fn decode_query_relation_entry(
    key: &[u8],
    value: &[u8],
    endpoint: RelationEndpoint,
    kind: Option<RelationKind>,
    incoming: bool,
) -> Result<RelationEdge, Diagnostic> {
    if !value.is_empty() {
        return Err(read_error(
            DiagnosticClass::Corrupt,
            "query_relation_value",
            "relation witness entry has a nonempty canonical value",
        ));
    }
    let edge = if incoming {
        decode_reverse_relation_key(key)?
    } else {
        decode_forward_relation_key(key)?
    };
    let observed = if incoming { edge.target } else { edge.source };
    if observed != endpoint || kind.is_some_and(|expected| edge.kind != expected) {
        return Err(read_error(
            DiagnosticClass::Corrupt,
            "query_relation_prefix_binding",
            "decoded relation disagrees with its selected endpoint or kind prefix",
        ));
    }
    Ok(edge)
}

fn query_visitor_abort() -> MapError {
    MapError {
        class: MapErrorClass::Resource,
        code: QUERY_VISITOR_ABORT,
        message: "query range visitor stopped with a typed diagnostic".to_owned(),
    }
}

fn checked_query_count(value: u64, dimension: &'static str) -> Result<u64, Diagnostic> {
    value.checked_add(1).ok_or_else(|| {
        read_error(
            DiagnosticClass::Resource,
            "query_work_overflow",
            format!("query {dimension} count overflowed"),
        )
    })
}

fn add_query_store_work(total: &mut StoreWork, additional: StoreWork) -> Result<(), Diagnostic> {
    total.catalog_lookups = checked_query_work_add(
        total.catalog_lookups,
        additional.catalog_lookups,
        "catalog lookups",
    )?;
    total.packs_opened =
        checked_query_work_add(total.packs_opened, additional.packs_opened, "packs opened")?;
    total.objects_read =
        checked_query_work_add(total.objects_read, additional.objects_read, "objects read")?;
    total.objects_staged = checked_query_work_add(
        total.objects_staged,
        additional.objects_staged,
        "objects staged",
    )?;
    total.objects_reused = checked_query_work_add(
        total.objects_reused,
        additional.objects_reused,
        "objects reused",
    )?;
    total.bytes_read =
        checked_query_work_add(total.bytes_read, additional.bytes_read, "store bytes read")?;
    total.bytes_staged = checked_query_work_add(
        total.bytes_staged,
        additional.bytes_staged,
        "store bytes staged",
    )?;
    total.pages_staged =
        checked_query_work_add(total.pages_staged, additional.pages_staged, "pages staged")?;
    total.packs_sealed =
        checked_query_work_add(total.packs_sealed, additional.packs_sealed, "packs sealed")?;
    Ok(())
}

fn checked_query_work_add(
    total: u64,
    additional: u64,
    dimension: &'static str,
) -> Result<u64, Diagnostic> {
    total.checked_add(additional).ok_or_else(|| {
        read_error(
            DiagnosticClass::Resource,
            "query_work_overflow",
            format!("query {dimension} work overflowed"),
        )
    })
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

fn admit_decoded_records(
    remaining: &mut u64,
    records: u64,
    code: &'static str,
    unit: &'static str,
) -> Result<(), Diagnostic> {
    if records > *remaining {
        return Err(read_error(
            DiagnosticClass::Resource,
            code,
            format!(
                "read requires {records} {unit}, exceeding the remaining admission {}",
                *remaining
            ),
        ));
    }
    *remaining -= records;
    Ok(())
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

    fn read_owner_admitted(
        &self,
        owner: OwnerKey,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
        self.owner_admitted(owner, &mut RepositoryReadAdmission::canonical(admission))
            .map(canonical_read)
    }

    fn read_type_object(
        &self,
        digest: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        RepositoryView::type_object(self, digest).map(canonical_read)
    }

    fn read_type_object_admitted(
        &self,
        digest: TypeObjectDigest,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        self.type_object_admitted(digest, &mut RepositoryReadAdmission::canonical(admission))
            .map(canonical_read)
    }

    fn read_package_interface_owner(
        &self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        self.package_interface_owner_from_dependency(dependency, owner)
            .map(canonical_read)
    }

    fn read_package_interface_owner_admitted(
        &self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        self.package_interface_owner_from_dependency_admitted(
            dependency,
            owner,
            &mut RepositoryReadAdmission::canonical(admission),
        )
        .map(canonical_read)
    }

    fn read_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        RepositoryView::dependency(self, package).map(canonical_read)
    }

    fn read_dependency_admitted(
        &self,
        package: PackageId,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        self.dependency_admitted(package, &mut RepositoryReadAdmission::canonical(admission))
            .map(canonical_read)
    }

    fn read_retirement(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic> {
        RepositoryView::retirement(self, owner).map(canonical_read)
    }

    fn read_retirement_admitted(
        &self,
        owner: OwnerKey,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic> {
        self.retirement_admitted(owner, &mut RepositoryReadAdmission::canonical(admission))
            .map(canonical_read)
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

    fn read_namespace_admitted(
        &self,
        key: &NamespaceKey,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<Option<OwnerKey>>, Diagnostic> {
        self.namespace_admitted(key, &mut RepositoryReadAdmission::witness(admission))
            .map(witness_read)
    }

    fn read_ownership(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<OwnershipEntry>>, Diagnostic> {
        RepositoryView::ownership(self, owner).map(witness_read)
    }

    fn read_ownership_admitted(
        &self,
        owner: OwnerKey,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<Option<OwnershipEntry>>, Diagnostic> {
        self.ownership_admitted(owner, &mut RepositoryReadAdmission::witness(admission))
            .map(witness_read)
    }

    fn contains_forward_relation(
        &self,
        edge: RelationEdge,
    ) -> Result<WitnessRead<bool>, Diagnostic> {
        RepositoryView::contains_forward_relation(self, edge).map(witness_read)
    }

    fn contains_forward_relation_admitted(
        &self,
        edge: RelationEdge,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<bool>, Diagnostic> {
        self.contains_forward_relation_admitted(
            edge,
            &mut RepositoryReadAdmission::witness(admission),
        )
        .map(witness_read)
    }

    fn read_owner_summary(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<BoundOwnerSummary>>, Diagnostic> {
        RepositoryView::bound_owner_summary(self, owner).map(witness_read)
    }

    fn read_owner_summary_admitted(
        &self,
        owner: OwnerKey,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<Option<BoundOwnerSummary>>, Diagnostic> {
        self.bound_owner_summary_admitted(owner, &mut RepositoryReadAdmission::witness(admission))
            .map(witness_read)
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

    fn read_outgoing_relations_admitted(
        &self,
        owner: OwnerKey,
        maximum_items: usize,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        let endpoint = RelationEndpoint::Owner(crate::platform::kernel::ExactOwnerKey {
            package: self.package(),
            owner,
        });
        self.outgoing_relations_admitted(
            endpoint,
            None,
            maximum_items,
            &mut RepositoryReadAdmission::witness(admission),
        )
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

    fn read_incoming_relations_admitted(
        &self,
        owner: OwnerKey,
        maximum_items: usize,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        let endpoint = RelationEndpoint::Owner(crate::platform::kernel::ExactOwnerKey {
            package: self.package(),
            owner,
        });
        self.incoming_relations_admitted(
            endpoint,
            None,
            maximum_items,
            &mut RepositoryReadAdmission::witness(admission),
        )
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

    fn read_incoming_package_relations_admitted(
        &self,
        package: PackageId,
        maximum_items: usize,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        self.incoming_package_relations_admitted(
            package,
            maximum_items,
            &mut RepositoryReadAdmission::witness(admission),
        )
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

    fn read_test_dependencies_admitted(
        &self,
        test: OwnerKey,
        maximum_items: usize,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessTestDependencyRead>, Diagnostic> {
        self.test_dependencies_admitted(
            test,
            maximum_items,
            &mut RepositoryReadAdmission::witness(admission),
        )
        .map(witness_test_dependency_read)
    }
}

impl WitnessMapBase for RepositoryView {
    fn update_witness_maps(
        &self,
        derived: &DerivedDelta,
        summaries: &SummaryDelta,
        tests: &TestDependencyDelta,
        admission: WitnessMapAdmission,
    ) -> Result<WitnessMapUpdate, Diagnostic> {
        RepositoryView::update_witness_maps(self, derived, summaries, tests, admission)
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

#[cfg(test)]
mod query_read_tests {
    use super::*;
    use crate::platform::kernel::ExactOwnerKey;
    use crate::platform::semantic_id::ModuleId;
    use crate::platform::witness::{forward_relation_key, reverse_relation_key};

    #[test]
    fn query_relation_entry_strictly_revalidates_key_value_prefix_and_direction() {
        let local_package = PackageId::migrate(b"query-relation-entry", 1);
        let foreign_package = PackageId::migrate(b"query-relation-entry", 2);
        let source = RelationEndpoint::Owner(ExactOwnerKey {
            package: local_package,
            owner: OwnerKey::Module(ModuleId::migrate(b"query-relation-entry", 3)),
        });
        let target = RelationEndpoint::Owner(ExactOwnerKey {
            package: foreign_package,
            owner: OwnerKey::Module(ModuleId::migrate(b"query-relation-entry", 4)),
        });
        let edge = RelationEdge {
            source,
            kind: RelationKind::FunctionCall,
            target,
        };
        let forward = forward_relation_key(edge);
        let reverse = reverse_relation_key(edge);
        assert_eq!(
            decode_query_relation_entry(
                &forward,
                &[],
                source,
                Some(RelationKind::FunctionCall),
                false,
            )
            .expect("canonical outgoing relation"),
            edge
        );
        assert_eq!(
            decode_query_relation_entry(
                &reverse,
                &[],
                target,
                Some(RelationKind::FunctionCall),
                true,
            )
            .expect("canonical incoming relation"),
            edge
        );
        assert_eq!(
            decode_query_relation_entry(&forward, &[1], source, None, false)
                .expect_err("nonempty relation value")
                .code,
            "query_relation_value"
        );
        assert_eq!(
            decode_query_relation_entry(&forward, &[], target, None, false)
                .expect_err("wrong selected endpoint")
                .code,
            "query_relation_prefix_binding"
        );
        assert_eq!(
            decode_query_relation_entry(
                &forward,
                &[],
                source,
                Some(RelationKind::NamedTypeUse),
                false,
            )
            .expect_err("wrong selected kind")
            .code,
            "query_relation_prefix_binding"
        );
        let mut trailing = forward.clone();
        trailing.push(0);
        assert_eq!(
            decode_query_relation_entry(&trailing, &[], source, None, false)
                .expect_err("trailing relation bytes")
                .code,
            "witness_relation_trailing"
        );
        assert!(
            decode_query_relation_entry(&[u8::MAX, 0], &[], source, None, false).is_err(),
            "malformed endpoint domain must reject"
        );
    }
}
