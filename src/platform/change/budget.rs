//! Independent deterministic resource budgets and observations for one semantic change.

use super::{
    CanonicalDelta, CanonicalReadWork, DerivedDelta, ImpactPlan, IncrementalValidationWork,
    PlannedSummaries, TestDependencyDelta, WitnessMapUpdate, WitnessReadWork,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use bincode::{Decode, Encode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

// Deterministic per-request engine ceilings. Count ceilings bound the named work unit at the
// change boundary; byte ceilings are measured in bytes retained by isolated preparation. They are
// implementation ceilings, not semantic language limits. Relation and test-prefix ceilings below
// also respect their witness-decoder format bounds.
pub const MAXIMUM_CHANGE_OPERATIONS: u64 = 10_000;
pub const MAXIMUM_CHANGE_PRECONDITIONS: u64 = 10_000;
pub const MAXIMUM_CHANGE_ALLOCATED_IDENTITIES: u64 = 1_000_000;
pub const MAXIMUM_CHANGE_AUTHORED_TYPE_NODES: u64 = 1_000_000;
pub const MAXIMUM_CHANGE_OWNER_EDITS: u64 = 1_000_000;
pub const MAXIMUM_CHANGE_TYPE_EDITS: u64 = 1_000_000;
pub const MAXIMUM_CHANGE_DEPENDENCY_EDITS: u64 = 100_000;
pub const MAXIMUM_CHANGE_RETIREMENT_EDITS: u64 = 1_000_000;
pub const MAXIMUM_CHANGE_CANONICAL_POINT_READS: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_CANONICAL_MAP_PAGES: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_CANONICAL_MAP_ENTRIES: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_CANONICAL_CATALOG_LOOKUPS: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_CANONICAL_OBJECTS: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_CANONICAL_BYTES: u64 = 512 * 1_048_576;
pub const MAXIMUM_CHANGE_CANONICAL_RECORDS: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_CANONICAL_MAP_PAGES_ENCODED: u64 = 1_000_000;
pub const MAXIMUM_CHANGE_CANONICAL_MAP_BYTES_ENCODED: u64 = 512 * 1_048_576;
pub const MAXIMUM_CHANGE_WITNESS_POINT_READS: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_WITNESS_MAP_PAGES: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_WITNESS_MAP_ENTRIES: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_WITNESS_CATALOG_LOOKUPS: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_WITNESS_OBJECTS: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_WITNESS_BYTES: u64 = 512 * 1_048_576;
pub const MAXIMUM_CHANGE_WITNESS_RECORDS: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_AFFECTED_OWNERS: u64 = 1_000_000;
pub const MAXIMUM_CHANGE_SUMMARY_OWNERS: u64 = 1_000_000;
pub const MAXIMUM_CHANGE_SUMMARY_EDITS: u64 = 1_000_000;
pub const MAXIMUM_CHANGE_IMPACT_OWNERSHIP_STEPS: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_BEHAVIOR_OWNERS: u64 = 1_000_000;
pub const MAXIMUM_CHANGE_RELATION_EDGES: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_RELATION_FANOUT: u64 =
    crate::platform::witness::MAXIMUM_RELATION_PREFIX_ITEMS as u64;
pub const MAXIMUM_CHANGE_VALIDATION_OWNER_RECORDS: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_VALIDATION_OWNERSHIP_ENTRIES: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_VALIDATION_TYPE_OBJECTS: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_VALIDATION_EXPRESSION_STEPS: u64 =
    crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK as u64;
pub const MAXIMUM_CHANGE_VALIDATION_DIAGNOSTICS: u64 = 100_000;
pub const MAXIMUM_CHANGE_SELECTED_TESTS: u64 = 100_000;
pub const MAXIMUM_CHANGE_TEST_DEPENDENCIES_PER_TEST: u64 =
    crate::platform::witness::MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS as u64;
pub const MAXIMUM_CHANGE_TEST_OWNERSHIP_STEPS: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_TEST_OWNERS_VISITED: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_WITNESS_EDITS: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_WITNESS_MAP_PAGES_ENCODED: u64 = 1_000_000;
pub const MAXIMUM_CHANGE_WITNESS_MAP_BYTES_ENCODED: u64 = 512 * 1_048_576;
pub const MAXIMUM_CHANGE_STAGED_OBJECTS: u64 = 1_000_000;
pub const MAXIMUM_CHANGE_STAGED_BYTES: u64 = 512 * 1_048_576;
pub const MAXIMUM_CHANGE_STAGED_PAGES: u64 = 1_000_000;

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.AuthoredChangeAdmissionV2")]
#[serde(deny_unknown_fields)]
pub struct AuthoredChangeAdmission {
    /// Authored operations in the normalized request.
    pub maximum_operations: u64,
    /// Exact preconditions evaluated against the pinned base.
    pub maximum_preconditions: u64,
    /// Durable identities allocated from named or anonymous request definitions.
    pub maximum_allocated_identities: u64,
    /// Distinct canonical type objects interned while lowering the request.
    pub maximum_type_nodes: u64,
}

impl Default for AuthoredChangeAdmission {
    fn default() -> Self {
        Self {
            maximum_operations: 1_000,
            maximum_preconditions: 1_000,
            maximum_allocated_identities: 100_000,
            maximum_type_nodes: 100_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.CanonicalEditAdmissionV1")]
#[serde(deny_unknown_fields)]
pub struct CanonicalEditAdmission {
    /// Distinct accepted-owner bindings created, replaced, or removed.
    pub maximum_owner_edits: u64,
    /// Distinct absent type objects proposed for insertion.
    pub maximum_type_edits: u64,
    /// Distinct package-dependency bindings created, replaced, or removed.
    pub maximum_dependency_edits: u64,
    /// Distinct retirement bindings created or replaced.
    pub maximum_retirement_edits: u64,
}

impl Default for CanonicalEditAdmission {
    fn default() -> Self {
        Self {
            maximum_owner_edits: 100_000,
            maximum_type_edits: 100_000,
            maximum_dependency_edits: 10_000,
            maximum_retirement_edits: 100_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.CanonicalReadAdmissionV2")]
#[serde(deny_unknown_fields)]
pub struct CanonicalReadAdmission {
    /// Logical point-read requests issued to accepted semantic authority.
    pub maximum_point_reads: u64,
    /// Persistent-map pages decoded for those reads.
    pub maximum_map_pages: u64,
    /// Persistent-map entries visited for those reads.
    pub maximum_map_entries: u64,
    /// Immutable-store catalog probes issued for those reads.
    pub maximum_catalog_lookups: u64,
    /// Immutable objects whose payload bytes were returned.
    pub maximum_objects: u64,
    /// Immutable object payload bytes returned.
    pub maximum_bytes: u64,
    /// Canonical semantic records decoded from returned payloads.
    pub maximum_decoded_records: u64,
}

impl Default for CanonicalReadAdmission {
    fn default() -> Self {
        Self {
            maximum_point_reads: 100_000,
            maximum_map_pages: 1_000_000,
            maximum_map_entries: 1_000_000,
            maximum_catalog_lookups: 100_000,
            maximum_objects: 100_000,
            maximum_bytes: 256 * 1_048_576,
            maximum_decoded_records: 100_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.CanonicalMapUpdateAdmissionV1")]
#[serde(deny_unknown_fields)]
pub struct CanonicalMapUpdateAdmission {
    /// Candidate persistent-map page encodings produced across semantic authority maps.
    pub maximum_pages_encoded: u64,
    /// Bytes produced by those candidate page encodings before page reuse is known.
    pub maximum_bytes_encoded: u64,
}

impl Default for CanonicalMapUpdateAdmission {
    fn default() -> Self {
        Self {
            maximum_pages_encoded: 100_000,
            maximum_bytes_encoded: 256 * 1_048_576,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.WitnessReadAdmissionV2")]
#[serde(deny_unknown_fields)]
pub struct WitnessReadAdmission {
    /// Logical point-read requests issued to accepted validation evidence.
    pub maximum_point_reads: u64,
    /// Persistent-map pages decoded for those reads.
    pub maximum_map_pages: u64,
    /// Persistent-map entries visited for those reads.
    pub maximum_map_entries: u64,
    /// Immutable-store catalog probes issued for those reads.
    pub maximum_catalog_lookups: u64,
    /// Immutable witness objects whose payload bytes were returned.
    pub maximum_objects: u64,
    /// Immutable witness object payload bytes returned.
    pub maximum_bytes: u64,
    /// Witness records decoded from returned payloads.
    pub maximum_decoded_records: u64,
}

impl Default for WitnessReadAdmission {
    fn default() -> Self {
        Self {
            maximum_point_reads: 100_000,
            maximum_map_pages: 1_000_000,
            maximum_map_entries: 1_000_000,
            maximum_catalog_lookups: 100_000,
            maximum_objects: 100_000,
            maximum_bytes: 256 * 1_048_576,
            maximum_decoded_records: 1_000_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.ImpactAdmissionV4")]
#[serde(deny_unknown_fields)]
pub struct ImpactAdmission {
    /// Distinct owners in the combined direct and transitive validation frontier.
    pub maximum_affected_owners: u64,
    /// Owner summaries examined while constructing the impact plan.
    pub maximum_summary_owners: u64,
    /// Candidate summary edits examined while constructing the impact plan.
    pub maximum_summary_edits: u64,
    /// Parent links traversed while resolving semantic ownership.
    pub maximum_ownership_steps: u64,
    /// Behavior owners visited while propagating executable impact.
    pub maximum_behavior_owners: u64,
    /// Relation edges extracted, verified, or traversed across the request.
    pub maximum_relation_edges: u64,
    /// Relation edges admitted for any one queried endpoint.
    pub maximum_relation_fanout: u64,
}

impl Default for ImpactAdmission {
    fn default() -> Self {
        Self {
            maximum_affected_owners: 10_000,
            maximum_summary_owners: 10_000,
            maximum_summary_edits: 10_000,
            maximum_ownership_steps: 1_000_000,
            maximum_behavior_owners: 10_000,
            maximum_relation_edges: 100_000,
            maximum_relation_fanout: MAXIMUM_CHANGE_RELATION_FANOUT,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.ValidationAdmissionV2")]
#[serde(deny_unknown_fields)]
pub struct ValidationAdmission {
    /// Owner records checked by structural or semantic validation.
    pub maximum_owner_records: u64,
    /// Ownership bindings checked by structural or semantic validation.
    pub maximum_ownership_entries: u64,
    /// Canonical type objects checked by semantic validation.
    pub maximum_type_objects: u64,
    /// Expression nodes or equivalent semantic inference steps executed.
    pub maximum_expression_steps: u64,
    /// Diagnostics retained before validation stops.
    pub maximum_diagnostics: u64,
}

impl Default for ValidationAdmission {
    fn default() -> Self {
        Self {
            maximum_owner_records: 100_000,
            maximum_ownership_entries: 100_000,
            maximum_type_objects: 100_000,
            maximum_expression_steps: 1_000_000,
            maximum_diagnostics: 1_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.TestAdmissionV2")]
#[serde(deny_unknown_fields)]
pub struct TestAdmission {
    /// Graph-owned tests selected by the impact frontier.
    pub maximum_selected: u64,
    /// Exact dependency records admitted for any one selected test.
    pub maximum_dependencies_per_test: u64,
    /// Parent links traversed while locating affected tests.
    pub maximum_ownership_steps: u64,
    /// Candidate owners visited while locating affected tests.
    pub maximum_owners_visited: u64,
}

impl Default for TestAdmission {
    fn default() -> Self {
        Self {
            maximum_selected: 10_000,
            maximum_dependencies_per_test: MAXIMUM_CHANGE_TEST_DEPENDENCIES_PER_TEST,
            maximum_ownership_steps: 1_000_000,
            maximum_owners_visited: 1_000_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.WitnessUpdateAdmissionV1")]
#[serde(deny_unknown_fields)]
pub struct WitnessUpdateAdmission {
    /// Logical witness-map entries inserted, replaced, removed, or confirmed unchanged.
    pub maximum_edits: u64,
    /// Candidate persistent-map page encodings produced across all witness maps.
    pub maximum_pages_encoded: u64,
    /// Bytes produced by those candidate page encodings before page reuse is known.
    pub maximum_bytes_encoded: u64,
}

impl Default for WitnessUpdateAdmission {
    fn default() -> Self {
        Self {
            maximum_edits: 1_000_000,
            maximum_pages_encoded: 100_000,
            maximum_bytes_encoded: 256 * 1_048_576,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.StagingAdmissionV1")]
#[serde(deny_unknown_fields)]
pub struct StagingAdmission {
    /// Newly isolated immutable objects retained for this preparation.
    pub maximum_objects: u64,
    /// Bytes retained by those newly isolated immutable objects.
    pub maximum_bytes: u64,
    /// Newly isolated persistent-map page objects retained for this preparation.
    pub maximum_pages: u64,
}

impl Default for StagingAdmission {
    fn default() -> Self {
        Self {
            maximum_objects: 1_000_000,
            maximum_bytes: 512 * 1_048_576,
            maximum_pages: 1_000_000,
        }
    }
}

#[derive(
    Clone, Copy, Debug, Decode, Default, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize,
)]
#[schemars(rename = "lkjscript.ChangeBudgetV6")]
#[serde(deny_unknown_fields)]
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
    pub fn validate(self) -> Result<Self, Diagnostic> {
        if self.authored.maximum_operations == 0
            || self.authored.maximum_operations > MAXIMUM_CHANGE_OPERATIONS
        {
            return Err(budget_error(
                "change_budget_invalid_operations",
                format!(
                    "operation maximum must be 1 through {MAXIMUM_CHANGE_OPERATIONS} operations"
                ),
            ));
        }
        for (dimension, maximum, ceiling) in [
            (
                "preconditions",
                self.authored.maximum_preconditions,
                MAXIMUM_CHANGE_PRECONDITIONS,
            ),
            (
                "allocated_identities",
                self.authored.maximum_allocated_identities,
                MAXIMUM_CHANGE_ALLOCATED_IDENTITIES,
            ),
            (
                "authored_type_nodes",
                self.authored.maximum_type_nodes,
                MAXIMUM_CHANGE_AUTHORED_TYPE_NODES,
            ),
            (
                "canonical_owner_edits",
                self.canonical_edits.maximum_owner_edits,
                MAXIMUM_CHANGE_OWNER_EDITS,
            ),
            (
                "canonical_type_edits",
                self.canonical_edits.maximum_type_edits,
                MAXIMUM_CHANGE_TYPE_EDITS,
            ),
            (
                "canonical_dependency_edits",
                self.canonical_edits.maximum_dependency_edits,
                MAXIMUM_CHANGE_DEPENDENCY_EDITS,
            ),
            (
                "canonical_retirement_edits",
                self.canonical_edits.maximum_retirement_edits,
                MAXIMUM_CHANGE_RETIREMENT_EDITS,
            ),
            (
                "canonical_point_reads",
                self.canonical_reads.maximum_point_reads,
                MAXIMUM_CHANGE_CANONICAL_POINT_READS,
            ),
            (
                "canonical_map_pages",
                self.canonical_reads.maximum_map_pages,
                MAXIMUM_CHANGE_CANONICAL_MAP_PAGES,
            ),
            (
                "canonical_map_entries",
                self.canonical_reads.maximum_map_entries,
                MAXIMUM_CHANGE_CANONICAL_MAP_ENTRIES,
            ),
            (
                "canonical_catalog_lookups",
                self.canonical_reads.maximum_catalog_lookups,
                MAXIMUM_CHANGE_CANONICAL_CATALOG_LOOKUPS,
            ),
            (
                "canonical_objects",
                self.canonical_reads.maximum_objects,
                MAXIMUM_CHANGE_CANONICAL_OBJECTS,
            ),
            (
                "canonical_bytes",
                self.canonical_reads.maximum_bytes,
                MAXIMUM_CHANGE_CANONICAL_BYTES,
            ),
            (
                "canonical_decoded_records",
                self.canonical_reads.maximum_decoded_records,
                MAXIMUM_CHANGE_CANONICAL_RECORDS,
            ),
            (
                "canonical_map_pages_encoded",
                self.canonical_map_update.maximum_pages_encoded,
                MAXIMUM_CHANGE_CANONICAL_MAP_PAGES_ENCODED,
            ),
            (
                "canonical_map_bytes_encoded",
                self.canonical_map_update.maximum_bytes_encoded,
                MAXIMUM_CHANGE_CANONICAL_MAP_BYTES_ENCODED,
            ),
            (
                "witness_point_reads",
                self.witness_reads.maximum_point_reads,
                MAXIMUM_CHANGE_WITNESS_POINT_READS,
            ),
            (
                "witness_map_pages",
                self.witness_reads.maximum_map_pages,
                MAXIMUM_CHANGE_WITNESS_MAP_PAGES,
            ),
            (
                "witness_map_entries",
                self.witness_reads.maximum_map_entries,
                MAXIMUM_CHANGE_WITNESS_MAP_ENTRIES,
            ),
            (
                "witness_catalog_lookups",
                self.witness_reads.maximum_catalog_lookups,
                MAXIMUM_CHANGE_WITNESS_CATALOG_LOOKUPS,
            ),
            (
                "witness_objects",
                self.witness_reads.maximum_objects,
                MAXIMUM_CHANGE_WITNESS_OBJECTS,
            ),
            (
                "witness_bytes",
                self.witness_reads.maximum_bytes,
                MAXIMUM_CHANGE_WITNESS_BYTES,
            ),
            (
                "witness_decoded_records",
                self.witness_reads.maximum_decoded_records,
                MAXIMUM_CHANGE_WITNESS_RECORDS,
            ),
            (
                "affected_frontier_owners",
                self.impact.maximum_affected_owners,
                MAXIMUM_CHANGE_AFFECTED_OWNERS,
            ),
            (
                "impact_summary_owners",
                self.impact.maximum_summary_owners,
                MAXIMUM_CHANGE_SUMMARY_OWNERS,
            ),
            (
                "impact_summary_edits",
                self.impact.maximum_summary_edits,
                MAXIMUM_CHANGE_SUMMARY_EDITS,
            ),
            (
                "impact_ownership_steps",
                self.impact.maximum_ownership_steps,
                MAXIMUM_CHANGE_IMPACT_OWNERSHIP_STEPS,
            ),
            (
                "impact_behavior_owners",
                self.impact.maximum_behavior_owners,
                MAXIMUM_CHANGE_BEHAVIOR_OWNERS,
            ),
            (
                "relation_edges",
                self.impact.maximum_relation_edges,
                MAXIMUM_CHANGE_RELATION_EDGES,
            ),
            (
                "relation_fanout",
                self.impact.maximum_relation_fanout,
                MAXIMUM_CHANGE_RELATION_FANOUT,
            ),
            (
                "validation_owner_records",
                self.validation.maximum_owner_records,
                MAXIMUM_CHANGE_VALIDATION_OWNER_RECORDS,
            ),
            (
                "validation_ownership_entries",
                self.validation.maximum_ownership_entries,
                MAXIMUM_CHANGE_VALIDATION_OWNERSHIP_ENTRIES,
            ),
            (
                "validation_type_objects",
                self.validation.maximum_type_objects,
                MAXIMUM_CHANGE_VALIDATION_TYPE_OBJECTS,
            ),
            (
                "validation_expression_steps",
                self.validation.maximum_expression_steps,
                MAXIMUM_CHANGE_VALIDATION_EXPRESSION_STEPS,
            ),
            (
                "validation_diagnostics",
                self.validation.maximum_diagnostics,
                MAXIMUM_CHANGE_VALIDATION_DIAGNOSTICS,
            ),
            (
                "tests_selected",
                self.tests.maximum_selected,
                MAXIMUM_CHANGE_SELECTED_TESTS,
            ),
            (
                "test_dependencies_per_test",
                self.tests.maximum_dependencies_per_test,
                MAXIMUM_CHANGE_TEST_DEPENDENCIES_PER_TEST,
            ),
            (
                "test_ownership_steps",
                self.tests.maximum_ownership_steps,
                MAXIMUM_CHANGE_TEST_OWNERSHIP_STEPS,
            ),
            (
                "test_owners_visited",
                self.tests.maximum_owners_visited,
                MAXIMUM_CHANGE_TEST_OWNERS_VISITED,
            ),
            (
                "witness_edits",
                self.witness_update.maximum_edits,
                MAXIMUM_CHANGE_WITNESS_EDITS,
            ),
            (
                "witness_map_pages_encoded",
                self.witness_update.maximum_pages_encoded,
                MAXIMUM_CHANGE_WITNESS_MAP_PAGES_ENCODED,
            ),
            (
                "witness_map_bytes_encoded",
                self.witness_update.maximum_bytes_encoded,
                MAXIMUM_CHANGE_WITNESS_MAP_BYTES_ENCODED,
            ),
            (
                "staged_objects",
                self.staging.maximum_objects,
                MAXIMUM_CHANGE_STAGED_OBJECTS,
            ),
            (
                "staged_bytes",
                self.staging.maximum_bytes,
                MAXIMUM_CHANGE_STAGED_BYTES,
            ),
            (
                "staged_pages",
                self.staging.maximum_pages,
                MAXIMUM_CHANGE_STAGED_PAGES,
            ),
        ] {
            validate_maximum(dimension, maximum, ceiling)?;
        }
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
        observed: u64,
        phase: &str,
    ) -> Result<u64, Diagnostic> {
        remaining_dimension(
            phase,
            "relation_edges",
            observed,
            self.impact.maximum_relation_edges,
        )
    }

    pub(crate) fn canonical_store_read_admission(
        self,
        observed: ChangeBudgetWork,
        phase: &str,
    ) -> Result<crate::platform::storage::object::StoreReadAdmission, Diagnostic> {
        self.validate()?;
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
        self.validate()?;
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

#[derive(
    Clone, Copy, Debug, Decode, Default, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize,
)]
#[schemars(rename = "lkjscript.CanonicalEditWorkV1")]
#[serde(deny_unknown_fields)]
pub struct CanonicalEditWork {
    pub owner_edits: u64,
    pub type_edits: u64,
    pub dependency_edits: u64,
    pub retirement_edits: u64,
}

#[derive(
    Clone, Copy, Debug, Decode, Default, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize,
)]
#[schemars(rename = "lkjscript.CanonicalMapUpdateWorkV1")]
#[serde(deny_unknown_fields)]
pub struct CanonicalMapUpdateWork {
    pub pages_encoded: u64,
    pub bytes_encoded: u64,
}

#[derive(
    Clone, Copy, Debug, Decode, Default, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize,
)]
#[schemars(rename = "lkjscript.ValidationBudgetWorkV2")]
#[serde(deny_unknown_fields)]
pub struct ValidationBudgetWork {
    pub owner_records: u64,
    pub ownership_entries: u64,
    pub type_objects: u64,
    pub expression_steps: u64,
    pub diagnostics: u64,
}

#[derive(
    Clone, Copy, Debug, Decode, Default, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize,
)]
#[schemars(rename = "lkjscript.TestBudgetWorkV1")]
#[serde(deny_unknown_fields)]
pub struct TestBudgetWork {
    pub selected: u64,
    pub ownership_steps: u64,
    pub owners_visited: u64,
}

#[derive(
    Clone, Copy, Debug, Decode, Default, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize,
)]
#[schemars(rename = "lkjscript.WitnessUpdateWorkV1")]
#[serde(deny_unknown_fields)]
pub struct WitnessUpdateWork {
    pub edits: u64,
    pub pages_encoded: u64,
    pub bytes_encoded: u64,
}

#[derive(
    Clone, Copy, Debug, Decode, Default, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize,
)]
#[schemars(rename = "lkjscript.StagingBudgetWorkV1")]
#[serde(deny_unknown_fields)]
pub struct StagingBudgetWork {
    pub objects: u64,
    pub bytes: u64,
    pub pages: u64,
}

#[derive(
    Clone, Copy, Debug, Decode, Default, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize,
)]
#[schemars(rename = "lkjscript.ChangeBudgetWorkV6")]
#[serde(deny_unknown_fields)]
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
}

impl ChangeBudgetWork {
    pub(crate) fn authored(
        operations: usize,
        preconditions: usize,
        allocated_identities: usize,
        authored_type_nodes: usize,
        canonical_reads: CanonicalReadWork,
        witness_reads: WitnessReadWork,
        relation_edges: u64,
    ) -> Self {
        Self {
            authored_operations: count(operations),
            preconditions_checked: count(preconditions),
            allocated_identities: count(allocated_identities),
            authored_type_nodes: count(authored_type_nodes),
            canonical_reads,
            witness_reads,
            relation_edges,
            ..Self::default()
        }
    }
}

pub(crate) struct ChangeBudgetMeter {
    budget: ChangeBudget,
    work: ChangeBudgetWork,
}

impl ChangeBudgetMeter {
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
        self.add_witness_reads(derived.read_work);
        self.work.impact_ownership_steps = self
            .work
            .impact_ownership_steps
            .saturating_add(derived.ownership_steps);
        self.add_relations(derived.relation_edges_examined);
        self.check("derived witness delta")
    }

    pub(crate) fn observe_structural(
        &mut self,
        structural: &super::StructuralValidationReport,
    ) -> Result<(), Diagnostic> {
        self.add_validation(structural.work, 0);
        self.check("structural validation")
    }

    pub(crate) fn observe_impact(&mut self, impact: &ImpactPlan) -> Result<(), Diagnostic> {
        let affected = impact
            .structurally_checked
            .iter()
            .chain(&impact.semantically_checked)
            .chain(&impact.summary_owners)
            .chain(&impact.compiler_units)
            .chain(&impact.tests)
            .copied()
            .collect::<BTreeSet<_>>()
            .len();
        self.work.affected_frontier_owners =
            self.work.affected_frontier_owners.max(count(affected));
        self.work.impact_summary_owners = self
            .work
            .impact_summary_owners
            .saturating_add(impact.work.summary_owners_examined);
        self.work.impact_summary_edits = self
            .work
            .impact_summary_edits
            .saturating_add(impact.work.summary_edits_examined);
        self.work.tests.selected = self.work.tests.selected.max(count(impact.tests.len()));
        self.work.impact_ownership_steps = self
            .work
            .impact_ownership_steps
            .saturating_add(impact.work.ownership_steps);
        self.work.impact_behavior_owners = self
            .work
            .impact_behavior_owners
            .saturating_add(impact.work.behavior_owners_visited);
        self.add_relations(impact.work.reverse_edges_visited);
        self.add_witness_reads(impact.work.witness_reads);
        self.check("impact planning")
    }

    pub(crate) fn observe_summaries(
        &mut self,
        summaries: &PlannedSummaries,
    ) -> Result<(), Diagnostic> {
        self.add_witness_reads(summaries.initial.read_work);
        self.add_witness_reads(summaries.final_delta.read_work);
        self.add_relations(
            summaries
                .initial
                .relation_edges_read
                .saturating_add(summaries.final_delta.relation_edges_read),
        );
        self.check("owner-summary rebuilding")
    }

    pub(crate) fn observe_expression_validation(
        &mut self,
        expression_steps: u64,
        diagnostics: u64,
    ) -> Result<(), Diagnostic> {
        self.work.validation.expression_steps = self
            .work
            .validation
            .expression_steps
            .saturating_add(expression_steps);
        self.work.validation.diagnostics =
            self.work.validation.diagnostics.saturating_add(diagnostics);
        self.check("semantic validation")
    }

    pub(crate) fn observe_tests(&mut self, tests: &TestDependencyDelta) -> Result<(), Diagnostic> {
        self.work.tests = TestBudgetWork {
            selected: self
                .work
                .tests
                .selected
                .max(count(tests.affected_tests.len())),
            ownership_steps: tests.work.ownership_steps,
            owners_visited: tests.work.owners_visited,
        };
        self.add_relations(tests.work.relation_edges_visited);
        self.add_witness_reads(tests.work.witness_reads);
        self.check("test dependency planning")
    }

    pub(crate) fn preflight_witness_edits(
        &self,
        derived: &DerivedDelta,
        summaries: &super::SummaryDelta,
        tests: &TestDependencyDelta,
    ) -> Result<(), Diagnostic> {
        let relation_edits = lengths([
            derived.relations.removed.len(),
            derived.relations.added.len(),
        ])
        .saturating_mul(2);
        let test_edits = lengths([tests.removed.len(), tests.added.len()]).saturating_mul(2);
        let edits = count(summaries.edits.len())
            .saturating_add(count(derived.namespaces.len()))
            .saturating_add(count(derived.ownership.len()))
            .saturating_add(relation_edits)
            .saturating_add(test_edits);
        check_dimension(
            "witness map update",
            "witness_edits",
            edits,
            self.budget.witness_update.maximum_edits,
        )
    }

    pub(crate) fn observe_witness(&mut self, witness: &WitnessMapUpdate) -> Result<(), Diagnostic> {
        self.work.witness_update.edits = witness
            .edits
            .inserted
            .saturating_add(witness.edits.replaced)
            .saturating_add(witness.edits.removed)
            .saturating_add(witness.edits.unchanged);
        self.work.witness_update.pages_encoded = witness.work.pages_encoded;
        self.work.witness_update.bytes_encoded = witness.work.bytes_encoded;
        self.work.witness_reads.add(witness.read_work);
        self.check("witness map update")
    }

    pub(crate) fn witness_map_admission(
        &self,
        phase: &str,
    ) -> Result<super::WitnessMapAdmission, Diagnostic> {
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

    pub(crate) fn observe_canonical_reads(
        &mut self,
        reads: CanonicalReadWork,
    ) -> Result<(), Diagnostic> {
        self.work.canonical_reads.add(reads);
        self.check("candidate canonical reads")
    }

    pub(crate) fn remaining_relation_edges(&self, phase: &str) -> Result<u64, Diagnostic> {
        self.budget
            .remaining_relation_edges(self.work.relation_edges, phase)
    }

    pub(crate) fn remaining_impact_ownership_steps(&self, phase: &str) -> Result<u64, Diagnostic> {
        remaining_dimension(
            phase,
            "impact_ownership_steps",
            self.work.impact_ownership_steps,
            self.budget.impact.maximum_ownership_steps,
        )
    }

    pub(crate) fn remaining_witness_records(&self, phase: &str) -> Result<u64, Diagnostic> {
        remaining_dimension(
            phase,
            "witness_decoded_records",
            self.work.witness_reads.witness_records_decoded,
            self.budget.witness_reads.maximum_decoded_records,
        )
    }

    pub(crate) fn remaining_test_dependency_changes(
        &self,
        derived: &DerivedDelta,
        summaries: &super::SummaryDelta,
        phase: &str,
    ) -> Result<u64, Diagnostic> {
        let fixed_edits = count(summaries.edits.len())
            .saturating_add(count(derived.namespaces.len()))
            .saturating_add(count(derived.ownership.len()))
            .saturating_add(
                lengths([
                    derived.relations.removed.len(),
                    derived.relations.added.len(),
                ])
                .saturating_mul(2),
            );
        remaining_dimension(
            phase,
            "witness_edits",
            fixed_edits,
            self.budget.witness_update.maximum_edits,
        )
        .map(|remaining| remaining / 2)
    }

    pub(crate) const fn finish(self) -> ChangeBudgetWork {
        self.work
    }

    fn add_validation(&mut self, work: IncrementalValidationWork, diagnostics: u64) {
        self.work.validation.owner_records = self
            .work
            .validation
            .owner_records
            .saturating_add(work.owner_records_checked);
        self.work.validation.ownership_entries = self
            .work
            .validation
            .ownership_entries
            .saturating_add(work.ownership_entries_checked);
        self.work.validation.type_objects = self
            .work
            .validation
            .type_objects
            .saturating_add(work.type_objects_checked);
        self.work.validation.diagnostics =
            self.work.validation.diagnostics.saturating_add(diagnostics);
        self.add_witness_reads(work.witness_reads);
    }

    fn add_witness_reads(&mut self, reads: WitnessReadWork) {
        self.work.witness_reads.add(reads);
    }

    fn add_relations(&mut self, edges: u64) {
        self.work.relation_edges = self.work.relation_edges.saturating_add(edges);
    }

    fn check(&self, phase: &str) -> Result<(), Diagnostic> {
        let work = self.work;
        let budget = self.budget;
        check_dimension(
            phase,
            "operations",
            work.authored_operations,
            budget.authored.maximum_operations,
        )?;
        check_dimension(
            phase,
            "preconditions",
            work.preconditions_checked,
            budget.authored.maximum_preconditions,
        )?;
        check_dimension(
            phase,
            "allocated_identities",
            work.allocated_identities,
            budget.authored.maximum_allocated_identities,
        )?;
        check_dimension(
            phase,
            "authored_type_nodes",
            work.authored_type_nodes,
            budget.authored.maximum_type_nodes,
        )?;
        check_dimension(
            phase,
            "canonical_owner_edits",
            work.canonical_edits.owner_edits,
            budget.canonical_edits.maximum_owner_edits,
        )?;
        check_dimension(
            phase,
            "canonical_type_edits",
            work.canonical_edits.type_edits,
            budget.canonical_edits.maximum_type_edits,
        )?;
        check_dimension(
            phase,
            "canonical_dependency_edits",
            work.canonical_edits.dependency_edits,
            budget.canonical_edits.maximum_dependency_edits,
        )?;
        check_dimension(
            phase,
            "canonical_retirement_edits",
            work.canonical_edits.retirement_edits,
            budget.canonical_edits.maximum_retirement_edits,
        )?;
        check_canonical_reads(phase, work.canonical_reads, budget.canonical_reads)?;
        check_dimension(
            phase,
            "canonical_map_pages_encoded",
            work.canonical_map_update.pages_encoded,
            budget.canonical_map_update.maximum_pages_encoded,
        )?;
        check_dimension(
            phase,
            "canonical_map_bytes_encoded",
            work.canonical_map_update.bytes_encoded,
            budget.canonical_map_update.maximum_bytes_encoded,
        )?;
        check_witness_reads(phase, work.witness_reads, budget.witness_reads)?;
        check_dimension(
            phase,
            "affected_frontier_owners",
            work.affected_frontier_owners,
            budget.impact.maximum_affected_owners,
        )?;
        check_dimension(
            phase,
            "impact_summary_owners",
            work.impact_summary_owners,
            budget.impact.maximum_summary_owners,
        )?;
        check_dimension(
            phase,
            "impact_summary_edits",
            work.impact_summary_edits,
            budget.impact.maximum_summary_edits,
        )?;
        check_dimension(
            phase,
            "impact_ownership_steps",
            work.impact_ownership_steps,
            budget.impact.maximum_ownership_steps,
        )?;
        check_dimension(
            phase,
            "impact_behavior_owners",
            work.impact_behavior_owners,
            budget.impact.maximum_behavior_owners,
        )?;
        check_dimension(
            phase,
            "relation_edges",
            work.relation_edges,
            budget.impact.maximum_relation_edges,
        )?;
        check_dimension(
            phase,
            "validation_owner_records",
            work.validation.owner_records,
            budget.validation.maximum_owner_records,
        )?;
        check_dimension(
            phase,
            "validation_ownership_entries",
            work.validation.ownership_entries,
            budget.validation.maximum_ownership_entries,
        )?;
        check_dimension(
            phase,
            "validation_type_objects",
            work.validation.type_objects,
            budget.validation.maximum_type_objects,
        )?;
        check_dimension(
            phase,
            "validation_expression_steps",
            work.validation.expression_steps,
            budget.validation.maximum_expression_steps,
        )?;
        check_dimension(
            phase,
            "validation_diagnostics",
            work.validation.diagnostics,
            budget.validation.maximum_diagnostics,
        )?;
        check_dimension(
            phase,
            "tests_selected",
            work.tests.selected,
            budget.tests.maximum_selected,
        )?;
        check_dimension(
            phase,
            "test_ownership_steps",
            work.tests.ownership_steps,
            budget.tests.maximum_ownership_steps,
        )?;
        check_dimension(
            phase,
            "test_owners_visited",
            work.tests.owners_visited,
            budget.tests.maximum_owners_visited,
        )?;
        check_dimension(
            phase,
            "witness_edits",
            work.witness_update.edits,
            budget.witness_update.maximum_edits,
        )?;
        check_dimension(
            phase,
            "witness_map_pages_encoded",
            work.witness_update.pages_encoded,
            budget.witness_update.maximum_pages_encoded,
        )?;
        check_dimension(
            phase,
            "witness_map_bytes_encoded",
            work.witness_update.bytes_encoded,
            budget.witness_update.maximum_bytes_encoded,
        )?;
        check_dimension(
            phase,
            "staged_objects",
            work.staging.objects,
            budget.staging.maximum_objects,
        )?;
        check_dimension(
            phase,
            "staged_bytes",
            work.staging.bytes,
            budget.staging.maximum_bytes,
        )?;
        check_dimension(
            phase,
            "staged_pages",
            work.staging.pages,
            budget.staging.maximum_pages,
        )
    }
}

fn check_canonical_reads(
    phase: &str,
    work: CanonicalReadWork,
    budget: CanonicalReadAdmission,
) -> Result<(), Diagnostic> {
    for (dimension, observed, maximum) in [
        (
            "canonical_point_reads",
            work.point_reads,
            budget.maximum_point_reads,
        ),
        (
            "canonical_map_pages",
            work.map_pages_read,
            budget.maximum_map_pages,
        ),
        (
            "canonical_map_entries",
            work.map_entries_visited,
            budget.maximum_map_entries,
        ),
        (
            "canonical_catalog_lookups",
            work.catalog_lookups,
            budget.maximum_catalog_lookups,
        ),
        (
            "canonical_objects",
            work.objects_read,
            budget.maximum_objects,
        ),
        ("canonical_bytes", work.bytes_read, budget.maximum_bytes),
        (
            "canonical_decoded_records",
            work.canonical_records_decoded,
            budget.maximum_decoded_records,
        ),
    ] {
        check_dimension(phase, dimension, observed, maximum)?;
    }
    Ok(())
}

fn check_witness_reads(
    phase: &str,
    work: WitnessReadWork,
    budget: WitnessReadAdmission,
) -> Result<(), Diagnostic> {
    for (dimension, observed, maximum) in [
        (
            "witness_point_reads",
            work.point_reads,
            budget.maximum_point_reads,
        ),
        (
            "witness_map_pages",
            work.map_pages_read,
            budget.maximum_map_pages,
        ),
        (
            "witness_map_entries",
            work.map_entries_visited,
            budget.maximum_map_entries,
        ),
        (
            "witness_catalog_lookups",
            work.catalog_lookups,
            budget.maximum_catalog_lookups,
        ),
        ("witness_objects", work.objects_read, budget.maximum_objects),
        ("witness_bytes", work.bytes_read, budget.maximum_bytes),
        (
            "witness_decoded_records",
            work.witness_records_decoded,
            budget.maximum_decoded_records,
        ),
    ] {
        check_dimension(phase, dimension, observed, maximum)?;
    }
    Ok(())
}

fn check_dimension(
    phase: &str,
    dimension: &'static str,
    observed: u64,
    maximum: u64,
) -> Result<(), Diagnostic> {
    if observed > maximum {
        return Err(budget_error(
            budget_code(dimension),
            format!(
                "{phase} requires {observed} {dimension}, exceeding the declared maximum {maximum}"
            ),
        ));
    }
    Ok(())
}

fn validate_maximum(
    dimension: &'static str,
    declared: u64,
    implementation_ceiling: u64,
) -> Result<(), Diagnostic> {
    if declared > implementation_ceiling {
        return Err(budget_error(
            budget_code(dimension),
            format!(
                "declared maximum {declared} {dimension} exceeds the change engine ceiling {implementation_ceiling}"
            ),
        ));
    }
    Ok(())
}

fn remaining_dimension(
    phase: &str,
    dimension: &'static str,
    observed: u64,
    maximum: u64,
) -> Result<u64, Diagnostic> {
    maximum.checked_sub(observed).ok_or_else(|| {
        budget_error(
            budget_code(dimension),
            format!(
                "{phase} already observed {observed} {dimension}, exceeding the declared maximum {maximum}"
            ),
        )
    })
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

fn lengths<const N: usize>(values: [usize; N]) -> u64 {
    values
        .into_iter()
        .fold(0_u64, |total, value| total.saturating_add(count(value)))
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
    type InvalidMaximum = fn(&mut ChangeBudget);

    #[test]
    fn every_admission_dimension_has_an_independent_diagnostic() {
        let cases: &[(Exhaustion, &str)] = &[
            (
                |budget, work| {
                    budget.authored.maximum_operations = 1;
                    work.authored_operations = 2;
                },
                "change_budget_operations",
            ),
            (
                |budget, work| {
                    budget.authored.maximum_preconditions = 0;
                    work.preconditions_checked = 1;
                },
                "change_budget_preconditions",
            ),
            (
                |budget, work| {
                    budget.authored.maximum_allocated_identities = 0;
                    work.allocated_identities = 1;
                },
                "change_budget_allocated_identities",
            ),
            (
                |budget, work| {
                    budget.authored.maximum_type_nodes = 0;
                    work.authored_type_nodes = 1;
                },
                "change_budget_authored_type_nodes",
            ),
            (
                |budget, work| {
                    budget.canonical_edits.maximum_owner_edits = 0;
                    work.canonical_edits.owner_edits = 1;
                },
                "change_budget_canonical_owner_edits",
            ),
            (
                |budget, work| {
                    budget.canonical_edits.maximum_type_edits = 0;
                    work.canonical_edits.type_edits = 1;
                },
                "change_budget_canonical_type_edits",
            ),
            (
                |budget, work| {
                    budget.canonical_edits.maximum_dependency_edits = 0;
                    work.canonical_edits.dependency_edits = 1;
                },
                "change_budget_canonical_dependency_edits",
            ),
            (
                |budget, work| {
                    budget.canonical_edits.maximum_retirement_edits = 0;
                    work.canonical_edits.retirement_edits = 1;
                },
                "change_budget_canonical_retirement_edits",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_point_reads = 0;
                    work.canonical_reads.point_reads = 1;
                },
                "change_budget_canonical_point_reads",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_map_pages = 0;
                    work.canonical_reads.map_pages_read = 1;
                },
                "change_budget_canonical_map_pages",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_map_entries = 0;
                    work.canonical_reads.map_entries_visited = 1;
                },
                "change_budget_canonical_map_entries",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_catalog_lookups = 0;
                    work.canonical_reads.catalog_lookups = 1;
                },
                "change_budget_canonical_catalog_lookups",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_objects = 0;
                    work.canonical_reads.objects_read = 1;
                },
                "change_budget_canonical_objects",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_bytes = 0;
                    work.canonical_reads.bytes_read = 1;
                },
                "change_budget_canonical_bytes",
            ),
            (
                |budget, work| {
                    budget.canonical_reads.maximum_decoded_records = 0;
                    work.canonical_reads.canonical_records_decoded = 1;
                },
                "change_budget_canonical_decoded_records",
            ),
            (
                |budget, work| {
                    budget.canonical_map_update.maximum_pages_encoded = 0;
                    work.canonical_map_update.pages_encoded = 1;
                },
                "change_budget_canonical_map_pages_encoded",
            ),
            (
                |budget, work| {
                    budget.canonical_map_update.maximum_bytes_encoded = 0;
                    work.canonical_map_update.bytes_encoded = 1;
                },
                "change_budget_canonical_map_bytes_encoded",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_point_reads = 0;
                    work.witness_reads.point_reads = 1;
                },
                "change_budget_witness_point_reads",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_map_pages = 0;
                    work.witness_reads.map_pages_read = 1;
                },
                "change_budget_witness_map_pages",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_map_entries = 0;
                    work.witness_reads.map_entries_visited = 1;
                },
                "change_budget_witness_map_entries",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_catalog_lookups = 0;
                    work.witness_reads.catalog_lookups = 1;
                },
                "change_budget_witness_catalog_lookups",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_objects = 0;
                    work.witness_reads.objects_read = 1;
                },
                "change_budget_witness_objects",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_bytes = 0;
                    work.witness_reads.bytes_read = 1;
                },
                "change_budget_witness_bytes",
            ),
            (
                |budget, work| {
                    budget.witness_reads.maximum_decoded_records = 0;
                    work.witness_reads.witness_records_decoded = 1;
                },
                "change_budget_witness_decoded_records",
            ),
            (
                |budget, work| {
                    budget.impact.maximum_affected_owners = 0;
                    work.affected_frontier_owners = 1;
                },
                "change_budget_affected_frontier_owners",
            ),
            (
                |budget, work| {
                    budget.impact.maximum_summary_owners = 0;
                    work.impact_summary_owners = 1;
                },
                "change_budget_impact_summary_owners",
            ),
            (
                |budget, work| {
                    budget.impact.maximum_summary_edits = 0;
                    work.impact_summary_edits = 1;
                },
                "change_budget_impact_summary_edits",
            ),
            (
                |budget, work| {
                    budget.impact.maximum_ownership_steps = 0;
                    work.impact_ownership_steps = 1;
                },
                "change_budget_impact_ownership_steps",
            ),
            (
                |budget, work| {
                    budget.impact.maximum_behavior_owners = 0;
                    work.impact_behavior_owners = 1;
                },
                "change_budget_impact_behavior_owners",
            ),
            (
                |budget, work| {
                    budget.impact.maximum_relation_edges = 0;
                    work.relation_edges = 1;
                },
                "change_budget_relation_edges",
            ),
            (
                |budget, work| {
                    budget.validation.maximum_owner_records = 0;
                    work.validation.owner_records = 1;
                },
                "change_budget_validation_owner_records",
            ),
            (
                |budget, work| {
                    budget.validation.maximum_ownership_entries = 0;
                    work.validation.ownership_entries = 1;
                },
                "change_budget_validation_ownership_entries",
            ),
            (
                |budget, work| {
                    budget.validation.maximum_type_objects = 0;
                    work.validation.type_objects = 1;
                },
                "change_budget_validation_type_objects",
            ),
            (
                |budget, work| {
                    budget.validation.maximum_expression_steps = 0;
                    work.validation.expression_steps = 1;
                },
                "change_budget_validation_expression_steps",
            ),
            (
                |budget, work| {
                    budget.validation.maximum_diagnostics = 0;
                    work.validation.diagnostics = 1;
                },
                "change_budget_validation_diagnostics",
            ),
            (
                |budget, work| {
                    budget.tests.maximum_selected = 0;
                    work.tests.selected = 1;
                },
                "change_budget_tests_selected",
            ),
            (
                |budget, work| {
                    budget.tests.maximum_ownership_steps = 0;
                    work.tests.ownership_steps = 1;
                },
                "change_budget_test_ownership_steps",
            ),
            (
                |budget, work| {
                    budget.tests.maximum_owners_visited = 0;
                    work.tests.owners_visited = 1;
                },
                "change_budget_test_owners_visited",
            ),
            (
                |budget, work| {
                    budget.witness_update.maximum_edits = 0;
                    work.witness_update.edits = 1;
                },
                "change_budget_witness_edits",
            ),
            (
                |budget, work| {
                    budget.witness_update.maximum_pages_encoded = 0;
                    work.witness_update.pages_encoded = 1;
                },
                "change_budget_witness_map_pages_encoded",
            ),
            (
                |budget, work| {
                    budget.witness_update.maximum_bytes_encoded = 0;
                    work.witness_update.bytes_encoded = 1;
                },
                "change_budget_witness_map_bytes_encoded",
            ),
            (
                |budget, work| {
                    budget.staging.maximum_objects = 0;
                    work.staging.objects = 1;
                },
                "change_budget_staged_objects",
            ),
            (
                |budget, work| {
                    budget.staging.maximum_bytes = 0;
                    work.staging.bytes = 1;
                },
                "change_budget_staged_bytes",
            ),
            (
                |budget, work| {
                    budget.staging.maximum_pages = 0;
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
    fn every_declared_maximum_rejects_values_above_its_engine_ceiling() {
        let cases: &[(InvalidMaximum, &str)] = &[
            (
                |budget| budget.authored.maximum_operations = u64::MAX,
                "change_budget_invalid_operations",
            ),
            (
                |budget| budget.authored.maximum_preconditions = u64::MAX,
                "change_budget_preconditions",
            ),
            (
                |budget| budget.authored.maximum_allocated_identities = u64::MAX,
                "change_budget_allocated_identities",
            ),
            (
                |budget| budget.authored.maximum_type_nodes = u64::MAX,
                "change_budget_authored_type_nodes",
            ),
            (
                |budget| budget.canonical_edits.maximum_owner_edits = u64::MAX,
                "change_budget_canonical_owner_edits",
            ),
            (
                |budget| budget.canonical_edits.maximum_type_edits = u64::MAX,
                "change_budget_canonical_type_edits",
            ),
            (
                |budget| budget.canonical_edits.maximum_dependency_edits = u64::MAX,
                "change_budget_canonical_dependency_edits",
            ),
            (
                |budget| budget.canonical_edits.maximum_retirement_edits = u64::MAX,
                "change_budget_canonical_retirement_edits",
            ),
            (
                |budget| budget.canonical_reads.maximum_point_reads = u64::MAX,
                "change_budget_canonical_point_reads",
            ),
            (
                |budget| budget.canonical_reads.maximum_map_pages = u64::MAX,
                "change_budget_canonical_map_pages",
            ),
            (
                |budget| budget.canonical_reads.maximum_map_entries = u64::MAX,
                "change_budget_canonical_map_entries",
            ),
            (
                |budget| budget.canonical_reads.maximum_catalog_lookups = u64::MAX,
                "change_budget_canonical_catalog_lookups",
            ),
            (
                |budget| budget.canonical_reads.maximum_objects = u64::MAX,
                "change_budget_canonical_objects",
            ),
            (
                |budget| budget.canonical_reads.maximum_bytes = u64::MAX,
                "change_budget_canonical_bytes",
            ),
            (
                |budget| budget.canonical_reads.maximum_decoded_records = u64::MAX,
                "change_budget_canonical_decoded_records",
            ),
            (
                |budget| budget.canonical_map_update.maximum_pages_encoded = u64::MAX,
                "change_budget_canonical_map_pages_encoded",
            ),
            (
                |budget| budget.canonical_map_update.maximum_bytes_encoded = u64::MAX,
                "change_budget_canonical_map_bytes_encoded",
            ),
            (
                |budget| budget.witness_reads.maximum_point_reads = u64::MAX,
                "change_budget_witness_point_reads",
            ),
            (
                |budget| budget.witness_reads.maximum_map_pages = u64::MAX,
                "change_budget_witness_map_pages",
            ),
            (
                |budget| budget.witness_reads.maximum_map_entries = u64::MAX,
                "change_budget_witness_map_entries",
            ),
            (
                |budget| budget.witness_reads.maximum_catalog_lookups = u64::MAX,
                "change_budget_witness_catalog_lookups",
            ),
            (
                |budget| budget.witness_reads.maximum_objects = u64::MAX,
                "change_budget_witness_objects",
            ),
            (
                |budget| budget.witness_reads.maximum_bytes = u64::MAX,
                "change_budget_witness_bytes",
            ),
            (
                |budget| budget.witness_reads.maximum_decoded_records = u64::MAX,
                "change_budget_witness_decoded_records",
            ),
            (
                |budget| budget.impact.maximum_affected_owners = u64::MAX,
                "change_budget_affected_frontier_owners",
            ),
            (
                |budget| budget.impact.maximum_summary_owners = u64::MAX,
                "change_budget_impact_summary_owners",
            ),
            (
                |budget| budget.impact.maximum_summary_edits = u64::MAX,
                "change_budget_impact_summary_edits",
            ),
            (
                |budget| budget.impact.maximum_ownership_steps = u64::MAX,
                "change_budget_impact_ownership_steps",
            ),
            (
                |budget| budget.impact.maximum_behavior_owners = u64::MAX,
                "change_budget_impact_behavior_owners",
            ),
            (
                |budget| budget.impact.maximum_relation_edges = u64::MAX,
                "change_budget_relation_edges",
            ),
            (
                |budget| budget.impact.maximum_relation_fanout = u64::MAX,
                "change_budget_relation_fanout",
            ),
            (
                |budget| budget.validation.maximum_owner_records = u64::MAX,
                "change_budget_validation_owner_records",
            ),
            (
                |budget| budget.validation.maximum_ownership_entries = u64::MAX,
                "change_budget_validation_ownership_entries",
            ),
            (
                |budget| budget.validation.maximum_type_objects = u64::MAX,
                "change_budget_validation_type_objects",
            ),
            (
                |budget| budget.validation.maximum_expression_steps = u64::MAX,
                "change_budget_validation_expression_steps",
            ),
            (
                |budget| budget.validation.maximum_diagnostics = u64::MAX,
                "change_budget_validation_diagnostics",
            ),
            (
                |budget| budget.tests.maximum_selected = u64::MAX,
                "change_budget_tests_selected",
            ),
            (
                |budget| budget.tests.maximum_dependencies_per_test = u64::MAX,
                "change_budget_test_dependencies_per_test",
            ),
            (
                |budget| budget.tests.maximum_ownership_steps = u64::MAX,
                "change_budget_test_ownership_steps",
            ),
            (
                |budget| budget.tests.maximum_owners_visited = u64::MAX,
                "change_budget_test_owners_visited",
            ),
            (
                |budget| budget.witness_update.maximum_edits = u64::MAX,
                "change_budget_witness_edits",
            ),
            (
                |budget| budget.witness_update.maximum_pages_encoded = u64::MAX,
                "change_budget_witness_map_pages_encoded",
            ),
            (
                |budget| budget.witness_update.maximum_bytes_encoded = u64::MAX,
                "change_budget_witness_map_bytes_encoded",
            ),
            (
                |budget| budget.staging.maximum_objects = u64::MAX,
                "change_budget_staged_objects",
            ),
            (
                |budget| budget.staging.maximum_bytes = u64::MAX,
                "change_budget_staged_bytes",
            ),
            (
                |budget| budget.staging.maximum_pages = u64::MAX,
                "change_budget_staged_pages",
            ),
        ];

        for (invalidate, expected) in cases {
            let mut budget = ChangeBudget::default();
            invalidate(&mut budget);
            let error = budget
                .validate()
                .expect_err("maximum above its owning engine ceiling must reject");
            assert_eq!(error.code, *expected);
        }
    }
}
