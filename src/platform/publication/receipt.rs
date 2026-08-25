//! Compact, honest evidence for one accepted semantic publication.

use super::contract::{
    MAXIMUM_INTENT_BYTES, MAXIMUM_RECEIPT_BYTES, RECEIPT_CONTRACT_VERSION, RECEIPT_ENVELOPE_DOMAIN,
    RECEIPT_MAGIC,
};
use super::digest::{ReceiptObjectDigest, SemanticDiffDigest, TransactionDigest};
use super::idempotency::idempotency_key_is_valid;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use bincode::{Decode, Encode};

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub enum PublicationStatus {
    AcceptedChange,
    ProjectCreated,
    MergeAccepted,
}

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub enum ValidationProfile {
    FullRebuild,
    #[default]
    IncrementalOwnerFrontier,
}

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub enum FullOracleStatus {
    NotApplicable,
    #[default]
    NotRun,
    Equal,
}

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct ChangeCounts {
    pub owners_created: u64,
    pub owners_updated: u64,
    pub owners_deleted: u64,
    pub type_objects_added: u64,
    pub dependencies_changed: u64,
    pub retirements_changed: u64,
    pub witness_entries_changed: u64,
}

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct ValidationEvidence {
    pub profile: ValidationProfile,
    pub structurally_checked: u64,
    pub semantically_checked: u64,
    pub summaries_reused: u64,
    pub reverse_edges_visited: u64,
    pub tests_selected: u64,
    pub tests_executed: u64,
    pub tests_passed: u64,
    pub compiler_units_planned: u64,
    pub full_oracle: FullOracleStatus,
}

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct WorkObservation {
    /// Exact multidimensional change-meter observation before this receipt and its enclosing
    /// revision are encoded. Bootstrap receipts retain the all-zero observation.
    pub budget: crate::platform::change::ChangeBudgetWork,
    /// Complete validator work when its implementation exposes one aggregate count.
    pub validation_work: u64,
    pub map_pages_read: u64,
    pub map_pages_written: u64,
    pub objects_read: u64,
    /// New authority, transaction, diff, and history-index objects staged before the receipt and
    /// revision are encoded. Including either enclosing object would make this observation
    /// self-referential.
    pub objects_staged: u64,
    /// Newly staged immutable persistent-map pages included by `objects_staged`.
    pub pages_staged: u64,
    pub owner_records_checked: u64,
    pub ownership_entries_checked: u64,
    pub type_objects_checked: u64,
    pub expression_work: u64,
    pub relation_edges_visited: u64,
    pub bytes_read: u64,
    /// Bytes corresponding to `objects_staged`, with the same deliberate enclosure exclusion.
    pub bytes_staged: u64,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PublicationReceipt {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub repository_id: RepositoryId,
    pub status: PublicationStatus,
    pub bases: Vec<RevisionId>,
    pub result: RevisionId,
    pub transaction: TransactionDigest,
    pub semantic_diff: SemanticDiffDigest,
    pub idempotency_key: Option<String>,
    pub counts: ChangeCounts,
    pub validation: ValidationEvidence,
    pub work: WorkObservation,
    pub intent: Option<String>,
}

impl PublicationReceipt {
    pub fn encode(&self) -> Result<(ReceiptObjectDigest, Vec<u8>), Diagnostic> {
        self.validate()?;
        let bytes = crate::platform::packed::encode(
            RECEIPT_MAGIC,
            RECEIPT_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_RECEIPT_BYTES,
        )?;
        Ok((ReceiptObjectDigest::of(&bytes), bytes))
    }

    pub fn decode(bytes: &[u8], expected: ReceiptObjectDigest) -> Result<Self, Diagnostic> {
        if ReceiptObjectDigest::of(bytes) != expected {
            return Err(receipt_error(
                DiagnosticClass::Corrupt,
                "publication_receipt_digest",
                "receipt bytes disagree with their object digest",
            ));
        }
        let value: Self = crate::platform::packed::decode(
            bytes,
            RECEIPT_MAGIC,
            RECEIPT_ENVELOPE_DOMAIN,
            MAXIMUM_RECEIPT_BYTES,
        )?;
        value.validate()?;
        if value.encode()?.1 != bytes {
            return Err(receipt_error(
                DiagnosticClass::Corrupt,
                "publication_receipt_canonical",
                "receipt object is not canonically encoded",
            ));
        }
        Ok(value)
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != RECEIPT_CONTRACT_VERSION
            || self.graph_contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
        {
            return Err(receipt_error(
                DiagnosticClass::Source,
                "publication_receipt_contract",
                "receipt uses a predecessor or foreign contract",
            ));
        }
        if self.bases.len() > 2
            || self.bases.windows(2).any(|pair| pair[0] >= pair[1])
            || self.bases.contains(&self.result)
        {
            return Err(receipt_error(
                DiagnosticClass::Corrupt,
                "publication_receipt_bases",
                "receipt bases are not zero, one, or two canonical prior revisions",
            ));
        }
        let expected_bases = match self.status {
            PublicationStatus::ProjectCreated => 0,
            PublicationStatus::AcceptedChange => 1,
            PublicationStatus::MergeAccepted => 2,
        };
        if self.bases.len() != expected_bases {
            return Err(receipt_error(
                DiagnosticClass::Corrupt,
                "publication_receipt_status_bases",
                "receipt status disagrees with its accepted base count",
            ));
        }
        if let Some(key) = &self.idempotency_key
            && (self.bases.len() != 1 || !idempotency_key_is_valid(key))
        {
            return Err(receipt_error(
                DiagnosticClass::Source,
                "publication_receipt_idempotency",
                "idempotency requires one exact base and 1 through 128 portable identifier bytes",
            ));
        }
        if self
            .intent
            .as_ref()
            .is_some_and(|intent| intent.len() > MAXIMUM_INTENT_BYTES)
        {
            return Err(receipt_error(
                DiagnosticClass::Resource,
                "publication_receipt_intent",
                "bounded nonsemantic intent exceeds the current byte limit",
            ));
        }
        if self.validation.tests_passed > self.validation.tests_executed
            || self.validation.tests_executed > self.validation.tests_selected
        {
            return Err(receipt_error(
                DiagnosticClass::Corrupt,
                "publication_receipt_tests",
                "test evidence counts are inconsistent",
            ));
        }
        let profile_is_consistent = match self.validation.profile {
            ValidationProfile::FullRebuild => {
                self.validation.full_oracle == FullOracleStatus::NotApplicable
                    && self.validation.structurally_checked == self.validation.semantically_checked
                    && self.validation.summaries_reused == 0
                    && self.validation.reverse_edges_visited == 0
            }
            ValidationProfile::IncrementalOwnerFrontier => {
                self.validation.full_oracle != FullOracleStatus::NotApplicable
            }
        };
        if !profile_is_consistent
            || (self.status == PublicationStatus::ProjectCreated
                && self.validation.profile != ValidationProfile::FullRebuild)
        {
            return Err(receipt_error(
                DiagnosticClass::Corrupt,
                "publication_receipt_validation_profile",
                "validation evidence is inconsistent with its profile or publication status",
            ));
        }
        Ok(())
    }
}

fn receipt_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
