//! One generic Graph 5 preparation pipeline before repository publication is bound.

use super::budget::ChangeBudgetMeter;
use super::{
    CanonicalBaseRead, CanonicalDelta, CanonicalReadWork, ChangeBudget, ChangeBudgetWork,
    DerivedDelta, IncrementalValidationReport, KernelOverlay, PlannedSummaries,
    TestDependencyDelta, WitnessMapBase, WitnessMapUpdate, WitnessReadWork, derive_local_delta,
    derive_test_dependency_delta, plan_impact_and_summaries, validate_incremental_frontier,
    validate_structural_frontier,
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
    let mut budget = ChangeBudgetMeter::new(budget, initial_work).map_err(|error| vec![error])?;
    budget
        .observe_canonical(&canonical)
        .map_err(|error| vec![error])?;
    let overlay = KernelOverlay::new(base, &canonical);
    let derived =
        derive_local_delta(&overlay, &canonical, base_witness).map_err(|error| vec![error])?;
    budget
        .observe_derived(&derived)
        .map_err(|error| vec![error])?;
    let structural = validate_structural_frontier(&overlay, &canonical, &derived, base_witness)?;
    budget
        .observe_structural(&structural)
        .map_err(|error| vec![error])?;
    let summaries = plan_impact_and_summaries(&overlay, &canonical, &derived, base_witness)
        .map_err(|error| vec![error])?;
    budget
        .observe_summaries(&summaries)
        .map_err(|error| vec![error])?;
    budget
        .observe_impact(&summaries.plan)
        .map_err(|error| vec![error])?;
    let validation = validate_incremental_frontier(
        &overlay,
        &canonical,
        &summaries.plan,
        &summaries.final_delta,
        base_witness,
        structural,
    )?;
    budget
        .observe_expression_validation(validation.work.expression_work)
        .map_err(|error| vec![error])?;
    let tests = derive_test_dependency_delta(&overlay, &canonical, &derived, base_witness)
        .map_err(|error| vec![error])?;
    budget.observe_tests(&tests).map_err(|error| vec![error])?;
    let witness = base_witness
        .update_witness_maps(&derived, &summaries.final_delta, &tests)
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
    })
}
