//! Independent deterministic resource budgets and observations for one semantic change.

use super::{
    CanonicalDelta, CanonicalReadWork, DerivedDelta, ImpactPlan, IncrementalValidationWork,
    PlannedSummaries, TestDependencyDelta, WitnessMapUpdate, WitnessReadWork,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use bincode::{Decode, Encode};
use std::collections::BTreeSet;

/// A fixed policy dimension, independent from semantic permissions and cancellation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum BudgetDimension {
    Operations,
    Preconditions,
    AllocatedIdentities,
    AuthoredTypeNodes,
    CanonicalOwnerEdits,
    CanonicalTypeEdits,
    CanonicalDependencyEdits,
    CanonicalRetirementEdits,
    CanonicalPointReads,
    CanonicalMapPages,
    CanonicalMapEntries,
    CanonicalCatalogLookups,
    CanonicalObjects,
    CanonicalBytes,
    CanonicalDecodedRecords,
    CanonicalMapPagesEncoded,
    CanonicalMapBytesEncoded,
    WitnessPointReads,
    WitnessMapPages,
    WitnessMapEntries,
    WitnessCatalogLookups,
    WitnessObjects,
    WitnessBytes,
    WitnessDecodedRecords,
    AffectedFrontierOwners,
    ImpactSummaryOwners,
    ImpactSummaryEdits,
    ImpactOwnershipSteps,
    ImpactBehaviorOwners,
    RelationEdges,
    RelationFanout,
    ValidationOwnerRecords,
    ValidationOwnershipEntries,
    ValidationTypeObjects,
    ValidationExpressionSteps,
    ValidationDiagnostics,
    TestsSelected,
    TestDependenciesPerTest,
    TestOwnershipSteps,
    TestOwnersVisited,
    WitnessEdits,
    WitnessMapPagesEncoded,
    WitnessMapBytesEncoded,
    StagedObjects,
    StagedBytes,
    StagedPages,
}

impl BudgetDimension {
    pub(crate) const ALL: [Self; 46] = [
        Self::Operations,
        Self::Preconditions,
        Self::AllocatedIdentities,
        Self::AuthoredTypeNodes,
        Self::CanonicalOwnerEdits,
        Self::CanonicalTypeEdits,
        Self::CanonicalDependencyEdits,
        Self::CanonicalRetirementEdits,
        Self::CanonicalPointReads,
        Self::CanonicalMapPages,
        Self::CanonicalMapEntries,
        Self::CanonicalCatalogLookups,
        Self::CanonicalObjects,
        Self::CanonicalBytes,
        Self::CanonicalDecodedRecords,
        Self::CanonicalMapPagesEncoded,
        Self::CanonicalMapBytesEncoded,
        Self::WitnessPointReads,
        Self::WitnessMapPages,
        Self::WitnessMapEntries,
        Self::WitnessCatalogLookups,
        Self::WitnessObjects,
        Self::WitnessBytes,
        Self::WitnessDecodedRecords,
        Self::AffectedFrontierOwners,
        Self::ImpactSummaryOwners,
        Self::ImpactSummaryEdits,
        Self::ImpactOwnershipSteps,
        Self::ImpactBehaviorOwners,
        Self::RelationEdges,
        Self::RelationFanout,
        Self::ValidationOwnerRecords,
        Self::ValidationOwnershipEntries,
        Self::ValidationTypeObjects,
        Self::ValidationExpressionSteps,
        Self::ValidationDiagnostics,
        Self::TestsSelected,
        Self::TestDependenciesPerTest,
        Self::TestOwnershipSteps,
        Self::TestOwnersVisited,
        Self::WitnessEdits,
        Self::WitnessMapPagesEncoded,
        Self::WitnessMapBytesEncoded,
        Self::StagedObjects,
        Self::StagedBytes,
        Self::StagedPages,
    ];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Operations => "operations",
            Self::Preconditions => "preconditions",
            Self::AllocatedIdentities => "allocated_identities",
            Self::AuthoredTypeNodes => "authored_type_nodes",
            Self::CanonicalOwnerEdits => "canonical_owner_edits",
            Self::CanonicalTypeEdits => "canonical_type_edits",
            Self::CanonicalDependencyEdits => "canonical_dependency_edits",
            Self::CanonicalRetirementEdits => "canonical_retirement_edits",
            Self::CanonicalPointReads => "canonical_point_reads",
            Self::CanonicalMapPages => "canonical_map_pages",
            Self::CanonicalMapEntries => "canonical_map_entries",
            Self::CanonicalCatalogLookups => "canonical_catalog_lookups",
            Self::CanonicalObjects => "canonical_objects",
            Self::CanonicalBytes => "canonical_bytes",
            Self::CanonicalDecodedRecords => "canonical_decoded_records",
            Self::CanonicalMapPagesEncoded => "canonical_map_pages_encoded",
            Self::CanonicalMapBytesEncoded => "canonical_map_bytes_encoded",
            Self::WitnessPointReads => "witness_point_reads",
            Self::WitnessMapPages => "witness_map_pages",
            Self::WitnessMapEntries => "witness_map_entries",
            Self::WitnessCatalogLookups => "witness_catalog_lookups",
            Self::WitnessObjects => "witness_objects",
            Self::WitnessBytes => "witness_bytes",
            Self::WitnessDecodedRecords => "witness_decoded_records",
            Self::AffectedFrontierOwners => "affected_frontier_owners",
            Self::ImpactSummaryOwners => "impact_summary_owners",
            Self::ImpactSummaryEdits => "impact_summary_edits",
            Self::ImpactOwnershipSteps => "impact_ownership_steps",
            Self::ImpactBehaviorOwners => "impact_behavior_owners",
            Self::RelationEdges => "relation_edges",
            Self::RelationFanout => "relation_fanout",
            Self::ValidationOwnerRecords => "validation_owner_records",
            Self::ValidationOwnershipEntries => "validation_ownership_entries",
            Self::ValidationTypeObjects => "validation_type_objects",
            Self::ValidationExpressionSteps => "validation_expression_steps",
            Self::ValidationDiagnostics => "validation_diagnostics",
            Self::TestsSelected => "tests_selected",
            Self::TestDependenciesPerTest => "test_dependencies_per_test",
            Self::TestOwnershipSteps => "test_ownership_steps",
            Self::TestOwnersVisited => "test_owners_visited",
            Self::WitnessEdits => "witness_edits",
            Self::WitnessMapPagesEncoded => "witness_map_pages_encoded",
            Self::WitnessMapBytesEncoded => "witness_map_bytes_encoded",
            Self::StagedObjects => "staged_objects",
            Self::StagedBytes => "staged_bytes",
            Self::StagedPages => "staged_pages",
        }
    }

    pub(crate) const fn policy_name(self) -> &'static str {
        match self {
            Self::Operations => "operations",
            Self::Preconditions => "preconditions",
            Self::AllocatedIdentities => "allocated-identities",
            Self::AuthoredTypeNodes => "authored-type-nodes",
            Self::CanonicalOwnerEdits => "canonical-owner-edits",
            Self::CanonicalTypeEdits => "canonical-type-edits",
            Self::CanonicalDependencyEdits => "canonical-dependency-edits",
            Self::CanonicalRetirementEdits => "canonical-retirement-edits",
            Self::CanonicalPointReads => "canonical-point-reads",
            Self::CanonicalMapPages => "canonical-map-pages",
            Self::CanonicalMapEntries => "canonical-map-entries",
            Self::CanonicalCatalogLookups => "canonical-catalog-lookups",
            Self::CanonicalObjects => "canonical-objects",
            Self::CanonicalBytes => "canonical-bytes",
            Self::CanonicalDecodedRecords => "canonical-decoded-records",
            Self::CanonicalMapPagesEncoded => "canonical-map-pages-encoded",
            Self::CanonicalMapBytesEncoded => "canonical-map-bytes-encoded",
            Self::WitnessPointReads => "witness-point-reads",
            Self::WitnessMapPages => "witness-map-pages",
            Self::WitnessMapEntries => "witness-map-entries",
            Self::WitnessCatalogLookups => "witness-catalog-lookups",
            Self::WitnessObjects => "witness-objects",
            Self::WitnessBytes => "witness-bytes",
            Self::WitnessDecodedRecords => "witness-decoded-records",
            Self::AffectedFrontierOwners => "affected-frontier-owners",
            Self::ImpactSummaryOwners => "impact-summary-owners",
            Self::ImpactSummaryEdits => "impact-summary-edits",
            Self::ImpactOwnershipSteps => "impact-ownership-steps",
            Self::ImpactBehaviorOwners => "impact-behavior-owners",
            Self::RelationEdges => "relation-edges",
            Self::RelationFanout => "relation-fanout",
            Self::ValidationOwnerRecords => "validation-owner-records",
            Self::ValidationOwnershipEntries => "validation-ownership-entries",
            Self::ValidationTypeObjects => "validation-type-objects",
            Self::ValidationExpressionSteps => "validation-expression-steps",
            Self::ValidationDiagnostics => "validation-diagnostics",
            Self::TestsSelected => "tests-selected",
            Self::TestDependenciesPerTest => "test-dependencies-per-test",
            Self::TestOwnershipSteps => "test-ownership-steps",
            Self::TestOwnersVisited => "test-owners-visited",
            Self::WitnessEdits => "witness-edits",
            Self::WitnessMapPagesEncoded => "witness-map-pages-encoded",
            Self::WitnessMapBytesEncoded => "witness-map-bytes-encoded",
            Self::StagedObjects => "staged-objects",
            Self::StagedBytes => "staged-bytes",
            Self::StagedPages => "staged-pages",
        }
    }

    const fn bit(self) -> u64 {
        // Every variant is in the fixed 46-dimension policy schema.
        1_u64 << self as u8
    }
}

/// Per-dimension exactness. The fixed bits are part of the current receipt profile.
#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct WorkSaturation {
    bits: u64,
}

impl WorkSaturation {
    pub const fn any(self) -> bool {
        self.bits != 0
    }

    pub(crate) const fn contains(self, dimension: BudgetDimension) -> bool {
        self.bits & dimension.bit() != 0
    }

    pub(crate) fn include(&mut self, other: Self) {
        self.bits |= other.bits;
    }

    pub(crate) fn mark(&mut self, dimension: BudgetDimension) {
        self.bits |= dimension.bit();
    }

    pub(crate) fn observe(&mut self, value: &mut u64, dimension: BudgetDimension, amount: BudgetCounter) {
        let mut counter = BudgetCounter { consumed: *value, saturated: self.contains(dimension) };
        counter.observe(amount);
        *value = counter.consumed;
        if counter.saturated {
            self.mark(dimension);
        }
    }

    pub(crate) const fn counter(self, dimension: BudgetDimension, consumed: u64) -> BudgetCounter {
        BudgetCounter { consumed, saturated: self.contains(dimension) }
    }

    pub(crate) fn charge(&mut self, value: &mut u64, dimension: BudgetDimension,
        amount: BudgetCounter, maximum: Option<u64>, phase: &str) -> Result<(), Diagnostic>
    {
        let mut counter = self.counter(dimension, *value);
        counter.charge(amount, maximum, phase, dimension.name())?;
        *value = counter.consumed;
        if counter.saturated { self.mark(dimension); }
        Ok(())
    }

