//! One generic Graph 5 preparation pipeline before repository publication is bound.

use super::budget::ChangeBudgetMeter;
use super::derived::derive_local_delta_with_admission;
use super::impact::plan_impact_and_summaries_with_admission;
use super::test_delta::{TestDependencyAdmission, derive_test_dependency_delta_with_admission};
use super::validate::{
    validate_incremental_frontier_with_admission, validate_structural_frontier_with_admission,
};
use super::{
    BudgetedCanonicalBase, BudgetedWitnessBase, CanonicalBaseRead, CanonicalDelta,
    CanonicalReadWork, ChangeBudget, ChangeBudgetWork, DerivedDelta, IncrementalValidationReport,
    KernelOverlay, PlannedSummaries, TestDependencyDelta, WitnessMapBase, WitnessMapUpdate,
    WitnessReadWork,
};
use crate::platform::diagnostic::Diagnostic;

#[derive(Clone, Debug)]
pub struct PreparedChangeAnalysis {
    pub canonical: CanonicalDelta,
    pub derived: DerivedDelta,
    pub summaries: PlannedSummaries,
    pub tests: TestDependencyDelta,
    pub witness: WitnessMapUpdate,
    pub validation: IncrementalValidationReport,
    pub canonical_read_work: CanonicalReadWork,
    pub witness_read_work: WitnessReadWork,
    pub budget_work: ChangeBudgetWork,
    pub budget: ChangeBudget,
}

pub fn prepare_change_analysis<B: CanonicalBaseRead + ?Sized, W: WitnessMapBase + ?Sized>(
    base: &B,
    base_witness: &W,
    canonical: CanonicalDelta,
) -> Result<PreparedChangeAnalysis, Vec<Diagnostic>> {
    prepare_change_analysis_with_budget(
        base,
        base_witness,
        canonical,
        ChangeBudget::default(),
        ChangeBudgetWork::default(),
    )
}

pub fn prepare_change_analysis_with_budget<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessMapBase + ?Sized,
>(
    base: &B,
    base_witness: &W,
    canonical: CanonicalDelta,
    budget: ChangeBudget,
    initial_work: ChangeBudgetWork,
) -> Result<PreparedChangeAnalysis, Vec<Diagnostic>> {
    let declared_budget = budget;
    let mut budget = ChangeBudgetMeter::new(budget, initial_work).map_err(|error| vec![error])?;
    let canonical_base = BudgetedCanonicalBase::new(
        base,
        declared_budget.canonical_reads,
        initial_work.canonical_reads,
    )
    .map_err(|error| vec![error])?;
    let witness_base = BudgetedWitnessBase::new(
        base_witness,
        declared_budget.witness_reads,
        initial_work.witness_reads,
    )
    .map_err(|error| vec![error])?;
    budget
        .observe_canonical(&canonical)
        .map_err(|error| vec![error])?;
    let overlay = KernelOverlay::new(&canonical_base, &canonical);
    let derived = derive_local_delta_with_admission(
        &overlay,
        &canonical,
        &witness_base,
        declared_budget.impact.maximum_ownership_steps,
        declared_budget.impact.maximum_relation_edges,
    )
    .map_err(|error| vec![error])?;
    budget
        .observe_derived(&derived)
        .map_err(|error| vec![error])?;
    let structural = validate_structural_frontier_with_admission(
        &overlay,
        &canonical,
        &derived,
        &witness_base,
        declared_budget.validation,
    )?;
    budget
        .observe_structural(&structural)
        .map_err(|error| vec![error])?;
    let mut impact_admission = declared_budget.impact;
    impact_admission.maximum_ownership_steps = budget
        .remaining_impact_ownership_steps("impact planning")
        .map_err(|error| vec![error])?;
    impact_admission.maximum_relation_edges = budget
        .remaining_relation_edges("impact planning")
        .map_err(|error| vec![error])?;
    let summaries = plan_impact_and_summaries_with_admission(
        &overlay,
        &canonical,
        &derived,
        &witness_base,
        impact_admission,
    )
    .map_err(|error| vec![error])?;
    budget
        .observe_summaries(&summaries)
        .map_err(|error| vec![error])?;
    budget
        .observe_impact(&summaries.plan)
        .map_err(|error| vec![error])?;
    let validation = validate_incremental_frontier_with_admission(
        &overlay,
        &canonical,
        &summaries.plan,
        &summaries.final_delta,
        &witness_base,
        structural,
        declared_budget.validation,
    )?;
    budget
        .observe_expression_validation(validation.work.expression_work, 0)
        .map_err(|error| vec![error])?;
    let tests = derive_test_dependency_delta_with_admission(
        &overlay,
        &canonical,
        &derived,
        &witness_base,
        TestDependencyAdmission::new(
            declared_budget.tests,
            budget
                .remaining_relation_edges("test dependency planning")
                .map_err(|error| vec![error])?,
            declared_budget.impact.maximum_relation_fanout,
            budget
                .remaining_witness_records("test dependency planning")
                .map_err(|error| vec![error])?,
            budget
                .remaining_test_dependency_changes(
                    &derived,
                    &summaries.final_delta,
                    "test dependency planning",
                )
                .map_err(|error| vec![error])?,
        ),
    )
    .map_err(|error| vec![error])?;
    budget.observe_tests(&tests).map_err(|error| vec![error])?;
    budget
        .preflight_witness_edits(&derived, &summaries.final_delta, &tests)
        .map_err(|error| vec![error])?;
    let witness_admission = budget
        .witness_map_admission("witness map update")
        .map_err(|error| vec![error])?;
    let witness = base_witness
        .update_witness_maps(&derived, &summaries.final_delta, &tests, witness_admission)
        .map_err(|error| vec![error])?;
    budget
        .observe_witness(&witness)
        .map_err(|error| vec![error])?;
    let canonical_read_work = overlay.work();
    budget
        .observe_canonical_reads(canonical_read_work)
        .map_err(|error| vec![error])?;
    Ok(PreparedChangeAnalysis {
        canonical,
        derived,
        summaries,
        tests,
        witness,
        validation,
        canonical_read_work,
        witness_read_work: WitnessReadWork::default(),
        budget_work: budget.finish(),
        budget: declared_budget,
    })
}
