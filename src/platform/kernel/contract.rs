//! Graph Contract 5 identities, codec domains, and hostile-decoder limits.

pub const GRAPH_CONTRACT_IDENTITY: &str = "lkjscript-meaning-graph-5";
pub const GRAPH_CONTRACT_VERSION: u16 = 5;

pub const OWNER_MAGIC: [u8; 8] = *b"LKJOWN05";
pub const TYPE_OBJECT_MAGIC: [u8; 8] = *b"LKJTYP05";
pub const ROOT_MAGIC: [u8; 8] = *b"LKJGRF05";
pub const DEPENDENCY_MAGIC: [u8; 8] = *b"LKJDEP05";
pub const RETIREMENT_MAGIC: [u8; 8] = *b"LKJRET05";

pub const OWNER_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.owner-envelope.v5";
pub const TYPE_OBJECT_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.type-envelope.v5";
pub const ROOT_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.root-envelope.v5";
pub const DEPENDENCY_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.dependency-envelope.v5";
pub const RETIREMENT_ENVELOPE_DOMAIN: &str = "lkjscript.kernel.retirement-envelope.v5";

pub const OWNER_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.kernel.owner-object.v5";
pub const TYPE_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.kernel.type-object.v5";
pub const BLOB_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.kernel.blob-object.v5";
pub const SEQUENCE_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.kernel.sequence-object.v5";
pub const SEMANTIC_ROOT_DIGEST_DOMAIN: &str = "lkjscript.kernel.semantic-root.v5";
pub const DEPENDENCY_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.kernel.dependency-object.v5";
pub const RETIREMENT_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.kernel.retirement-object.v5";
pub const PACKAGE_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.kernel.package-object.v5";
pub const CHANGE_DIGEST_DOMAIN: &str = "lkjscript.kernel.change.v5";
pub const PACKAGE_ID_MIGRATION_DOMAIN: &str = "lkjscript.kernel.package-identity-migration.v5";

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
pub const MAXIMUM_TYPE_DEPTH: usize = 256;
pub const MAXIMUM_EXPRESSION_DEPTH: usize = 1_024;
pub const MAXIMUM_VALIDATION_WORK: usize = 10_000_000;
