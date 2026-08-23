//! One generic Graph 5 preparation pipeline before repository publication is bound.

use super::{
    CanonicalDelta, DerivedDelta, IncrementalValidationReport, KernelOverlay, PlannedSummaries,
    TestDependencyDelta, WitnessMapUpdate, derive_local_delta, derive_test_dependency_delta,
    plan_impact_and_summaries, update_witness_maps, validate_incremental_frontier,
    validate_structural_frontier,
};
use crate::platform::diagnostic::Diagnostic;
use crate::platform::kernel::KernelSnapshot;
use crate::platform::witness::FullWitness;

#[derive(Clone, Debug)]
pub struct PreparedChangeAnalysis {
    pub canonical: CanonicalDelta,
    pub derived: DerivedDelta,
    pub summaries: PlannedSummaries,
    pub tests: TestDependencyDelta,
    pub witness: WitnessMapUpdate,
    pub validation: IncrementalValidationReport,
}

pub fn prepare_change_analysis(
    base: &KernelSnapshot,
    base_witness: &FullWitness,
    canonical: CanonicalDelta,
) -> Result<PreparedChangeAnalysis, Vec<Diagnostic>> {
    let overlay = KernelOverlay::new(base, &canonical);
    let derived = derive_local_delta(base, &overlay, &canonical, base_witness)
        .map_err(|error| vec![error])?;
    let structural = validate_structural_frontier(&overlay, &canonical, &derived, base_witness)?;
    let summaries = plan_impact_and_summaries(&overlay, &canonical, &derived, base_witness)
        .map_err(|error| vec![error])?;
    let validation = validate_incremental_frontier(
        &overlay,
        &canonical,
        &summaries.plan,
        base_witness,
        structural,
    )?;
    let tests = derive_test_dependency_delta(&overlay, &canonical, &derived, base_witness)
        .map_err(|error| vec![error])?;
    let witness = update_witness_maps(base_witness, &derived, &summaries.final_delta, &tests)
        .map_err(|error| vec![error])?;
    Ok(PreparedChangeAnalysis {
        canonical,
        derived,
        summaries,
        tests,
        witness,
        validation,
    })
}
