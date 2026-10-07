use super::*;

pub(super) const WAIT: std::time::Duration = std::time::Duration::from_secs(5);

pub(super) fn pair(executor: &StructuredExecutor) -> StructuredPair<i64, thread::ThreadId> {
    executor
        .handle()
        .run(
            &ExecutionControl::uncancelled(),
            || 17,
            || thread::current().id(),
        )
        .unwrap()
}

pub(super) fn assert_balance(observation: &StructuredExecutorObservation) {
    assert_eq!(
        observation.workers_started + observation.workers_received,
        observation.joined_workers
            + observation.workers_handed_off
            + observation.remaining_workers as u64,
        "physical worker custody did not balance: {observation:?}"
    );
}

#[test]
fn idle_worker_moves_between_live_owners_without_a_new_thread() {
    let mut first = StructuredExecutor::for_test(1);
    let mut second = first.sharing_capacity_for_test();
    let first_pair = pair(&first);
    assert!(first_pair.dispatched);
    let second_pair = pair(&second);
    assert!(
        second_pair.dispatched,
        "an idle foreign owner retained all physical capacity"
    );
    assert_eq!(first_pair.right, second_pair.right);
    assert_eq!(first_pair.left + second_pair.left, 34);
    assert_eq!(
        first.observe().workers_started + second.observe().workers_started,
        1
    );
    assert_eq!(first.observe().workers_handed_off, 1);
    assert_eq!(second.observe().workers_received, 1);
    assert_balance(&first.shutdown().unwrap());
    assert_balance(&second.shutdown().unwrap());
    assert_eq!(first.inner.capacity.reserved.load(Ordering::Acquire), 0);
}

#[test]
fn repeated_handoffs_reuse_bounded_slots_and_leave_no_retention_root() {
    let first = StructuredExecutor::for_test(1);
    let second = first.sharing_capacity_for_test();
    let capacity = Arc::clone(&first.inner.capacity);
    let weak_first = Arc::downgrade(&first.inner);
    let weak_second = Arc::downgrade(&second.inner);
    let mut thread = None;
    for index in 0..1000 {
        let executor = if index % 2 == 0 { &first } else { &second };
        let result = pair(executor);
        assert!(result.dispatched);
        assert_eq!(result.left, 17);
        assert_eq!(*thread.get_or_insert(result.right), result.right);
        assert_eq!(capacity.reserved.load(Ordering::Acquire), 1);
        assert_eq!(lock(&capacity.idle).len(), 1);
        assert!(lock(&first.inner.state).workers.len() <= 1);
        assert!(lock(&second.inner.state).workers.len() <= 1);
        assert_balance(&first.observe());
        assert_balance(&second.observe());
    }
    let observations = [first.observe(), second.observe()];
    assert_eq!(
        observations.iter().map(|o| o.workers_started).sum::<u64>(),
        1
    );
    assert_eq!(
        observations.iter().map(|o| o.workers_received).sum::<u64>(),
        999
    );
    assert_eq!(
        observations
            .iter()
            .map(|o| o.completed_dispatches)
            .sum::<u64>(),
        1000
    );
    println!("worker_handoff: dispatches=1000 starts=1 handoffs=999 peak_physical=1");
    drop(first);
    assert!(weak_first.upgrade().is_none());
    drop(second);
    assert!(weak_second.upgrade().is_none());
    assert_eq!(capacity.reserved.load(Ordering::Acquire), 0);
    assert!(lock(&capacity.idle).is_empty());
}

#[test]
fn local_idle_worker_is_preferred_over_a_foreign_idle_worker() {
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
    let first_thread = initial.right;
    let second_thread = initial.left.right;
    assert_ne!(first_thread, second_thread);
    for _ in 0..8 {
        assert_eq!(pair(&first).right, first_thread);
        assert_eq!(pair(&second).right, second_thread);
    }
    assert_eq!(
        first.observe().workers_received + second.observe().workers_received,
        0
    );
    assert_balance(&first.shutdown().unwrap());
    assert_balance(&second.shutdown().unwrap());
}

#[test]
fn completed_child_cannot_be_lent_before_its_result_custody_is_joined() {
    let first = StructuredExecutor::for_test(1);
    let second = first.sharing_capacity_for_test();
    let (finished, wait_finished) = mpsc::sync_channel(1);
    let initial = first
        .handle()
        .run(
            &ExecutionControl::uncancelled(),
            || {
                wait_finished.recv_timeout(WAIT).unwrap();
                assert_eq!(first.observe().active_dispatches, 1);
                let blocked = pair(&second);
                assert!(!blocked.dispatched);
                assert_eq!(blocked.right, thread::current().id());
                assert_eq!(second.observe().workers_received, 0);
                17
            },
            move || {
                finished.send(()).unwrap();
                thread::current().id()
            },
        )
        .unwrap();
    assert!(initial.dispatched);
    let available = pair(&second);
    assert!(available.dispatched);
    assert_eq!(initial.right, available.right);
}

#[test]
fn nested_foreign_owner_never_waits_for_its_ancestor_worker() {
    let first = StructuredExecutor::for_test(1);
    let second = first.sharing_capacity_for_test();
    let child = second.handle();
    let initial = first
        .handle()
        .run(
            &ExecutionControl::uncancelled(),
            || 17,
            move || {
                let thread = thread::current().id();
                let nested = child
                    .run(
                        &ExecutionControl::uncancelled(),
                        || 29,
                        || thread::current().id(),
                    )
                    .unwrap();
                assert!(!nested.dispatched);
                assert_eq!(nested.right, thread);
                thread
            },
        )
        .unwrap();
    let available = pair(&second);
    assert!(available.dispatched);
    assert_eq!(initial.right, available.right);
}
