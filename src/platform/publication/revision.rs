//! Immutable Graph 8 revisions, separate acceptance bindings, and atomic HEAD records.

use super::contract::{
    HEAD_ENVELOPE_DOMAIN, HEAD_MAGIC, MAXIMUM_HEAD_BYTES, MAXIMUM_REVISION_BYTES,
    REVISION_CONTRACT_VERSION, REVISION_ENVELOPE_DOMAIN, REVISION_IDENTITY_DIGEST_DOMAIN,
    REVISION_MAGIC,
};
use super::digest::{
    ReceiptObjectDigest, RevisionObjectDigest, SemanticDiffDigest, TransactionDigest,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{SemanticRootDigest, SemanticStateDigest};
use crate::platform::persistent_map::MapRoot;
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use crate::platform::witness::{
    ValidationCertificateDigest, ValidationWitnessDigest, ValidationWitnessManifest,
    ValidatorContractDigest,
};
use bincode::de::{BorrowDecoder, Decoder};
use bincode::error::DecodeError;
use bincode::{BorrowDecode, Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(deny_unknown_fields)]
pub struct ParentRevision {
    pub revision: RevisionId,
    pub record: RevisionObjectDigest,
}

#[derive(Clone, Debug, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionCore {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub repository_id: RepositoryId,
    pub parents: Vec<RevisionId>,
    pub semantic_state: SemanticStateDigest,
}

