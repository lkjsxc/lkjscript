use lkjscript_contracts::ResourceKind;

use super::*;

#[test]
fn reservation_cancel_and_failed_acquisition_publish_no_key() {
    let session = scope(1);
    let origin = provider(1);
    let mut table = ResourceTable::new(session, limits(1, 4));

    let reservation = table
        .reserve_owned(ResourceKind::TcpStream, origin)
        .unwrap();
    assert_eq!(reservation.observation().state(), ResourceState::Reserved);
    assert_eq!(reservation.stats().reserved_owned(), 1);
    reservation.cancel();
    assert_eq!(table.stats().vacant(), 1);
    assert_eq!(table.stats().ordinary_obligations(), 0);

    let failure = table
        .reserve_owned(ResourceKind::TcpStream, origin)
        .unwrap()
        .acquire(|| Err::<usize, _>("provider-failed"));
    assert_eq!(failure, Err("provider-failed"));
    assert_eq!(table.stats().vacant(), 1);
    assert_eq!(table.stats().reserved(), 0);
}

#[test]
fn reusable_slots_are_lifo_and_advance_generation() {
    let session = scope(2);
    let origin = provider(2);
    let kind = ResourceKind::FileWriter;
    let mut table = ResourceTable::new(session, limits(2, 8));
    let first = table.reserve_owned(kind, origin).unwrap().commit(1);
    let second = table.reserve_owned(kind, origin).unwrap().commit(2);
    let stale_second = second.clone();
    let second_slot = second.slot;
    let second_generation = second.generation;

    assert_eq!(table.close_owned(first, kind, origin, session).unwrap(), 1);
    assert_eq!(table.close_owned(second, kind, origin, session).unwrap(), 2);
    assert_eq!(table.stats().closed(), 2);

    let reused = table.reserve_owned(kind, origin).unwrap().commit(3);
    assert_eq!(reused.slot, second_slot);
    assert!(reused.generation > second_generation);
    assert_eq!(
        table.owned(&stale_second, kind, origin, session),
        Err(ResourceTableError::StaleKey)
    );
    assert_eq!(table.close_owned(reused, kind, origin, session).unwrap(), 3);
}

#[test]
fn generation_exhaustion_permanently_retires_the_slot() {
    let session = scope(3);
    let origin = provider(3);
    let kind = ResourceKind::TcpListener;
    let mut table = ResourceTable::new(session, limits(1, 1));
    let key = table.reserve_owned(kind, origin).unwrap().commit(7);
    table.close_owned(key, kind, origin, session).unwrap();

    let error = table.reserve_owned(kind, origin).err().unwrap();
    assert_eq!(
        error,
        ResourceTableError::GenerationExhausted { retired_slots: 1 }
    );
    assert_eq!(table.observations()[0].state(), ResourceState::Retired);
    assert_eq!(table.stats().retired(), 1);
}

#[test]
fn parent_cannot_close_while_a_child_is_live() {
    let session = scope(5);
    let origin = provider(5);
    let mut table = ResourceTable::new(session, limits(2, 4));
    let parent = table
        .reserve_owned(ResourceKind::SqliteConnection, origin)
        .unwrap()
        .commit(10);
    let child = table
        .reserve_owned_child(
            &parent,
            ResourceKind::SqliteConnection,
            ResourceKind::SqliteStatement,
            origin,
        )
        .unwrap()
        .commit(20);

    let error = table
        .close_owned(
            parent.clone(),
            ResourceKind::SqliteConnection,
            origin,
            session,
        )
        .unwrap_err();
    assert_eq!(
        error,
        ResourceTableError::ParentHasLiveChildren { children: 1 }
    );
    table
        .close_owned(child, ResourceKind::SqliteStatement, origin, session)
        .unwrap();
    table
        .reserve_owned_child(
            &parent,
            ResourceKind::SqliteConnection,
            ResourceKind::SqliteStatement,
            origin,
        )
        .unwrap()
        .cancel();
    assert_eq!(table.observations()[parent.slot].live_children(), 0);
    assert_eq!(
        table
            .close_owned(parent, ResourceKind::SqliteConnection, origin, session)
            .unwrap(),
        10
    );
}
