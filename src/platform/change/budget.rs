//! Explicit resource policy and observed semantic work for one Graph 5 change preparation.

use super::{
    CanonicalDelta, CanonicalReadWork, DerivedDelta, ImpactPlan, IncrementalValidationWork,
    PlannedSummaries, StructuralValidationReport, TestDependencyDelta, WitnessMapUpdate,
    WitnessReadWork,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAXIMUM_CHANGE_OPERATIONS: u64 = 10_000;
pub const MAXIMUM_CHANGE_AFFECTED_OWNERS: u64 = 100_000;
pub const MAXIMUM_CHANGE_RELATION_EDGES: u64 = 10_000_000;
pub const MAXIMUM_CHANGE_WORK: u64 = 10_000_000;

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeBudget {
    pub maximum_operations: u64,
    pub maximum_affected_owners: u64,
    pub maximum_relation_edges: u64,
    pub maximum_work: u64,
}

impl Default for ChangeBudget {
    fn default() -> Self {
        Self {
            maximum_operations: 1_000,
            maximum_affected_owners: 10_000,
            maximum_relation_edges: 100_000,
            maximum_work: 1_000_000,
        }
    }
}

impl ChangeBudget {
    pub fn validate(self) -> Result<Self, Diagnostic> {
        if self.maximum_operations == 0
            || self.maximum_operations > MAXIMUM_CHANGE_OPERATIONS
            || self.maximum_affected_owners == 0
            || self.maximum_affected_owners > MAXIMUM_CHANGE_AFFECTED_OWNERS
            || self.maximum_relation_edges == 0
            || self.maximum_relation_edges > MAXIMUM_CHANGE_RELATION_EDGES
            || self.maximum_work == 0
            || self.maximum_work > MAXIMUM_CHANGE_WORK
        {
            return Err(budget_error(
                "change_budget_invalid",
                format!(
                    "change budget must use positive values no greater than operations={MAXIMUM_CHANGE_OPERATIONS}, affected_owners={MAXIMUM_CHANGE_AFFECTED_OWNERS}, relation_edges={MAXIMUM_CHANGE_RELATION_EDGES}, and work={MAXIMUM_CHANGE_WORK}"
                ),
            ));
        }
        Ok(self)
    }

    pub(crate) fn validate_request_counts(
        self,
        changes: usize,
        preconditions: usize,
    ) -> Result<Self, Diagnostic> {
        let budget = self.validate()?;
        let changes = u64::try_from(changes).unwrap_or(u64::MAX);
        let preconditions = u64::try_from(preconditions).unwrap_or(u64::MAX);
        if changes == 0 || changes > budget.maximum_operations {
            return Err(budget_error(
                "change_budget_operations",
                format!(
                    "authored change contains {changes} operations but its declared maximum is {}",
                    budget.maximum_operations
                ),
            ));
        }
        if preconditions > budget.maximum_operations {
            return Err(budget_error(
                "change_budget_preconditions",
                format!(
                    "authored change contains {preconditions} preconditions but its declared operation maximum is {}",
                    budget.maximum_operations
                ),
            ));
        }
        Ok(budget)
    }

    pub(crate) fn check_observed(
        self,
        work: ChangeBudgetWork,
        phase: &str,
    ) -> Result<(), Diagnostic> {
        let meter = ChangeBudgetMeter {
            budget: self.validate()?,
            work,
        };
        meter.check(phase)
    }
}

/// One stable, subsystem-independent semantic-work accounting record.
///
/// It deliberately counts semantic items and exact reads rather than wall time. Byte and page I/O
/// remain separately retained in the publication receipt.
#[derive(Clone, Copy, Debug, Decode, Default, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeBudgetWork {
    pub authored_operations: u64,
    pub preconditions_checked: u64,
    pub canonical_point_reads: u64,
    pub witness_point_reads: u64,
    pub canonical_items: u64,
    pub derived_items: u64,
    pub affected_owners: u64,
    pub relation_edges: u64,
    pub validation_items: u64,
    pub test_items: u64,
    pub witness_edits: u64,
    pub consumed: u64,
}

