//! Bounded, newest-first projection of the existing immutable publication chain.
use super::contract::{MAXIMUM_RECEIPT_BYTES, MAXIMUM_REVISION_BYTES};
use super::{HeadRecord, ParentRevision, PublicationReceipt, RevisionObjectDigest, RevisionRecord};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::ExecutionControl;
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StoreReadAdmission, StoreReadLimits, StoreWork,
};
use std::collections::BTreeSet;

pub const DEFAULT_HISTORY_ITEMS: usize = 20;
pub const MAXIMUM_HISTORY_ITEMS: usize = 100;
pub const MAXIMUM_HISTORY_STORE_OBJECTS: u64 = 2 * MAXIMUM_HISTORY_ITEMS as u64;
pub const MAXIMUM_HISTORY_STORE_BYTES: u64 = 8 * 1_048_576;

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub record: RevisionObjectDigest,
    pub revision: RevisionRecord,
    pub receipt: PublicationReceipt,
}

#[derive(Clone, Debug)]
pub struct RepositoryHistory {
    pub head: HeadRecord,
    pub entries: Vec<HistoryEntry>,
    /// A committed but unread parent, not an independently admitted continuation.
    pub next: Option<ParentRevision>,
    pub work: StoreWork,
}

pub(super) const fn history_read_limits() -> StoreReadLimits {
    StoreReadLimits {
        maximum_catalog_lookups: MAXIMUM_HISTORY_STORE_OBJECTS,
        maximum_objects: MAXIMUM_HISTORY_STORE_OBJECTS,
        maximum_bytes: MAXIMUM_HISTORY_STORE_BYTES,
    }
}

pub(super) fn validate_limit(limit: usize) -> Result<(), Diagnostic> {
    if !(1..=MAXIMUM_HISTORY_ITEMS).contains(&limit) {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "publication_history_limit",
            format!("history limit must be 1 through {MAXIMUM_HISTORY_ITEMS}"),
        ));
    }
    Ok(())
}

pub(super) fn checkpoint(control: &ExecutionControl) -> Result<(), Diagnostic> {
    control
        .check()
        .map_err(|error| Diagnostic::new(DiagnosticClass::Cancelled, error.code, error.message))
}

/// The caller captures HEAD and this append-only store snapshot under one repository lock.
/// Only record/receipt bindings are inspected: historical claims are not current validation.
pub(super) fn read_history(
    store: &impl ImmutableObjectStore,
    head: HeadRecord,
    limit: usize,
    limits: StoreReadLimits,
    control: &ExecutionControl,
) -> Result<RepositoryHistory, Diagnostic> {
    validate_limit(limit)?;
    checkpoint(control)?;
    let _ = head.encode()?;
    let mut result = RepositoryHistory {
        head,
        entries: Vec::new(),
        next: Some(ParentRevision {
            revision: head.revision,
            record: head.record,
        }),
        work: StoreWork::default(),
    };
    let mut admission = StoreReadAdmission::new(limits);
    let mut seen = BTreeSet::new();
    while result.entries.len() < limit {
        let Some(link) = result.next.take() else {
            break;
        };
        checkpoint(control)?;
        if !seen.insert(link.revision) {
            return Err(corrupt(
                "publication_history_cycle",
                "history repeats a revision",
            ));
        }
        let bytes = read_object(
            store,
            ObjectDomain::Revision,
            link.record.bytes(),
            MAXIMUM_REVISION_BYTES,
            &mut admission,
            &mut result.work,
        )?;
        checkpoint(control)?;
        let revision = RevisionRecord::decode(&bytes, link.record)?;
        if revision.revision != link.revision
            || revision.core.repository_id != head.repository_id
            || (result.entries.is_empty()
                && revision.core.graph_contract_version != head.graph_contract_version)
        {
            return Err(corrupt(
                "publication_history_binding",
                "history revision does not match its exact parent or HEAD binding",
            ));
        }
        if revision.publication.parents.len() > 1 {
            return Err(Diagnostic::new(
                DiagnosticClass::Source,
                "publication_history_merge",
                "two-parent history is reserved; this projection does not silently select a parent",
            ));
        }
        let bytes = read_object(
            store,
            ObjectDomain::Receipt,
            revision.publication.receipt.bytes(),
            MAXIMUM_RECEIPT_BYTES,
            &mut admission,
            &mut result.work,
        )?;
        checkpoint(control)?;
        let receipt = PublicationReceipt::decode(&bytes, revision.publication.receipt)?;
        if receipt.graph_contract_version != revision.core.graph_contract_version
            || receipt.repository_id != head.repository_id
            || receipt.result != revision.revision
            || receipt.bases != revision.core.parents
            || receipt.transaction != revision.publication.transaction
            || receipt.semantic_diff != revision.publication.semantic_diff
        {
            return Err(corrupt(
                "publication_history_binding",
                "history receipt and revision do not form one exact recorded acceptance",
            ));
        }
        result.next = revision.publication.parents.first().copied();
        result.entries.push(HistoryEntry {
            record: link.record,
            revision,
            receipt,
        });
    }
    checkpoint(control)?;
    Ok(result)
}

fn read_object(
    store: &impl ImmutableObjectStore,
    domain: ObjectDomain,
    digest: [u8; 32],
    maximum_bytes: usize,
    admission: &mut StoreReadAdmission,
    work: &mut StoreWork,
) -> Result<Vec<u8>, Diagnostic> {
    store
        .read_admitted(
            ObjectKey::from_digest(domain, digest),
            maximum_bytes,
            admission,
            work,
        )
        .map_err(super::repository::store_diagnostic)?
        .ok_or_else(|| {
            corrupt(
                "publication_history_missing",
                format!("history references a missing {} object", domain.name()),
            )
        })
}

fn corrupt(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}

#[cfg(test)]
mod tests;
