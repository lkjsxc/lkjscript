//! Independent expected errors exercise the common production/reference join owner.
use super::*;
use crate::platform::runtime::structured::StructuredExecutor;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

fn cancelled() -> ExecutionError {
    let control = ExecutionControl::uncancelled();
    control.cancel();
    control.check().unwrap_err()
}

fn deadline() -> ExecutionError {
    ExecutionControl::with_deadline(Instant::now())
        .check()
        .unwrap_err()
}

#[test]
fn deadline_is_not_masked_by_sibling_scope_cancellation_in_either_position() {
    let original = deadline();
    assert_eq!(original.class, ExecutionFailureClass::Cancelled);
    assert_eq!(original.code, "execution_deadline");
    for (left, right) in [
        (cancelled(), original.clone()),
        (original.clone(), cancelled()),
    ] {
        assert_eq!(
            results::<(), ()>(Err(left), Err(right)),
            Err(original.clone())
        );
    }
}

#[test]
fn non_cancellation_failures_retain_precedence_and_the_original_error() {
    for class in [
        ExecutionFailureClass::Trap,
        ExecutionFailureClass::Capability,
        ExecutionFailureClass::PossibleVisibility,
        ExecutionFailureClass::Resource,
        ExecutionFailureClass::Infrastructure,
    ] {
        let original = ExecutionError::new(class, "original_child_failure", "retain every field");
        for signal in [cancelled(), deadline()] {
            for (left, right) in [
                (signal.clone(), original.clone()),
                (original.clone(), signal),
            ] {
                assert_eq!(
                    results::<(), ()>(Err(left), Err(right)),
                    Err(original.clone())
                );
            }
        }
    }
}

#[test]
fn nested_joins_do_not_erase_the_original_deadline() {
    let original = deadline();
    let inner = results::<(), ()>(Err(cancelled()), Err(original.clone()));
    let outer = results::<(), ((), ())>(Err(cancelled()), inner);
    assert_eq!(outer.unwrap_err(), original);
}

#[test]
fn worker_deadline_survives_joined_caller_cancellation() {
    let executor = StructuredExecutor::for_test(1);
    let control = ExecutionControl::uncancelled();
    let original = deadline();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let child_control = control.clone();
    let child_original = original.clone();
    let pair = run(
        &executor.handle(),
        &control,
        || {
            receiver.recv_timeout(Duration::from_secs(5)).unwrap();
            control.check()
        },
        move || {
            // Inject the original operational failure, then signal its sibling.
            // Synchronization fixes this causal order without wall-clock timing.
            child_control.cancel();
            sender.send(()).unwrap();
            Err::<(), _>(child_original)
        },
    )
    .unwrap();
    assert!(pair.dispatched);
    assert_eq!(executor.observe().active_dispatches, 0);
    assert_eq!(results(pair.left, pair.right).unwrap_err(), original);
}

#[test]
fn saturated_worker_fallback_retains_the_callers_deadline() {
    let executor = StructuredExecutor::for_test(0);
    let control = ExecutionControl::uncancelled();
    let original = deadline();
    let child_control = control.clone();
    let pair = run(
        &executor.handle(),
        &control,
        || {
            control.cancel();
            Err::<(), _>(original.clone())
        },
        move || child_control.check(),
    )
    .unwrap();
    assert!(!pair.dispatched);
    assert_eq!(executor.observe().active_dispatches, 0);
    assert_eq!(results(pair.left, pair.right).unwrap_err(), original);
}

#[test]
fn plain_cancellation_and_successful_pairs_keep_their_meaning() {
    assert_eq!(results(Ok::<_, ExecutionError>(17), Ok(29)), Ok((17, 29)));
    let signal = cancelled();
    assert_eq!(
        results::<(), ()>(Err(signal.clone()), Err(signal.clone())),
        Err(signal)
    );
}

struct Dropped<'a>(&'a AtomicUsize);
impl Drop for Dropped<'_> {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::AcqRel);
    }
}

#[test]
fn failure_releases_a_successful_siblings_value_exactly_once() {
    let drops = AtomicUsize::new(0);
    let original = deadline();
    let left = results::<_, ()>(Ok(Dropped(&drops)), Err(original.clone()));
    assert!(matches!(left, Err(ref error) if error == &original));
    assert_eq!(drops.load(Ordering::Acquire), 1);
    let right = results::<(), _>(Err(original.clone()), Ok(Dropped(&drops)));
    assert!(matches!(right, Err(ref error) if error == &original));
    assert_eq!(drops.load(Ordering::Acquire), 2);
}
