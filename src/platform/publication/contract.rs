//! Current Graph 10 accepted-history contract facts.

pub const REVISION_CONTRACT_IDENTITY: &str = "lkjscript-revision-7";
pub const REVISION_CONTRACT_VERSION: u16 = 7;
pub const RECEIPT_CONTRACT_IDENTITY: &str = "lkjscript-receipt-5";
pub const RECEIPT_CONTRACT_VERSION: u16 = 5;
pub const TRANSACTION_CONTRACT_IDENTITY: &str = "lkjscript-transaction-5";
pub const TRANSACTION_CONTRACT_VERSION: u16 = 5;
pub const SEMANTIC_DIFF_CONTRACT_IDENTITY: &str = "lkjscript-semantic-diff-3";
pub const SEMANTIC_DIFF_CONTRACT_VERSION: u16 = 3;

pub const REVISION_MAGIC: [u8; 8] = *b"LKJREV07";
pub const RECEIPT_MAGIC: [u8; 8] = *b"LKJRCPT5";
pub const TRANSACTION_MAGIC: [u8; 8] = *b"LKJTXN05";
pub const SEMANTIC_DIFF_MAGIC: [u8; 8] = *b"LKJDIFF3";
pub const HEAD_MAGIC: [u8; 8] = *b"LKJHEAD7";
pub const IDEMPOTENCY_BINDING_MAGIC: [u8; 8] = *b"LKJIDEM1";
pub const IDEMPOTENCY_BINDING_CONTRACT_VERSION: u16 = 1;

pub const REVISION_ENVELOPE_DOMAIN: &str = "lkjscript.revision-record-envelope.v7";
pub const RECEIPT_ENVELOPE_DOMAIN: &str = "lkjscript.receipt-envelope.v5";
pub const TRANSACTION_ENVELOPE_DOMAIN: &str = "lkjscript.transaction-envelope.v5";
pub const SEMANTIC_DIFF_ENVELOPE_DOMAIN: &str = "lkjscript.semantic-diff-envelope.v3";
pub const HEAD_ENVELOPE_DOMAIN: &str = "lkjscript.head-envelope.v7";
pub const REVISION_IDENTITY_DIGEST_DOMAIN: &str = "lkjscript.semantic-revision.v7";
pub const IDEMPOTENCY_BINDING_ENVELOPE_DOMAIN: &str = "lkjscript.idempotency-binding-envelope.v1";

pub const MAXIMUM_REVISION_BYTES: usize = 64 * 1024;
pub const MAXIMUM_RECEIPT_BYTES: usize = 64 * 1024;
pub const MAXIMUM_TRANSACTION_BYTES: usize = 4 * 1024 * 1024;
pub const MAXIMUM_SEMANTIC_DIFF_BYTES: usize = 4 * 1024 * 1024;
pub const MAXIMUM_HEAD_BYTES: usize = 4 * 1024;
pub const MAXIMUM_INLINE_HISTORY_EDITS: usize = 100_000;
pub const MAXIMUM_INTENT_BYTES: usize = 4 * 1024;
pub const MAXIMUM_IDEMPOTENCY_KEY_BYTES: usize = 128;
pub const MAXIMUM_IDEMPOTENCY_BINDING_BYTES: usize = 4 * 1024;
