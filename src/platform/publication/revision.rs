//! Immutable Graph 5 revisions, exact witness bindings, and atomic HEAD records.

use super::contract::{
    HEAD_ENVELOPE_DOMAIN, HEAD_MAGIC, MAXIMUM_HEAD_BYTES, MAXIMUM_REVISION_BYTES,
    REVISION_CONTRACT_VERSION, REVISION_ENVELOPE_DOMAIN, REVISION_IDENTITY_DIGEST_DOMAIN,
    REVISION_MAGIC,
};
use super::digest::{
    ReceiptObjectDigest, RevisionObjectDigest, SemanticDiffDigest, TransactionDigest,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::SemanticRootDigest;
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use crate::platform::witness::{
    ValidationCertificateDigest, ValidationWitnessDigest, ValidationWitnessManifest,
    ValidatorContractDigest,
};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(deny_unknown_fields)]
pub struct ParentRevision {
    pub revision: RevisionId,
    pub record: RevisionObjectDigest,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionCore {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub witness_contract_version: u16,
    pub repository_id: RepositoryId,
    pub parents: Vec<ParentRevision>,
    pub semantic_root: SemanticRootDigest,
    pub validation_witness: ValidationWitnessDigest,
    pub validation_certificate: ValidationCertificateDigest,
    pub validator_contract: ValidatorContractDigest,
    pub transaction: TransactionDigest,
    pub semantic_diff: SemanticDiffDigest,
}

impl RevisionCore {
    pub fn revision_id(&self) -> Result<RevisionId, Diagnostic> {
        self.validate()?;
        let configuration = bincode::config::standard()
            .with_little_endian()
            .with_variable_int_encoding()
            .with_limit::<{ crate::platform::packed::MAXIMUM_PACKED_PAYLOAD_BYTES }>();
        let bytes = bincode::encode_to_vec(self, configuration).map_err(|error| {
            revision_error(
                DiagnosticClass::Infrastructure,
                "publication_revision_identity_encode",
                format!("revision identity encoding failed: {error}"),
            )
        })?;
        if bytes.len() > MAXIMUM_REVISION_BYTES {
            return Err(revision_error(
                DiagnosticClass::Resource,
                "publication_revision_identity_size",
                "revision core exceeds its hostile decoder bound",
            ));
        }
        let mut hasher = blake3::Hasher::new_derive_key(REVISION_IDENTITY_DIGEST_DOMAIN);
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
        Ok(RevisionId::from_digest(*hasher.finalize().as_bytes()))
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != REVISION_CONTRACT_VERSION
            || self.graph_contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
            || self.witness_contract_version
                != crate::platform::witness::contract::WITNESS_CONTRACT_VERSION
        {
            return Err(revision_error(
                DiagnosticClass::Source,
                "publication_revision_contract",
                "revision uses a predecessor or foreign graph, witness, or history contract",
            ));
        }
        if self.parents.len() > 2
            || self
                .parents
                .windows(2)
                .any(|pair| pair[0].revision >= pair[1].revision)
        {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "publication_revision_parents",
                "revision parents must be zero, one, or two unique canonical revision identities",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionRecord {
    pub revision: RevisionId,
    pub core: RevisionCore,
    pub receipt: ReceiptObjectDigest,
}

impl RevisionRecord {
    pub fn new(core: RevisionCore, receipt: ReceiptObjectDigest) -> Result<Self, Diagnostic> {
        let revision = core.revision_id()?;
        Ok(Self {
            revision,
            core,
            receipt,
        })
    }

    pub fn encode(&self) -> Result<(RevisionObjectDigest, Vec<u8>), Diagnostic> {
        self.validate()?;
        let bytes = crate::platform::packed::encode(
            REVISION_MAGIC,
            REVISION_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_REVISION_BYTES,
        )?;
        Ok((RevisionObjectDigest::of(&bytes), bytes))
    }

    pub fn decode(bytes: &[u8], expected: RevisionObjectDigest) -> Result<Self, Diagnostic> {
        if RevisionObjectDigest::of(bytes) != expected {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "publication_revision_digest",
                "revision bytes disagree with their object digest",
            ));
        }
        let value: Self = crate::platform::packed::decode(
            bytes,
            REVISION_MAGIC,
            REVISION_ENVELOPE_DOMAIN,
            MAXIMUM_REVISION_BYTES,
        )?;
        value.validate()?;
        if value.encode()?.1 != bytes {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "publication_revision_canonical",
                "revision record is not canonically encoded",
            ));
        }
        Ok(value)
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.revision != self.core.revision_id()? {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "publication_revision_identity",
                "revision identity disagrees with its exact core",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HeadRecord {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub repository_id: RepositoryId,
    pub revision: RevisionId,
    pub record: RevisionObjectDigest,
}

impl HeadRecord {
    pub fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        crate::platform::packed::encode(HEAD_MAGIC, HEAD_ENVELOPE_DOMAIN, self, MAXIMUM_HEAD_BYTES)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self = crate::platform::packed::decode(
            bytes,
            HEAD_MAGIC,
            HEAD_ENVELOPE_DOMAIN,
            MAXIMUM_HEAD_BYTES,
        )?;
        value.validate()?;
        if value.encode()? != bytes {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "publication_head_canonical",
                "HEAD is not canonically encoded",
            ));
        }
        Ok(value)
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != REVISION_CONTRACT_VERSION
            || self.graph_contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
        {
            return Err(revision_error(
                DiagnosticClass::Source,
                "publication_head_contract",
                "HEAD uses a predecessor or foreign contract",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcceptedBinding {
    pub head: HeadRecord,
    pub semantic_root: SemanticRootDigest,
    pub validation_witness: ValidationWitnessDigest,
    pub validation_certificate: ValidationCertificateDigest,
    pub validator_contract: ValidatorContractDigest,
}

impl AcceptedBinding {
    pub fn verify(
        head: HeadRecord,
        record: &RevisionRecord,
        witness_digest: ValidationWitnessDigest,
        witness: &ValidationWitnessManifest,
    ) -> Result<Self, Diagnostic> {
        head.validate()?;
        let (record_digest, _) = record.encode()?;
        if head.repository_id != record.core.repository_id
            || head.revision != record.revision
            || head.record != record_digest
            || record.core.validation_witness != witness_digest
            || record.core.semantic_root != witness.semantic_root
            || record.core.validation_certificate != witness.certificate
            || record.core.validator_contract != witness.validator_contract
            || record.core.repository_id != witness.repository_id
        {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "publication_current_binding",
                "HEAD, revision, semantic root, and validation witness do not form one exact binding",
            ));
        }
        if !witness.contract_is_current() {
            return Err(revision_error(
                DiagnosticClass::Source,
                "publication_current_witness_contract",
                "accepted witness uses a predecessor or foreign contract",
            ));
        }
        Ok(Self {
            head,
            semantic_root: record.core.semantic_root,
            validation_witness: witness_digest,
            validation_certificate: witness.certificate,
            validator_contract: witness.validator_contract,
        })
    }

    pub const fn parent(self) -> ParentRevision {
        ParentRevision {
            revision: self.head.revision,
            record: self.head.record,
        }
    }
}

fn revision_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
