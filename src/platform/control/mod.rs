//! Transport-neutral parsing and rendering for the compact agent control plane.

mod change;
mod compact;
mod logical_plan;

pub(crate) use change::{
    AUTHORED_CHANGE_CODEC_IDENTITY, AUTHORED_CHANGE_CODEC_VERSION,
    CHANGE_REQUEST_COMMITMENT_DOMAIN, COMPACT_CHANGE_CONTRACT_IDENTITY,
    COMPACT_CHANGE_CONTRACT_VERSION, COMPACT_CHANGE_OPERATION_DESCRIPTORS,
    COMPACT_CHANGE_PRECONDITION_FIELDS, COMPACT_CHANGE_PRECONDITIONS,
    COMPACT_DECLARATION_VISIBILITIES, COMPACT_DELETE_POLICIES, COMPACT_EXPRESSION_FORMS,
    COMPACT_FUNCTION_EFFECTS, COMPACT_NAMESPACE_CLASSES, COMPACT_TYPE_FORMS,
    ChangeRequestCommitment, CompactChangeFieldForm, CompactChangeOperation,
    NormalizedChangeRequest, compact_change_operation_descriptor, decode_compact_change,
    normalize_change_request,
};
pub use compact::{
    CompactField, CompactRecord, CompactResponseLimits, CompactResponseWriter,
    MAXIMUM_COMPACT_INPUT_BYTES, MAXIMUM_COMPACT_RECORDS, parse_records, render_record,
};
#[cfg(test)]
pub(crate) use logical_plan::encode_logical_change_plan_with_limits;
pub(crate) use logical_plan::{
    ChangePlanToken, LOGICAL_PLAN_RECORD_DESCRIPTORS, LogicalChangePlan, LogicalPlanEncoding,
    encode_logical_change_plan,
};
pub use logical_plan::{
    DecodedLogicalPlan, LOGICAL_CHANGE_PLAN_CONTRACT_IDENTITY,
    LOGICAL_CHANGE_PLAN_CONTRACT_VERSION, LogicalPlanCounts, MAXIMUM_LOGICAL_PLAN_BYTES,
    MAXIMUM_LOGICAL_PLAN_RECORDS, PREPARED_CHANGE_PLAN_COMMITMENT_DOMAIN,
    decode_logical_change_plan,
};
