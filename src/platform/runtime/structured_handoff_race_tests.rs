use super::handoff_tests::{assert_balance, pair};
use super::*;
use std::sync::Barrier;

#[test]
fn simultaneous_handoff_and_shutdown_keep_exactly_one_join_owner() {
    for _ in 0..64 {
        let mut first = StructuredExecutor::for_test(1);
        let mut second = first.sharing_capacity_for_test();
        let capacity = Arc::clone(&first.inner.capacity);
        assert!(pair(&first).dispatched);
        let start = Barrier::new(3);
        let (first, second) = thread::scope(|scope| {
            let a = scope.spawn(|| {
                start.wait();
                first.shutdown().unwrap()
            });
            let b = scope.spawn(|| {
                start.wait();
                assert_eq!(pair(&second).left, 17);
                second.shutdown().unwrap()
            });
            start.wait();
            (a.join().unwrap(), b.join().unwrap())
        });
        for observation in [&first, &second] {
            assert_balance(observation);
            assert_eq!(
                (observation.active_dispatches, observation.remaining_workers),
                (0, 0)
            );
        }
        assert_eq!(
            first.workers_received + second.workers_received,
            first.workers_handed_off + second.workers_handed_off
        );
        assert_eq!(
            first.workers_started + second.workers_started,
            first.joined_workers + second.joined_workers
        );
        assert!(first.workers_started + second.workers_started <= 2);
        assert_eq!(capacity.reserved.load(Ordering::Acquire), 0);
        assert!(lock(&capacity.custody).is_empty());
    }
}

#[test]
fn concurrent_borrowers_cannot_duplicate_one_idle_worker() {
    let first = StructuredExecutor::for_test(1);
    let second = first.sharing_capacity_for_test();
    let third = first.sharing_capacity_for_test();
    assert!(pair(&first).dispatched);
    let entered = Arc::new(Barrier::new(3));
    let release = Arc::new(Barrier::new(3));
    let (a, b) = thread::scope(|scope| {
        let run = |handle: StructuredExecutorHandle| {
            let entered = Arc::clone(&entered);
            let release = Arc::clone(&release);
            scope.spawn(move || {
                handle
                    .run(
                        &ExecutionControl::uncancelled(),
                        || 17,
                        move || {
                            entered.wait();
                            release.wait();
                            29
                        },
                    )
                    .unwrap()
            })
        };
        let a = run(second.handle());
        let b = run(third.handle());
        entered.wait();
        assert_eq!(
            second.observe().active_dispatches + third.observe().active_dispatches,
            1
        );
        assert_eq!(first.inner.capacity.reserved.load(Ordering::Acquire), 1);
        release.wait();
        (a.join().unwrap(), b.join().unwrap())
    });
    assert_ne!(a.dispatched, b.dispatched);
    assert_eq!((a.left, a.right, b.left, b.right), (17, 29, 17, 29));
    assert_eq!(first.observe().workers_handed_off, 1);
    assert_eq!(
        second.observe().workers_received + third.observe().workers_received,
        1
    );
    for owner in [&first, &second, &third] {
        assert_balance(&owner.observe());
    }
}
