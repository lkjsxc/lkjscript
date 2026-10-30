use super::*;
use crate::platform::kernel::{Name, OwnerRecord, encode_owner};

fn fixture(
    path: &Path,
) -> (
    GraphRepository,
    super::super::super::ExportedPackageTransport,
    KernelSnapshot,
    PreparedPublication,
) {
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(path, &logical, None).unwrap();
    let owner = logical.owners.values().find(|owner| {
        matches!(owner, OwnerRecord::Declaration(declaration) if declaration.name.as_str() == "callee")
    }).unwrap();
    let expected = encode_owner(owner).unwrap().0;
    let mut replacement = owner.clone();
    let OwnerRecord::Declaration(declaration) = &mut replacement else {
        panic!("declaration")
    };
    declaration.name = Name::new("before-republication").unwrap();
    let old = created
        .repository
        .prepare_change(
            vec![PrimitiveEdit::ReplaceOwner {
                expected,
                record: replacement,
            }],
            PublicationOptions {
                idempotency_key: Some("before-format-cut".to_owned()),
                intent: None,
            },
        )
        .unwrap();
    created.repository.publish(&old).unwrap();
    let source = created.repository.export_package_transport().unwrap();
    let snapshot = created
        .repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    (created.repository, source, snapshot, old)
}

#[test]
fn republication_preserves_complete_meaning_and_rejects_old_retry_bases() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("project");
    let (repository, source, before, old_retry) = fixture(&path);
    let old_bytes = fs::read(path.join("HEAD")).unwrap();
    let result = GraphRepository::republish_current_transport(
        &path,
        &sha256(&old_bytes),
        source.transport_digest,
        &source.container,
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_ne!(result.previous.revision, result.current.revision);
    assert_eq!(result.source_sha256, sha256(&source.container));
    let view = repository.view_current().unwrap();
    let after = view.reconstruct_full_oracle().unwrap().value;
    assert_eq!(before.owners, after.owners);
    assert_eq!(before.types, after.types);
    assert_eq!(before.retirements, after.retirements);
    assert_eq!(before.dependencies, after.dependencies);
    assert_eq!(before.blobs, after.blobs);
    assert_eq!(before.dependency_types, after.dependency_types);
    assert_eq!(before.dependency_interfaces, after.dependency_interfaces);
    assert_eq!(
        before.representation_dependencies,
        after.representation_dependencies
    );
    assert_eq!(before.root.repository_id, after.root.repository_id);
    assert_eq!(before.root.package_id, after.root.package_id);
    assert_eq!(
        semantic_state_digest_from_root(&before.root).unwrap(),
        result.semantic_state
    );
    assert_eq!(
        semantic_state_digest_from_root(&after.root).unwrap(),
        result.semantic_state
    );
    assert!(view.current().revision.core.parents.is_empty());
    assert_eq!(
        view.current()
            .revision
            .publication
            .idempotency_receipts
            .entries(),
        0
    );
    assert!(
        view.current()
            .receipt
            .intent
            .as_ref()
            .unwrap()
            .contains(&result.source_sha256)
    );
    drop(view);
    let new_bytes = fs::read(path.join("HEAD")).unwrap();
    assert!(matches!(
        repository.publish(&old_retry).unwrap(),
        PublicationOutcome::Stale { .. }
    ));
    assert_eq!(fs::read(path.join("HEAD")).unwrap(), new_bytes);
    assert_eq!(
        GraphRepository::open(&path)
            .unwrap()
            .current()
            .unwrap()
            .head,
        result.current
    );
}

