//! Immutable semantic revisions, atomic visibility records, and compact transaction receipts.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::meaning::GRAPH_CONTRACT_VERSION;
use super::package::PackageId;
use super::packed;
use super::semantic_digest::{
    ReceiptDigest, RevisionRecordDigest, RootObjectDigest, SemanticDiffDigest, TransactionDigest,
};
use super::semantic_id::{DeclarationId, ModuleId, RepositoryId, RevisionId, TargetId};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub const REVISION_CONTRACT_VERSION: u16 = 1;
pub const RECEIPT_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_REVISION_BYTES: usize = 256 * 1024;
pub const MAXIMUM_RECEIPT_BYTES: usize = 4 * 1_048_576;
const REVISION_MAGIC: [u8; 8] = *b"LKJREV01";
const REVISION_DOMAIN: &str = "lkjscript.revision-record-envelope.v1";
const RECEIPT_MAGIC: [u8; 8] = *b"LKJRCPT1";
const RECEIPT_DOMAIN: &str = "lkjscript.transaction-receipt-envelope.v1";
const HEAD_MAGIC: [u8; 8] = *b"LKJHEAD1";
const HEAD_DOMAIN: &str = "lkjscript.semantic-head-envelope.v1";

#[derive(Decode, Encode, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParentRevision {
    pub revision: RevisionId,
    pub record: RevisionRecordDigest,
}

impl Ord for ParentRevision {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.revision, self.record).cmp(&(other.revision, other.record))
    }
}

impl PartialOrd for ParentRevision {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionCore {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub repository_id: RepositoryId,
    pub parents: Vec<ParentRevision>,
    pub root: RootObjectDigest,
    pub semantic_diff: SemanticDiffDigest,
    pub transaction: TransactionDigest,
}

impl RevisionCore {
    pub fn revision_id(&self) -> Result<RevisionId, Diagnostic> {
        self.validate()?;
        let bytes = bincode::encode_to_vec(self, packed_configuration()).map_err(|error| {
            revision_error(
                DiagnosticClass::Infrastructure,
                "revision_identity_encode",
                format!("revision identity encoding failed: {error}"),
            )
        })?;
        let mut hasher = blake3::Hasher::new_derive_key("lkjscript.semantic-revision.v1");
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
        Ok(RevisionId::from_digest(*hasher.finalize().as_bytes()))
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != REVISION_CONTRACT_VERSION
            || self.graph_contract_version != GRAPH_CONTRACT_VERSION
        {
            return Err(revision_error(
                DiagnosticClass::Source,
                "revision_contract",
                "revision uses an unknown record or graph contract",
            ));
        }
        if self.parents.len() > 2 || (self.parents.len() == 2 && self.parents[0] >= self.parents[1])
        {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "revision_parents",
                "revision parents must be zero, one, or two unique canonical references",
            ));
        }
        Ok(())
    }
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionRecord {
    pub revision: RevisionId,
    pub core: RevisionCore,
    pub receipt: ReceiptDigest,
}

impl RevisionRecord {
    pub fn new(core: RevisionCore, receipt: ReceiptDigest) -> Result<Self, Diagnostic> {
        let revision = core.revision_id()?;
        Ok(Self {
            revision,
            core,
            receipt,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        packed::encode(
            REVISION_MAGIC,
            REVISION_DOMAIN,
            self,
            MAXIMUM_REVISION_BYTES,
        )
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self = packed::decode(
            bytes,
            REVISION_MAGIC,
            REVISION_DOMAIN,
            MAXIMUM_REVISION_BYTES,
        )?;
        value.validate()?;
        Ok(value)
    }

    pub fn digest(&self) -> Result<RevisionRecordDigest, Diagnostic> {
        Ok(RevisionRecordDigest::of(&self.encode()?))
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.revision != self.core.revision_id()? {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "revision_identity",
                "revision identity does not match its canonical core",
            ));
        }
        Ok(())
    }
}

