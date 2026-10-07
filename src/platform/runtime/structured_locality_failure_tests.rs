use super::handoff_tests::{WAIT, assert_balance, pair};
use super::*;

fn without_directory<R: Send + 'static>(
    executor: &StructuredExecutor,
    action: impl FnOnce(StructuredExecutorHandle) -> R + Send + 'static,
) -> R {
    let handle = executor.handle();
    let (started, wait_started) = mpsc::sync_channel(1);
    let (finished, wait_finished) = mpsc::sync_channel(1);
    let custody = lock(&executor.inner.capacity.custody);
    let caller = thread::spawn(move || {
        started.send(()).unwrap();
        assert!(finished.send(action(handle)).is_ok());
    });
    wait_started.recv_timeout(WAIT).unwrap();
    let result = wait_finished.recv_timeout(WAIT);
    drop(custody);
    caller.join().unwrap();
    assert!(result.is_ok(), "local completion waited for shared custody");
    result.unwrap()
}

#[test]
fn refused_local_submission_preserves_one_payload_without_directory_access() {
    let mut executor = StructuredExecutor::for_test(1);
    let initial = pair(&executor);
    lock(&executor.inner.state).refuse_next_submission = true;
    let executions = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&executions);
    let payload = vec![17_u8, 29, 137];
    let pointer = payload.as_ptr() as usize;
    let result = without_directory(&executor, move |handle| {
        handle
            .run(
                &ExecutionControl::uncancelled(),
                || 17,
                move || {
                    count.fetch_add(1, Ordering::AcqRel);
                    payload
                },
            )
            .unwrap()
    });
    assert!(!result.dispatched);
    assert_eq!(result.right, [17, 29, 137]);
    assert_eq!(result.right.as_ptr() as usize, pointer);
    assert_eq!(executions.load(Ordering::Acquire), 1);
    assert_eq!(pair(&executor).right, initial.right);
    let stopped = executor.shutdown().unwrap();
    assert_eq!(
        (stopped.completed_dispatches, stopped.inline_fallbacks),
        (2, 1)
    );
    assert_balance(&stopped);
}

struct Dropped(Arc<AtomicUsize>);
impl Drop for Dropped {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::AcqRel);
    }
}

#[test]
fn unwinding_parent_disposes_its_child_without_directory_access() {
    let mut executor = StructuredExecutor::for_test(1);
    let initial = pair(&executor);
    let drops = Arc::new(AtomicUsize::new(0));
    let child_drops = Arc::clone(&drops);
    let cancelled = without_directory(&executor, move |handle| {
        let control = ExecutionControl::uncancelled();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handle.run(
                &control,
                || -> () { panic!("controlled local parent unwind") },
                move || Dropped(child_drops),
            )
        }));
        assert!(outcome.is_err());
        control.check().is_err()
    });
    assert!(cancelled);
    assert_eq!(drops.load(Ordering::Acquire), 1);
    assert_eq!(executor.observe().active_dispatches, 0);
    assert_eq!(pair(&executor).right, initial.right);
    assert_balance(&executor.shutdown().unwrap());
}

struct FailedDisposal(Arc<AtomicUsize>);
impl Drop for FailedDisposal {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::AcqRel);
        panic!("controlled local result cleanup failure");
    }
}

#[test]
fn failed_disposal_returns_local_accounting_without_donating_the_worker() {
    let mut first = StructuredExecutor::for_test(1);
    let mut second = first.sharing_capacity_for_test();
    assert!(pair(&first).dispatched);
    let drops = Arc::new(AtomicUsize::new(0));
    let child_drops = Arc::clone(&drops);
    without_directory(&first, move |handle| {
        let control = ExecutionControl::uncancelled();
        let Dispatch::Accepted(child) =
            handle.dispatch(&control, move || FailedDisposal(child_drops))
        else {
            panic!("the controlled local worker was not admitted")
        };
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(child)));
        assert!(outcome.is_err());
        assert!(control.check().is_err());
    });
    assert_eq!(drops.load(Ordering::Acquire), 1);
    assert_eq!(first.observe().active_dispatches, 0);
    assert_eq!(lock(&first.inner.capacity.custody).len(), 1);
    assert!(!pair(&second).dispatched);
    assert_eq!(second.observe().workers_received, 0);
    assert!(first.shutdown().is_err());
    let stopped = first.observe();
    assert_eq!((stopped.remaining_workers, stopped.joined_workers), (0, 1));
    assert_balance(&stopped);
    assert!(pair(&second).dispatched);
    assert_balance(&second.shutdown().unwrap());
}
