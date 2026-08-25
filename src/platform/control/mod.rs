//! Transport-neutral parsing and rendering for the compact agent control plane.

mod compact;

pub use compact::{
    CompactField, CompactRecord, CompactResponseLimits, CompactResponseWriter,
    MAXIMUM_COMPACT_INPUT_BYTES, MAXIMUM_COMPACT_RECORDS, parse_records, render_record,
};
