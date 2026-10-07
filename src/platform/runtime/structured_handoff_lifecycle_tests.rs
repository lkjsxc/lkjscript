use super::handoff_tests::{WAIT, assert_balance, pair};
use super::*;

#[test]
fn former_owner_shutdown_does_not_wait_for_or_cancel_transferred_work() {
    let mut first = StructuredExecutor::for_test(1);
    let mut second = first.sharing_capacity_for_test();
    let initial = pair(&first);
    let control = ExecutionControl::uncancelled();
    let (release, wait_release) = mpsc::sync_channel(1);
    let handle = second.handle();
    let Dispatch::Accepted(mut child) = handle.dispatch(&control, move || {
        wait_release.recv_timeout(WAIT).unwrap();
        thread::current().id()
    }) else {
        panic!("idle worker was not transferred");
    };
    let (stopped, wait_stopped) = mpsc::sync_channel(1);
    let shutdown = thread::spawn(move || stopped.send(first.shutdown().unwrap()).unwrap());
    let receipt = wait_stopped.recv_timeout(WAIT).unwrap();
    assert_eq!(
        (
            receipt.workers_handed_off,
            receipt.joined_workers,
            receipt.remaining_workers
        ),
        (1, 0, 0)
    );
    assert_balance(&receipt);
    assert_eq!(second.observe().active_dispatches, 1);
    assert!(!control.is_cancelled());
    release.send(()).unwrap();
    assert_eq!(child.receive().unwrap(), initial.right);
    shutdown.join().unwrap();
    let receipt = second.shutdown().unwrap();
    assert_eq!(
        (
            receipt.workers_started,
            receipt.workers_received,
            receipt.joined_workers
        ),
        (0, 1, 1)
    );
    assert_balance(&receipt);
}

#[test]
fn handoff_submission_refusal_preserves_one_unexecuted_payload_owner() {
    let first = StructuredExecutor::for_test(1);
    let mut second = first.sharing_capacity_for_test();
    let initial = pair(&first);
    lock(&second.inner.state).refuse_next_submission = true;
    let executions = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&executions);
    let payload = vec![17_u8, 29, 137];
    let pointer = payload.as_ptr() as usize;
    let refused = second
        .handle()
        .run(
            &ExecutionControl::uncancelled(),
            || 17,
            move || {
                count.fetch_add(1, Ordering::AcqRel);
                payload
            },
        )
        .unwrap();
    assert!(!refused.dispatched);
    assert_eq!(refused.right, [17, 29, 137]);
    assert_eq!(refused.right.as_ptr() as usize, pointer);
    assert_eq!(executions.load(Ordering::Acquire), 1);
    let observed = second.observe();
    assert_eq!(
        (
            observed.workers_started,
            observed.workers_received,
            observed.active_dispatches
        ),
        (0, 1, 0)
    );
    assert_eq!(observed.completed_dispatches, 0);
    assert_eq!(pair(&second).right, initial.right);
    assert_balance(&second.shutdown().unwrap());
}

#[test]
fn cancelled_admission_and_caught_child_failure_do_not_cross_owner_handoffs() {
    let first = StructuredExecutor::for_test(1);
    let second = first.sharing_capacity_for_test();
    let initial = pair(&first);
    let cancelled = ExecutionControl::uncancelled();
    cancelled.cancel();
    assert_eq!(
        second
            .handle()
            .run(&cancelled, || 17, || 29)
            .unwrap_err()
            .code,
        "execution_cancelled"
    );
    assert_eq!(second.observe().workers_received, 0);
    let failed = ExecutionControl::uncancelled();
    assert_eq!(
        first
            .handle()
            .run(
                &failed,
                || 17,
                || -> i64 { panic!("controlled child failure before idle handoff") }
            )
            .unwrap_err()
            .code,
        "normalized_parallel_worker"
    );
    assert!(failed.is_cancelled());
    let fresh = ExecutionControl::uncancelled();
    let recovered = second
        .handle()
        .run(&fresh, || 29, || thread::current().id())
        .unwrap();
    assert!(recovered.dispatched);
    assert_eq!(recovered.right, initial.right);
    assert!(!fresh.is_cancelled());
    assert_balance(&first.observe());
    assert_balance(&second.observe());
}

#[test]
fn missing_completion_receipt_keeps_the_worker_with_its_cleanup_owner() {
    struct FailingPanicPayload;
    impl Drop for FailingPanicPayload {
        fn drop(&mut self) {
            panic!("controlled panic-payload cleanup failure");
        }
    }
    let mut first = StructuredExecutor::for_test(1);
    let second = first.sharing_capacity_for_test();
    let failed = ExecutionControl::uncancelled();
    let error = first
        .handle()
        .run(
            &failed,
            || 17,
            || -> () {
                std::panic::panic_any(FailingPanicPayload);
            },
        )
        .unwrap_err();
    assert_eq!(error.code, "normalized_parallel_worker");
    assert!(failed.is_cancelled());
    assert_eq!(first.observe().active_dispatches, 0);
    assert_eq!(lock(&first.inner.capacity.custody).len(), 1);
    assert!(
        lock(&first.inner.state)
            .workers
            .iter()
            .flatten()
            .all(|worker| !worker.available)
    );
    assert!(!pair(&second).dispatched);
    assert_eq!(first.observe().workers_handed_off, 0);
    assert_balance(&first.shutdown().unwrap());
    assert!(lock(&first.inner.capacity.custody).is_empty());
    assert!(pair(&second).dispatched);
}