    pub(crate) const fn valid(self) -> bool {
        self.bits >> 46 == 0
    }
}

/// Checked finite enforcement and honest passive observation of one resource dimension.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct BudgetCounter {
    pub consumed: u64,
    pub saturated: bool,
}

impl BudgetCounter {
    pub(crate) const fn exact(consumed: u64) -> Self {
        Self { consumed, saturated: false }
    }

    pub(crate) fn observe(&mut self, amount: Self) {
        self.saturated |= amount.saturated;
        match self.consumed.checked_add(amount.consumed) {
            Some(value) => self.consumed = value,
            None => {
                self.consumed = u64::MAX;
                self.saturated = true;
            }
        }
    }

    pub(crate) fn charge(&mut self, amount: Self, maximum: Option<u64>, phase: &str, dimension: &'static str) -> Result<(), Diagnostic> {
        if let Some(maximum) = maximum {
            let next = if self.saturated || amount.saturated {
                None
            } else {
                self.consumed.checked_add(amount.consumed).filter(|next| *next <= maximum)
            };
            let Some(next) = next else {
                return Err(budget_error(budget_code(dimension), format!(
                    "{phase} exceeds the declared maximum {maximum} {dimension}"
                )));
            };
            self.consumed = next;
        } else {
            self.observe(amount);
        }
        Ok(())
    }

    pub(crate) fn remaining(self, maximum: Option<u64>, phase: &str, dimension: &'static str) -> Result<Option<u64>, Diagnostic> {
        let Some(maximum) = maximum else { return Ok(None); };
        if self.saturated {
            return Err(budget_error(budget_code(dimension), format!(
                "{phase} has already exceeded the finite {dimension} representation"
            )));
        }
        maximum.checked_sub(self.consumed).map(Some).ok_or_else(|| {
            budget_error(budget_code(dimension), format!(
                "{phase} already observed {} {dimension}, exceeding the declared maximum {maximum}", self.consumed
            ))
        })
    }

