//! One generic Graph 5 preparation pipeline before repository publication is bound.

use super::{
    CanonicalBaseRead, CanonicalDelta, CanonicalReadWork, DerivedDelta,
    IncrementalValidationReport, KernelOverlay, PlannedSummaries, TestDependencyDelta,
    WitnessMapBase, WitnessMapUpdate, derive_local_delta, derive_test_dependency_delta,
    plan_impact_and_summaries, validate_incremental_frontier, validate_structural_frontier,
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
}

pub fn prepare_change_analysis<B: CanonicalBaseRead + ?Sized, W: WitnessMapBase + ?Sized>(
    base: &B,
    base_witness: &W,
    canonical: CanonicalDelta,
) -> Result<PreparedChangeAnalysis, Vec<Diagnostic>> {
    let overlay = KernelOverlay::new(base, &canonical);
    let derived =
        derive_local_delta(&overlay, &canonical, base_witness).map_err(|error| vec![error])?;
    let structural = validate_structural_frontier(&overlay, &canonical, &derived, base_witness)?;
    let summaries = plan_impact_and_summaries(&overlay, &canonical, &derived, base_witness)
        .map_err(|error| vec![error])?;
    let validation = validate_incremental_frontier(
        &overlay,
        &canonical,
        &summaries.plan,
        &summaries.final_delta,
        base_witness,
        structural,
    )?;
    let tests = derive_test_dependency_delta(&overlay, &canonical, &derived, base_witness)
        .map_err(|error| vec![error])?;
    let witness = base_witness
        .update_witness_maps(&derived, &summaries.final_delta, &tests)
        .map_err(|error| vec![error])?;
    let canonical_read_work = overlay.work();
    Ok(PreparedChangeAnalysis {
        canonical,
        derived,
        summaries,
        tests,
        witness,
        validation,
        canonical_read_work,
    })
}
