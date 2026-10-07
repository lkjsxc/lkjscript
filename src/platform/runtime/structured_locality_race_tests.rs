use super::handoff_tests::{WAIT, assert_balance, pair};
use super::*;

#[test]
fn independent_warm_owners_finish_while_the_directory_is_locked() {
    let mut first = StructuredExecutor::for_test(2);
    let mut second = first.sharing_capacity_for_test();
    let initial = first
        .handle()
        .run(
            &ExecutionControl::uncancelled(),
            || pair(&second),
            || thread::current().id(),
        )
        .unwrap();
    assert!(initial.dispatched && initial.left.dispatched);
    let expected = [initial.right, initial.left.right];
    assert_ne!(expected[0], expected[1]);
    let custody = lock(&first.inner.capacity.custody);
    let (finished, wait_finished) = mpsc::channel();
    let callers: Vec<_> = [first.handle(), second.handle()]
        .into_iter()
        .enumerate()
        .map(|(owner, handle)| {
            let finished = finished.clone();
            thread::spawn(move || {
                for index in 0..64 {
                    let result = handle
                        .run(
                            &ExecutionControl::uncancelled(),
                            || index,
                            || thread::current().id(),
                        )
                        .unwrap();
                    assert!(result.dispatched);
                    assert_eq!(result.left, index);
                    assert_eq!(result.right, expected[owner]);
                }
                finished.send(owner).unwrap();
            })
        })
        .collect();
    drop(finished);
    let results = [
        wait_finished.recv_timeout(WAIT),
        wait_finished.recv_timeout(WAIT),
    ];
    drop(custody);
    for caller in callers {
        caller.join().unwrap();
    }
    assert!(results.iter().all(Result::is_ok));
    assert_eq!(
        first.observe().workers_received + second.observe().workers_received,
        0
    );
    assert_balance(&first.shutdown().unwrap());
    assert_balance(&second.shutdown().unwrap());
}

#[test]
fn custody_entries_for_unreceived_results_never_confer_availability() {
    let mut first = StructuredExecutor::for_test(1);
    let mut second = first.sharing_capacity_for_test();
    let initial = pair(&first);
    for _ in 0..64 {
        let (returned, wait_returned) = mpsc::sync_channel(1);
        let result = first
            .handle()
            .run(
                &ExecutionControl::uncancelled(),
                || {
                    wait_returned.recv_timeout(WAIT).unwrap();
                    assert_eq!(lock(&first.inner.capacity.custody).len(), 1);
                    assert!(!pair(&second).dispatched);
                    assert_eq!(lock(&first.inner.capacity.custody).len(), 1);
                    29
                },
                move || {
                    returned.send(()).unwrap();
                    thread::current().id()
                },
            )
            .unwrap();
        assert!(result.dispatched);
        assert_eq!(result.right, initial.right);
    }
    let received = pair(&second);
    assert!(received.dispatched);
    assert_eq!(received.right, initial.right);
    assert_eq!(second.observe().workers_received, 1);
    assert_balance(&first.shutdown().unwrap());
    assert_balance(&second.shutdown().unwrap());
}

#[test]
fn concurrent_local_reuse_handoff_and_close_preserve_bounded_custody() {
    const OWNERS: usize = 8;
    const ITERATIONS: usize = 128;
    let first = StructuredExecutor::for_test(4);
    let capacity = Arc::clone(&first.inner.capacity);
    let mut owners: Vec<_> = (1..OWNERS)
        .map(|_| first.sharing_capacity_for_test())
        .collect();
    owners.push(first);
    let weak: Vec<_> = owners
        .iter()
        .map(|owner| Arc::downgrade(&owner.inner))
        .collect();
    let start = Arc::new(std::sync::Barrier::new(OWNERS));
    let callers: Vec<_> = owners
        .into_iter()
        .enumerate()
        .map(|(identity, mut owner)| {
            let start = Arc::clone(&start);
            let capacity = Arc::clone(&capacity);
            thread::spawn(move || {
                start.wait();
                let mut dispatched = 0;
                for index in 0..ITERATIONS {
                    if identity % 2 == 0 && index == ITERATIONS / 2 {
                        owner.close_dispatch();
                    }
                    let result = owner
                        .handle()
                        .run(
                            &ExecutionControl::uncancelled(),
                            || 17,
                            move || (identity, index, 29),
                        )
                        .unwrap();
                    assert_eq!((result.left, result.right), (17, (identity, index, 29)));
                    dispatched += usize::from(result.dispatched);
                    assert!(capacity.reserved.load(Ordering::Acquire) <= 4);
                    assert!(lock(&capacity.custody).len() <= 4);
                }
                let stopped = owner.shutdown().unwrap();
                assert_eq!(stopped.completed_dispatches, dispatched as u64);
                assert_eq!(stopped.inline_fallbacks, (ITERATIONS - dispatched) as u64);
                assert_eq!(
                    (stopped.active_dispatches, stopped.remaining_workers),
                    (0, 0)
                );
                assert_balance(&stopped);
                stopped
            })
        })
        .collect();
    for caller in callers {
        assert_balance(&caller.join().unwrap());
    }
    assert_eq!(capacity.reserved.load(Ordering::Acquire), 0);
    assert!(lock(&capacity.custody).is_empty());
    assert!(weak.iter().all(|owner| owner.upgrade().is_none()));
}
