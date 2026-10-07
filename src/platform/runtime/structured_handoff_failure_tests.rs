use super::handoff_tests::{WAIT, assert_balance, pair};
use super::*;

#[test]
fn failure_first_observed_after_handoff_keeps_the_recipient_as_join_owner() {
    let mut first = StructuredExecutor::for_test(1);
    let mut second = first.sharing_capacity_for_test();
    let mut third = first.sharing_capacity_for_test();
    let reservation = first.inner.capacity.reserve().unwrap();
    let (sender, receiver) = mpsc::sync_channel::<Command>(1);
    let (disconnected, wait_disconnected) = mpsc::sync_channel(1);
    let (finish, wait_finish) = mpsc::sync_channel(1);
    let thread = thread::spawn(move || {
        drop(receiver);
        disconnected.send(()).unwrap();
        wait_finish.recv_timeout(WAIT).unwrap();
    });
    wait_disconnected.recv_timeout(WAIT).unwrap();
    // Inject a mailbox failure after eligibility but before the next submission.
    // The thread remains live: is_finished cannot predict this late failure.
    assert!(!thread.is_finished());
    {
        let mut custody = lock(&first.inner.capacity.custody);
        let mut state = lock(&first.inner.state);
        let slot = state.retain_worker(Worker {
            sender,
            thread,
            available: true,
            _reservation: reservation,
        });
        state.observation.workers_started = 1;
        custody.push(WorkerCustody::new(&first.inner, slot));
    }
    let executions = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&executions);
    let payload = vec![17_u8, 29, 137];
    let pointer = payload.as_ptr() as usize;
    let result = second
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
    assert!(!result.dispatched);
    assert_eq!(result.right, [17, 29, 137]);
    assert_eq!(result.right.as_ptr() as usize, pointer);
    assert_eq!(executions.load(Ordering::Acquire), 1);
    assert_eq!(first.observe().workers_handed_off, 1);
    assert_eq!(second.observe().workers_received, 1);
    assert_eq!(second.observe().active_dispatches, 0);
    assert_eq!(second.observe().completed_dispatches, 0);
    // Custody remains recorded, but a disconnected mailbox is never available.
    assert_eq!(lock(&first.inner.capacity.custody).len(), 1);
    assert!(!pair(&third).dispatched);
    let stopped = first.shutdown().unwrap();
    assert_eq!((stopped.joined_workers, stopped.remaining_workers), (0, 0));
    assert_balance(&stopped);
    assert_eq!(second.inner.capacity.reserved.load(Ordering::Acquire), 1);
    finish.send(()).unwrap();
    let stopped = second.shutdown().unwrap();
    assert_eq!((stopped.joined_workers, stopped.remaining_workers), (1, 0));
    assert_balance(&stopped);
    assert_eq!(second.inner.capacity.reserved.load(Ordering::Acquire), 0);
    assert!(pair(&third).dispatched);
    assert_balance(&third.shutdown().unwrap());
}
