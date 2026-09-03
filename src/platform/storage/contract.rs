//! Current Graph 8 immutable-store contract facts.

pub const OBJECT_STORE_CONTRACT_IDENTITY: &str = "lkjscript-immutable-object-store-1";
pub const PACK_CONTRACT_IDENTITY: &str = "lkjscript-immutable-object-pack-1";
pub const CATALOG_CONTRACT_IDENTITY: &str = "lkjscript-object-catalog-2";

pub const PACK_MAGIC: [u8; 8] = *b"LKJPAK01";
pub const PACK_INDEX_MAGIC: [u8; 8] = *b"LKJIDX01";
pub const PACK_END_MAGIC: [u8; 8] = *b"LKJEND01";
pub const PACK_CONTRACT_VERSION: u16 = 1;

pub const CATALOG_MANIFEST_MAGIC: [u8; 8] = *b"LKJCMN02";
pub const CATALOG_MANIFEST_END_MAGIC: [u8; 8] = *b"LKJCMNE2";
pub const CATALOG_SEGMENT_MAGIC: [u8; 8] = *b"LKJCSE02";
pub const CATALOG_SEGMENT_METADATA_MAGIC: [u8; 8] = *b"LKJCSM02";
pub const CATALOG_SEGMENT_END_MAGIC: [u8; 8] = *b"LKJCSEN2";
pub const CATALOG_CONTRACT_VERSION: u16 = 2;

pub const PACK_NONCE_DOMAIN: &str = "lkjscript.object-pack.nonce.v1";
pub const PACK_ID_DOMAIN: &str = "lkjscript.object-pack.identity.v1";
pub const PACK_ENTRY_CHECKSUM_DOMAIN: &str = "lkjscript.object-pack.entry.v1";
pub const PACK_INDEX_CHECKSUM_DOMAIN: &str = "lkjscript.object-pack.index.v1";
pub const PACK_CHECKSUM_DOMAIN: &str = "lkjscript.object-pack.complete.v1";
pub const CATALOG_MANIFEST_CHECKSUM_DOMAIN: &str = "lkjscript.object-catalog.manifest.complete.v2";
pub const CATALOG_SEGMENT_CHECKSUM_DOMAIN: &str = "lkjscript.object-catalog.segment.complete.v2";
pub const CATALOG_BLOCK_CHECKSUM_DOMAIN: &str = "lkjscript.object-catalog.segment-block.v2";
pub const CATALOG_LOGICAL_ENTRY_DOMAIN: &str = "lkjscript.object-catalog.logical-entry.v2";
pub const CATALOG_LOGICAL_COMMITMENT_DOMAIN: &str =
    "lkjscript.object-catalog.logical-commitment.v2";

pub use crate::platform::kernel::contract::{
    BLOB_OBJECT_DIGEST_DOMAIN, CHANGE_DIGEST_DOMAIN, DEPENDENCY_OBJECT_DIGEST_DOMAIN,
    OWNER_OBJECT_DIGEST_DOMAIN, PACKAGE_REVISION_DIGEST_DOMAIN, PACKAGE_TRANSPORT_DIGEST_DOMAIN,
    RETIREMENT_OBJECT_DIGEST_DOMAIN, SEMANTIC_ROOT_DIGEST_DOMAIN, SEQUENCE_OBJECT_DIGEST_DOMAIN,
    TYPE_OBJECT_DIGEST_DOMAIN,
};
pub use crate::platform::witness::contract::{
    OWNER_SUMMARY_DIGEST_DOMAIN, VALIDATION_WITNESS_DIGEST_DOMAIN,
};
pub const REVISION_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.revision-object.v7";
pub const RECEIPT_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.receipt-object.v5";
pub const TRANSACTION_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.transaction-object.v5";
pub const SEMANTIC_DIFF_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.semantic-diff-object.v3";
pub const DRAFT_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.draft-object.v5";
pub const CONFLICT_OBJECT_DIGEST_DOMAIN: &str = "lkjscript.conflict-object.v5";
pub const COMPILER_UNIT_DIGEST_DOMAIN: &str = "lkjscript.compiler-unit.v1";
pub const COMPILATION_MANIFEST_DIGEST_DOMAIN: &str = "lkjscript.compilation-manifest-object.v1";
pub const ARTIFACT_MANIFEST_DIGEST_DOMAIN: &str = "lkjscript.artifact-manifest.v13";
pub const BACKUP_MANIFEST_DIGEST_DOMAIN: &str = "lkjscript.backup-manifest.v5";
pub const BACKUP_SEGMENT_DIGEST_DOMAIN: &str = "lkjscript.backup-segment.v5";
pub const PACKAGE_INTERFACE_OWNER_DIGEST_DOMAIN: &str = "lkjscript.package-interface-owner.v6";
pub const MAP_PAGE_DIGEST_DOMAIN: &str =
    crate::platform::contract::registry::MAP_PAGE_DIGEST_DOMAIN;

pub const TARGET_PACK_BYTES: usize = 256 * 1024;
pub const MAXIMUM_PACK_BYTES: usize = 1_073_741_824;
pub const MAXIMUM_PACK_ENTRY_BYTES: usize = 64 * 1024 * 1024;
pub const MAXIMUM_PACK_ENTRIES: usize = 2_000_000;
pub const CATALOG_BLOCK_ENTRIES: usize = 64;
pub const CATALOG_BLOCK_FILTER_BYTES: usize = 128;
pub const MAXIMUM_CATALOG_ENTRIES: usize = 8_000_000;
pub const MAXIMUM_CATALOG_PACKS: usize = 100_000;
pub const MAXIMUM_CATALOG_SEGMENTS: usize = 32;
pub const MAXIMUM_CATALOG_LEVEL: u16 = 31;
pub const CATALOG_RECOVERY_LEVEL: u16 = MAXIMUM_CATALOG_LEVEL;
pub const MAXIMUM_CATALOG_BLOCKS: usize = MAXIMUM_CATALOG_ENTRIES.div_ceil(CATALOG_BLOCK_ENTRIES);
pub const MAXIMUM_CATALOG_MANIFEST_BYTES: usize = 64 * 1024;
pub const MAXIMUM_CATALOG_SEGMENT_METADATA_BYTES: usize = 64 * 1024 * 1024;
pub const MAXIMUM_CATALOG_SEGMENT_BYTES: usize = 1024 * 1024 * 1024;
pub const MAXIMUM_CATALOG_LEFTOVERS: usize = 128;
