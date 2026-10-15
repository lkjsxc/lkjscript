//! Current accepted-history kinds within the canonical schema.

pub const REVISION_CONTRACT_IDENTITY: &str = "lkjscript-revision";
pub const REVISION_CONTRACT_VERSION: u16 = 7;
pub const RECEIPT_CONTRACT_IDENTITY: &str = "lkjscript-receipt";
pub const RECEIPT_CONTRACT_VERSION: u16 = 6;
pub const TRANSACTION_CONTRACT_IDENTITY: &str = "lkjscript-transaction";
pub const TRANSACTION_CONTRACT_VERSION: u16 = 6;
pub const SEMANTIC_DIFF_CONTRACT_IDENTITY: &str = "lkjscript-semantic-diff";
pub const SEMANTIC_DIFF_CONTRACT_VERSION: u16 = 4;

pub const REVISION_MAGIC: [u8; 8] = *b"LKJREVIS";
pub const RECEIPT_MAGIC: [u8; 8] = *b"LKJRECPT";
pub const TRANSACTION_MAGIC: [u8; 8] = *b"LKJTRANS";
pub const SEMANTIC_DIFF_MAGIC: [u8; 8] = *b"LKJSDIFF";
pub const HEAD_MAGIC: [u8; 8] = *b"LKJHEAD_";
pub const IDEMPOTENCY_BINDING_MAGIC: [u8; 8] = *b"LKJIDEMP";
pub const IDEMPOTENCY_BINDING_CONTRACT_VERSION: u16 = 1;

pub const REVISION_ENVELOPE_DOMAIN: &str = "lkjscript.revision-record-envelope";
pub const RECEIPT_ENVELOPE_DOMAIN: &str = "lkjscript.receipt-envelope";
pub const TRANSACTION_ENVELOPE_DOMAIN: &str = "lkjscript.transaction-envelope";
pub const SEMANTIC_DIFF_ENVELOPE_DOMAIN: &str = "lkjscript.semantic-diff-envelope";
pub const HEAD_ENVELOPE_DOMAIN: &str = "lkjscript.head-envelope";
pub const REVISION_IDENTITY_DIGEST_DOMAIN: &str = "lkjscript.semantic-revision";
pub const IDEMPOTENCY_BINDING_ENVELOPE_DOMAIN: &str = "lkjscript.idempotency-binding-envelope";

pub const MAXIMUM_REVISION_BYTES: usize = 64 * 1024;
pub const MAXIMUM_RECEIPT_BYTES: usize = 64 * 1024;
pub const MAXIMUM_TRANSACTION_BYTES: usize = 4 * 1024 * 1024;
pub const MAXIMUM_SEMANTIC_DIFF_BYTES: usize = 4 * 1024 * 1024;
pub const MAXIMUM_HEAD_BYTES: usize = 4 * 1024;
pub const MAXIMUM_INLINE_HISTORY_EDITS: usize = 100_000;
pub const MAXIMUM_INTENT_BYTES: usize = 4 * 1024;
pub const MAXIMUM_IDEMPOTENCY_KEY_BYTES: usize = 128;
pub const MAXIMUM_IDEMPOTENCY_BINDING_BYTES: usize = 4 * 1024;
