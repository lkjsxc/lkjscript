//! Recovery observations must not convert an owned exclusive lock back to shared.
use super::*;
use crate::platform::change::{AuthoredChange, ChangeBudget, OwnerSelector};
use crate::platform::kernel::{Name, OwnerKey, OwnerRecord};
use crate::platform::storage::catalog::CatalogManifest;

fn fixture() -> (tempfile::TempDir, CreatedRepository, OwnerKey) {
    let temporary = tempfile::tempdir().unwrap();
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let owner = logical
        .owners
        .iter()
        .find_map(|(key, record)| matches!(record, OwnerRecord::Module(_)).then_some(*key))
        .unwrap();
    let created =
        GraphRepository::create(&temporary.path().join("project"), &logical, None).unwrap();
    (temporary, created, owner)
}

fn stale_catalog(root: &Path) {
    fs::write(
        root.join("catalog/current.lkjc"),
        CatalogManifest::empty().encode().unwrap(),
    )
    .unwrap();
}

fn would_block(result: std::io::Result<()>) {
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
}

#[test]
fn recovery_lock_healthy_observation_preserves_concurrent_shared_readers() {
    let (_temporary, created, _) = fixture();
    let root = created.repository.root();
    let directory = open_directory(root).unwrap();
    let lock = open_lock(&directory).unwrap();
    FileExt::lock_shared(&lock).unwrap();
    let store = open_store_shared(&directory, root, &lock).unwrap();
    let peer = open_lock(&directory).unwrap();
    FileExt::try_lock_shared(&peer).unwrap();
    let writer = open_lock(&directory).unwrap();
    would_block(FileExt::try_lock_exclusive(&writer));
    assert_eq!(
        read_current_optional(&directory, &store)
            .unwrap()
            .unwrap()
            .head,
        created.current.head
    );
    assert_eq!(store.catalog_observation().history.full_rebuilds, 0);
    drop(peer);
    drop(lock);
    FileExt::try_lock_exclusive(&writer).unwrap();
}

#[test]
fn recovery_lock_keeps_recovered_catalog_and_head_in_one_exclusive_observation() {
    let (_temporary, created, _) = fixture();
    let root = created.repository.root();
    let before = fs::read(root.join("HEAD")).unwrap();
    stale_catalog(root);
    let directory = open_directory(root).unwrap();
    let lock = open_lock(&directory).unwrap();
    FileExt::lock_shared(&lock).unwrap();
    let store = open_store_shared(&directory, root, &lock).unwrap();
    // Separate open-file descriptions, not duplicated descriptors sharing our own flock.
    let peer = open_lock(&directory).unwrap();
    let writer = open_lock(&directory).unwrap();
    would_block(FileExt::try_lock_shared(&peer));
    would_block(FileExt::try_lock_exclusive(&writer));
    assert_eq!(
        read_current_optional(&directory, &store)
            .unwrap()
            .unwrap()
            .head,
        created.current.head
    );
    assert_eq!(store.catalog_observation().history.full_rebuilds, 1);
    assert_eq!(fs::read(root.join("HEAD")).unwrap(), before);
    drop(lock);
    FileExt::try_lock_shared(&peer).unwrap();
    drop(peer);
    FileExt::try_lock_exclusive(&writer).unwrap();
    drop(writer);
    assert_eq!(
        created.repository.current().unwrap().head,
        created.current.head
    );
}

#[test]
fn recovery_lock_downgrade_gap_can_pair_a_new_head_with_a_stale_catalog() {
    let (_temporary, created, owner) = fixture();
    let root = created.repository.root();
    let prepared = created
        .repository
        .prepare_authored_change(
            &AuthoredChangeSet {
                base: created.current.head.revision,
                preconditions: Vec::new(),
                budget: ChangeBudget::default(),
                changes: vec![AuthoredChange::RenameOwner {
                    owner: OwnerSelector::Exact { owner },
                    name: Name::new("after-recovery").unwrap(),
                }],
            },
            PublicationOptions::default(),
        )
        .unwrap();
    stale_catalog(root);
    let directory = open_directory(root).unwrap();
    let lock = open_lock(&directory).unwrap();
    FileExt::lock_exclusive(&lock).unwrap();
    let initial = open_validated_store(&directory, root).unwrap_err();
    let stale = recover_validated_store(&directory, root, &initial).unwrap();

    // Deterministically model a legal flock conversion gap. This is not a claim
    // that the scheduler happened to produce the gap on this test machine.
    FileExt::unlock(&lock).unwrap();
    let publisher = created.repository.clone();
    let after =
        std::thread::spawn(
            move || match publisher.publish(&prepared.publication).unwrap() {
                PublicationOutcome::Accepted { current, .. } => current.head,
                other => panic!("expected an accepted change: {other:?}"),
            },
        )
        .join()
        .unwrap();
    FileExt::lock_shared(&lock).unwrap();
    assert_ne!(after, created.current.head);
    assert_eq!(
        read_current_optional(&directory, &stale).unwrap_err().code,
        "publication_repository_object_missing",
    );
    // Old immutable objects remain valid; only combining different observations fails.
    assert_eq!(
        read_publication(&stale, created.current.head).unwrap().head,
        created.current.head
    );
    let current = open_validated_store(&directory, root).unwrap();
    assert_eq!(
        read_current_optional(&directory, &current)
            .unwrap()
            .unwrap()
            .head,
        after
    );
}

#[test]
fn recovery_lock_failed_open_releases_ownership_without_repairing_accepted_head() {
    let (_temporary, created, _) = fixture();
    let root = created.repository.root();
    let invalid = b"not-an-accepted-head";
    fs::write(root.join("HEAD"), invalid).unwrap();
    assert!(GraphRepository::open(root).is_err());
    assert_eq!(fs::read(root.join("HEAD")).unwrap(), invalid);
    let directory = open_directory(root).unwrap();
    let lock = open_lock(&directory).unwrap();
    FileExt::try_lock_exclusive(&lock).unwrap();
}
