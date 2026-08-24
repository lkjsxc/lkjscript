use super::*;
use crate::platform::change::{CanonicalDelta, PrimitiveEdit, prepare_change_analysis};
use crate::platform::kernel::{
    DeclarationPayload, ExpressionOperation, Name, OwnerKey, OwnerRecord, encode_owner,
};

#[test]
fn repository_create_reopen_and_exact_current_reads_bind_every_object() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(
        &destination,
        &logical,
        Some("repository fixture".to_owned()),
    )
    .expect("Graph 5 repository creation");

    assert_eq!(created.repository.root(), destination.as_path());
    assert_eq!(created.current.head, created.initial.publication.head);
    assert_eq!(
        created.current.semantic_root,
        created.initial.publication.authority.semantic.root
    );
    assert_eq!(
        created.current.witness,
        created.initial.publication.authority.witness.manifest
    );
    assert_eq!(
        created.current.receipt.status,
        PublicationStatus::ProjectCreated
    );
    assert!(created.current.store_work.objects_read >= 6);
    assert!(
        created
            .repository
            .head_staging_leftovers()
            .unwrap()
            .is_empty()
    );

    let reopened = GraphRepository::open(&destination).expect("repository reopen");
    let current = reopened.current().expect("exact current read");
    assert_eq!(current.head, created.current.head);
    assert_eq!(current.revision, created.current.revision);
    assert_eq!(current.receipt, created.current.receipt);
    assert_eq!(current.transaction, created.current.transaction);
    assert_eq!(current.semantic_diff, created.current.semantic_diff);
    assert_eq!(current.accepted, created.current.accepted);
}

#[test]
fn locked_publication_accepts_once_reconciles_exact_retry_and_rejects_stale() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let prepared = prepare_body_publication(&created, PublicationOptions::default());

    let mut tampered = prepared.clone();
    tampered.receipt.intent = Some("changed after preparation".to_owned());
    let error = created
        .repository
        .publish(&tampered)
        .expect_err("commit must revalidate every typed prepared record");
    assert_eq!(error.code, "publication_repository_prepared_binding");
    assert_eq!(
        created.repository.current().unwrap().head,
        created.current.head
    );

    let outcome = created
        .repository
        .publish(&prepared)
        .expect("first publication");
    let PublicationOutcome::Accepted {
        current,
        seal,
        store_work,
    } = outcome
    else {
        panic!("first publication must advance HEAD")
    };
    assert_eq!(current.head, prepared.head);
    assert!(!seal.packs.is_empty());
    assert!(store_work.packs_sealed > 0);

    let retry = created
        .repository
        .publish(&prepared)
        .expect("exact retry reconciliation");
    let PublicationOutcome::AlreadyAccepted { current: replay } = retry else {
        panic!("exact retry must return original accepted publication")
    };
    assert_eq!(replay.head, prepared.head);
    assert_eq!(replay.receipt, prepared.receipt);

    let stale = prepare_rename_publication(&created, "stale_rename", None);
    let outcome = created
        .repository
        .publish(&stale)
        .expect("stale is a nonpublishing outcome");
    let PublicationOutcome::Stale {
        expected,
        current: observed,
    } = outcome
    else {
        panic!("competing publication from the old base must be stale")
    };
    assert_eq!(expected, Some(created.current.head));
    assert_eq!(observed, Some(prepared.head));
    let ReconciliationStatus::Stale {
        expected,
        current: observed,
    } = created.repository.reconcile(&stale).unwrap()
    else {
        panic!("stale reconciliation must stay distinct")
    };
    assert_eq!(expected, Some(created.current.head));
    assert_eq!(observed, Some(prepared.head));
    assert_eq!(created.repository.current().unwrap().head, prepared.head);
}