#[derive(Decode, Encode, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptStatus {
    AcceptedChange,
    SemanticNoChange,
    Validated,
    MergeAccepted,
    RestoreAccepted,
    ImportAccepted,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum AffectedOwner {
    Module(ModuleId),
    Declaration(DeclarationId),
    Target(TargetId),
    Package(PackageId),
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationFacts {
    pub profile: String,
    pub graph_valid: bool,
    pub full_oracle_equal: bool,
    pub modules_checked: u64,
    pub declarations_checked: u64,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionReceipt {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub repository_id: RepositoryId,
    pub status: ReceiptStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<RevisionId>,
    pub result: RevisionId,
    pub transaction: TransactionDigest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    pub semantic_diff: SemanticDiffDigest,
    pub affected_owners: Vec<AffectedOwner>,
    pub validation: ValidationFacts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
}

impl TransactionReceipt {
    pub fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        packed::encode(RECEIPT_MAGIC, RECEIPT_DOMAIN, self, MAXIMUM_RECEIPT_BYTES)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self =
            packed::decode(bytes, RECEIPT_MAGIC, RECEIPT_DOMAIN, MAXIMUM_RECEIPT_BYTES)?;
        value.validate()?;
        Ok(value)
    }

    pub fn digest(&self) -> Result<ReceiptDigest, Diagnostic> {
        Ok(ReceiptDigest::of(&self.encode()?))
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != RECEIPT_CONTRACT_VERSION
            || self.graph_contract_version != GRAPH_CONTRACT_VERSION
        {
            return Err(revision_error(
                DiagnosticClass::Source,
                "receipt_contract",
                "receipt uses an unknown record or graph contract",
            ));
        }
        if !self.validation.graph_valid || !self.validation.full_oracle_equal {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "receipt_validation",
                "accepted receipt does not attest complete graph validation",
            ));
        }
        if self
            .affected_owners
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "receipt_affected_order",
                "affected owners are not unique and canonically ordered",
            ));
        }
        if self
            .intent
            .as_ref()
            .is_some_and(|value| value.len() > 4_096)
        {
            return Err(revision_error(
                DiagnosticClass::Resource,
                "receipt_intent_limit",
                "nonsemantic intent exceeds 4096 bytes",
            ));
        }
        if self.idempotency_key.as_ref().is_some_and(|value| {
            value.is_empty()
                || value.len() > 128
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        }) {
            return Err(revision_error(
                DiagnosticClass::Source,
                "receipt_idempotency_key",
                "idempotency key must contain 1 through 128 portable identifier bytes",
            ));
        }
        Ok(())
    }
}

#[derive(Decode, Encode, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticHead {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub repository_id: RepositoryId,
    pub revision: RevisionId,
    pub record: RevisionRecordDigest,
}

impl SemanticHead {
    pub fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        packed::encode(HEAD_MAGIC, HEAD_DOMAIN, self, 4096)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self = packed::decode(bytes, HEAD_MAGIC, HEAD_DOMAIN, 4096)?;
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != REVISION_CONTRACT_VERSION
            || self.graph_contract_version != GRAPH_CONTRACT_VERSION
        {
            return Err(revision_error(
                DiagnosticClass::Source,
                "head_contract",
                "HEAD uses an unknown record or graph contract",
            ));
        }
        Ok(())
    }
}

fn packed_configuration() -> impl bincode::config::Config {
    bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding()
}

fn revision_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_receipt_and_head_bind_each_layer() {
        let repository_id = RepositoryId::migrate(b"revision", 1);
        let transaction = TransactionDigest::of(b"transaction");
        let semantic_diff = SemanticDiffDigest::of(b"diff");
        let core = RevisionCore {
            contract_version: REVISION_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id,
            parents: Vec::new(),
            root: RootObjectDigest::of(b"root"),
            semantic_diff,
            transaction,
        };
        let revision = core.revision_id().expect("revision");
        let receipt = TransactionReceipt {
            contract_version: RECEIPT_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id,
            status: ReceiptStatus::ImportAccepted,
            base: None,
            result: revision,
            transaction,
            idempotency_key: None,
            semantic_diff,
            affected_owners: Vec::new(),
            validation: ValidationFacts {
                profile: "full".to_owned(),
                graph_valid: true,
                full_oracle_equal: true,
                modules_checked: 1,
                declarations_checked: 1,
            },
            intent: None,
        };
        let receipt_digest = receipt.digest().expect("receipt digest");
        let record = RevisionRecord::new(core, receipt_digest).expect("record");
        assert_eq!(record.revision, revision);
        assert_eq!(
            RevisionRecord::decode(&record.encode().expect("encode record")).expect("record"),
            record
        );
        assert_eq!(
            TransactionReceipt::decode(&receipt.encode().expect("encode receipt"))
                .expect("receipt"),
            receipt
        );
        let head = SemanticHead {
            contract_version: REVISION_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id,
            revision,
            record: record.digest().expect("record digest"),
        };
        assert_eq!(
            SemanticHead::decode(&head.encode().expect("head bytes")).expect("head"),
            head
        );
    }
}
