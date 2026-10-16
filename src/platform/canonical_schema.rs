//! The sole current binary schema. Implementation identity only partitions caches.

pub const SCHEMA_EPOCH: u16 = 1;

include!(concat!(env!("OUT_DIR"), "/canonical_schema.rs"));

pub const CANONICAL_SCHEMA_IDENTITY: &str = "lkjscript-canonical-schema";

/// Exact generated implementation metadata admitted by this compiled product.
pub fn generated_source() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/canonical_schema.rs"))
}

/// Exact build outputs bound by contributor Cargo-origin evidence.
pub fn generated_outputs() -> [(&'static str, &'static [u8]); 3] {
    [
        ("canonical_schema.rs", generated_source().as_bytes()),
        (
            "standard.lkjp.deflate",
            include_bytes!(concat!(env!("OUT_DIR"), "/standard.lkjp.deflate")),
        ),
        (
            "standard.lkja.deflate",
            include_bytes!(concat!(env!("OUT_DIR"), "/standard.lkja.deflate")),
        ),
    ]
}