#[test]
fn concurrent_readers_observe_only_old_or_new_complete_publications() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let prepared = prepare_body_publication(&created, PublicationOptions::default());
    let old = created.current.head;
    let new = prepared.head;
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(5));
    let mut readers = Vec::new();
    for _ in 0..4 {
        let repository = created.repository.clone();
        let barrier = barrier.clone();
        readers.push(std::thread::spawn(move || {
            barrier.wait();
            (0..16)
                .map(|_| repository.current().map(|current| current.head))
                .collect::<Result<Vec<_>, _>>()
        }));
    }
    barrier.wait();
    assert!(matches!(
        created.repository.publish(&prepared).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    for reader in readers {
        let observed = reader
            .join()
            .expect("reader thread must not panic")
            .expect("reader must observe complete authority");
        assert!(observed.into_iter().all(|head| head == old || head == new));
    }
    assert_eq!(created.repository.current().unwrap().head, new);
}

#[test]
fn idempotency_key_is_exactly_bound_to_base_and_normalized_transaction() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let options = PublicationOptions {
        idempotency_key: Some("request-1".to_owned()),
        intent: None,
    };
    let accepted = prepare_body_publication(&created, options.clone());
    let conflict = prepare_rename_publication(&created, "different_request", Some("request-1"));

    assert!(matches!(
        created.repository.publish(&accepted).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    assert!(matches!(
        created.repository.publish(&accepted).unwrap(),
        PublicationOutcome::AlreadyAccepted { .. }
    ));
    let error = created
        .repository
        .publish(&conflict)
        .expect_err("one key cannot identify two normalized transactions");
    assert_eq!(error.code, "publication_repository_idempotency_conflict");
    let ReconciliationStatus::ConflictingIdempotency { current } = created
        .repository
        .reconcile(&conflict)
        .expect("idempotency reconciliation")
    else {
        panic!("idempotency conflict must remain a typed reconciliation outcome")
    };
    assert_eq!(current, accepted.head);
}

#[test]
fn deterministic_publication_interruptions_reopen_old_or_new_complete_head() {
    for (ordinal, point, expects_new, leaves_head_stage) in [
        (0, PublicationPoint::BeforeObjectStage, false, false),
        (1, PublicationPoint::AfterFirstObjectStage, false, false),
        (2, PublicationPoint::AfterPacksSealed, false, false),
        (3, PublicationPoint::AfterHeadFileSynced, false, true),
        (4, PublicationPoint::AfterHeadRenamed, true, false),
        (5, PublicationPoint::AfterHeadDirectorySynced, true, false),
    ] {
        let temporary = tempfile::tempdir().expect("temporary repository parent");
        let destination = temporary.path().join(format!("meaning-{ordinal}"));
        let logical = crate::platform::kernel::tests::witness_snapshot();
        let created =
            GraphRepository::create(&destination, &logical, None).expect("create repository");
        let prepared = prepare_body_publication(&created, PublicationOptions::default());
        let error = created
            .repository
            .publish_with_fault(&prepared, point)
            .expect_err("injected interruption");
        assert_eq!(
            error.code, "publication_repository_injected_interruption",
            "unexpected diagnostic at {point:?}"
        );

        let reopened = GraphRepository::open(&destination).expect("interrupted repository reopens");
        let current = reopened.current().expect("complete accepted current");
        assert_eq!(
            current.head,
            if expects_new {
                prepared.head
            } else {
                created.current.head
            },
            "wrong visible side of interruption at {point:?}"
        );
        assert_eq!(
            reopened.head_staging_leftovers().unwrap().len(),
            usize::from(leaves_head_stage),
            "unexpected HEAD-stage classification at {point:?}"
        );
        match reopened.reconcile(&prepared).expect("exact reconciliation") {
            ReconciliationStatus::Accepted { current } if expects_new => {
                assert_eq!(current.head, prepared.head);
            }
            ReconciliationStatus::NotStarted { current } if !expects_new => {
                assert_eq!(current, Some(created.current.head));
            }
            other => panic!("unexpected reconciliation {other:?} at {point:?}"),
        }
        if expects_new {
            assert!(matches!(
                reopened.publish(&prepared).unwrap(),
                PublicationOutcome::AlreadyAccepted { .. }
            ));
        }
    }
}

#[test]
fn repository_rejects_predecessor_head_and_missing_accepted_pack() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let predecessor = temporary.path().join("predecessor");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let _ = GraphRepository::create(&predecessor, &logical, None).expect("create repository");
    let mut head = std::fs::read(predecessor.join("HEAD")).expect("HEAD bytes");
    head[..8].copy_from_slice(b"LKJHEAD4");
    std::fs::write(predecessor.join("HEAD"), head).expect("replace test HEAD");
    assert_eq!(
        GraphRepository::open(&predecessor)
            .expect_err("predecessor HEAD must reject")
            .code,
        "packed_contract"
    );

    let missing = temporary.path().join("missing-pack");
    let _ = GraphRepository::create(&missing, &logical, None).expect("create repository");
    for entry in std::fs::read_dir(missing.join("packs")).expect("pack directory") {
        let path = entry.expect("pack entry").path();
        std::fs::remove_file(path).expect("remove accepted test pack");
    }
    assert_eq!(
        GraphRepository::open(&missing)
            .expect_err("missing accepted pack must reject")
            .code,
        "publication_repository_object_missing"
    );
}

#[cfg(unix)]
#[test]
fn repository_rejects_symlinked_lock_and_head_stage() {
    use std::os::unix::fs::symlink;

    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let target = destination.join("lock-target");
    std::fs::rename(destination.join("LOCK"), &target).expect("move real lock");
    symlink(&target, destination.join("LOCK")).expect("symlink lock");
    assert!(created.repository.current().is_err());

    std::fs::remove_file(destination.join("LOCK")).expect("remove lock symlink");
    std::fs::rename(&target, destination.join("LOCK")).expect("restore lock");
    let stage_target = destination.join("stage-target");
    std::fs::write(&stage_target, b"not a HEAD").expect("stage target");
    symlink(
        &stage_target,
        destination.join(".HEAD-stage-hostile-symlink"),
    )
    .expect("stage symlink");
    assert_eq!(
        created
            .repository
            .head_staging_leftovers()
            .expect_err("symlinked HEAD stage must reject")
            .code,
        "publication_repository_stage_type"
    );
}

fn prepare_body_publication(
    created: &CreatedRepository,
    options: PublicationOptions,
) -> PreparedPublication {
    let body = function_body(&created.initial.snapshot, "callee");
    let mut replacement = created.initial.snapshot.owners[&body].clone();
    let OwnerRecord::Expression(expression) = &mut replacement else {
        panic!("callee body must be an expression")
    };
    expression.operation = ExpressionOperation::Unit;
    let expected = encode_owner(&created.initial.snapshot.owners[&body])
        .expect("base body encoding")
        .0;
    prepare_publication(
        created,
        vec![PrimitiveEdit::ReplaceOwner {
            expected,
            record: replacement,
        }],
        options,
    )
}

fn prepare_rename_publication(
    created: &CreatedRepository,
    name: &str,
    idempotency_key: Option<&str>,
) -> PreparedPublication {
    let callee = owner_named(&created.initial.snapshot, "callee");
    let mut replacement = created.initial.snapshot.owners[&callee].clone();
    let OwnerRecord::Declaration(declaration) = &mut replacement else {
        panic!("callee must be a declaration")
    };
    declaration.name = Name::new(name).expect("replacement name");
    let expected = encode_owner(&created.initial.snapshot.owners[&callee])
        .expect("base declaration encoding")
        .0;
    prepare_publication(
        created,
        vec![PrimitiveEdit::ReplaceOwner {
            expected,
            record: replacement,
        }],
        PublicationOptions {
            idempotency_key: idempotency_key.map(str::to_owned),
            intent: None,
        },
    )
}

fn prepare_publication(
    created: &CreatedRepository,
    edits: Vec<PrimitiveEdit>,
    options: PublicationOptions,
) -> PreparedPublication {
    let delta = CanonicalDelta::normalize(&created.initial.snapshot, edits)
        .expect("canonical repository change");
    let analysis =
        prepare_change_analysis(&created.initial.snapshot, &created.initial.witness, delta)
            .expect("repository change analysis");
    let store = created
        .repository
        .object_store()
        .expect("packed object store");
    prepare_change_publication(
        created.current.accepted,
        &created.initial.snapshot,
        &created.initial.witness,
        &analysis,
        &store,
        options,
    )
    .expect("prepared repository publication")
}

fn owner_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Declaration(declaration) if declaration.name.as_str() == name => {
                Some(*owner)
            }
            _ => None,
        })
        .expect("named declaration")
}

fn function_body(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    let declaration = owner_named(snapshot, name);
    let OwnerRecord::Declaration(record) = &snapshot.owners[&declaration] else {
        panic!("named owner must be a declaration")
    };
    let DeclarationPayload::Function(function) = &record.payload else {
        panic!("named declaration must be a function")
    };
    OwnerKey::Expression(function.body)
}
