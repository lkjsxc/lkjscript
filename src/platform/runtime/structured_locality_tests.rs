use super::super::*;
use super::{WAIT, assert_balance, pair};

#[test]
fn warm_local_execution_does_not_wait_for_the_shared_catalogue() {
    let mut executor = StructuredExecutor::for_test(1);
    let initial = pair(&executor);
    assert!(initial.dispatched);
    let handle = executor.handle();
    let (started, wait_started) = mpsc::sync_channel(1);
    let (finished, wait_finished) = mpsc::sync_channel(1);
    let catalogue = lock(&executor.inner.capacity.custody);
    let caller = thread::spawn(move || {
        started.send(()).unwrap();
        let result = handle.run(
            &ExecutionControl::uncancelled(),
            || 29,
            || thread::current().id(),
        );
        finished.send(result).unwrap();
    });
    wait_started.recv_timeout(WAIT).unwrap();
    let while_locked = wait_finished.recv_timeout(WAIT);
    // Release the deliberate obstruction and join before asserting, including on
    // the predecessor. A failing regression must not leave a blocked owner.
    drop(catalogue);
    caller.join().unwrap();
    assert!(
        while_locked.is_ok(),
        "warm local work waited for the shared catalogue"
    );
    let result = while_locked.unwrap().unwrap();
    assert!(result.dispatched);
    assert_eq!(result.left, 29);
    assert_eq!(result.right, initial.right);
    let stopped = executor.shutdown().unwrap();
    assert_eq!(stopped.workers_started, 1);
    assert_eq!(stopped.completed_dispatches, 2);
    assert_balance(&stopped);
}
