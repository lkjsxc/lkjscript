//! Independent synchronization proves custody, reuse and joined lifecycle.
use super::*;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(5);

fn wait_cancelled(control: &ExecutionControl) {
    let deadline = Instant::now() + WAIT;
    while !control.is_cancelled() && Instant::now() < deadline {
        thread::yield_now();
    }
    assert!(control.is_cancelled());
}

struct Dropped(Arc<AtomicUsize>);

impl Drop for Dropped {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::AcqRel);
    }
}

#[test]
fn no_worker_starts_without_a_dispatch_and_shutdown_is_idempotent() {
    let mut executor = StructuredExecutor::for_test(1);
    let handle = executor.handle();
    let cold = executor.observe();
    assert!(cold.dispatch_open);
    assert_eq!(cold.workers_started, 0);
    assert_eq!(cold.remaining_workers, 0);
    let receipt = executor.shutdown().unwrap();
    assert!(!receipt.dispatch_open);
    assert_eq!(receipt.remaining_workers, 0);
    assert_eq!(receipt.joined_workers, 0);
    assert_eq!(executor.shutdown().unwrap(), receipt);
    assert_eq!(handle.observe(), receipt);
    let pair = handle
        .run(&ExecutionControl::uncancelled(), || 17, || 29)
        .unwrap();
    assert!(!pair.dispatched);
    assert_eq!((pair.left, pair.right), (17, 29));
    assert_eq!(handle.observe().inline_fallbacks, 1);
}

#[test]
fn successive_jobs_reuse_the_same_worker_and_join_every_started_thread() {
    let mut executor = StructuredExecutor::for_test(1);
    let handle = executor.handle();
    let caller = thread::current().id();
    let mut observed = Vec::new();
    for expected in [17, -137, 29] {
        let pair = handle
            .run(
                &ExecutionControl::uncancelled(),
                || expected,
                move || (expected, thread::current().id()),
            )
            .unwrap();
        assert!(pair.dispatched);
        assert_eq!(pair.left, pair.right.0);
        assert_ne!(pair.right.1, caller);
        observed.push(pair.right.1);
    }
    assert!(observed.iter().all(|worker| *worker == observed[0]));
    let warm = executor.observe();
    assert_eq!(warm.workers_started, 1);
    assert_eq!(warm.active_dispatches, 0);
    assert_eq!(warm.maximum_active_dispatches, 1);
    assert_eq!(warm.completed_dispatches, 3);
    assert_eq!(warm.inline_fallbacks, 0);
    let receipt = executor.shutdown().unwrap();
    assert_eq!(receipt.workers_started, 1);
    assert_eq!(receipt.joined_workers, 1);
    assert_eq!(receipt.remaining_workers, 0);
}

#[test]
fn zero_capacity_executes_both_children_on_the_callers_thread() {
    let mut executor = StructuredExecutor::for_test(0);
    let caller = thread::current().id();
    let pair = executor
        .handle()
        .run(
            &ExecutionControl::uncancelled(),
            || thread::current().id(),
            || thread::current().id(),
        )
        .unwrap();
    assert!(!pair.dispatched);
    assert_eq!(pair.left, caller);
    assert_eq!(pair.right, caller);
    let receipt = executor.shutdown().unwrap();
    assert_eq!(receipt.workers_started, 0);
    assert_eq!(receipt.completed_dispatches, 0);
    assert_eq!(receipt.inline_fallbacks, 1);
}

#[test]
fn a_completed_child_holds_its_slot_until_the_caller_receives_its_result() {
    let executor = StructuredExecutor::for_test(1);
    let handle = executor.handle();
    let (finished, wait_finished) = mpsc::sync_channel(1);
    let caller = thread::current().id();
    let pair = handle
        .run(
            &ExecutionControl::uncancelled(),
            || {
                wait_finished.recv_timeout(WAIT).unwrap();
                let held = handle.observe();
                assert_eq!(held.active_dispatches, 1);
                assert_eq!(held.completed_dispatches, 0);
                let nested = handle
                    .run(
                        &ExecutionControl::uncancelled(),
                        || thread::current().id(),
                        || thread::current().id(),
                    )
                    .unwrap();
                assert!(!nested.dispatched);
                assert_eq!(nested.left, caller);
                assert_eq!(nested.right, caller);
                17
            },
            move || {
                finished.send(()).unwrap();
                29
            },
        )
        .unwrap();
    assert!(pair.dispatched);
    assert_eq!((pair.left, pair.right), (17, 29));
    let observation = executor.observe();
    assert_eq!(observation.completed_dispatches, 1);
    assert_eq!(observation.inline_fallbacks, 1);
}

