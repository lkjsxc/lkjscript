#[path = "edge_tests.rs"]
mod edges;

use super::*;
use crate::platform::project_creation::{ProjectTemplate, create_project};
use crate::platform::publication::{GraphRepository, PublicationStatus, TransactionDigest};
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use crate::platform::storage::memory::MemoryPackedStore;

fn fixture() -> (tempfile::TempDir, GraphRepository) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("project");
    create_project(&path, "history", ProjectTemplate::Minimal).unwrap();
    let repository = GraphRepository::open(&path).unwrap();
    (root, repository)
}

fn stage(
    revision: &RevisionRecord,
    receipt: Option<&PublicationReceipt>,
    mut head: HeadRecord,
) -> (MemoryPackedStore, HeadRecord) {
    let mut store = MemoryPackedStore::default();
    let mut work = StoreWork::default();
    let (record, bytes) = revision.encode().unwrap();
    store
        .stage(
            ObjectKey::from_digest(ObjectDomain::Revision, record.bytes()),
            &bytes,
            &mut work,
        )
        .unwrap();
    if let Some(receipt) = receipt {
        let (digest, bytes) = receipt.encode().unwrap();
        store
            .stage(
                ObjectKey::from_digest(ObjectDomain::Receipt, digest.bytes()),
                &bytes,
                &mut work,
            )
            .unwrap();
    }
    head.record = record;
    (store, head)
}

fn read(
    store: &impl ImmutableObjectStore,
    head: HeadRecord,
    limit: usize,
) -> Result<RepositoryHistory, Diagnostic> {
    read_history(
        store,
        head,
        limit,
        history_read_limits(),
        &ExecutionControl::uncancelled(),
    )
}

#[test]
fn history_reads_recorded_creation_and_never_requires_an_executable_graph() {
    let (_root, repository) = fixture();
    let current = repository.current().unwrap();
    let before = std::fs::read(repository.root().join("HEAD")).unwrap();
    let result = repository.history(100).unwrap();
    assert_eq!(result.head, current.head);
    assert_eq!(result.entries.len(), 1);
    assert!(result.next.is_none());
    assert_eq!(result.entries[0].receipt, current.receipt);
    assert_eq!(result.work.objects_read, 2);
    assert_eq!(
        std::fs::read(repository.root().join("HEAD")).unwrap(),
        before
    );
    // This disjoint store deliberately has no witness, semantic root, code or test objects.
    let (store, head) = stage(&current.revision, Some(&current.receipt), current.head);
    assert_eq!(
        read(&store, head, 1).unwrap().entries[0].receipt,
        current.receipt
    );
}

#[test]
fn history_rejects_missing_receipts_wrong_head_and_individually_valid_unbound_receipts() {
    let (_root, repository) = fixture();
    let current = repository.current().unwrap();
    let (store, head) = stage(&current.revision, None, current.head);
    assert_eq!(
        read(&store, head, 1).unwrap_err().code,
        "publication_history_missing"
    );
    let (store, head) = stage(&current.revision, Some(&current.receipt), current.head);
    for wrong in [
        HeadRecord {
            revision: RevisionId::from_digest([42; 32]),
            ..head
        },
        HeadRecord {
            repository_id: RepositoryId::migrate(b"foreign", 0),
            ..head
        },
    ] {
        assert_eq!(
            read(&store, wrong, 1).unwrap_err().code,
            "publication_history_binding"
        );
    }
    for fault in 0..4 {
        let mut receipt = current.receipt.clone();
        match fault {
            0 => receipt.repository_id = RepositoryId::migrate(b"foreign-receipt", 0),
            1 => receipt.result = RevisionId::from_digest([43; 32]),
            2 => receipt.transaction = TransactionDigest::from_bytes([44; 32]),
            _ => receipt.semantic_diff = super::super::SemanticDiffDigest::from_bytes([45; 32]),
        }
        let mut revision = current.revision.clone();
        revision.publication.receipt = receipt.encode().unwrap().0;
        let (store, head) = stage(&revision, Some(&receipt), current.head);
        assert_eq!(
            read(&store, head, 1).unwrap_err().code,
            "publication_history_binding"
        );
    }
}

