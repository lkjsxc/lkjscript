//! Private Graph 5 generic semantic-change substrate.

#![allow(
    unused_imports,
    reason = "private change exports become crate consumers at the Graph 5 repository cutover"
)]

mod authority;
mod base_read;
mod budget;
mod delta;
mod derived;
mod impact;
mod overlay;
mod prepare;
mod relation_view;
mod request;
mod summary_delta;
mod test_delta;
mod validate;
mod witness_update;

pub use authority::{
    CanonicalMapEditCounts, FullStagedAuthority, PreparedAuthority, SemanticAuthority,
    WitnessAuthority, stage_full_authority,
};
pub(crate) use authority::{prepare_owner_map_edits, stage_prepared_authority};
pub use base_read::{
    BoundOwnerSummary, CanonicalBaseRead, CanonicalRead, CanonicalReadWork, WitnessBaseRead,
    WitnessRead, WitnessReadWork, WitnessRelationRead, WitnessTestDependencyRead,
};
pub(crate) use base_read::{BudgetedCanonicalBase, BudgetedWitnessBase};
pub use budget::{
    AuthoredChangeAdmission, CanonicalEditAdmission, CanonicalEditWork,
    CanonicalMapUpdateAdmission, CanonicalMapUpdateWork, CanonicalReadAdmission, ChangeBudget,
    ChangeBudgetWork, ImpactAdmission, MAXIMUM_CHANGE_OPERATIONS, StagingAdmission,
    StagingBudgetWork, TestAdmission, TestBudgetWork, ValidationAdmission, ValidationBudgetWork,
    WitnessReadAdmission, WitnessUpdateAdmission, WitnessUpdateWork,
};
pub use delta::{CanonicalDelta, CanonicalNormalization, ExactEdit, PrimitiveEdit};
pub use derived::{DerivedDelta, DerivedValueEdit, RelationDelta, derive_local_delta};
pub use impact::{
    ImpactPlan, ImpactReason, ImpactReasonKind, ImpactWork, PlannedSummaries,
    SummaryDimensionChange, plan_impact_and_summaries, summary_dimension_change,
};
pub use overlay::KernelOverlay;
pub use prepare::{
    PreparedChangeAnalysis, prepare_change_analysis, prepare_change_analysis_with_budget,
};
pub use request::{
    AuthoredAnnotationValue, AuthoredBindingDefinition, AuthoredCase, AuthoredCaseReference,
    AuthoredChange, AuthoredChangeSet, AuthoredDeclarationReference, AuthoredDeletePolicy,
    AuthoredExpression, AuthoredExpressionOperation, AuthoredField, AuthoredFieldReference,
    AuthoredFieldSelector, AuthoredFunctionEffect, AuthoredLetBinding, AuthoredLocalReference,
    AuthoredLowering, AuthoredLoweringWork, AuthoredMapExpressionEntry, AuthoredMatchExpressionArm,
    AuthoredOperation, AuthoredOperationReference, AuthoredOwnerParent, AuthoredParameter,
    AuthoredPort, AuthoredPortImplementation, AuthoredPortReference, AuthoredPrecondition,
    AuthoredRecordExpressionField, AuthoredRequirement, AuthoredRequirementReference,
    AuthoredResourceLimit, AuthoredStructuralTypeField, AuthoredType, AuthoredTypeParameter,
    AuthoredTypeParameterReference, DeclarationSelector, MAXIMUM_AUTHORED_CHANGE_BYTES,
    MAXIMUM_AUTHORED_CHANGES, ModuleSelector, OwnerSelector, ParameterParentSelector,
    lower_authored_changes,
};
pub(crate) use request::{canonical_authored_budget_bytes, canonical_authored_intent_bytes};
pub use summary_delta::{
    OwnerSummaryEdit, SummaryDelta, derive_summary_delta, derive_summary_delta_for,
};
pub use test_delta::{TestDeltaWork, TestDependencyDelta, derive_test_dependency_delta};
pub use validate::{
    INCREMENTAL_VALIDATION_PROFILE, IncrementalValidationReport, IncrementalValidationWork,
    StructuralValidationReport, validate_incremental_frontier, validate_structural_frontier,
};
pub use witness_update::{
    WitnessEditCounts, WitnessMapAdmission, WitnessMapBase, WitnessMapUpdate, update_witness_maps,
    update_witness_maps_from,
};

#[cfg(test)]
mod tests;
