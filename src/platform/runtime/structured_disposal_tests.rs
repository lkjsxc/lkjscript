use super::handoff_tests::{assert_balance, pair};
use super::*;

#[test]
fn discarded_result_finishes_cleanup_before_another_owner_can_claim_its_worker() {
    struct ResultProbe {
        other: StructuredExecutorHandle,
        drops: Arc<AtomicUsize>,
    }
    impl Drop for ResultProbe {
        fn drop(&mut self) {
            let nested = self
                .other
                .run(&ExecutionControl::uncancelled(), || 17, || 29)
                .unwrap();
            assert!(
                !nested.dispatched,
                "result cleanup released its worker early"
            );
            assert_eq!((nested.left, nested.right), (17, 29));
            self.drops.fetch_add(1, Ordering::AcqRel);
        }
    }
    let mut first = StructuredExecutor::for_test(1);
    let mut second = first.sharing_capacity_for_test();
    let thread = pair(&first).right;
    let control = ExecutionControl::uncancelled();
    let drops = Arc::new(AtomicUsize::new(0));
    let other = second.handle();
    let count = Arc::clone(&drops);
    let Dispatch::Accepted(child) = first.handle().dispatch(&control, move || ResultProbe {
        other,
        drops: count,
    }) else {
        panic!("expected accepted result custody");
    };
    drop(child);
    assert_eq!(drops.load(Ordering::Acquire), 1);
    assert!(control.is_cancelled());
    assert_eq!(second.observe().workers_received, 0);
    let fresh = pair(&second);
    assert!(fresh.dispatched);
    assert_eq!(fresh.right, thread);
    assert_balance(&first.shutdown().unwrap());
    assert_balance(&second.shutdown().unwrap());
}

#[test]
fn result_cleanup_unwind_cannot_strand_a_receipt_or_report_successful_shutdown() {
    struct PanicOnDrop(Arc<AtomicUsize>);
    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::AcqRel);
            panic!("controlled unreturned-result cleanup failure");
        }
    }
    let mut first = StructuredExecutor::for_test(1);
    let mut second = first.sharing_capacity_for_test();
    let control = ExecutionControl::uncancelled();
    let drops = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&drops);
    let Dispatch::Accepted(child) = first
        .handle()
        .dispatch(&control, move || PanicOnDrop(count))
    else {
        panic!("expected accepted result custody");
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(child)));
    assert!(result.is_err());
    assert!(control.is_cancelled());
    assert_eq!(drops.load(Ordering::Acquire), 1);
    let stranded = first.observe().active_dispatches;
    assert!(!pair(&second).dispatched);
    assert_eq!(first.observe().workers_handed_off, 0);
    // Test-harness-only recovery keeps a broken predecessor from hanging the
    // whole suite. Record the defect first; the assertions below still reject it.
    // Correct implementations never enter this branch.
    if stranded != 0 {
        lock(&first.inner.state).observation.active_dispatches = 0;
    }
    let stopped = first.shutdown();
    assert_eq!(
        stranded, 0,
        "result cleanup unwind stranded an accepted receipt"
    );
    assert!(
        stopped.is_err(),
        "failed cleanup was reported as successful shutdown"
    );
    let error = stopped.unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
    assert_eq!(error.code, "normalized_parallel_worker");
    assert_eq!(first.shutdown().unwrap_err(), error);
    assert_eq!(first.observe().remaining_workers, 0);
    assert_balance(&first.observe());
    assert!(pair(&second).dispatched);
    assert_balance(&second.shutdown().unwrap());
}
