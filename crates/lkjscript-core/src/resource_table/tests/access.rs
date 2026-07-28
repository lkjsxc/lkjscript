use lkjscript_contracts::ResourceKind;

use super::*;

#[test]
fn every_registered_kind_is_exact_before_payload_access() {
    let session = scope(1);
    let origin = provider(1);
    let mut table = ResourceTable::new(session, limits(1, 32));

    for (index, kind) in ResourceKind::ALL.into_iter().enumerate() {
        let key = table.reserve_owned(kind, origin).unwrap().commit(index);
        assert_eq!(*table.owned(&key, kind, origin, session).unwrap(), index);
        assert_eq!(table.stats().active_for(kind), 1);

        let wrong = ResourceKind::ALL[(index + 1) % ResourceKind::ALL.len()];
        let error = table.owned(&key, wrong, origin, session).unwrap_err();
        assert_eq!(
            error,
            ResourceTableError::WrongKind {
                expected: wrong,
                actual: kind,
            }
        );
        let closed = table
            .close_owned_with(key, kind, origin, session, |observation, payload| {
                assert_eq!(observation.state(), ResourceState::Closing);
                assert_eq!(observation.kind(), Some(kind));
                payload
            })
            .unwrap();
        assert_eq!(closed, index);
    }
}

#[test]
fn provider_scope_and_session_are_checked_before_lookup() {
    let session = scope(7);
    let other_session = scope(8);
    let origin = provider(11);
    let other_origin = provider(12);
    let kind = ResourceKind::Directory;
    let mut table = ResourceTable::new(session, limits(2, 4));
    let key = table.reserve_owned(kind, origin).unwrap().commit(41);

    assert!(matches!(
        table.owned(&key, kind, other_origin, session),
        Err(ResourceTableError::ProviderMismatch { .. })
    ));
    assert!(matches!(
        table.owned(&key, kind, origin, other_session),
        Err(ResourceTableError::ScopeMismatch { .. })
    ));

    let isolated: ResourceTable<i32> = ResourceTable::new(other_session, limits(2, 4));
    assert!(matches!(
        isolated.owned(&key, kind, origin, session),
        Err(ResourceTableError::ScopeMismatch { .. })
    ));
    assert_eq!(*table.owned(&key, kind, origin, session).unwrap(), 41);
}

#[test]
fn failed_explicit_close_is_closed_and_never_retryable() {
    let session = scope(2);
    let origin = provider(3);
    let kind = ResourceKind::FileAppender;
    let mut table = ResourceTable::new(session, limits(1, 4));
    let key = table.reserve_owned(kind, origin).unwrap().commit(5);
    let stale = key.clone();

    let outcome = table
        .close_owned_with(key, kind, origin, session, |_, _| {
            Err::<(), _>("host-failed")
        })
        .unwrap();
    assert_eq!(outcome, Err("host-failed"));
    assert_eq!(
        table.owned(&stale, kind, origin, session),
        Err(ResourceTableError::StaleKey)
    );
    assert_eq!(table.stats().closed(), 1);
    table.assert_zero_ordinary_obligations().unwrap();
}

#[test]
fn owned_close_and_borrowed_removal_are_distinct() {
    let session = scope(3);
    let origin = provider(4);
    let mut table = ResourceTable::new(session, limits(2, 4));
    let owned = table
        .reserve_owned(ResourceKind::FileReader, origin)
        .unwrap()
        .commit(10);
    let borrowed = table
        .reserve_borrowed(ResourceKind::InputStream, origin)
        .unwrap()
        .commit(20);

    assert_eq!(
        table
            .close_owned(owned, ResourceKind::FileReader, origin, session)
            .unwrap(),
        10
    );
    let error = table
        .close_owned(borrowed.clone(), ResourceKind::InputStream, origin, session)
        .unwrap_err();
    assert!(matches!(
        error,
        ResourceTableError::OwnershipMismatch {
            expected: ResourceOwnership::Owned,
            actual: ResourceOwnership::Borrowed,
        }
    ));
    assert_eq!(
        table
            .remove_borrowed(borrowed, ResourceKind::InputStream, origin, session)
            .unwrap(),
        20
    );
    table.assert_zero_ordinary_obligations().unwrap();
}