#[test]
fn nested_worker_execution_falls_back_without_waiting_for_its_own_slot() {
    let executor = StructuredExecutor::for_test(1);
    let handle = executor.handle();
    let child_handle = handle.clone();
    let outer = handle
        .run(
            &ExecutionControl::uncancelled(),
            || 17,
            move || {
                let caller = thread::current().id();
                let nested = child_handle
                    .run(
                        &ExecutionControl::uncancelled(),
                        || thread::current().id(),
                        || thread::current().id(),
                    )
                    .unwrap();
                assert!(!nested.dispatched);
                assert_eq!(nested.left, caller);
                assert_eq!(nested.right, caller);
                29
            },
        )
        .unwrap();
    assert_eq!((outer.left, outer.right), (17, 29));
    assert_eq!(executor.observe().maximum_active_dispatches, 1);
    assert_eq!(executor.observe().inline_fallbacks, 1);
}

#[test]
fn caller_unwind_cancels_and_receives_child_cleanup_before_releasing_the_slot() {
    let executor = StructuredExecutor::for_test(1);
    let handle = executor.handle();
    let control = ExecutionControl::uncancelled();
    let child_control = control.clone();
    let child_handle = handle.clone();
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = Dropped(Arc::clone(&drops));
    let child_drops = Arc::clone(&drops);
    let (started, wait_started) = mpsc::sync_channel(1);
    let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = handle.run(
            &control,
            || {
                wait_started.recv_timeout(WAIT).unwrap();
                panic!("controlled caller unwind");
            },
            move || {
                started.send(()).unwrap();
                wait_cancelled(&child_control);
                assert_eq!(child_drops.load(Ordering::Acquire), 0);
                assert_eq!(child_handle.observe().active_dispatches, 1);
                owner
            },
        );
    }));
    assert!(unwind.is_err());
    assert!(control.is_cancelled());
    assert_eq!(drops.load(Ordering::Acquire), 1);
    assert_eq!(executor.observe().active_dispatches, 0);
    assert_eq!(executor.observe().completed_dispatches, 1);
    let retry = handle
        .run(&ExecutionControl::uncancelled(), || 17, || 29)
        .unwrap();
    assert!(retry.dispatched);
    assert_eq!((retry.left, retry.right), (17, 29));
    assert_eq!(executor.observe().workers_started, 1);
}

#[test]
fn child_panic_cancels_the_caller_and_the_worker_accepts_a_fresh_job() {
    let executor = StructuredExecutor::for_test(1);
    let handle = executor.handle();
    let control = ExecutionControl::uncancelled();
    let drops = Arc::new(AtomicUsize::new(0));
    let child_owner = Dropped(Arc::clone(&drops));
    let caller_owner = Dropped(Arc::clone(&drops));
    let (started, wait_started) = mpsc::sync_channel(1);
    let outcome = handle.run(
        &control,
        || {
            wait_started.recv_timeout(WAIT).unwrap();
            wait_cancelled(&control);
            caller_owner
        },
        move || {
            let _owner = child_owner;
            started.send(()).unwrap();
            panic!("controlled child unwind");
        },
    );
    let error = match outcome {
        Err(error) => error,
        Ok(_) => panic!("panicking child returned success"),
    };
    assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
    assert_eq!(error.code, "normalized_parallel_worker");
    assert_eq!(drops.load(Ordering::Acquire), 2);
    assert_eq!(executor.observe().active_dispatches, 0);
    let fresh = ExecutionControl::uncancelled();
    let pair = handle
        .run(&fresh, || 17, || (29, thread::current().id()))
        .unwrap();
    assert!(pair.dispatched);
    assert!(!fresh.is_cancelled());
    assert_eq!((pair.left, pair.right.0), (17, 29));
    assert_eq!(executor.observe().workers_started, 1);
    assert_eq!(executor.observe().completed_dispatches, 2);
}

#[test]
fn refused_thread_creation_and_submission_run_the_intact_owner_inline() {
    for refuse_creation in [true, false] {
        let mut executor = StructuredExecutor::for_test(1);
        {
            let mut state = lock(&executor.inner.state);
            state.refuse_next_worker_start = refuse_creation;
            state.refuse_next_submission = !refuse_creation;
        }
        let payload = vec![17_u8, 29, 137];
        let pointer = payload.as_ptr() as usize;
        let pair = executor
            .handle()
            .run(&ExecutionControl::uncancelled(), || 17, move || payload)
            .unwrap();
        assert!(!pair.dispatched);
        assert_eq!(pair.right, [17, 29, 137]);
        assert_eq!(pair.right.as_ptr() as usize, pointer);
        let refused = executor.observe();
        assert_eq!(refused.completed_dispatches, 0);
        assert_eq!(refused.active_dispatches, 0);
        assert_eq!(refused.inline_fallbacks, 1);
        assert_eq!(refused.workers_started, u64::from(!refuse_creation));
        assert_eq!(
            executor.inner.capacity.reserved.load(Ordering::Acquire),
            usize::from(!refuse_creation)
        );
        let recovered = executor
            .handle()
            .run(&ExecutionControl::uncancelled(), || 17, || 29)
            .unwrap();
        assert!(recovered.dispatched);
        let receipt = executor.shutdown().unwrap();
        assert_eq!(receipt.workers_started, 1);
        assert_eq!(receipt.joined_workers, 1);
        assert_eq!(executor.inner.capacity.reserved.load(Ordering::Acquire), 0);
    }
}

