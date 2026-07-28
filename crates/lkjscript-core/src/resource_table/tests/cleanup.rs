use lkjscript_contracts::ResourceKind;

use super::*;

#[test]
fn owned_cleanup_is_reverse_acquisition_and_continues_after_failures() {
    let session = scope(9);
    let origin = provider(9);
    let mut table = ResourceTable::new(session, limits(3, 8));
    let parent = table
        .reserve_owned(ResourceKind::SqliteConnection, origin)
        .unwrap()
        .commit(10);
    table
        .reserve_owned_child(
            &parent,
            ResourceKind::SqliteConnection,
            ResourceKind::SqliteStatement,
            origin,
        )
        .unwrap()
        .commit(20);
    table
        .reserve_owned(ResourceKind::FileAppender, origin)
        .unwrap()
        .commit(30);

    let mut supplied = Vec::new();
    let report = table
        .cleanup_owned_reverse(|observation, payload| {
            assert_eq!(observation.state(), ResourceState::Closing);
            supplied.push(payload);
            if payload == 20 {
                Err("close-failed")
            } else {
                Ok(())
            }
        })
        .unwrap();

    assert_eq!(supplied, vec![30, 20, 10]);
    assert_eq!(report.count(), 3);
    assert_eq!(report.attempts()[0].outcome(), &Ok(()));
    assert_eq!(report.attempts()[1].outcome(), &Err("close-failed"));
    assert_eq!(report.attempts()[2].outcome(), &Ok(()));
    assert_eq!(table.stats().closed(), 3);
    table.assert_zero_ordinary_obligations().unwrap();
}

#[test]
fn emergency_observation_is_exact_and_excludes_borrowed_resources() {
    let session = scope(10);
    let origin = provider(10);
    let mut table = ResourceTable::new(session, limits(3, 8));
    table
        .reserve_owned(ResourceKind::FileReader, origin)
        .unwrap()
        .commit(1);
    table
        .reserve_owned(ResourceKind::TcpStream, origin)
        .unwrap()
        .commit(2);
    let borrowed = table
        .reserve_borrowed(ResourceKind::OutputStream, origin)
        .unwrap()
        .commit(3);

    let emergency = table.emergency_obligations();
    assert_eq!(emergency.count(), 2);
    assert_eq!(
        emergency.resources()[0].kind(),
        Some(ResourceKind::TcpStream)
    );
    assert_eq!(
        emergency.resources()[1].kind(),
        Some(ResourceKind::FileReader)
    );
    assert_eq!(table.stats().owned_open(), 2);
    assert_eq!(table.stats().borrowed_open(), 1);
    assert_eq!(
        table.assert_zero_ordinary_obligations(),
        Err(ResourceTableError::OutstandingOrdinaryObligations { count: 2 })
    );

    let report = table.cleanup_owned_reverse(|_, payload| payload).unwrap();
    assert_eq!(
        report
            .into_attempts()
            .into_iter()
            .map(ResourceCleanupAttempt::into_outcome)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    assert!(table.emergency_obligations().is_empty());
    table.assert_zero_ordinary_obligations().unwrap();
    assert_eq!(
        table
            .remove_borrowed(borrowed, ResourceKind::OutputStream, origin, session,)
            .unwrap(),
        3
    );
}