#[test]
fn republication_rejects_stale_corrupt_and_foreign_sources_before_head_change() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("project");
    let (_repository, source, _, _) = fixture(&path);
    let before = fs::read(path.join("HEAD")).unwrap();
    let error = GraphRepository::republish_current_transport(
        &path,
        &"0".repeat(64),
        source.transport_digest,
        &source.container,
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    assert_eq!(error.code, "publication_republish_stale");
    let mut corrupt = source.container.clone();
    corrupt.pop();
    assert!(
        GraphRepository::republish_current_transport(
            &path,
            &sha256(&before),
            source.transport_digest,
            &corrupt,
            &ExecutionControl::uncancelled(),
        )
        .is_err()
    );
    let foreign = GraphRepository::create(
        &parent.path().join("foreign"),
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap()
    .repository
    .export_package_transport()
    .unwrap();
    let error = GraphRepository::republish_current_transport(
        &path,
        &sha256(&before),
        foreign.transport_digest,
        &foreign.container,
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    assert_eq!(error.code, "publication_republish_source_binding");
    assert_eq!(fs::read(path.join("HEAD")).unwrap(), before);
    assert!(GraphRepository::open(&path).is_ok());
}

#[test]
fn republication_cancellation_and_previsibility_faults_keep_the_accepted_head() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("project");
    let (_repository, source, _, _) = fixture(&path);
    let before = fs::read(path.join("HEAD")).unwrap();
    for steps in [0, 1, 10] {
        assert!(
            GraphRepository::republish_current_transport(
                &path,
                &sha256(&before),
                source.transport_digest,
                &source.container,
                &ExecutionControl::cancel_after_checks(steps),
            )
            .is_err()
        );
        assert_eq!(fs::read(path.join("HEAD")).unwrap(), before);
    }
    for point in [
        PublicationPoint::BeforeObjectStage,
        PublicationPoint::AfterFirstObjectStage,
        PublicationPoint::AfterPacksSealed,
        PublicationPoint::AfterHeadFileSynced,
    ] {
        assert!(
            republish(
                &path,
                &sha256(&before),
                source.transport_digest,
                &source.container,
                &ExecutionControl::uncancelled(),
                Some(point),
            )
            .is_err()
        );
        assert_eq!(fs::read(path.join("HEAD")).unwrap(), before);
        assert!(GraphRepository::open(&path).is_ok());
    }
    assert!(
        republish(
            &path,
            &sha256(&before),
            source.transport_digest,
            &source.container,
            &ExecutionControl::uncancelled(),
            Some(PublicationPoint::AfterHeadRenamed),
        )
        .is_err()
    );
    let recovered = GraphRepository::open(&path).unwrap().current().unwrap();
    assert!(recovered.revision.core.parents.is_empty());
    assert_eq!(
        recovered.accepted.semantic_state,
        source.revision.revision.semantic_state
    );
    assert_ne!(fs::read(path.join("HEAD")).unwrap(), before);
}

#[test]
fn concurrent_republications_accept_exactly_one_pinned_head() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("project");
    let (_repository, source, _, _) = fixture(&path);
    let expected = sha256(&fs::read(path.join("HEAD")).unwrap());
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let run = || {
            barrier.wait();
            GraphRepository::republish_current_transport(
                &path,
                &expected,
                source.transport_digest,
                &source.container,
                &ExecutionControl::uncancelled(),
            )
        };
        let first = scope.spawn(run);
        let second = scope.spawn(run);
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(
                |result| matches!(result, Err(error) if error.code == "publication_republish_stale")
            )
            .count(),
        1
    );
    assert!(GraphRepository::open(&path).is_ok());
}

#[test]
fn republication_invalidates_prepared_genesis_changes_even_when_semantic_revision_is_equal() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("project");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let repository = GraphRepository::create(&path, &logical, None)
        .unwrap()
        .repository;
    let source = repository.export_package_transport().unwrap();
    let owner = logical.owners.values().find(|owner| {
        matches!(owner, OwnerRecord::Declaration(declaration) if declaration.name.as_str() == "callee")
    }).unwrap();
    let expected = encode_owner(owner).unwrap().0;
    let mut replacement = owner.clone();
    let OwnerRecord::Declaration(declaration) = &mut replacement else {
        panic!("declaration")
    };
    declaration.name = Name::new("prepared-before-genesis-cut").unwrap();
    let pending = repository
        .prepare_change(
            vec![PrimitiveEdit::ReplaceOwner {
                expected,
                record: replacement,
            }],
            PublicationOptions {
                idempotency_key: Some("pending-before-genesis-cut".to_owned()),
                intent: None,
            },
        )
        .unwrap();
    let old_bytes = fs::read(path.join("HEAD")).unwrap();
    let result = GraphRepository::republish_current_transport(
        &path,
        &sha256(&old_bytes),
        source.transport_digest,
        &source.container,
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!(result.previous.revision, result.current.revision);
    assert_ne!(result.previous.record, result.current.record);
    let current_bytes = fs::read(path.join("HEAD")).unwrap();
    assert!(matches!(
        repository.publish(&pending).unwrap(),
        PublicationOutcome::Stale { .. }
    ));
    assert_eq!(fs::read(path.join("HEAD")).unwrap(), current_bytes);
}
