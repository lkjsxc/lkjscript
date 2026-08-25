//! Transport-neutral parsing and rendering for the compact agent control plane.

mod change;
mod compact;

pub(crate) use change::{
    CHANGE_PLAN_DIGEST_DOMAIN, COMPACT_CHANGE_CONTRACT_IDENTITY, COMPACT_CHANGE_OPERATIONS,
    COMPACT_EXPRESSION_FORMS, COMPACT_TYPE_FORMS, ChangePlanDigest, decode_compact_change,
};
pub use compact::{
    CompactField, CompactRecord, CompactResponseLimits, CompactResponseWriter,
    MAXIMUM_COMPACT_INPUT_BYTES, MAXIMUM_COMPACT_RECORDS, parse_records, render_record,
};