impl<Context> Decode<Context> for RevisionCore {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let contract_version = u16::decode(decoder)?;
        let graph_contract_version = u16::decode(decoder)?;
        let repository_id = RepositoryId::decode(decoder)?;
        let encoded_length = u64::decode(decoder)?;
        let parent_count = usize::try_from(encoded_length)
            .map_err(|_| DecodeError::OutsideUsizeRange(encoded_length))?;
        if parent_count > 2 {
            return Err(DecodeError::OtherString(
                "revision parent count exceeds 2 before allocation".to_owned(),
            ));
        }
        decoder.claim_container_read::<RevisionId>(parent_count)?;
        let mut parents = Vec::with_capacity(parent_count);
        for _ in 0..parent_count {
            decoder.unclaim_bytes_read(std::mem::size_of::<RevisionId>());
            parents.push(RevisionId::decode(decoder)?);
        }
        let semantic_state = SemanticStateDigest::decode(decoder)?;
        Ok(Self {
            contract_version,
            graph_contract_version,
            repository_id,
            parents,
            semantic_state,
        })
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for RevisionCore {
    fn borrow_decode<D: BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Self::decode(decoder)
    }
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
        {
            return Err(revision_error(
                DiagnosticClass::Source,
                "publication_revision_contract",
                "revision uses a predecessor or foreign graph or history contract",
            ));
        }
        if self.parents.len() > 2 || self.parents.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "publication_revision_parents",
                "revision parents must be zero, one, or two unique canonical revision identities",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationEvidenceBinding {
    pub semantic_state: SemanticStateDigest,
    pub witness_contract_version: u16,
    pub witness: ValidationWitnessDigest,
    pub certificate: ValidationCertificateDigest,
    pub validator_contract: ValidatorContractDigest,
}

impl ValidationEvidenceBinding {
    fn verify(
        self,
        core: &RevisionCore,
        semantic_root: SemanticRootDigest,
        witness_digest: ValidationWitnessDigest,
        witness: &ValidationWitnessManifest,
    ) -> Result<(), Diagnostic> {
        if self.semantic_state != core.semantic_state
            || self.witness_contract_version != witness.contract_version
            || self.witness != witness_digest
            || self.certificate != witness.certificate
            || self.validator_contract != witness.validator_contract
            || semantic_root != witness.semantic_root
            || core.repository_id != witness.repository_id
        {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "publication_validation_evidence_binding",
                "revision meaning and validation evidence do not form one exact acceptance binding",
            ));
        }
        if !witness.contract_is_current() {
            return Err(revision_error(
                DiagnosticClass::Source,
                "publication_current_witness_contract",
                "accepted witness uses a predecessor or foreign contract",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationBinding {
    pub parents: Vec<ParentRevision>,
    pub semantic_root: SemanticRootDigest,
    pub validation: ValidationEvidenceBinding,
    pub idempotency_receipts: MapRoot,
    pub transaction: TransactionDigest,
    pub semantic_diff: SemanticDiffDigest,
    pub receipt: ReceiptObjectDigest,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionRecord {
    pub revision: RevisionId,
    pub core: RevisionCore,
    pub publication: PublicationBinding,
}

impl RevisionRecord {
    pub fn new(core: RevisionCore, publication: PublicationBinding) -> Result<Self, Diagnostic> {
        let revision = core.revision_id()?;
        let record = Self {
            revision,
            core,
            publication,
        };
        record.validate()?;
        Ok(record)
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
        if self.publication.parents.len() != self.core.parents.len()
            || self
                .publication
                .parents
                .iter()
                .zip(&self.core.parents)
                .any(|(binding, revision)| binding.revision != *revision)
        {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "publication_revision_parent_binding",
                "publication parent records do not bind the revision's exact semantic parents",
            ));
        }
        if self.publication.validation.semantic_state != self.core.semantic_state {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "publication_revision_state_binding",
                "publication evidence does not bind the revision's exact semantic state",
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
    pub semantic_state: SemanticStateDigest,
    pub semantic_root: SemanticRootDigest,
    pub validation_witness: ValidationWitnessDigest,
    pub validation_certificate: ValidationCertificateDigest,
    pub validator_contract: ValidatorContractDigest,
    pub idempotency_receipts: MapRoot,
    pub receipt: ReceiptObjectDigest,
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
        {
            return Err(revision_error(
                DiagnosticClass::Corrupt,
                "publication_current_binding",
                "HEAD and the exact publication record do not form one current binding",
            ));
        }
        record.publication.validation.verify(
            &record.core,
            record.publication.semantic_root,
            witness_digest,
            witness,
        )?;
        Ok(Self {
            head,
            semantic_state: record.core.semantic_state,
            semantic_root: record.publication.semantic_root,
            validation_witness: witness_digest,
            validation_certificate: witness.certificate,
            validator_contract: witness.validator_contract,
            idempotency_receipts: record.publication.idempotency_receipts,
            receipt: record.publication.receipt,
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

#[cfg(test)]
mod tests {
    use super::*;

    struct ExcessParents;

    impl Encode for ExcessParents {
        fn encode<E: bincode::enc::Encoder>(
            &self,
            encoder: &mut E,
        ) -> Result<(), bincode::error::EncodeError> {
            3_u64.encode(encoder)
        }
    }

    #[derive(Encode)]
    struct OversizedRevisionCore {
        contract_version: u16,
        graph_contract_version: u16,
        repository_id: RepositoryId,
        parents: ExcessParents,
        semantic_state: SemanticStateDigest,
    }

    #[test]
    fn revision_core_rejects_parent_length_before_allocation() {
        let encoded = bincode::encode_to_vec(
            OversizedRevisionCore {
                contract_version: REVISION_CONTRACT_VERSION,
                graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                repository_id: RepositoryId::migrate(b"oversized-revision-parents", 0),
                parents: ExcessParents,
                semantic_state: SemanticStateDigest::from_bytes([0x91; 32]),
            },
            bincode::config::standard()
                .with_little_endian()
                .with_variable_int_encoding(),
        )
        .expect("encode hostile parent length without allocating parents");
        let error = bincode::decode_from_slice::<RevisionCore, _>(
            &encoded,
            bincode::config::standard()
                .with_little_endian()
                .with_variable_int_encoding(),
        )
        .expect_err("hostile parent length must reject before allocation");
        assert!(error.to_string().contains("before allocation"));
    }
}
