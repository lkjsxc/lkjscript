//! Supported graph generations, current type-object codec, and hostile-decoder limits.

pub const GRAPH_CONTRACT_IDENTITY: &str = "lkjscript-meaning-graph-30";
pub const GRAPH_CONTRACT_VERSION: u16 = 30;
pub const SHARE_GRAPH_CONTRACT_VERSION: u16 = 30;
pub const COMPOSABLE_IMPLEMENTATION_GRAPH_CONTRACT_VERSION: u16 = 29;
pub const IMPLEMENTATION_SCHEME_GRAPH_CONTRACT_VERSION: u16 = 28;
pub const BORROW_RESULT_GRAPH_CONTRACT_VERSION: u16 = 27;
pub const PARAMETERIZED_CONTRACT_GRAPH_CONTRACT_VERSION: u16 = 26;
pub const SEQUENCE_GRAPH_CONTRACT_VERSION: u16 = 25;
pub const BORROW_GRAPH_CONTRACT_VERSION: u16 = 24;
pub const OWNED_EFFECT_GRAPH_CONTRACT_VERSION: u16 = 23;
pub const TRANSFER_GRAPH_CONTRACT_VERSION: u16 = 22;
pub const PARALLEL_GRAPH_CONTRACT_VERSION: u16 = 21;
pub const CHOICE_GRAPH_CONTRACT_VERSION: u16 = 20;
pub const PRODUCT_GRAPH_CONTRACT_VERSION: u16 = 19;
pub const OWNED_GRAPH_CONTRACT_VERSION: u16 = 18;
pub const SCALAR_GRAPH_CONTRACT_VERSION: u16 = 17;
pub const TRANSACTION_GRAPH_CONTRACT_VERSION: u16 = 16;
pub const REQUIREMENT_GRAPH_CONTRACT_VERSION: u16 = 15;
pub const PREDECESSOR_GRAPH_CONTRACT_VERSION: u16 = 14;
pub const fn supported_graph_contract(version: u16) -> bool {
    version == GRAPH_CONTRACT_VERSION
        || version == COMPOSABLE_IMPLEMENTATION_GRAPH_CONTRACT_VERSION
        || version == IMPLEMENTATION_SCHEME_GRAPH_CONTRACT_VERSION
        || version == BORROW_RESULT_GRAPH_CONTRACT_VERSION
        || version == PARAMETERIZED_CONTRACT_GRAPH_CONTRACT_VERSION
        || version == SEQUENCE_GRAPH_CONTRACT_VERSION
        || version == BORROW_GRAPH_CONTRACT_VERSION
        || version == OWNED_EFFECT_GRAPH_CONTRACT_VERSION
        || version == TRANSFER_GRAPH_CONTRACT_VERSION
        || version == PARALLEL_GRAPH_CONTRACT_VERSION
        || version == CHOICE_GRAPH_CONTRACT_VERSION
        || version == PRODUCT_GRAPH_CONTRACT_VERSION
        || version == OWNED_GRAPH_CONTRACT_VERSION
        || version == SCALAR_GRAPH_CONTRACT_VERSION
        || version == TRANSACTION_GRAPH_CONTRACT_VERSION
        || version == REQUIREMENT_GRAPH_CONTRACT_VERSION
        || version == PREDECESSOR_GRAPH_CONTRACT_VERSION
}
/// Existing base type bytes and identities stay unchanged; extensions use disjoint envelopes.
pub const TYPE_OBJECT_CONTRACT_IDENTITY: &str = "lkjscript-type-object-10";
pub const TYPE_OBJECT_CONTRACT_VERSION: u16 = 10;
pub const OWNED_SEQUENCE_TYPE_CONTRACT_VERSION: u16 = 1;
pub const OWNED_SEQUENCE_TYPE_MAGIC: [u8; 8] = *b"LKJSEQ01";
pub const OWNED_SEQUENCE_TYPE_ENVELOPE_DOMAIN: &str =
    "lkjscript.kernel.owned-sequence-type-envelope.v1";
pub const OWNED_CHOICE_TYPE_CONTRACT_VERSION: u16 = 1;
pub const OWNED_CHOICE_TYPE_MAGIC: [u8; 8] = *b"LKJCHO01";
pub const OWNED_CHOICE_TYPE_ENVELOPE_DOMAIN: &str =
    "lkjscript.kernel.owned-choice-type-envelope.v1";
pub const OWNED_PRODUCT_TYPE_CONTRACT_VERSION: u16 = 1;
pub const OWNED_PRODUCT_TYPE_MAGIC: [u8; 8] = *b"LKJPRD01";
pub const OWNED_PRODUCT_TYPE_ENVELOPE_DOMAIN: &str =
    "lkjscript.kernel.owned-product-type-envelope.v1";
