//! Current semantic contracts, canonical object kinds, and hostile-decoder limits.

pub const GRAPH_CONTRACT_IDENTITY: &str = "lkjscript-meaning-graph-34";
pub const GRAPH_CONTRACT_VERSION: u16 = 34;
pub const READ_VIEW_GRAPH_CONTRACT_VERSION: u16 = 34;
pub const ASSOCIATED_READ_VIEW_CONTRACT_IDENTITY: &str = "lkjscript-associated-read-views-1";
pub const ASSOCIATED_READ_VIEW_CONTRACT_VERSION: u16 = 1;
/// A separately identified additive wire capability; semantic admission remains gated.
pub const OWNED_ASSOCIATED_CONTRACT_IDENTITY: &str = "lkjscript-owned-associated-types-1";
pub const OWNED_ASSOCIATED_CONTRACT_VERSION: u16 = 1;
pub const MIXED_STORAGE_GRAPH_CONTRACT_VERSION: u16 = 33;
pub const SHARE_GRAPH_CONTRACT_VERSION: u16 = 30;
pub const TRANSFER_GRAPH_CONTRACT_VERSION: u16 = 22;
pub const PARALLEL_GRAPH_CONTRACT_VERSION: u16 = 21;
pub const OWNED_GRAPH_CONTRACT_VERSION: u16 = 18;
pub const REQUIREMENT_GRAPH_CONTRACT_VERSION: u16 = 15;
pub const fn supported_graph_contract(version: u16) -> bool {
    version == GRAPH_CONTRACT_VERSION
}
/// Current type-object semantic profile. Schema identity is bound by canonical framing.
pub const TYPE_OBJECT_CONTRACT_IDENTITY: &str = "lkjscript-type-object-10";
pub const TYPE_OBJECT_CONTRACT_VERSION: u16 = 10;
pub const NOMINAL_APPLICATION_CONTRACT_VERSION: u16 = 1;
pub const SEMANTIC_STATE_CONTRACT_VERSION: u16 = 1;

pub const OWNER_MAGIC: [u8; 8] = *b"LKJOWNER";
pub const TYPE_OBJECT_MAGIC: [u8; 8] = *b"LKJTYPES";
pub const ROOT_MAGIC: [u8; 8] = *b"LKJSMR01";
pub const DEPENDENCY_MAGIC: [u8; 8] = *b"LKJDEP14";
pub const RETIREMENT_MAGIC: [u8; 8] = *b"LKJRET14";

pub const OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope";
pub const TYPE_OBJECT_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.type-envelope";
pub const ROOT_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.root-envelope";
pub const DEPENDENCY_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.dependency-envelope";
pub const RETIREMENT_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.retirement-envelope";

pub const OWNER_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.kernel.owner-object.v14";
pub const TYPE_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.kernel.type-object.v10";
pub const BLOB_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.kernel.blob-object.v5";
pub const SEQUENCE_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.kernel.sequence-object.v5";
pub const SEMANTIC_ROOT_DIGEST_DOMAIN: &str = "lkjscript.kernel.semantic-root.v14";
pub const SEMANTIC_STATE_DIGEST_DOMAIN: &str = "lkjscript.kernel.semantic-state.v1";
pub const DEPENDENCY_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.kernel.dependency-object.v14";
pub const RETIREMENT_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.kernel.retirement-object.v14";
pub const PACKAGE_REVISION_DIGEST_DOMAIN: &str = "lkjscript.kernel.package-revision.v1";
pub const PACKAGE_INTERFACE_DIGEST_DOMAIN: &str = "lkjscript.kernel.package-interface.v1";
pub const PACKAGE_TRANSPORT_DIGEST_DOMAIN: &str = "lkjscript.kernel.package-transport.v1";
pub const CHANGE_DIGEST_DOMAIN: &str = "lkjscript.kernel.change.v14";
pub const PACKAGE_ID_MIGRATION_DOMAIN: &str = "lkjscript.kernel.package-identity-migration.v11";

pub const MAXIMUM_OWNER_OBJECT_BYTES: usize = 4 * 1_048_576;
pub const MAXIMUM_TYPE_OBJECT_BYTES: usize = 1_048_576;
pub const MAXIMUM_ROOT_BYTES: usize = 64 * 1024;
pub const MAXIMUM_DEPENDENCY_BYTES: usize = 1_048_576;
pub const MAXIMUM_RETIREMENT_BYTES: usize = 64 * 1024;

pub const MAXIMUM_NAME_BYTES: usize = 128;
pub const MAXIMUM_INLINE_TEXT_BYTES: usize = 64 * 1024;
pub const MAXIMUM_DOCUMENTATION_BYTES: usize = 16 * 1_048_576;
pub const MAXIMUM_CHILDREN: usize = 100_000;
pub const MAXIMUM_RESOURCE_LIMITS: usize = 1_024;
pub const MAXIMUM_HTTP_ROUTES_PER_TARGET: usize = 4_096;
pub const MAXIMUM_HTTP_ROUTE_METHOD_BYTES: usize = 32;
pub const MAXIMUM_HTTP_ROUTE_PATH_BYTES: usize = 16 * 1_024;
pub const MAXIMUM_HTTP_ROUTE_KEY_BYTES_PER_TARGET: usize = 4 * 1_048_576;
pub const MAXIMUM_HTTP_PATTERN_SEGMENTS: usize = 64;
pub const MAXIMUM_HTTP_PATTERN_CAPTURES: usize = 32;
pub const MAXIMUM_HTTP_PATTERN_SEGMENTS_PER_TARGET: usize = 65_536;
pub const MAXIMUM_TYPE_DEPTH: usize = 256;
pub const MAXIMUM_EXPRESSION_DEPTH: usize = 1_024;
pub const MAXIMUM_VALIDATION_WORK: usize = 10_000_000;
