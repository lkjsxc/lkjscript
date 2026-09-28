//! Independently constructed history edges and limits spanning multiple entries.
use super::*;
use crate::platform::publication::CurrentPublication;

fn child(
    current: &CurrentPublication,
    parents: Vec<ParentRevision>,
) -> (RevisionRecord, PublicationReceipt) {
    let mut revision = current.revision.clone();
    revision.core.parents = parents.iter().map(|parent| parent.revision).collect();
    revision.revision = revision.core.revision_id().unwrap();
    revision.publication.parents = parents;
    let mut receipt = current.receipt.clone();
    receipt.status = if revision.core.parents.len() == 2 {
        PublicationStatus::MergeAccepted
    } else {
        PublicationStatus::AcceptedChange
    };
    receipt.bases = revision.core.parents.clone();
    receipt.result = revision.revision;
    revision.publication.receipt = receipt.encode().unwrap().0;
    (revision, receipt)
}

fn append_current(store: &mut MemoryPackedStore, current: &CurrentPublication) {
    let (record, revision) = current.revision.encode().unwrap();
    let (receipt, bytes) = current.receipt.encode().unwrap();
    for (key, value) in [
        (
            ObjectKey::from_digest(ObjectDomain::Revision, record.bytes()),
            revision,
        ),
        (
            ObjectKey::from_digest(ObjectDomain::Receipt, receipt.bytes()),
            bytes,
        ),
    ] {
        store.stage(key, &value, &mut StoreWork::default()).unwrap();
    }
}

#[test]
fn history_admission_is_cumulative_across_entries_not_just_within_one_pair() {
    let (_root, repository) = fixture();
    let current = repository.current().unwrap();
    let (revision, receipt) = child(
        &current,
        vec![ParentRevision {
            revision: current.head.revision,
            record: current.head.record,
        }],
    );
    let (mut store, head) = stage(
        &revision,
        Some(&receipt),
        HeadRecord {
            revision: revision.revision,
            ..current.head
        },
    );
    append_current(&mut store, &current);
    let complete = read(&store, head, 2).unwrap();
    assert_eq!(complete.entries.len(), 2);
    assert_eq!(complete.work.objects_read, 4);
    assert!(complete.next.is_none());
    for (limits, code) in [
        (
            StoreReadLimits {
                maximum_catalog_lookups: 3,
                ..history_read_limits()
            },
            "object_read_catalog_lookups_exhausted",
        ),
        (
            StoreReadLimits {
                maximum_objects: 3,
                ..history_read_limits()
            },
            "object_read_objects_exhausted",
        ),
        (
            StoreReadLimits {
                maximum_bytes: complete.work.bytes_read - 1,
                ..history_read_limits()
            },
            "object_read_bytes_exhausted",
        ),
    ] {
        assert_eq!(
            read_history(&store, head, 2, limits, &ExecutionControl::uncancelled())
                .unwrap_err()
                .code,
            code
        );
    }
}

#[test]
fn history_valid_but_wrong_older_record_rejects_the_entire_requested_prefix() {
    let (_root, repository) = fixture();
    let current = repository.current().unwrap();
    // The record exists and decodes, but is not the semantic parent that the child names.
    let wrong_parent = ParentRevision {
        revision: RevisionId::from_digest([46; 32]),
        record: current.head.record,
    };
    let (revision, receipt) = child(&current, vec![wrong_parent]);
    let (mut store, head) = stage(
        &revision,
        Some(&receipt),
        HeadRecord {
            revision: revision.revision,
            ..current.head
        },
    );
    append_current(&mut store, &current);
    assert_eq!(read(&store, head, 1).unwrap().next, Some(wrong_parent));
    assert_eq!(
        read(&store, head, 2).unwrap_err().code,
        "publication_history_binding"
    );
}

#[test]
fn history_two_parent_record_is_reserved_even_when_its_receipt_is_well_formed() {
    let (_root, repository) = fixture();
    let current = repository.current().unwrap();
    let (revision, receipt) = child(
        &current,
        vec![
            ParentRevision {
                revision: RevisionId::from_digest([46; 32]),
                record: RevisionObjectDigest::from_bytes([47; 32]),
            },
            ParentRevision {
                revision: RevisionId::from_digest([48; 32]),
                record: RevisionObjectDigest::from_bytes([49; 32]),
            },
        ],
    );
    let (store, head) = stage(
        &revision,
        Some(&receipt),
        HeadRecord {
            revision: revision.revision,
            ..current.head
        },
    );
    for limit in [1, 100] {
        assert_eq!(
            read(&store, head, limit).unwrap_err().code,
            "publication_history_merge"
        );
    }
}
