//! Private immutable-store contract facts pending executable-registry cutover.

pub const OBJECT_STORE_CONTRACT_IDENTITY: &str = "lkjscript-immutable-object-store-1";
pub const PACK_CONTRACT_IDENTITY: &str = "lkjscript-immutable-object-pack-1";
pub const CATALOG_CONTRACT_IDENTITY: &str = "lkjscript-object-catalog-1";

pub const PACK_MAGIC: [u8; 8] = *b"LKJPAK01";
pub const PACK_INDEX_MAGIC: [u8; 8] = *b"LKJIDX01";
pub const PACK_END_MAGIC: [u8; 8] = *b"LKJEND01";
pub const PACK_CONTRACT_VERSION: u16 = 1;

pub const CATALOG_MAGIC: [u8; 8] = *b"LKJCAT01";
pub const CATALOG_END_MAGIC: [u8; 8] = *b"LKJCEND1";
pub const CATALOG_CONTRACT_VERSION: u16 = 1;

pub const PACK_NONCE_DOMAIN: &str = "lkjscript.object-pack.nonce.v1";
pub const PACK_ID_DOMAIN: &str = "lkjscript.object-pack.identity.v1";
pub const PACK_ENTRY_CHECKSUM_DOMAIN: &str = "lkjscript.object-pack.entry.v1";
pub const PACK_INDEX_CHECKSUM_DOMAIN: &str = "lkjscript.object-pack.index.v1";
pub const PACK_CHECKSUM_DOMAIN: &str = "lkjscript.object-pack.complete.v1";
pub const CATALOG_CHECKSUM_DOMAIN: &str = "lkjscript.object-catalog.complete.v1";
pub const CATALOG_GENERATION_DOMAIN: &str = "lkjscript.object-catalog.generation.v1";

pub use crate::platform::kernel::contract::{
    BLOB_OBJECT_DIGEST_DOMAIN, CHANGE_DIGEST_DOMAIN, DEPENDENCY_OBJECT_DIGEST_DOMAIN,
    OWNER_OBJECT_DIGEST_DOMAIN, PACKAGE_OBJECT_DIGEST_DOMAIN, RETIREMENT_OBJECT_DIGEST_DOMAIN,
    SEMANTIC_ROOT_DIGEST_DOMAIN, SEQUENCE_OBJECT_DIGEST_DOMAIN, TYPE_OBJECT_DIGEST_DOMAIN,
};
pub const VALIDATION_WITNESS_DIGEST_DOMAIN: &str = "lkjscript.validation-witness.v1";
pub const OWNER_SUMMARY_DIGEST_DOMAIN: &str = "lkjscript.owner-summary.v1";
pub const REVISION_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.revision-object.v5";
pub const RECEIPT_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.receipt-object.v5";
pub const DRAFT_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.draft-object.v5";
pub const CONFLICT_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.conflict-object.v5";
pub const COMPILER_UNIT_DIGEST_DOMAIN: &str = "lkjscript.compiler-unit.v1";
pub const ARTIFACT_MANIFEST_DIGEST_DOMAIN: &str = "lkjscript.artifact-manifest.v5";
pub const BACKUP_MANIFEST_DIGEST_DOMAIN: &str = "lkjscript.backup-manifest.v5";
pub const BACKUP_SEGMENT_DIGEST_DOMAIN: &str = "lkjscript.backup-segment.v5";
pub const MAP_PAGE_DIGEST_DOMAIN: &str =
    crate::platform::contract::registry::MAP_PAGE_DIGEST_DOMAIN;

pub const TARGET_PACK_BYTES: usize = 256 * 1024;
pub const MAXIMUM_PACK_BYTES: usize = 1_073_741_824;
pub const MAXIMUM_PACK_ENTRY_BYTES: usize = 64 * 1024 * 1024;
pub const MAXIMUM_PACK_ENTRIES: usize = 2_000_000;
pub const MAXIMUM_CATALOG_BYTES: usize = 512 * 1024 * 1024;