pub const OWNED_CELL_TYPE_CONTRACT_VERSION: u16 = 1;
pub const OWNED_CELL_TYPE_MAGIC: [u8; 8] = *b"LKJCEL01";
pub const OWNED_CELL_TYPE_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owned-cell-type-envelope.v1";
pub const BYTE_BUFFER_TYPE_CONTRACT_VERSION: u16 = 1;
pub const BYTE_BUFFER_TYPE_MAGIC: [u8; 8] = *b"LKJBUF01";
pub const BYTE_BUFFER_TYPE_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.byte-buffer-type-envelope.v1";
pub const F64_TYPE_CONTRACT_VERSION: u16 = 1;
pub const F64_TYPE_MAGIC: [u8; 8] = *b"LKJF6401";
pub const F64_TYPE_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.f64-type-envelope.v1";
pub const NOMINAL_APPLICATION_CONTRACT_VERSION: u16 = 1;
pub const TASK_FUNCTION_CONTRACT_VERSION: u16 = 1;
pub const TASK_FUNCTION_MAGIC: [u8; 8] = *b"LKJTFN01";
pub const TASK_FUNCTION_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.task-function-envelope.v1";
pub const REQUIREMENT_TASK_FUNCTION_MAGIC: [u8; 8] = *b"LKJTFN02";
pub const REQUIREMENT_TASK_FUNCTION_ENVELOPE_DOMAIN: &str =
    "lkjscript.kernel.task-function-envelope.v2";
pub const NOMINAL_APPLICATION_MAGIC: [u8; 8] = *b"LKJTAP01";
pub const NOMINAL_APPLICATION_ENVELOPE_DOMAIN: &str =
    "lkjscript.kernel.nominal-application-envelope.v1";
pub const SEMANTIC_STATE_CONTRACT_VERSION: u16 = 1;

pub const OWNER_MAGIC: [u8; 8] = *b"LKJOWN30";
pub const COMPOSABLE_IMPLEMENTATION_OWNER_MAGIC: [u8; 8] = *b"LKJOWN29";
pub const IMPLEMENTATION_SCHEME_OWNER_MAGIC: [u8; 8] = *b"LKJOWN28";
pub const BORROW_RESULT_OWNER_MAGIC: [u8; 8] = *b"LKJOWN27";
pub const PARAMETERIZED_CONTRACT_OWNER_MAGIC: [u8; 8] = *b"LKJOWN26";
pub const SEQUENCE_OWNER_MAGIC: [u8; 8] = *b"LKJOWN25";
pub const BORROW_OWNER_MAGIC: [u8; 8] = *b"LKJOWN24";
pub const OWNED_EFFECT_OWNER_MAGIC: [u8; 8] = *b"LKJOWN23";
pub const TRANSFER_OWNER_MAGIC: [u8; 8] = *b"LKJOWN22";
pub const PARALLEL_OWNER_MAGIC: [u8; 8] = *b"LKJOWN21";
pub const CHOICE_OWNER_MAGIC: [u8; 8] = *b"LKJOWN20";
pub const PRODUCT_OWNER_MAGIC: [u8; 8] = *b"LKJOWN19";
pub const OWNED_OWNER_MAGIC: [u8; 8] = *b"LKJOWN18";
pub const SCALAR_OWNER_MAGIC: [u8; 8] = *b"LKJOWN17";
pub const TRANSACTION_OWNER_MAGIC: [u8; 8] = *b"LKJOWN16";
pub const REQUIREMENT_OWNER_MAGIC: [u8; 8] = *b"LKJOWN15";
pub const PREDECESSOR_OWNER_MAGIC: [u8; 8] = *b"LKJOWN14";
pub const TYPE_OBJECT_MAGIC: [u8; 8] = *b"LKJTYP10";
pub const ROOT_MAGIC: [u8; 8] = *b"LKJSMR01";
pub const DEPENDENCY_MAGIC: [u8; 8] = *b"LKJDEP14";
pub const RETIREMENT_MAGIC: [u8; 8] = *b"LKJRET14";

pub const OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v30";
pub const COMPOSABLE_IMPLEMENTATION_OWNER_ENVELOPE_DOMAIN: &str =
    "lkjscript.kernel.owner-envelope.v29";
pub const IMPLEMENTATION_SCHEME_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v28";
pub const BORROW_RESULT_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v27";
pub const PARAMETERIZED_CONTRACT_OWNER_ENVELOPE_DOMAIN: &str =
    "lkjscript.kernel.owner-envelope.v26";
pub const SEQUENCE_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v25";
pub const BORROW_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v24";
pub const OWNED_EFFECT_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v23";
pub const TRANSFER_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v22";
pub const PARALLEL_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v21";
pub const CHOICE_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v20";
pub const PRODUCT_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v19";
pub const OWNED_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v18";
pub const SCALAR_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v17";
pub const TRANSACTION_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v16";
pub const REQUIREMENT_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v15";
pub const PREDECESSOR_OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v14";
pub const TYPE_OBJECT_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.type-envelope.v10";
pub const ROOT_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.root-envelope.v14";
pub const DEPENDENCY_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.dependency-envelope.v14";
pub const RETIREMENT_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.retirement-envelope.v14";

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
