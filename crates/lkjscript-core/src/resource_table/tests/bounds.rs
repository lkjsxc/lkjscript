use std::num::NonZeroU64;

use lkjscript_contracts::ResourceKind;

use super::*;

fn assert_limit(error: ResourceTableError, expected: ResourceTableLimit) {
    assert!(matches!(
        error,
        ResourceTableError::LimitExceeded { limit, .. } if limit == expected
    ));
}

#[test]
fn every_configured_runtime_limit_rejects_before_mutation() {
    let session = scope(4);
    let origin = provider(4);
    let generation = NonZeroU64::new(3).unwrap();
    let no_reservations = ResourceTableLimits::new(2, 0, 2, 2, 2, generation).unwrap();
    let mut table: ResourceTable<()> = ResourceTable::new(session, no_reservations);
    let before = table.stats();
    let error = table
        .reserve_owned(ResourceKind::Directory, origin)
        .err()
        .unwrap();
    assert_limit(error, ResourceTableLimit::Reservations);
    assert_eq!(table.stats(), before);

    let one_owned = ResourceTableLimits::new(2, 2, 1, 2, 2, generation).unwrap();
    let mut table = ResourceTable::new(session, one_owned);
    table
        .reserve_owned(ResourceKind::Directory, origin)
        .unwrap()
        .commit(());
    let before = table.stats();
    let error = table
        .reserve_owned(ResourceKind::FileReader, origin)
        .err()
        .unwrap();
    assert_limit(error, ResourceTableLimit::Owned);
    assert_eq!(table.stats(), before);

    let one_borrowed = ResourceTableLimits::new(2, 2, 2, 1, 2, generation).unwrap();
    let mut table = ResourceTable::new(session, one_borrowed);
    table
        .reserve_borrowed(ResourceKind::InputStream, origin)
        .unwrap()
        .commit(());
    let before = table.stats();
    let error = table
        .reserve_borrowed(ResourceKind::OutputStream, origin)
        .err()
        .unwrap();
    assert_limit(error, ResourceTableLimit::Borrowed);
    assert_eq!(table.stats(), before);

    let mut table = ResourceTable::new(session, limits(1, 3));
    table
        .reserve_owned(ResourceKind::FileReader, origin)
        .unwrap()
        .commit(());
    let before = table.stats();
    let error = table
        .reserve_borrowed(ResourceKind::InputStream, origin)
        .err()
        .unwrap();
    assert_limit(error, ResourceTableLimit::Slots);
    assert_eq!(table.stats(), before);
}

#[test]
fn child_limit_preserves_parent_and_slot_state() {
    let session = scope(6);
    let origin = provider(6);
    let generation = NonZeroU64::new(3).unwrap();
    let no_children = ResourceTableLimits::new(2, 2, 2, 2, 0, generation).unwrap();
    let mut table = ResourceTable::new(session, no_children);
    let parent = table
        .reserve_owned(ResourceKind::SqliteConnection, origin)
        .unwrap()
        .commit(());
    let before = table.stats();
    let error = table
        .reserve_owned_child(
            &parent,
            ResourceKind::SqliteConnection,
            ResourceKind::SqliteStatement,
            origin,
        )
        .err()
        .unwrap();
    assert_limit(error, ResourceTableLimit::ChildrenPerParent);
    assert_eq!(table.stats(), before);
    assert_eq!(table.observations()[0].live_children(), 0);
}