impl ChangeBudgetWork {
    pub(crate) fn authored(
        operations: usize,
        preconditions: usize,
        canonical: CanonicalReadWork,
        witness: WitnessReadWork,
        relation_edges: u64,
    ) -> Self {
        let authored_operations = u64::try_from(operations).unwrap_or(u64::MAX);
        let preconditions_checked = u64::try_from(preconditions).unwrap_or(u64::MAX);
        let canonical_point_reads = canonical.point_reads;
        let witness_point_reads = witness.point_reads;
        let consumed = authored_operations
            .saturating_add(preconditions_checked)
            .saturating_add(canonical_point_reads)
            .saturating_add(canonical.map_entries_visited)
            .saturating_add(witness_point_reads)
            .saturating_add(witness.map_entries_visited)
            .saturating_add(relation_edges);
        Self {
            authored_operations,
            preconditions_checked,
            canonical_point_reads,
            witness_point_reads,
            relation_edges,
            consumed,
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
        let items = lengths([
            canonical.owners.len(),
            canonical.type_additions.len(),
            canonical.dependencies.len(),
            canonical.retirements.len(),
        ]);
        self.work.canonical_items = items;
        self.work.affected_owners = self
            .work
            .affected_owners
            .max(u64::try_from(canonical.owners.len()).unwrap_or(u64::MAX));
        self.charge(items);
        self.check("canonical delta")
    }

    pub(crate) fn observe_derived(&mut self, derived: &DerivedDelta) -> Result<(), Diagnostic> {
        let relations = lengths([
            derived.relations.removed.len(),
            derived.relations.added.len(),
        ]);
        let non_relation_items = lengths([
            derived.namespaces.len(),
            derived.ownership.len(),
            derived.summary_candidates.len(),
        ]);
        self.work.derived_items = non_relation_items.saturating_add(relations);
        self.add_witness_reads(derived.read_work);
        self.add_relations(relations);
        self.charge(non_relation_items);
        self.check("derived witness delta")
    }

    pub(crate) fn observe_structural(
        &mut self,
        structural: &StructuralValidationReport,
    ) -> Result<(), Diagnostic> {
        self.add_validation(structural.work);
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
        self.work.affected_owners = self
            .work
            .affected_owners
            .max(u64::try_from(affected).unwrap_or(u64::MAX));
        self.add_relations(impact.work.reverse_edges_visited);
        self.add_witness_reads(impact.work.witness_reads);
        self.charge(
            impact
                .work
                .summary_edits_examined
                .saturating_add(impact.work.ownership_steps)
                .saturating_add(impact.work.behavior_owners_visited),
        );
        self.check("impact planning")
    }

    pub(crate) fn observe_summaries(
        &mut self,
        summaries: &PlannedSummaries,
    ) -> Result<(), Diagnostic> {
        let items = lengths([
            summaries.initial.selected.len(),
            summaries.final_delta.selected.len(),
        ]);
        self.add_witness_reads(summaries.initial.read_work);
        self.add_witness_reads(summaries.final_delta.read_work);
        self.charge(items);
        self.check("owner-summary rebuilding")
    }

    pub(crate) fn observe_expression_validation(
        &mut self,
        expression_work: u64,
    ) -> Result<(), Diagnostic> {
        self.work.validation_items = self.work.validation_items.saturating_add(expression_work);
        self.charge(expression_work);
        self.check("semantic validation")
    }

    pub(crate) fn observe_tests(&mut self, tests: &TestDependencyDelta) -> Result<(), Diagnostic> {
        let items = u64::try_from(tests.affected_tests.len())
            .unwrap_or(u64::MAX)
            .saturating_add(tests.work.ownership_steps)
            .saturating_add(tests.work.owners_visited);
        self.work.test_items = items;
        self.add_relations(tests.work.relation_edges_visited);
        self.add_witness_reads(tests.work.witness_reads);
        self.charge(items);
        self.check("test dependency planning")
    }

    pub(crate) fn observe_witness(&mut self, witness: &WitnessMapUpdate) -> Result<(), Diagnostic> {
        let edits = witness
            .edits
            .inserted
            .saturating_add(witness.edits.replaced)
            .saturating_add(witness.edits.removed)
            .saturating_add(witness.edits.unchanged);
        self.work.witness_edits = edits;
        self.charge(
            edits
                .saturating_add(witness.work.pages_read)
                .saturating_add(witness.work.pages_written),
        );
        self.check("witness map update")
    }

    pub(crate) fn observe_canonical_reads(
        &mut self,
        reads: CanonicalReadWork,
    ) -> Result<(), Diagnostic> {
        self.work.canonical_point_reads = self
            .work
            .canonical_point_reads
            .saturating_add(reads.point_reads);
        self.charge(reads.point_reads.saturating_add(reads.map_entries_visited));
        self.check("candidate canonical reads")
    }

    pub(crate) const fn finish(self) -> ChangeBudgetWork {
        self.work
    }

    fn add_validation(&mut self, work: IncrementalValidationWork) {
        let items = work
            .owner_records_checked
            .saturating_add(work.ownership_entries_checked)
            .saturating_add(work.type_objects_checked);
        self.work.validation_items = self.work.validation_items.saturating_add(items);
        self.add_witness_reads(work.witness_reads);
        self.charge(items);
    }

    fn add_witness_reads(&mut self, reads: WitnessReadWork) {
        self.work.witness_point_reads = self
            .work
            .witness_point_reads
            .saturating_add(reads.point_reads);
        self.charge(reads.point_reads.saturating_add(reads.map_entries_visited));
    }

    fn add_relations(&mut self, edges: u64) {
        self.work.relation_edges = self.work.relation_edges.saturating_add(edges);
        self.charge(edges);
    }

    fn charge(&mut self, units: u64) {
        self.work.consumed = self.work.consumed.saturating_add(units);
    }

    fn check(&self, phase: &str) -> Result<(), Diagnostic> {
        if self.work.affected_owners > self.budget.maximum_affected_owners {
            return Err(budget_error(
                "change_budget_affected_owners",
                format!(
                    "{phase} requires {} affected owners, exceeding the declared maximum {}",
                    self.work.affected_owners, self.budget.maximum_affected_owners
                ),
            ));
        }
        if self.work.relation_edges > self.budget.maximum_relation_edges {
            return Err(budget_error(
                "change_budget_relation_edges",
                format!(
                    "{phase} requires {} relation edges, exceeding the declared maximum {}",
                    self.work.relation_edges, self.budget.maximum_relation_edges
                ),
            ));
        }
        if self.work.consumed > self.budget.maximum_work {
            return Err(budget_error(
                "change_budget_work",
                format!(
                    "{phase} consumed {} semantic work units, exceeding the declared maximum {}",
                    self.work.consumed, self.budget.maximum_work
                ),
            ));
        }
        Ok(())
    }
}

fn lengths<const N: usize>(values: [usize; N]) -> u64 {
    values.into_iter().fold(0_u64, |total, value| {
        total.saturating_add(u64::try_from(value).unwrap_or(u64::MAX))
    })
}

fn budget_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Resource, code, message)
}
