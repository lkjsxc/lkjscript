//! Private Graph 5 generic semantic-change substrate.

#![allow(
    unused_imports,
    reason = "private change exports become crate consumers at the Graph 5 repository cutover"
)]

mod authority;
mod delta;
mod derived;
mod impact;
mod overlay;
mod prepare;
mod relation_view;
mod summary_delta;
mod test_delta;
mod validate;
mod witness_update;

pub use authority::{
    CanonicalMapEditCounts, FullStagedAuthority, PreparedAuthority, SemanticAuthority,
    WitnessAuthority, stage_full_authority, stage_prepared_authority,
};
pub use delta::{CanonicalDelta, ExactEdit, PrimitiveEdit};
pub use derived::{DerivedDelta, DerivedValueEdit, RelationDelta, derive_local_delta};
pub use impact::{
    ImpactPlan, ImpactReason, ImpactReasonKind, ImpactWork, PlannedSummaries,
    SummaryDimensionChange, plan_impact_and_summaries, summary_dimension_change,
};
pub use overlay::KernelOverlay;
pub use prepare::{PreparedChangeAnalysis, prepare_change_analysis};
pub use summary_delta::{
    OwnerSummaryEdit, SummaryDelta, derive_summary_delta, derive_summary_delta_for,
};
pub use test_delta::{TestDeltaWork, TestDependencyDelta, derive_test_dependency_delta};
pub use validate::{
    INCREMENTAL_VALIDATION_PROFILE, IncrementalValidationReport, IncrementalValidationWork,
    StructuralValidationReport, validate_incremental_frontier, validate_structural_frontier,
};
pub use witness_update::{WitnessEditCounts, WitnessMapUpdate, update_witness_maps};

#[cfg(test)]
mod tests;