#[test]
fn history_truncation_does_not_admit_an_unread_parent_or_hide_a_requested_missing_link() {
    let (_root, repository) = fixture();
    let current = repository.current().unwrap();
    let mut revision = current.revision.clone();
    let parent = ParentRevision {
        revision: RevisionId::from_digest([46; 32]),
        record: RevisionObjectDigest::from_bytes([47; 32]),
    };
    revision.core.parents = vec![parent.revision];
    revision.revision = revision.core.revision_id().unwrap();
    revision.publication.parents = vec![parent];
    let mut receipt = current.receipt.clone();
    receipt.status = PublicationStatus::AcceptedChange;
    receipt.bases = vec![parent.revision];
    receipt.result = revision.revision;
    revision.publication.receipt = receipt.encode().unwrap().0;
    let (store, head) = stage(
        &revision,
        Some(&receipt),
        HeadRecord {
            revision: revision.revision,
            ..current.head
        },
    );
    let result = read(&store, head, 1).unwrap();
    assert_eq!(result.next, Some(parent));
    assert_eq!(result.work.objects_read, 2);
    assert_eq!(
        read(&store, head, 2).unwrap_err().code,
        "publication_history_missing"
    );
}

#[test]
fn history_shares_preconsumption_admission_and_honors_cancellation() {
    use crate::platform::storage::object::StoreError;
    use std::cell::Cell;
    struct Meter<'a> {
        store: &'a MemoryPackedStore,
        consumed: Cell<u64>,
        cancel: Option<ExecutionControl>,
    }
    impl ImmutableObjectStore for Meter<'_> {
        fn stage(
            &mut self,
            _: ObjectKey,
            _: &[u8],
            _: &mut StoreWork,
        ) -> Result<crate::platform::storage::object::StageOutcome, StoreError> {
            panic!("history must never stage objects")
        }
        fn read(
            &self,
            _: ObjectKey,
            _: usize,
            _: &mut StoreWork,
        ) -> Result<Option<Vec<u8>>, StoreError> {
            panic!("history must use admitted reads")
        }
        fn read_admitted(
            &self,
            key: ObjectKey,
            maximum: usize,
            admission: &mut StoreReadAdmission,
            work: &mut StoreWork,
        ) -> Result<Option<Vec<u8>>, StoreError> {
            let result = self.store.read_admitted(key, maximum, admission, work)?;
            self.consumed.set(work.objects_read);
            if let Some(control) = &self.cancel {
                control.cancel();
            }
            Ok(result)
        }
    }
    let (_root, repository) = fixture();
    let current = repository.current().unwrap();
    let (store, head) = stage(&current.revision, Some(&current.receipt), current.head);
    for (limits, consumed, code) in [
        (
            StoreReadLimits {
                maximum_catalog_lookups: 0,
                ..history_read_limits()
            },
            0,
            "object_read_catalog_lookups_exhausted",
        ),
        (
            StoreReadLimits {
                maximum_objects: 1,
                ..history_read_limits()
            },
            1,
            "object_read_objects_exhausted",
        ),
        (
            StoreReadLimits {
                maximum_bytes: current.revision.encode().unwrap().1.len() as u64,
                ..history_read_limits()
            },
            1,
            "object_read_bytes_exhausted",
        ),
    ] {
        let meter = Meter {
            store: &store,
            consumed: Cell::new(0),
            cancel: None,
        };
        assert_eq!(
            read_history(&meter, head, 1, limits, &ExecutionControl::uncancelled())
                .unwrap_err()
                .code,
            code
        );
        assert_eq!(meter.consumed.get(), consumed);
    }
    for limit in [0, 101, usize::MAX] {
        let meter = Meter {
            store: &store,
            consumed: Cell::new(0),
            cancel: None,
        };
        assert_eq!(
            read(&meter, head, limit).unwrap_err().code,
            "publication_history_limit"
        );
        assert_eq!(meter.consumed.get(), 0);
    }
    let control = ExecutionControl::uncancelled();
    let meter = Meter {
        store: &store,
        consumed: Cell::new(0),
        cancel: Some(control.clone()),
    };
    assert_eq!(
        read_history(&meter, head, 1, history_read_limits(), &control)
            .unwrap_err()
            .class,
        DiagnosticClass::Cancelled
    );
    assert_eq!(meter.consumed.get(), 1);
    assert_eq!(
        repository
            .history_with_control(1, &control)
            .unwrap_err()
            .class,
        DiagnosticClass::Cancelled
    );
}
