//! Build-derived identity of the sole accepted canonical wire schema.

include!(concat!(env!("OUT_DIR"), "/canonical_schema.rs"));

pub const CANONICAL_SCHEMA_IDENTITY: &str = "lkjscript-canonical-schema";

/// Exact generated input admitted by this compiled product's source closure.
pub fn generated_source() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/canonical_schema.rs"))
}
