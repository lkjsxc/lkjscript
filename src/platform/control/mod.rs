//! Transport-neutral parsing and rendering for the compact agent control plane.

mod change;
mod compact;

pub(crate) use change::{
    CHANGE_PLAN_DIGEST_DOMAIN, COMPACT_CHANGE_CONTRACT_IDENTITY,
    COMPACT_CHANGE_OPERATION_DESCRIPTORS, COMPACT_CHANGE_PRECONDITION_FIELDS,
    COMPACT_CHANGE_PRECONDITIONS, COMPACT_DECLARATION_VISIBILITIES, COMPACT_DELETE_POLICIES,
    COMPACT_EXPRESSION_FORMS, COMPACT_FUNCTION_EFFECTS, COMPACT_NAMESPACE_CLASSES,
    COMPACT_TYPE_FORMS, ChangePlanDigest, CompactChangeFieldForm, CompactChangeOperation,
    NormalizedChangeRequest, compact_change_operation_descriptor, decode_compact_change,
    normalize_change_request,
};
pub use compact::{
    CompactField, CompactRecord, CompactResponseLimits, CompactResponseWriter,
    MAXIMUM_COMPACT_INPUT_BYTES, MAXIMUM_COMPACT_RECORDS, parse_records, render_record,
};