#[test]
fn worker_creation_refusal_preserves_a_previously_started_workers_join_owner() {
    let mut executor = StructuredExecutor::for_test(2);
    let handle = executor.handle();
    let capacity = Arc::clone(&executor.inner.capacity);
    let pair = handle
        .run(
            &ExecutionControl::uncancelled(),
            || {
                lock(&executor.inner.state).refuse_next_worker_start = true;
                let nested = handle
                    .run(&ExecutionControl::uncancelled(), || 17, || 29)
                    .unwrap();
                assert!(!nested.dispatched);
                assert_eq!((nested.left, nested.right), (17, 29));
                assert_eq!(capacity.reserved.load(Ordering::Acquire), 1);
                17
            },
            || 29,
        )
        .unwrap();
    assert!(pair.dispatched);
    assert_eq!((pair.left, pair.right), (17, 29));
    let receipt = executor.shutdown().unwrap();
    assert_eq!(receipt.workers_started, 1);
    assert_eq!(receipt.joined_workers, 1);
    assert_eq!(receipt.remaining_workers, 0);
    assert_eq!(capacity.reserved.load(Ordering::Acquire), 0);
}

#[test]
fn closing_owner_retains_physical_capacity_until_its_threads_are_joined() {
    let mut first = StructuredExecutor::for_test(1);
    let capacity = Arc::clone(&first.inner.capacity);
    let mut second = first.sharing_capacity_for_test();
    let initial = first
        .handle()
        .run(&ExecutionControl::uncancelled(), || 17, || 29)
        .unwrap();
    assert!(initial.dispatched);
    first.close_dispatch();
    let unavailable = second
        .handle()
        .run(&ExecutionControl::uncancelled(), || 17, || 29)
        .unwrap();
    assert!(!unavailable.dispatched);
    assert_eq!(capacity.reserved.load(Ordering::Acquire), 1);
    first.shutdown().unwrap();
    assert_eq!(capacity.reserved.load(Ordering::Acquire), 0);
    let available = second
        .handle()
        .run(&ExecutionControl::uncancelled(), || 17, || 29)
        .unwrap();
    assert!(available.dispatched);
    assert_eq!(capacity.reserved.load(Ordering::Acquire), 1);
    second.shutdown().unwrap();
    assert_eq!(capacity.reserved.load(Ordering::Acquire), 0);
}

#[test]
fn shutdown_waits_for_receipt_custody_and_does_not_cancel_invocations() {
    let executor = StructuredExecutor::for_test(1);
    let handle = executor.handle();
    let observe = handle.clone();
    let control = ExecutionControl::uncancelled();
    let invocation_control = control.clone();
    let (entered, wait_entered) = mpsc::sync_channel(1);
    let (release, wait_release) = mpsc::sync_channel(1);
    let invocation = thread::spawn(move || {
        handle
            .run(
                &invocation_control,
                || {
                    wait_release.recv_timeout(WAIT).unwrap();
                    17
                },
                move || {
                    entered.send(()).unwrap();
                    29
                },
            )
            .unwrap()
    });
    wait_entered.recv_timeout(WAIT).unwrap();
    executor.close_dispatch();
    assert!(!observe.observe().dispatch_open);
    let (stopped, wait_stopped) = mpsc::sync_channel(1);
    let shutdown = thread::spawn(move || {
        let mut executor = executor;
        let receipt = executor.shutdown().unwrap();
        stopped.send(receipt).unwrap();
    });
    assert!(wait_stopped.try_recv().is_err());
    assert!(!control.is_cancelled());
    release.send(()).unwrap();
    let pair = invocation.join().unwrap();
    assert_eq!((pair.left, pair.right), (17, 29));
    let receipt = wait_stopped.recv_timeout(WAIT).unwrap();
    shutdown.join().unwrap();
    assert!(!control.is_cancelled());
    assert_eq!(receipt.active_dispatches, 0);
    assert_eq!(receipt.completed_dispatches, 1);
    assert_eq!(receipt.remaining_workers, 0);
    assert_eq!(receipt.joined_workers, 1);
}

#[test]
fn owner_drop_joins_workers_even_with_outstanding_dispatch_handles() {
    let executor = StructuredExecutor::for_test(1);
    let handle = executor.handle();
    let pair = handle
        .run(&ExecutionControl::uncancelled(), || 17, || 29)
        .unwrap();
    assert!(pair.dispatched);
    drop(executor);
    let observation = handle.observe();
    assert!(!observation.dispatch_open);
    assert_eq!(observation.remaining_workers, 0);
    assert_eq!(observation.joined_workers, 1);
    let later = handle
        .run(&ExecutionControl::uncancelled(), || 31, || 43)
        .unwrap();
    assert!(!later.dispatched);
    assert_eq!((later.left, later.right), (31, 43));
}