    pub(crate) fn product(count: u64, width: u64) -> Self {
        match count.checked_mul(width) {
            Some(consumed) => Self::exact(consumed),
            None => Self { consumed: u64::MAX, saturated: true },
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct AuthoredChangeAdmission {
    /// Authored operations in the normalized request.
    pub maximum_operations: Option<u64>,
    /// Exact preconditions evaluated against the pinned base.
    pub maximum_preconditions: Option<u64>,
    /// Durable identities allocated from named or anonymous request definitions.
    pub maximum_allocated_identities: Option<u64>,
    /// Distinct canonical type objects interned while lowering the request.
    pub maximum_type_nodes: Option<u64>,
}

impl Default for AuthoredChangeAdmission {
    fn default() -> Self {
        Self {
            maximum_operations: None,
            maximum_preconditions: None,
            maximum_allocated_identities: None,
            maximum_type_nodes: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CanonicalEditAdmission {
    /// Distinct accepted-owner bindings created, replaced, or removed.
    pub maximum_owner_edits: Option<u64>,
    /// Distinct absent type objects proposed for insertion.
    pub maximum_type_edits: Option<u64>,
    /// Distinct package-dependency bindings created, replaced, or removed.
    pub maximum_dependency_edits: Option<u64>,
    /// Distinct retirement bindings created or replaced.
    pub maximum_retirement_edits: Option<u64>,
}

impl Default for CanonicalEditAdmission {
    fn default() -> Self {
        Self {
            maximum_owner_edits: None,
            maximum_type_edits: None,
            maximum_dependency_edits: None,
            maximum_retirement_edits: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CanonicalReadAdmission {
    /// Logical point-read requests issued to accepted semantic authority.
    pub maximum_point_reads: Option<u64>,
    /// Persistent-map pages decoded for those reads.
    pub maximum_map_pages: Option<u64>,
    /// Persistent-map entries visited for those reads.
    pub maximum_map_entries: Option<u64>,
    /// Immutable-store catalog probes issued for those reads.
    pub maximum_catalog_lookups: Option<u64>,
    /// Immutable objects whose payload bytes were returned.
    pub maximum_objects: Option<u64>,
    /// Immutable object payload bytes returned.
    pub maximum_bytes: Option<u64>,
    /// Canonical semantic records decoded from returned payloads.
    pub maximum_decoded_records: Option<u64>,
}

impl Default for CanonicalReadAdmission {
    fn default() -> Self {
        Self {
            maximum_point_reads: None,
            maximum_map_pages: None,
            maximum_map_entries: None,
            maximum_catalog_lookups: None,
            maximum_objects: None,
            maximum_bytes: None,
            maximum_decoded_records: None,
        }
    }
}

impl CanonicalReadAdmission {
    /// Internal adapter for oracle and test readers that do not own a request budget.
    pub(crate) const fn unbounded() -> Self {
        Self {
            maximum_point_reads: None,
            maximum_map_pages: None,
            maximum_map_entries: None,
            maximum_catalog_lookups: None,
            maximum_objects: None,
            maximum_bytes: None,
            maximum_decoded_records: None,
        }
    }

    pub(crate) fn check_work(self, work: CanonicalReadWork, phase: &str) -> Result<(), Diagnostic> {
        check_canonical_reads(phase, work, self)
    }

    pub(crate) fn remaining_after(
        self,
        work: CanonicalReadWork,
        phase: &str,
    ) -> Result<Self, Diagnostic> {
        self.check_work(work, phase)?;
        Ok(Self {
            maximum_point_reads: remaining_dimension(
                phase,
                "canonical_point_reads",
                work.point_reads,
                self.maximum_point_reads,
            )?,
            maximum_map_pages: remaining_dimension(
                phase,
                "canonical_map_pages",
                work.map_pages_read,
                self.maximum_map_pages,
            )?,
            maximum_map_entries: remaining_dimension(
                phase,
                "canonical_map_entries",
                work.map_entries_visited,
                self.maximum_map_entries,
            )?,
            maximum_catalog_lookups: remaining_dimension(
                phase,
                "canonical_catalog_lookups",
                work.catalog_lookups,
                self.maximum_catalog_lookups,
            )?,
            maximum_objects: remaining_dimension(
                phase,
                "canonical_objects",
                work.objects_read,
                self.maximum_objects,
            )?,
            maximum_bytes: remaining_dimension(
                phase,
                "canonical_bytes",
                work.bytes_read,
                self.maximum_bytes,
            )?,
            maximum_decoded_records: remaining_dimension(
                phase,
                "canonical_decoded_records",
                work.canonical_records_decoded,
                self.maximum_decoded_records,
            )?,
        })
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CanonicalMapUpdateAdmission {
    /// Candidate persistent-map page encodings produced across semantic authority maps.
    pub maximum_pages_encoded: Option<u64>,
    /// Bytes produced by those candidate page encodings before page reuse is known.
    pub maximum_bytes_encoded: Option<u64>,
}

impl Default for CanonicalMapUpdateAdmission {
    fn default() -> Self {
        Self {
            maximum_pages_encoded: None,
            maximum_bytes_encoded: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct WitnessReadAdmission {
    /// Logical point-read requests issued to accepted validation evidence.
    pub maximum_point_reads: Option<u64>,
    /// Persistent-map pages decoded for those reads.
    pub maximum_map_pages: Option<u64>,
    /// Persistent-map entries visited for those reads.
    pub maximum_map_entries: Option<u64>,
    /// Immutable-store catalog probes issued for those reads.
    pub maximum_catalog_lookups: Option<u64>,
    /// Immutable witness objects whose payload bytes were returned.
    pub maximum_objects: Option<u64>,
    /// Immutable witness object payload bytes returned.
    pub maximum_bytes: Option<u64>,
    /// Witness records decoded from returned payloads.
    pub maximum_decoded_records: Option<u64>,
}

impl Default for WitnessReadAdmission {
    fn default() -> Self {
        Self {
            maximum_point_reads: None,
            maximum_map_pages: None,
            maximum_map_entries: None,
            maximum_catalog_lookups: None,
            maximum_objects: None,
            maximum_bytes: None,
            maximum_decoded_records: None,
        }
    }
}

impl WitnessReadAdmission {
    /// Internal adapter for oracle and test readers that do not own a request budget.
    pub(crate) const fn unbounded() -> Self {
        Self {
            maximum_point_reads: None,
            maximum_map_pages: None,
            maximum_map_entries: None,
            maximum_catalog_lookups: None,
            maximum_objects: None,
            maximum_bytes: None,
            maximum_decoded_records: None,
        }
    }

    pub(crate) fn check_work(self, work: WitnessReadWork, phase: &str) -> Result<(), Diagnostic> {
        check_witness_reads(phase, work, self)
    }

    pub(crate) fn remaining_after(
        self,
        work: WitnessReadWork,
        phase: &str,
    ) -> Result<Self, Diagnostic> {
        self.check_work(work, phase)?;
        Ok(Self {
            maximum_point_reads: remaining_dimension(
                phase,
                "witness_point_reads",
                work.point_reads,
                self.maximum_point_reads,
            )?,
            maximum_map_pages: remaining_dimension(
                phase,
                "witness_map_pages",
                work.map_pages_read,
                self.maximum_map_pages,
            )?,
            maximum_map_entries: remaining_dimension(
                phase,
                "witness_map_entries",
                work.map_entries_visited,
                self.maximum_map_entries,
            )?,
            maximum_catalog_lookups: remaining_dimension(
                phase,
                "witness_catalog_lookups",
                work.catalog_lookups,
                self.maximum_catalog_lookups,
            )?,
            maximum_objects: remaining_dimension(
                phase,
                "witness_objects",
                work.objects_read,
                self.maximum_objects,
            )?,
            maximum_bytes: remaining_dimension(
                phase,
                "witness_bytes",
                work.bytes_read,
                self.maximum_bytes,
            )?,
            maximum_decoded_records: remaining_dimension(
                phase,
                "witness_decoded_records",
                work.witness_records_decoded,
                self.maximum_decoded_records,
            )?,
        })
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct ImpactAdmission {
    /// Distinct owners in the combined direct and transitive validation frontier.
    pub maximum_affected_owners: Option<u64>,
    /// Owner summaries examined while constructing the impact plan.
    pub maximum_summary_owners: Option<u64>,
    /// Candidate summary edits examined while constructing the impact plan.
    pub maximum_summary_edits: Option<u64>,
    /// Parent links traversed while resolving semantic ownership.
    pub maximum_ownership_steps: Option<u64>,
    /// Behavior owners visited while propagating executable impact.
    pub maximum_behavior_owners: Option<u64>,
    /// Relation edges extracted, verified, or traversed across the request.
    pub maximum_relation_edges: Option<u64>,
    /// Relation edges admitted for any one queried endpoint.
    pub maximum_relation_fanout: Option<u64>,
}

impl Default for ImpactAdmission {
    fn default() -> Self {
        Self {
            maximum_affected_owners: None,
            maximum_summary_owners: None,
            maximum_summary_edits: None,
            maximum_ownership_steps: None,
            maximum_behavior_owners: None,
            maximum_relation_edges: None,
            maximum_relation_fanout: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct ValidationAdmission {
    /// Owner records checked by structural or semantic validation.
    pub maximum_owner_records: Option<u64>,
    /// Ownership bindings checked by structural or semantic validation.
    pub maximum_ownership_entries: Option<u64>,
    /// Canonical type objects checked by semantic validation.
    pub maximum_type_objects: Option<u64>,
    /// Expression nodes or equivalent semantic inference steps executed.
    pub maximum_expression_steps: Option<u64>,
    /// Diagnostics retained before validation stops.
    pub maximum_diagnostics: Option<u64>,
}

impl Default for ValidationAdmission {
    fn default() -> Self {
        Self {
            maximum_owner_records: None,
            maximum_ownership_entries: None,
            maximum_type_objects: None,
            maximum_expression_steps: None,
            maximum_diagnostics: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct TestAdmission {
    /// Graph-owned tests selected by the impact frontier.
    pub maximum_selected: Option<u64>,
    /// Exact dependency records admitted for any one selected test.
    pub maximum_dependencies_per_test: Option<u64>,
    /// Parent links traversed while locating affected tests.
    pub maximum_ownership_steps: Option<u64>,
    /// Candidate owners visited while locating affected tests.
    pub maximum_owners_visited: Option<u64>,
}

impl Default for TestAdmission {
    fn default() -> Self {
        Self {
            maximum_selected: None,
            maximum_dependencies_per_test: None,
            maximum_ownership_steps: None,
            maximum_owners_visited: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct WitnessUpdateAdmission {
    /// Logical witness-map entries inserted, replaced, removed, or confirmed unchanged.
    pub maximum_edits: Option<u64>,
    /// Candidate persistent-map page encodings produced across all witness maps.
    pub maximum_pages_encoded: Option<u64>,
    /// Bytes produced by those candidate page encodings before page reuse is known.
    pub maximum_bytes_encoded: Option<u64>,
}

impl Default for WitnessUpdateAdmission {
    fn default() -> Self {
        Self {
            maximum_edits: None,
            maximum_pages_encoded: None,
            maximum_bytes_encoded: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct StagingAdmission {
    /// Newly isolated immutable objects retained for this preparation.
    pub maximum_objects: Option<u64>,
    /// Bytes retained by those newly isolated immutable objects.
    pub maximum_bytes: Option<u64>,
    /// Newly isolated persistent-map page objects retained for this preparation.
    pub maximum_pages: Option<u64>,
}

impl Default for StagingAdmission {
    fn default() -> Self {
        Self {
            maximum_objects: None,
            maximum_bytes: None,
            maximum_pages: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct ChangeBudget {
    pub authored: AuthoredChangeAdmission,
    pub canonical_edits: CanonicalEditAdmission,
    pub canonical_reads: CanonicalReadAdmission,
    pub canonical_map_update: CanonicalMapUpdateAdmission,
    pub witness_reads: WitnessReadAdmission,
    pub impact: ImpactAdmission,
    pub validation: ValidationAdmission,
    pub tests: TestAdmission,
    pub witness_update: WitnessUpdateAdmission,
    pub staging: StagingAdmission,
}

impl ChangeBudget {
    pub fn policy_limits(self) -> impl Iterator<Item = (&'static str, Option<u64>)> {
        BudgetDimension::ALL.into_iter().map(move |dimension| (dimension.policy_name(), self.maximum(dimension)))
    }

    /// Assign one canonical resource policy field; omitted fields retain their existing state.
    pub fn set_policy_limit(&mut self, name: &str, maximum: Option<u64>) -> Result<(), Diagnostic> {
        let dimension = BudgetDimension::ALL.into_iter().find(|dimension| dimension.policy_name() == name)
            .ok_or_else(|| budget_error("change_budget_unknown_dimension", format!("unknown change resource policy {name}")))?;
        match dimension {
            BudgetDimension::Operations => self.authored.maximum_operations = maximum,
            BudgetDimension::Preconditions => self.authored.maximum_preconditions = maximum,
            BudgetDimension::AllocatedIdentities => self.authored.maximum_allocated_identities = maximum,
            BudgetDimension::AuthoredTypeNodes => self.authored.maximum_type_nodes = maximum,
            BudgetDimension::CanonicalOwnerEdits => self.canonical_edits.maximum_owner_edits = maximum,
            BudgetDimension::CanonicalTypeEdits => self.canonical_edits.maximum_type_edits = maximum,
            BudgetDimension::CanonicalDependencyEdits => self.canonical_edits.maximum_dependency_edits = maximum,
            BudgetDimension::CanonicalRetirementEdits => self.canonical_edits.maximum_retirement_edits = maximum,
            BudgetDimension::CanonicalPointReads => self.canonical_reads.maximum_point_reads = maximum,
            BudgetDimension::CanonicalMapPages => self.canonical_reads.maximum_map_pages = maximum,
            BudgetDimension::CanonicalMapEntries => self.canonical_reads.maximum_map_entries = maximum,
            BudgetDimension::CanonicalCatalogLookups => self.canonical_reads.maximum_catalog_lookups = maximum,
            BudgetDimension::CanonicalObjects => self.canonical_reads.maximum_objects = maximum,
            BudgetDimension::CanonicalBytes => self.canonical_reads.maximum_bytes = maximum,
            BudgetDimension::CanonicalDecodedRecords => self.canonical_reads.maximum_decoded_records = maximum,
            BudgetDimension::CanonicalMapPagesEncoded => self.canonical_map_update.maximum_pages_encoded = maximum,
            BudgetDimension::CanonicalMapBytesEncoded => self.canonical_map_update.maximum_bytes_encoded = maximum,
            BudgetDimension::WitnessPointReads => self.witness_reads.maximum_point_reads = maximum,
            BudgetDimension::WitnessMapPages => self.witness_reads.maximum_map_pages = maximum,
            BudgetDimension::WitnessMapEntries => self.witness_reads.maximum_map_entries = maximum,
            BudgetDimension::WitnessCatalogLookups => self.witness_reads.maximum_catalog_lookups = maximum,
            BudgetDimension::WitnessObjects => self.witness_reads.maximum_objects = maximum,
            BudgetDimension::WitnessBytes => self.witness_reads.maximum_bytes = maximum,
            BudgetDimension::WitnessDecodedRecords => self.witness_reads.maximum_decoded_records = maximum,
            BudgetDimension::AffectedFrontierOwners => self.impact.maximum_affected_owners = maximum,
            BudgetDimension::ImpactSummaryOwners => self.impact.maximum_summary_owners = maximum,
            BudgetDimension::ImpactSummaryEdits => self.impact.maximum_summary_edits = maximum,
            BudgetDimension::ImpactOwnershipSteps => self.impact.maximum_ownership_steps = maximum,
            BudgetDimension::ImpactBehaviorOwners => self.impact.maximum_behavior_owners = maximum,
            BudgetDimension::RelationEdges => self.impact.maximum_relation_edges = maximum,
            BudgetDimension::RelationFanout => self.impact.maximum_relation_fanout = maximum,
            BudgetDimension::ValidationOwnerRecords => self.validation.maximum_owner_records = maximum,
            BudgetDimension::ValidationOwnershipEntries => self.validation.maximum_ownership_entries = maximum,
            BudgetDimension::ValidationTypeObjects => self.validation.maximum_type_objects = maximum,
            BudgetDimension::ValidationExpressionSteps => self.validation.maximum_expression_steps = maximum,
            BudgetDimension::ValidationDiagnostics => self.validation.maximum_diagnostics = maximum,
            BudgetDimension::TestsSelected => self.tests.maximum_selected = maximum,
            BudgetDimension::TestDependenciesPerTest => self.tests.maximum_dependencies_per_test = maximum,
            BudgetDimension::TestOwnershipSteps => self.tests.maximum_ownership_steps = maximum,
            BudgetDimension::TestOwnersVisited => self.tests.maximum_owners_visited = maximum,
            BudgetDimension::WitnessEdits => self.witness_update.maximum_edits = maximum,
            BudgetDimension::WitnessMapPagesEncoded => self.witness_update.maximum_pages_encoded = maximum,
            BudgetDimension::WitnessMapBytesEncoded => self.witness_update.maximum_bytes_encoded = maximum,
            BudgetDimension::StagedObjects => self.staging.maximum_objects = maximum,
            BudgetDimension::StagedBytes => self.staging.maximum_bytes = maximum,
            BudgetDimension::StagedPages => self.staging.maximum_pages = maximum,
        }
        Ok(())
    }

    pub(crate) const fn maximum(self, dimension: BudgetDimension) -> Option<u64> {
        match dimension {
            BudgetDimension::Operations => self.authored.maximum_operations,
            BudgetDimension::Preconditions => self.authored.maximum_preconditions,
            BudgetDimension::AllocatedIdentities => self.authored.maximum_allocated_identities,
            BudgetDimension::AuthoredTypeNodes => self.authored.maximum_type_nodes,
            BudgetDimension::CanonicalOwnerEdits => self.canonical_edits.maximum_owner_edits,
            BudgetDimension::CanonicalTypeEdits => self.canonical_edits.maximum_type_edits,
            BudgetDimension::CanonicalDependencyEdits => self.canonical_edits.maximum_dependency_edits,
            BudgetDimension::CanonicalRetirementEdits => self.canonical_edits.maximum_retirement_edits,
            BudgetDimension::CanonicalPointReads => self.canonical_reads.maximum_point_reads,
            BudgetDimension::CanonicalMapPages => self.canonical_reads.maximum_map_pages,
            BudgetDimension::CanonicalMapEntries => self.canonical_reads.maximum_map_entries,
            BudgetDimension::CanonicalCatalogLookups => self.canonical_reads.maximum_catalog_lookups,
            BudgetDimension::CanonicalObjects => self.canonical_reads.maximum_objects,
            BudgetDimension::CanonicalBytes => self.canonical_reads.maximum_bytes,
            BudgetDimension::CanonicalDecodedRecords => self.canonical_reads.maximum_decoded_records,
            BudgetDimension::CanonicalMapPagesEncoded => self.canonical_map_update.maximum_pages_encoded,
            BudgetDimension::CanonicalMapBytesEncoded => self.canonical_map_update.maximum_bytes_encoded,
            BudgetDimension::WitnessPointReads => self.witness_reads.maximum_point_reads,
            BudgetDimension::WitnessMapPages => self.witness_reads.maximum_map_pages,
            BudgetDimension::WitnessMapEntries => self.witness_reads.maximum_map_entries,
            BudgetDimension::WitnessCatalogLookups => self.witness_reads.maximum_catalog_lookups,
            BudgetDimension::WitnessObjects => self.witness_reads.maximum_objects,
            BudgetDimension::WitnessBytes => self.witness_reads.maximum_bytes,
            BudgetDimension::WitnessDecodedRecords => self.witness_reads.maximum_decoded_records,
            BudgetDimension::AffectedFrontierOwners => self.impact.maximum_affected_owners,
            BudgetDimension::ImpactSummaryOwners => self.impact.maximum_summary_owners,
            BudgetDimension::ImpactSummaryEdits => self.impact.maximum_summary_edits,
            BudgetDimension::ImpactOwnershipSteps => self.impact.maximum_ownership_steps,
            BudgetDimension::ImpactBehaviorOwners => self.impact.maximum_behavior_owners,
            BudgetDimension::RelationEdges => self.impact.maximum_relation_edges,
            BudgetDimension::RelationFanout => self.impact.maximum_relation_fanout,
            BudgetDimension::ValidationOwnerRecords => self.validation.maximum_owner_records,
            BudgetDimension::ValidationOwnershipEntries => self.validation.maximum_ownership_entries,
            BudgetDimension::ValidationTypeObjects => self.validation.maximum_type_objects,
            BudgetDimension::ValidationExpressionSteps => self.validation.maximum_expression_steps,
            BudgetDimension::ValidationDiagnostics => self.validation.maximum_diagnostics,
            BudgetDimension::TestsSelected => self.tests.maximum_selected,
            BudgetDimension::TestDependenciesPerTest => self.tests.maximum_dependencies_per_test,
            BudgetDimension::TestOwnershipSteps => self.tests.maximum_ownership_steps,
            BudgetDimension::TestOwnersVisited => self.tests.maximum_owners_visited,
            BudgetDimension::WitnessEdits => self.witness_update.maximum_edits,
            BudgetDimension::WitnessMapPagesEncoded => self.witness_update.maximum_pages_encoded,
            BudgetDimension::WitnessMapBytesEncoded => self.witness_update.maximum_bytes_encoded,
            BudgetDimension::StagedObjects => self.staging.maximum_objects,
            BudgetDimension::StagedBytes => self.staging.maximum_bytes,
            BudgetDimension::StagedPages => self.staging.maximum_pages,
        }
    }

    /// Every explicit u64 allowance is valid, including zero and the maximum value.
    /// Structural limits remain the responsibility of the owning representation.
    pub fn validate(self) -> Result<Self, Diagnostic> {
        Ok(self)
    }

    pub(crate) fn validate_request_counts(
        self,
        changes: usize,
        preconditions: usize,
    ) -> Result<Self, Diagnostic> {
        let budget = self.validate()?;
        check_dimension(
            "authored request",
            "operations",
            count(changes),
            budget.authored.maximum_operations,
        )?;
        check_dimension(
            "authored request",
            "preconditions",
            count(preconditions),
            budget.authored.maximum_preconditions,
        )?;
        Ok(budget)
    }

    pub(crate) fn check_allocated_identities(self, identities: usize) -> Result<(), Diagnostic> {
        check_dimension(
            "authored allocation",
            "allocated_identities",
            count(identities),
            self.authored.maximum_allocated_identities,
        )
    }

    pub(crate) fn check_canonical_edit_counts(
        self,
        owner_edits: u64,
        type_edits: u64,
        dependency_edits: u64,
        retirement_edits: u64,
        phase: &str,
    ) -> Result<(), Diagnostic> {
        self.check_observed(
            ChangeBudgetWork {
                canonical_edits: CanonicalEditWork {
                    owner_edits,
                    type_edits,
                    dependency_edits,
                    retirement_edits,
                },
                ..ChangeBudgetWork::default()
            },
            phase,
        )
    }

    pub(crate) fn check_observed(
        self,
        work: ChangeBudgetWork,
        phase: &str,
    ) -> Result<(), Diagnostic> {
        ChangeBudgetMeter {
            budget: self.validate()?,
            work,
        }
        .check(phase)
    }

    pub(crate) fn remaining_relation_edges(
        self,
        observed: BudgetCounter,
        phase: &str,
    ) -> Result<Option<u64>, Diagnostic> {
        observed.remaining(self.impact.maximum_relation_edges, phase, "relation_edges")
    }

    pub(crate) fn canonical_store_read_admission(
        self,
        observed: ChangeBudgetWork,
        phase: &str,
    ) -> Result<crate::platform::storage::object::StoreReadAdmission, Diagnostic> {
        self.check_observed(observed, phase)?;
        Ok(crate::platform::storage::object::StoreReadAdmission::new(
            crate::platform::storage::object::StoreReadLimits {
                maximum_catalog_lookups: remaining_dimension(
                    phase,
                    "canonical_catalog_lookups",
                    observed.canonical_reads.catalog_lookups,
                    self.canonical_reads.maximum_catalog_lookups,
                )?,
                maximum_objects: remaining_dimension(
                    phase,
                    "canonical_objects",
                    observed.canonical_reads.objects_read,
                    self.canonical_reads.maximum_objects,
                )?,
                maximum_bytes: remaining_dimension(
                    phase,
                    "canonical_bytes",
                    observed.canonical_reads.bytes_read,
                    self.canonical_reads.maximum_bytes,
                )?,
            },
        ))
    }

    pub(crate) fn canonical_map_admission(
        self,
        observed: ChangeBudgetWork,
        phase: &str,
    ) -> Result<crate::platform::persistent_map::MapAdmission, Diagnostic> {
        self.check_observed(observed, phase)?;
        Ok(crate::platform::persistent_map::MapAdmission {
            maximum_pages_read: remaining_dimension(
                phase,
                "canonical_map_pages",
                observed.canonical_reads.map_pages_read,
                self.canonical_reads.maximum_map_pages,
            )?,
            maximum_bytes_read: remaining_dimension(
                phase,
                "canonical_bytes",
                observed.canonical_reads.bytes_read,
                self.canonical_reads.maximum_bytes,
            )?,
            maximum_entries_visited: remaining_dimension(
                phase,
                "canonical_map_entries",
                observed.canonical_reads.map_entries_visited,
                self.canonical_reads.maximum_map_entries,
            )?,
            maximum_pages_encoded: remaining_dimension(
                phase,
                "canonical_map_pages_encoded",
                observed.canonical_map_update.pages_encoded,
                self.canonical_map_update.maximum_pages_encoded,
            )?,
            maximum_bytes_encoded: remaining_dimension(
                phase,
                "canonical_map_bytes_encoded",
                observed.canonical_map_update.bytes_encoded,
                self.canonical_map_update.maximum_bytes_encoded,
            )?,
        })
    }
}

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct CanonicalEditWork {
    pub owner_edits: u64,
    pub type_edits: u64,
    pub dependency_edits: u64,
    pub retirement_edits: u64,
}

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct CanonicalMapUpdateWork {
    pub pages_encoded: u64,
    pub bytes_encoded: u64,
}

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct ValidationBudgetWork {
    pub owner_records: u64,
    pub ownership_entries: u64,
    pub type_objects: u64,
    pub expression_steps: u64,
    pub diagnostics: u64,
}

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct TestBudgetWork {
    pub selected: u64,
    pub ownership_steps: u64,
    pub owners_visited: u64,
}

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct WitnessUpdateWork {
    pub edits: u64,
    pub pages_encoded: u64,
    pub bytes_encoded: u64,
}

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct StagingBudgetWork {
    pub objects: u64,
    pub bytes: u64,
    pub pages: u64,
}

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct ChangeBudgetWork {
    pub authored_operations: u64,
    pub preconditions_checked: u64,
    pub allocated_identities: u64,
    pub authored_type_nodes: u64,
    pub canonical_edits: CanonicalEditWork,
    pub canonical_reads: CanonicalReadWork,
    pub canonical_map_update: CanonicalMapUpdateWork,
    pub witness_reads: WitnessReadWork,
    pub affected_frontier_owners: u64,
    pub impact_summary_owners: u64,
    pub impact_summary_edits: u64,
    pub impact_ownership_steps: u64,
    pub impact_behavior_owners: u64,
    pub relation_edges: u64,
    pub validation: ValidationBudgetWork,
    pub tests: TestBudgetWork,
    pub witness_update: WitnessUpdateWork,
    pub staging: StagingBudgetWork,
    pub saturation: WorkSaturation,
}

impl ChangeBudgetWork {
    pub fn observation_saturation(self) -> WorkSaturation {
        let mut saturation = self.saturation;
        saturation.include(self.canonical_reads.saturation);
        saturation.include(self.witness_reads.saturation);
        saturation
    }

    pub fn observation_flags_valid(self) -> bool {
        let saturation = self.observation_saturation();
        saturation.valid() && BudgetDimension::ALL.into_iter().all(|dimension| {
            match self.observation(dimension) {
                Some(counter) => !counter.saturated || counter.consumed == u64::MAX,
                None => !saturation.contains(dimension),
            }
        })
    }

    pub(crate) fn observation(self, dimension: BudgetDimension) -> Option<BudgetCounter> {
        let consumed = match dimension {
            BudgetDimension::Operations => self.authored_operations,
            BudgetDimension::Preconditions => self.preconditions_checked,
            BudgetDimension::AllocatedIdentities => self.allocated_identities,
            BudgetDimension::AuthoredTypeNodes => self.authored_type_nodes,
            BudgetDimension::CanonicalOwnerEdits => self.canonical_edits.owner_edits,
            BudgetDimension::CanonicalTypeEdits => self.canonical_edits.type_edits,
            BudgetDimension::CanonicalDependencyEdits => self.canonical_edits.dependency_edits,
            BudgetDimension::CanonicalRetirementEdits => self.canonical_edits.retirement_edits,
            BudgetDimension::CanonicalPointReads => self.canonical_reads.point_reads,
            BudgetDimension::CanonicalMapPages => self.canonical_reads.map_pages_read,
            BudgetDimension::CanonicalMapEntries => self.canonical_reads.map_entries_visited,
            BudgetDimension::CanonicalCatalogLookups => self.canonical_reads.catalog_lookups,
            BudgetDimension::CanonicalObjects => self.canonical_reads.objects_read,
            BudgetDimension::CanonicalBytes => self.canonical_reads.bytes_read,
            BudgetDimension::CanonicalDecodedRecords => self.canonical_reads.canonical_records_decoded,
            BudgetDimension::CanonicalMapPagesEncoded => self.canonical_map_update.pages_encoded,
            BudgetDimension::CanonicalMapBytesEncoded => self.canonical_map_update.bytes_encoded,
            BudgetDimension::WitnessPointReads => self.witness_reads.point_reads,
            BudgetDimension::WitnessMapPages => self.witness_reads.map_pages_read,
            BudgetDimension::WitnessMapEntries => self.witness_reads.map_entries_visited,
            BudgetDimension::WitnessCatalogLookups => self.witness_reads.catalog_lookups,
            BudgetDimension::WitnessObjects => self.witness_reads.objects_read,
            BudgetDimension::WitnessBytes => self.witness_reads.bytes_read,
            BudgetDimension::WitnessDecodedRecords => self.witness_reads.witness_records_decoded,
            BudgetDimension::AffectedFrontierOwners => self.affected_frontier_owners,
            BudgetDimension::ImpactSummaryOwners => self.impact_summary_owners,
            BudgetDimension::ImpactSummaryEdits => self.impact_summary_edits,
            BudgetDimension::ImpactOwnershipSteps => self.impact_ownership_steps,
            BudgetDimension::ImpactBehaviorOwners => self.impact_behavior_owners,
            BudgetDimension::RelationEdges => self.relation_edges,
            BudgetDimension::ValidationOwnerRecords => self.validation.owner_records,
            BudgetDimension::ValidationOwnershipEntries => self.validation.ownership_entries,
            BudgetDimension::ValidationTypeObjects => self.validation.type_objects,
            BudgetDimension::ValidationExpressionSteps => self.validation.expression_steps,
            BudgetDimension::ValidationDiagnostics => self.validation.diagnostics,
            BudgetDimension::TestsSelected => self.tests.selected,
            BudgetDimension::TestOwnershipSteps => self.tests.ownership_steps,
            BudgetDimension::TestOwnersVisited => self.tests.owners_visited,
            BudgetDimension::WitnessEdits => self.witness_update.edits,
            BudgetDimension::WitnessMapPagesEncoded => self.witness_update.pages_encoded,
            BudgetDimension::WitnessMapBytesEncoded => self.witness_update.bytes_encoded,
            BudgetDimension::StagedObjects => self.staging.objects,
            BudgetDimension::StagedBytes => self.staging.bytes,
            BudgetDimension::StagedPages => self.staging.pages,
            BudgetDimension::RelationFanout | BudgetDimension::TestDependenciesPerTest => return None,
        };
        Some(BudgetCounter { consumed, saturated: self.observation_saturation().contains(dimension) })
    }

    pub(crate) fn charge(&mut self, dimension: BudgetDimension, amount: BudgetCounter, maximum: Option<u64>, phase: &str) -> Result<(), Diagnostic> {
        let Some(mut observed) = self.observation(dimension) else {
            return Err(budget_error("change_budget_observation_dimension", "per-value policy has no cumulative work counter"));
        };
        observed.charge(amount, maximum, phase, dimension.name())?;
        match dimension {
            BudgetDimension::Operations => self.authored_operations = observed.consumed,
            BudgetDimension::Preconditions => self.preconditions_checked = observed.consumed,
            BudgetDimension::AllocatedIdentities => self.allocated_identities = observed.consumed,
            BudgetDimension::AuthoredTypeNodes => self.authored_type_nodes = observed.consumed,
            BudgetDimension::CanonicalOwnerEdits => self.canonical_edits.owner_edits = observed.consumed,
            BudgetDimension::CanonicalTypeEdits => self.canonical_edits.type_edits = observed.consumed,
            BudgetDimension::CanonicalDependencyEdits => self.canonical_edits.dependency_edits = observed.consumed,
            BudgetDimension::CanonicalRetirementEdits => self.canonical_edits.retirement_edits = observed.consumed,
            BudgetDimension::CanonicalPointReads => self.canonical_reads.point_reads = observed.consumed,
            BudgetDimension::CanonicalMapPages => self.canonical_reads.map_pages_read = observed.consumed,
            BudgetDimension::CanonicalMapEntries => self.canonical_reads.map_entries_visited = observed.consumed,
            BudgetDimension::CanonicalCatalogLookups => self.canonical_reads.catalog_lookups = observed.consumed,
            BudgetDimension::CanonicalObjects => self.canonical_reads.objects_read = observed.consumed,
            BudgetDimension::CanonicalBytes => self.canonical_reads.bytes_read = observed.consumed,
            BudgetDimension::CanonicalDecodedRecords => self.canonical_reads.canonical_records_decoded = observed.consumed,
            BudgetDimension::CanonicalMapPagesEncoded => self.canonical_map_update.pages_encoded = observed.consumed,
            BudgetDimension::CanonicalMapBytesEncoded => self.canonical_map_update.bytes_encoded = observed.consumed,
            BudgetDimension::WitnessPointReads => self.witness_reads.point_reads = observed.consumed,
            BudgetDimension::WitnessMapPages => self.witness_reads.map_pages_read = observed.consumed,
            BudgetDimension::WitnessMapEntries => self.witness_reads.map_entries_visited = observed.consumed,
            BudgetDimension::WitnessCatalogLookups => self.witness_reads.catalog_lookups = observed.consumed,
            BudgetDimension::WitnessObjects => self.witness_reads.objects_read = observed.consumed,
            BudgetDimension::WitnessBytes => self.witness_reads.bytes_read = observed.consumed,
            BudgetDimension::WitnessDecodedRecords => self.witness_reads.witness_records_decoded = observed.consumed,
            BudgetDimension::AffectedFrontierOwners => self.affected_frontier_owners = observed.consumed,
            BudgetDimension::ImpactSummaryOwners => self.impact_summary_owners = observed.consumed,
            BudgetDimension::ImpactSummaryEdits => self.impact_summary_edits = observed.consumed,
            BudgetDimension::ImpactOwnershipSteps => self.impact_ownership_steps = observed.consumed,
            BudgetDimension::ImpactBehaviorOwners => self.impact_behavior_owners = observed.consumed,
            BudgetDimension::RelationEdges => self.relation_edges = observed.consumed,
            BudgetDimension::ValidationOwnerRecords => self.validation.owner_records = observed.consumed,
            BudgetDimension::ValidationOwnershipEntries => self.validation.ownership_entries = observed.consumed,
            BudgetDimension::ValidationTypeObjects => self.validation.type_objects = observed.consumed,
            BudgetDimension::ValidationExpressionSteps => self.validation.expression_steps = observed.consumed,
            BudgetDimension::ValidationDiagnostics => self.validation.diagnostics = observed.consumed,
            BudgetDimension::TestsSelected => self.tests.selected = observed.consumed,
            BudgetDimension::TestOwnershipSteps => self.tests.ownership_steps = observed.consumed,
            BudgetDimension::TestOwnersVisited => self.tests.owners_visited = observed.consumed,
            BudgetDimension::WitnessEdits => self.witness_update.edits = observed.consumed,
            BudgetDimension::WitnessMapPagesEncoded => self.witness_update.pages_encoded = observed.consumed,
            BudgetDimension::WitnessMapBytesEncoded => self.witness_update.bytes_encoded = observed.consumed,
            BudgetDimension::StagedObjects => self.staging.objects = observed.consumed,
            BudgetDimension::StagedBytes => self.staging.bytes = observed.consumed,
            BudgetDimension::StagedPages => self.staging.pages = observed.consumed,
            BudgetDimension::RelationFanout | BudgetDimension::TestDependenciesPerTest => return Err(budget_error("change_budget_observation_dimension", "per-value policy has no cumulative work counter")),
        }
        if observed.saturated {
            self.saturation.mark(dimension);
            match dimension {
                BudgetDimension::CanonicalPointReads | BudgetDimension::CanonicalMapPages | BudgetDimension::CanonicalMapEntries | BudgetDimension::CanonicalCatalogLookups | BudgetDimension::CanonicalObjects | BudgetDimension::CanonicalBytes | BudgetDimension::CanonicalDecodedRecords => self.canonical_reads.saturation.mark(dimension),
                BudgetDimension::WitnessPointReads | BudgetDimension::WitnessMapPages | BudgetDimension::WitnessMapEntries | BudgetDimension::WitnessCatalogLookups | BudgetDimension::WitnessObjects | BudgetDimension::WitnessBytes | BudgetDimension::WitnessDecodedRecords => self.witness_reads.saturation.mark(dimension),
                _ => {},
            }
        }
        Ok(())
    }
}

pub(crate) struct ChangeBudgetMeter {
    budget: ChangeBudget,
    work: ChangeBudgetWork,
}

impl ChangeBudgetMeter {
    fn charge(&mut self, dimension: BudgetDimension, amount: BudgetCounter, phase: &str) -> Result<(), Diagnostic> {
        self.work.charge(dimension, amount, self.budget.maximum(dimension), phase)
    }

    pub(crate) fn new(budget: ChangeBudget, initial: ChangeBudgetWork) -> Result<Self, Diagnostic> {
        let meter = Self {
            budget: budget.validate()?,
            work: initial,
        };
        meter.check("authored normalization")?;
        Ok(meter)
    }

    pub(crate) fn observe_canonical(
        &mut self,
        canonical: &CanonicalDelta,
    ) -> Result<(), Diagnostic> {
        self.work.canonical_edits = CanonicalEditWork {
            owner_edits: count(canonical.owners.len()),
            type_edits: count(canonical.type_additions.len()),
            dependency_edits: count(canonical.dependencies.len()),
            retirement_edits: count(canonical.retirements.len()),
        };
        self.check("canonical delta")
    }

    pub(crate) fn observe_derived(&mut self, derived: &DerivedDelta) -> Result<(), Diagnostic> {
        self.add_witness_reads(derived.read_work)?;
        self.charge(BudgetDimension::ImpactOwnershipSteps, BudgetCounter { consumed: derived.ownership_steps, saturated: derived.saturation.contains(BudgetDimension::ImpactOwnershipSteps) }, "derived witness delta")?;
        self.add_relations(BudgetCounter { consumed: derived.relation_edges_examined, saturated: derived.saturation.contains(BudgetDimension::RelationEdges) })?;
        self.check("derived witness delta")
    }

    pub(crate) fn observe_structural(
        &mut self,
        structural: &super::StructuralValidationReport,
    ) -> Result<(), Diagnostic> {
        self.add_validation(structural.work, 0)?;
        self.check("structural validation")
    }

    pub(crate) fn observe_impact(&mut self, impact: &ImpactPlan) -> Result<(), Diagnostic> {
        let affected = impact.structurally_checked.iter()
            .chain(&impact.semantically_checked).chain(&impact.summary_owners)
            .chain(&impact.compiler_units).chain(&impact.tests).copied().collect::<BTreeSet<_>>().len();
        self.work.affected_frontier_owners = self.work.affected_frontier_owners.max(count(affected));
        self.work.tests.selected = self.work.tests.selected.max(count(impact.tests.len()));
        for (dimension, consumed) in [
            (BudgetDimension::ImpactSummaryOwners, impact.work.summary_owners_examined),
            (BudgetDimension::ImpactSummaryEdits, impact.work.summary_edits_examined),
            (BudgetDimension::ImpactOwnershipSteps, impact.work.ownership_steps),
            (BudgetDimension::ImpactBehaviorOwners, impact.work.behavior_owners_visited),
            (BudgetDimension::RelationEdges, impact.work.reverse_edges_visited),
        ] {
            self.charge(dimension, BudgetCounter { consumed, saturated: impact.work.saturation.contains(dimension) }, "impact planning")?;
        }
        self.add_witness_reads(impact.work.witness_reads)?;
        self.check("impact planning")
    }

    pub(crate) fn observe_summaries(&mut self, summaries: &PlannedSummaries) -> Result<(), Diagnostic> {
        self.add_witness_reads(summaries.initial.read_work)?;
        self.add_witness_reads(summaries.final_delta.read_work)?;
        for summary in [&summaries.initial, &summaries.final_delta] {
            self.add_relations(BudgetCounter { consumed: summary.relation_edges_read, saturated: summary.saturation.contains(BudgetDimension::RelationEdges) })?;
        }
        self.check("owner-summary rebuilding")
    }

    pub(crate) fn validation_admission(&self, phase: &str) -> Result<ValidationAdmission, Diagnostic> {
        let remaining = |dimension: BudgetDimension| {
            let observed = self.work.observation(dimension).ok_or_else(|| budget_error(
                "change_budget_observation_dimension", "validation requires a cumulative work dimension"))?;
            observed.remaining(self.budget.maximum(dimension), phase, dimension.name())
        };
        Ok(ValidationAdmission {
            maximum_owner_records: remaining(BudgetDimension::ValidationOwnerRecords)?,
            maximum_ownership_entries: remaining(BudgetDimension::ValidationOwnershipEntries)?,
            maximum_type_objects: remaining(BudgetDimension::ValidationTypeObjects)?,
            maximum_expression_steps: remaining(BudgetDimension::ValidationExpressionSteps)?,
            maximum_diagnostics: remaining(BudgetDimension::ValidationDiagnostics)?,
        })
    }

    pub(crate) fn observe_completed_validation(&mut self, work: IncrementalValidationWork) -> Result<(), Diagnostic> {
        self.add_validation(work, 0)?;
        self.observe_expression_validation(work.saturation.counter(
            BudgetDimension::ValidationExpressionSteps, work.expression_work), 0)
    }

    pub(crate) fn observe_expression_validation(&mut self, expression_steps: BudgetCounter, diagnostics: u64) -> Result<(), Diagnostic> {
        self.charge(BudgetDimension::ValidationExpressionSteps, expression_steps, "semantic validation")?;
        self.charge(BudgetDimension::ValidationDiagnostics, BudgetCounter::exact(diagnostics), "semantic validation")?;
        self.check("semantic validation")
    }

    pub(crate) fn observe_tests(&mut self, tests: &TestDependencyDelta) -> Result<(), Diagnostic> {
        self.work.tests.selected = self.work.tests.selected.max(count(tests.affected_tests.len()));
        self.work.tests.ownership_steps = tests.work.ownership_steps;
        self.work.tests.owners_visited = tests.work.owners_visited;
        for dimension in [BudgetDimension::TestOwnershipSteps, BudgetDimension::TestOwnersVisited] {
            if tests.work.saturation.contains(dimension) { self.work.saturation.mark(dimension); }
        }
        self.add_relations(BudgetCounter { consumed: tests.work.relation_edges_visited, saturated: tests.work.saturation.contains(BudgetDimension::RelationEdges) })?;
        self.add_witness_reads(tests.work.witness_reads)?;
        self.check("test dependency planning")
    }

    pub(crate) fn preflight_witness_edits(&self, derived: &DerivedDelta, summaries: &super::SummaryDelta, tests: &TestDependencyDelta) -> Result<(), Diagnostic> {
        let mut edits = BudgetCounter::exact(count(summaries.edits.len()));
        for size in [derived.namespaces.len(), derived.ownership.len()] { edits.observe(BudgetCounter::exact(count(size))); }
        for size in [derived.relations.removed.len(), derived.relations.added.len(), tests.removed.len(), tests.added.len()] {
            edits.observe(BudgetCounter::product(count(size), 2));
        }
        edits.remaining(self.budget.witness_update.maximum_edits, "witness map update", "witness_edits")?;
        Ok(())
    }

    pub(crate) fn observe_witness(&mut self, witness: &WitnessMapUpdate) -> Result<(), Diagnostic> {
        let mut edits = BudgetCounter::default();
        for value in [witness.edits.inserted, witness.edits.replaced, witness.edits.removed, witness.edits.unchanged] { edits.observe(BudgetCounter::exact(value)); }
        edits.saturated |= witness.edits.saturated;
        self.work.witness_update.edits = edits.consumed;
        if edits.saturated { self.work.saturation.mark(BudgetDimension::WitnessEdits); }
        self.work.witness_update.pages_encoded = witness.work.pages_encoded;
        self.work.witness_update.bytes_encoded = witness.work.bytes_encoded;
        if witness.work.counter_saturated(crate::platform::persistent_map::MapWorkCounter::PagesEncoded) { self.work.saturation.mark(BudgetDimension::WitnessMapPagesEncoded); }
        if witness.work.counter_saturated(crate::platform::persistent_map::MapWorkCounter::BytesEncoded) { self.work.saturation.mark(BudgetDimension::WitnessMapBytesEncoded); }
        self.add_witness_reads(witness.read_work)?;
        self.check("witness map update")
    }

    pub(crate) fn witness_map_admission(
        &self,
        phase: &str,
    ) -> Result<super::WitnessMapAdmission, Diagnostic> {
        self.check(phase)?;
        let maximum_bytes = remaining_dimension(
            phase,
            "witness_bytes",
            self.work.witness_reads.bytes_read,
            self.budget.witness_reads.maximum_bytes,
        )?;
        Ok(super::WitnessMapAdmission {
            map: crate::platform::persistent_map::MapAdmission {
                maximum_pages_read: remaining_dimension(
                    phase,
                    "witness_map_pages",
                    self.work.witness_reads.map_pages_read,
                    self.budget.witness_reads.maximum_map_pages,
                )?,
                maximum_entries_visited: remaining_dimension(
                    phase,
                    "witness_map_entries",
                    self.work.witness_reads.map_entries_visited,
                    self.budget.witness_reads.maximum_map_entries,
                )?,
                maximum_bytes_read: maximum_bytes,
                maximum_pages_encoded: remaining_dimension(
                    phase,
                    "witness_map_pages_encoded",
                    self.work.witness_update.pages_encoded,
                    self.budget.witness_update.maximum_pages_encoded,
                )?,
                maximum_bytes_encoded: remaining_dimension(
                    phase,
                    "witness_map_bytes_encoded",
                    self.work.witness_update.bytes_encoded,
                    self.budget.witness_update.maximum_bytes_encoded,
                )?,
            },
            maximum_catalog_lookups: remaining_dimension(
                phase,
                "witness_catalog_lookups",
                self.work.witness_reads.catalog_lookups,
                self.budget.witness_reads.maximum_catalog_lookups,
            )?,
            maximum_objects: remaining_dimension(
                phase,
                "witness_objects",
                self.work.witness_reads.objects_read,
                self.budget.witness_reads.maximum_objects,
            )?,
            maximum_bytes,
        })
    }

    pub(crate) fn observe_canonical_reads(&mut self, reads: CanonicalReadWork) -> Result<(), Diagnostic> {
        for (dimension, consumed) in [
            (BudgetDimension::CanonicalPointReads, reads.point_reads),
            (BudgetDimension::CanonicalMapPages, reads.map_pages_read),
            (BudgetDimension::CanonicalMapEntries, reads.map_entries_visited),
            (BudgetDimension::CanonicalCatalogLookups, reads.catalog_lookups),
            (BudgetDimension::CanonicalObjects, reads.objects_read),
            (BudgetDimension::CanonicalBytes, reads.bytes_read),
            (BudgetDimension::CanonicalDecodedRecords, reads.canonical_records_decoded),
        ] { self.charge(dimension, reads.observation(dimension, consumed), "candidate canonical reads")?; }
        self.check("candidate canonical reads")
    }

    pub(crate) fn remaining_relation_edges(&self, phase: &str) -> Result<Option<u64>, Diagnostic> {
        self.budget
            .remaining_relation_edges(self.work.observation_saturation().counter(BudgetDimension::RelationEdges, self.work.relation_edges), phase)
    }

    pub(crate) fn remaining_impact_ownership_steps(&self, phase: &str) -> Result<Option<u64>, Diagnostic> {
        remaining_dimension(
            phase,
            "impact_ownership_steps",
            self.work.impact_ownership_steps,
            self.budget.impact.maximum_ownership_steps,
        )
    }

    pub(crate) fn remaining_witness_records(&self, phase: &str) -> Result<Option<u64>, Diagnostic> {
        remaining_dimension(
            phase,
            "witness_decoded_records",
            self.work.witness_reads.witness_records_decoded,
            self.budget.witness_reads.maximum_decoded_records,
        )
    }

    pub(crate) fn remaining_test_dependency_changes(&self, derived: &DerivedDelta, summaries: &super::SummaryDelta, phase: &str) -> Result<Option<u64>, Diagnostic> {
        let mut fixed = BudgetCounter::exact(count(summaries.edits.len()));
        for size in [derived.namespaces.len(), derived.ownership.len()] { fixed.observe(BudgetCounter::exact(count(size))); }
        for size in [derived.relations.removed.len(), derived.relations.added.len()] { fixed.observe(BudgetCounter::product(count(size), 2)); }
        fixed.remaining(self.budget.witness_update.maximum_edits, phase, "witness_edits").map(|remaining| remaining.map(|n| n / 2))
    }

    pub(crate) const fn finish(self) -> ChangeBudgetWork {
        self.work
    }

    fn add_validation(&mut self, work: IncrementalValidationWork, diagnostics: u64) -> Result<(), Diagnostic> {
        for (dimension, consumed) in [
            (BudgetDimension::ValidationOwnerRecords, work.owner_records_checked),
            (BudgetDimension::ValidationOwnershipEntries, work.ownership_entries_checked),
            (BudgetDimension::ValidationTypeObjects, work.type_objects_checked),
        ] { self.charge(dimension, BudgetCounter { consumed, saturated: work.saturation.contains(dimension) }, "validation accounting")?; }
        self.charge(BudgetDimension::ValidationDiagnostics, BudgetCounter::exact(diagnostics), "validation accounting")?;
        self.add_witness_reads(work.witness_reads)
    }

    fn add_witness_reads(&mut self, reads: WitnessReadWork) -> Result<(), Diagnostic> {
        for (dimension, consumed) in [
            (BudgetDimension::WitnessPointReads, reads.point_reads),
            (BudgetDimension::WitnessMapPages, reads.map_pages_read),
            (BudgetDimension::WitnessMapEntries, reads.map_entries_visited),
            (BudgetDimension::WitnessCatalogLookups, reads.catalog_lookups),
            (BudgetDimension::WitnessObjects, reads.objects_read),
            (BudgetDimension::WitnessBytes, reads.bytes_read),
            (BudgetDimension::WitnessDecodedRecords, reads.witness_records_decoded),
        ] { self.charge(dimension, reads.observation(dimension, consumed), "witness read accounting")?; }
        Ok(())
    }

    fn add_relations(&mut self, edges: BudgetCounter) -> Result<(), Diagnostic> {
        self.charge(BudgetDimension::RelationEdges, edges, "relation accounting")
    }

    fn check(&self, phase: &str) -> Result<(), Diagnostic> {
        if !self.work.observation_flags_valid() {
            return Err(budget_error("change_budget_observation_flags", "change work has invalid saturation flags"));
        }
        for dimension in BudgetDimension::ALL {
            if let Some(observed) = self.work.observation(dimension) {
                observed.remaining(self.budget.maximum(dimension), phase, dimension.name())?;
            }
        }
        Ok(())
    }
}

fn check_canonical_reads(phase: &str, work: CanonicalReadWork, budget: CanonicalReadAdmission) -> Result<(), Diagnostic> {
    for (dimension, consumed, maximum) in [
        (BudgetDimension::CanonicalPointReads, work.point_reads, budget.maximum_point_reads),
        (BudgetDimension::CanonicalMapPages, work.map_pages_read, budget.maximum_map_pages),
        (BudgetDimension::CanonicalMapEntries, work.map_entries_visited, budget.maximum_map_entries),
        (BudgetDimension::CanonicalCatalogLookups, work.catalog_lookups, budget.maximum_catalog_lookups),
        (BudgetDimension::CanonicalObjects, work.objects_read, budget.maximum_objects),
        (BudgetDimension::CanonicalBytes, work.bytes_read, budget.maximum_bytes),
        (BudgetDimension::CanonicalDecodedRecords, work.canonical_records_decoded, budget.maximum_decoded_records),
    ] {
        work.observation(dimension, consumed).remaining(maximum, phase, dimension.name())?;
    }
    Ok(())
}

fn check_witness_reads(phase: &str, work: WitnessReadWork, budget: WitnessReadAdmission) -> Result<(), Diagnostic> {
    for (dimension, consumed, maximum) in [
        (BudgetDimension::WitnessPointReads, work.point_reads, budget.maximum_point_reads),
        (BudgetDimension::WitnessMapPages, work.map_pages_read, budget.maximum_map_pages),
        (BudgetDimension::WitnessMapEntries, work.map_entries_visited, budget.maximum_map_entries),
        (BudgetDimension::WitnessCatalogLookups, work.catalog_lookups, budget.maximum_catalog_lookups),
        (BudgetDimension::WitnessObjects, work.objects_read, budget.maximum_objects),
        (BudgetDimension::WitnessBytes, work.bytes_read, budget.maximum_bytes),
        (BudgetDimension::WitnessDecodedRecords, work.witness_records_decoded, budget.maximum_decoded_records),
    ] {
        work.observation(dimension, consumed).remaining(maximum, phase, dimension.name())?;
    }
    Ok(())
}

pub(crate) fn check_dimension(
    phase: &str,
    dimension: &'static str,
    observed: u64,
    maximum: Option<u64>,
) -> Result<(), Diagnostic> {
    if let Some(maximum) = maximum {
        if observed > maximum {
            return Err(budget_error(
                budget_code(dimension),
                format!(
                    "{phase} requires {observed} {dimension}, exceeding the declared maximum {maximum}"
                ),
            ));
        }
    }
    Ok(())
}



pub(crate) fn remaining_dimension(
    phase: &str,
    dimension: &'static str,
    observed: u64,
    maximum: Option<u64>,
) -> Result<Option<u64>, Diagnostic> {
    maximum.map(|maximum| {
        maximum.checked_sub(observed).ok_or_else(|| {
            budget_error(
                budget_code(dimension),
                format!(
                    "{phase} already observed {observed} {dimension}, exceeding the declared maximum {maximum}"
                ),
            )
        })
    }).transpose()
}

/// Intersect two caller-selected permissions. No policy is the identity element.
pub(crate) fn minimum_limit<T: Ord>(left: Option<T>, right: Option<T>) -> Option<T> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (None, right) => right,
        (left, None) => left,
    }
}

pub(crate) fn optional_usize(maximum: Option<u64>, dimension: &'static str) -> Result<Option<usize>, Diagnostic> {
    maximum.map(|maximum| usize::try_from(maximum).map_err(|_| budget_error(
        budget_code(dimension), "explicit policy exceeds the host index representation"
    ))).transpose()
}

fn budget_code(dimension: &str) -> &'static str {
    match dimension {
        "operations" => "change_budget_operations",
        "preconditions" => "change_budget_preconditions",
        "allocated_identities" => "change_budget_allocated_identities",
        "authored_type_nodes" => "change_budget_authored_type_nodes",
        "canonical_owner_edits" => "change_budget_canonical_owner_edits",
        "canonical_type_edits" => "change_budget_canonical_type_edits",
        "canonical_dependency_edits" => "change_budget_canonical_dependency_edits",
        "canonical_retirement_edits" => "change_budget_canonical_retirement_edits",
        "canonical_point_reads" => "change_budget_canonical_point_reads",
        "canonical_map_pages" => "change_budget_canonical_map_pages",
        "canonical_map_entries" => "change_budget_canonical_map_entries",
        "canonical_catalog_lookups" => "change_budget_canonical_catalog_lookups",
        "canonical_objects" => "change_budget_canonical_objects",
        "canonical_bytes" => "change_budget_canonical_bytes",
        "canonical_decoded_records" => "change_budget_canonical_decoded_records",
        "canonical_map_pages_encoded" => "change_budget_canonical_map_pages_encoded",
        "canonical_map_bytes_encoded" => "change_budget_canonical_map_bytes_encoded",
        "witness_point_reads" => "change_budget_witness_point_reads",
        "witness_map_pages" => "change_budget_witness_map_pages",
        "witness_map_entries" => "change_budget_witness_map_entries",
        "witness_catalog_lookups" => "change_budget_witness_catalog_lookups",
        "witness_objects" => "change_budget_witness_objects",
        "witness_bytes" => "change_budget_witness_bytes",
        "witness_decoded_records" => "change_budget_witness_decoded_records",
        "affected_frontier_owners" => "change_budget_affected_frontier_owners",
        "impact_summary_owners" => "change_budget_impact_summary_owners",
        "impact_summary_edits" => "change_budget_impact_summary_edits",
        "impact_ownership_steps" => "change_budget_impact_ownership_steps",
        "impact_behavior_owners" => "change_budget_impact_behavior_owners",
        "relation_edges" => "change_budget_relation_edges",
        "relation_fanout" => "change_budget_relation_fanout",
        "validation_owner_records" => "change_budget_validation_owner_records",
        "validation_ownership_entries" => "change_budget_validation_ownership_entries",
        "validation_type_objects" => "change_budget_validation_type_objects",
        "validation_expression_steps" => "change_budget_validation_expression_steps",
        "validation_diagnostics" => "change_budget_validation_diagnostics",
        "tests_selected" => "change_budget_tests_selected",
        "test_dependencies_per_test" => "change_budget_test_dependencies_per_test",
        "test_ownership_steps" => "change_budget_test_ownership_steps",
        "test_owners_visited" => "change_budget_test_owners_visited",
        "witness_edits" => "change_budget_witness_edits",
        "witness_map_pages_encoded" => "change_budget_witness_map_pages_encoded",
        "witness_map_bytes_encoded" => "change_budget_witness_map_bytes_encoded",
        "staged_objects" => "change_budget_staged_objects",
        "staged_bytes" => "change_budget_staged_bytes",
        "staged_pages" => "change_budget_staged_pages",
        _ => "change_budget_unknown_dimension",
    }
}

fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn budget_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Resource, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    type Exhaustion = fn(&mut ChangeBudget, &mut ChangeBudgetWork);

    #[test]
    fn every_admission_dimension_has_an_independent_diagnostic() {
        let cases: &[(Exhaustion, &str)] = &[
            (
                |budget, work| {
                    budget.authored.maximum_operations = Some(1);
                    work.authored_operations = 2;
                },
                "change_budget_operations",
            ),
            (
                |budget, work| {
                    budget.authored.maximum_preconditions = Some(0);
                    work.preconditions_checked = 1;
                },
                "change_budget_preconditions",
            ),
            (
                |budget, work| {
                    budget.authored.maximum_allocated_identities = Some(0);
                    work.allocated_identities = 1;
                },
                "change_budget_allocated_identities",
            ),
            (
                |budget, work| {
                    budget.authored.maximum_type_nodes = Some(0);
                    work.authored_type_nodes = 1;
                },
                "change_budget_authored_type_nodes",
            ),
            (
                |budget, work| {
                    budget.canonical_edits.maximum_owner_edits = Some(0);
                    work.canonical_edits.owner_edits = 1;
                },
                "change_budget_canonical_owner_edits",
            ),
            (
                |budget, work| {
                    budget.canonical_edits.maximum_type_edits = Some(0);
                    work.canonical_edits.type_edits = 1;
                },
                "change_budget_canonical_type_edits",
            ),
            (
                |budget, work| {
                    budget.canonical_edits.maximum_dependency_edits = Some(0);
                    work.canonical_edits.dependency_edits = 1;
                },
                "change_budget_canonical_dependency_edits",
            ),
            (
                |budget, work| {
                    budget.canonical_edits.maximum_retirement_edits = Some(0);
                    work.canonical_edits.retirement_edits = 1;
                },
                "change_budget_canonical_retirement_edits",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_point_reads = Some(0);
                    work.canonical_reads.point_reads = 1;
                },
                "change_budget_canonical_point_reads",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_map_pages = Some(0);
                    work.canonical_reads.map_pages_read = 1;
                },
                "change_budget_canonical_map_pages",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_map_entries = Some(0);
                    work.canonical_reads.map_entries_visited = 1;
                },
                "change_budget_canonical_map_entries",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_catalog_lookups = Some(0);
                    work.canonical_reads.catalog_lookups = 1;
                },
                "change_budget_canonical_catalog_lookups",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_objects = Some(0);
                    work.canonical_reads.objects_read = 1;
                },
                "change_budget_canonical_objects",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_bytes = Some(0);
                    work.canonical_reads.bytes_read = 1;
                },
                "change_budget_canonical_bytes",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_decoded_records = Some(0);
                    work.canonical_reads.canonical_records_decoded = 1;
                },
                "change_budget_canonical_decoded_records",
            ),
            (
                |budget, work| {
                    budget.canonical_map_update.maximum_pages_encoded = Some(0);
                    work.canonical_map_update.pages_encoded = 1;
                },
                "change_budget_canonical_map_pages_encoded",
            ),
            (
                |budget, work| {
                    budget.canonical_map_update.maximum_bytes_encoded = Some(0);
                    work.canonical_map_update.bytes_encoded = 1;
                },
                "change_budget_canonical_map_bytes_encoded",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_point_reads = Some(0);
                    work.witness_reads.point_reads = 1;
                },
                "change_budget_witness_point_reads",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_map_pages = Some(0);
                    work.witness_reads.map_pages_read = 1;
                },
                "change_budget_witness_map_pages",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_map_entries = Some(0);
                    work.witness_reads.map_entries_visited = 1;
                },
                "change_budget_witness_map_entries",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_catalog_lookups = Some(0);
                    work.witness_reads.catalog_lookups = 1;
                },
                "change_budget_witness_catalog_lookups",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_objects = Some(0);
                    work.witness_reads.objects_read = 1;
                },
                "change_budget_witness_objects",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_bytes = Some(0);
                    work.witness_reads.bytes_read = 1;
                },
                "change_budget_witness_bytes",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_decoded_records = Some(0);
                    work.witness_reads.witness_records_decoded = 1;
                },
                "change_budget_witness_decoded_records",
            ),
            (
                |budget, work| {
                    budget.impact.maximum_affected_owners = Some(0);
                    work.affected_frontier_owners = 1;
                },
                "change_budget_affected_frontier_owners",
            ),
            (
                |budget, work| {
                    budget.impact.maximum_summary_owners = Some(0);
                    work.impact_summary_owners = 1;
                },
                "change_budget_impact_summary_owners",
            ),
            (
                |budget, work| {
                    budget.impact.maximum_summary_edits = Some(0);
                    work.impact_summary_edits = 1;
                },
                "change_budget_impact_summary_edits",
            ),
            (
                |budget, work| {
                    budget.impact.maximum_ownership_steps = Some(0);
                    work.impact_ownership_steps = 1;
                },
                "change_budget_impact_ownership_steps",
            ),
            (
                |budget, work| {
                    budget.impact.maximum_behavior_owners = Some(0);
                    work.impact_behavior_owners = 1;
                },
                "change_budget_impact_behavior_owners",
            ),
            (
                |budget, work| {
                    budget.impact.maximum_relation_edges = Some(0);
                    work.relation_edges = 1;
                },
                "change_budget_relation_edges",
            ),
            (
                |budget, work| {
                    budget.validation.maximum_owner_records = Some(0);
                    work.validation.owner_records = 1;
                },
                "change_budget_validation_owner_records",
            ),
            (
                |budget, work| {
                    budget.validation.maximum_ownership_entries = Some(0);
                    work.validation.ownership_entries = 1;
                },
                "change_budget_validation_ownership_entries",
            ),
            (
                |budget, work| {
                    budget.validation.maximum_type_objects = Some(0);
                    work.validation.type_objects = 1;
                },
                "change_budget_validation_type_objects",
            ),
            (
                |budget, work| {
                    budget.validation.maximum_expression_steps = Some(0);
                    work.validation.expression_steps = 1;
                },
                "change_budget_validation_expression_steps",
            ),
            (
                |budget, work| {
                    budget.validation.maximum_diagnostics = Some(0);
                    work.validation.diagnostics = 1;
                },
                "change_budget_validation_diagnostics",
            ),
            (
                |budget, work| {
                    budget.tests.maximum_selected = Some(0);
                    work.tests.selected = 1;
                },
                "change_budget_tests_selected",
            ),
            (
                |budget, work| {
                    budget.tests.maximum_ownership_steps = Some(0);
                    work.tests.ownership_steps = 1;
                },
                "change_budget_test_ownership_steps",
            ),
            (
                |budget, work| {
                    budget.tests.maximum_owners_visited = Some(0);
                    work.tests.owners_visited = 1;
                },
                "change_budget_test_owners_visited",
            ),
            (
                |budget, work| {
                    budget.witness_update.maximum_edits = Some(0);
                    work.witness_update.edits = 1;
                },
                "change_budget_witness_edits",
            ),
            (
                |budget, work| {
                    budget.witness_update.maximum_pages_encoded = Some(0);
                    work.witness_update.pages_encoded = 1;
                },
                "change_budget_witness_map_pages_encoded",
            ),
            (
                |budget, work| {
                    budget.witness_update.maximum_bytes_encoded = Some(0);
                    work.witness_update.bytes_encoded = 1;
                },
                "change_budget_witness_map_bytes_encoded",
            ),
            (
                |budget, work| {
                    budget.staging.maximum_objects = Some(0);
                    work.staging.objects = 1;
                },
                "change_budget_staged_objects",
            ),
            (
                |budget, work| {
                    budget.staging.maximum_bytes = Some(0);
                    work.staging.bytes = 1;
                },
                "change_budget_staged_bytes",
            ),
            (
                |budget, work| {
                    budget.staging.maximum_pages = Some(0);
                    work.staging.pages = 1;
                },
                "change_budget_staged_pages",
            ),
        ];

        for (exhaust, expected) in cases {
            let mut budget = ChangeBudget::default();
            let mut work = ChangeBudgetWork::default();
            exhaust(&mut budget, &mut work);
            let error = budget
                .check_observed(work, "independent admission test")
                .expect_err("one dimension must be exhausted independently");
            assert_eq!(error.code, *expected);
        }
    }

    #[test]
    fn every_policy_dimension_preserves_unbounded_zero_finite_and_maximum() {
        let ordinary = ChangeBudget::default();
        assert_eq!(ordinary.policy_limits().count(), 46);
        assert!(ordinary.policy_limits().all(|(_, maximum)| maximum.is_none()));
        for dimension in BudgetDimension::ALL {
            for maximum in [None, Some(0), Some(1), Some(10_000_001), Some(u64::MAX)] {
                let mut budget = ordinary;
                budget.set_policy_limit(dimension.policy_name(), maximum).unwrap();
                assert_eq!(budget.maximum(dimension), maximum);
                assert_eq!(budget.validate().unwrap(), budget);
                assert_eq!(budget.policy_limits().find(|(name, _)| *name == dimension.policy_name()),
                    Some((dimension.policy_name(), maximum)));
            }
        }
        let mut budget = ordinary;
        assert_eq!(budget.set_policy_limit("not-a-resource", Some(0)).unwrap_err().code,
            "change_budget_unknown_dimension");
        assert_eq!(budget, ordinary);
    }

    #[test]
    fn finite_meter_is_exact_at_zero_at_fit_and_at_u64_maximum() {
        let mut checked = 0;
        for dimension in BudgetDimension::ALL {
            if ChangeBudgetWork::default().observation(dimension).is_none() {
                continue;
            }
            checked += 1;
            for maximum in [0, 1, 10_000_001, u64::MAX] {
                let mut budget = ChangeBudget::default();
                budget.set_policy_limit(dimension.policy_name(), Some(maximum)).unwrap();
                let mut meter = ChangeBudgetMeter::new(budget, ChangeBudgetWork::default()).unwrap();
                meter.charge(dimension, BudgetCounter::exact(maximum), "exact-fit test").unwrap();
                assert_eq!(meter.work.observation(dimension), Some(BudgetCounter::exact(maximum)));
                assert!(!meter.work.observation_saturation().any());
                let before = meter.work;
                assert_eq!(meter.charge(dimension, BudgetCounter::exact(1), "one-over test")
                    .unwrap_err().code, budget_code(dimension.name()));
                assert_eq!(meter.work, before, "failed finite charge must be atomic");
            }
        }
        assert_eq!(checked, 44);
    }

    #[test]
    fn unbounded_observation_saturates_honestly_without_affecting_another_policy() {
        let mut work = ChangeBudgetWork::default();
        work.charge(BudgetDimension::CanonicalBytes, BudgetCounter::exact(u64::MAX), None,
            "passive observation").unwrap();
        assert_eq!(work.observation(BudgetDimension::CanonicalBytes),
            Some(BudgetCounter::exact(u64::MAX)));
        work.charge(BudgetDimension::CanonicalBytes, BudgetCounter::exact(1), None,
            "passive observation").unwrap();
        assert_eq!(work.observation(BudgetDimension::CanonicalBytes),
            Some(BudgetCounter { consumed: u64::MAX, saturated: true }));
        assert!(work.observation_flags_valid());
        let mut budget = ChangeBudget::default();
        budget.canonical_reads.maximum_objects = Some(0);
        budget.check_observed(work, "unrelated finite policy").unwrap();
        let remaining = budget.canonical_reads.remaining_after(work.canonical_reads,
            "nested policy").unwrap();
        assert_eq!(remaining.maximum_bytes, None);
        assert_eq!(remaining.maximum_objects, Some(0));
        budget.canonical_reads.maximum_bytes = Some(u64::MAX);
        assert_eq!(budget.check_observed(work, "finite policy after saturation").unwrap_err().code,
            "change_budget_canonical_bytes");
    }

    #[test]
    fn nested_read_limits_keep_none_zero_and_exact_remaining() {
        let admission = CanonicalReadAdmission {
            maximum_point_reads: Some(3),
            maximum_bytes: Some(u64::MAX),
            ..CanonicalReadAdmission::default()
        };
        let work = CanonicalReadWork {
            point_reads: 3,
            bytes_read: 9,
            ..CanonicalReadWork::default()
        };
        let remaining = admission.remaining_after(work, "nested exact fit").unwrap();
        assert_eq!(remaining.maximum_point_reads, Some(0));
        assert_eq!(remaining.maximum_bytes, Some(u64::MAX - 9));
        assert_eq!(remaining.maximum_objects, None);
        assert_eq!(admission.check_work(CanonicalReadWork { point_reads: 4, ..work },
            "one read over").unwrap_err().code, "change_budget_canonical_point_reads");
        let witness = WitnessReadAdmission { maximum_decoded_records: Some(0),
            ..WitnessReadAdmission::default() };
        assert_eq!(witness.remaining_after(WitnessReadWork::default(), "zero witness policy")
            .unwrap().maximum_decoded_records, Some(0));
        assert_eq!(optional_usize(None, "operations").unwrap(), None);
        assert_eq!(optional_usize(Some(0), "operations").unwrap(), Some(0));
        // The maintained bootstrap and native targets use a 64-bit usize domain.
        assert_eq!(optional_usize(Some(u64::MAX), "operations").unwrap(), Some(usize::MAX));
    }

    #[test]
    fn observation_flags_reject_unknown_or_non_saturated_values() {
        let mut work = ChangeBudgetWork::default();
        work.saturation.mark(BudgetDimension::CanonicalBytes);
        assert!(!work.observation_flags_valid());
        work.canonical_reads.bytes_read = u64::MAX;
        assert!(work.observation_flags_valid());
        work.saturation.bits |= 1_u64 << 63;
        assert!(!work.observation_flags_valid());
    }

    #[test]
    fn validation_admission_debits_every_initial_dimension_exactly() {
        let mut budget = ChangeBudget::default();
        let mut initial = ChangeBudgetWork::default();
        for (dimension, maximum, consumed) in [
            (BudgetDimension::ValidationOwnerRecords, 3, 1),
            (BudgetDimension::ValidationOwnershipEntries, 4, 2),
            (BudgetDimension::ValidationTypeObjects, 5, 3),
            (BudgetDimension::ValidationExpressionSteps, u64::MAX, u64::MAX - 1),
            (BudgetDimension::ValidationDiagnostics, 0, 0),
        ] {
            budget.set_policy_limit(dimension.policy_name(), Some(maximum)).unwrap();
            initial.charge(dimension, BudgetCounter::exact(consumed), None, "prior phase").unwrap();
        }
        let meter = ChangeBudgetMeter::new(budget, initial).unwrap();
        assert_eq!(meter.validation_admission("next validation phase").unwrap(), ValidationAdmission {
            maximum_owner_records: Some(2),
            maximum_ownership_entries: Some(2),
            maximum_type_objects: Some(2),
            maximum_expression_steps: Some(1),
            maximum_diagnostics: Some(0),
        });
        let mut unbounded = ChangeBudgetWork::default();
        unbounded.charge(BudgetDimension::ValidationExpressionSteps,
            BudgetCounter::exact(u64::MAX), None, "passive work").unwrap();
        unbounded.charge(BudgetDimension::ValidationExpressionSteps,
            BudgetCounter::exact(1), None, "passive work").unwrap();
        assert!(unbounded.observation_saturation().any());
        let meter = ChangeBudgetMeter::new(ChangeBudget::default(), unbounded).unwrap();
        assert_eq!(meter.validation_admission("unbounded next phase").unwrap(), ValidationAdmission::default());
    }

}
