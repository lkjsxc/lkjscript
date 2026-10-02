//! Bounded structured CPU execution. The caller always does useful work.
//! Additional worker capacity is never awaited, and a permit survives until join.
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

// This bounds native evaluator nesting, independently of cumulative execution fuel.
pub(super) const MAXIMUM_STRUCTURED_DEPTH: usize = 32;

struct Workers {
    active: AtomicUsize,
    maximum: usize,
}
struct Permit<'a>(&'a Workers);
struct CancelOnUnwind<'a> {
    control: &'a ExecutionControl,
    armed: bool,
}
impl Drop for CancelOnUnwind<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.control.cancel();
        }
    }
}
impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::AcqRel);
    }
}
impl Workers {
    fn try_acquire(&self) -> Option<Permit<'_>> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < self.maximum).then(|| active + 1)
            })
            .ok()
            .map(|_| Permit(self))
    }
}

pub(super) struct Pair<L, R> {
    pub(super) left: L,
    pub(super) right: R,
    pub(super) spawned: bool,
}

/// Prefer the originating failure over its sibling's cooperative cancellation.
pub(super) fn results<L, R>(
    left: Result<L, ExecutionError>,
    right: Result<R, ExecutionError>,
) -> Result<(L, R), ExecutionError> {
    match (left, right) {
        (Ok(left), Ok(right)) => Ok((left, right)),
        (Err(left), Err(right))
            if left.class == ExecutionFailureClass::Cancelled
                && right.class != ExecutionFailureClass::Cancelled =>
        {
            Err(right)
        }
        (Err(error), _) | (_, Err(error)) => Err(error),
    }
}

pub(super) fn include_value_work(
    value: &mut super::value::ValueWork,
    child: &super::value::ValueWork,
) {
    macro_rules! add { ($($field:ident),* $(,)?) => { $(value.$field = value.$field.saturating_add(child.$field);)* }; }
    add!(
        input_admission_nodes,
        raw_result_admission_nodes,
        capture_admission_nodes,
        constructor_child_visits,
        internal_guard_descendant_visits,
        classification_decisions,
        local_value_moves,
        local_value_copies
    );
    let value = &mut value.bytes;
    let child = &child.bytes;
    macro_rules! add_bytes { ($($field:ident),* $(,)?) => { $(value.$field = value.$field.saturating_add(child.$field);)* }; }
    add_bytes!(
        concatenations,
        empty_operand_reuses,
        in_place_appends,
        buffer_growths,
        fresh_buffers,
        payload_bytes_copied,
        requested_capacity_bytes
    );
}

pub(super) fn run<L, R: Send>(
    control: &ExecutionControl,
    left: impl FnOnce() -> L,
    right: impl FnOnce() -> R + Send,
) -> Result<Pair<L, R>, ExecutionError> {
    #[cfg(test)]
    if let Some(workers) = TEST_WORKERS.with(|slot| slot.borrow().clone()) {
        return run_with_workers(&workers, control, left, right);
    }
    static WORKERS: OnceLock<Workers> = OnceLock::new();
    let workers = WORKERS.get_or_init(|| Workers {
        active: AtomicUsize::new(0),
        maximum: std::thread::available_parallelism()
            .map_or(1, |n| n.get().saturating_sub(1).max(1)),
    });
    run_with_workers(workers, control, left, right)
}

#[cfg(test)]
thread_local! { static TEST_WORKERS: std::cell::RefCell<Option<std::sync::Arc<Workers>>> = const { std::cell::RefCell::new(None) }; }
#[cfg(test)]
pub(super) struct TestLane(Option<std::sync::Arc<Workers>>);
#[cfg(test)]
pub(super) fn isolated_test_lane() -> TestLane {
    TestLane(TEST_WORKERS.with(|slot| {
        slot.replace(Some(std::sync::Arc::new(Workers {
            active: AtomicUsize::new(0),
            maximum: 1,
        })))
    }))
}
#[cfg(test)]
impl Drop for TestLane {
    fn drop(&mut self) {
        TEST_WORKERS.with(|slot| {
            slot.replace(self.0.take());
        });
    }
}

fn run_with_workers<L, R: Send>(
    workers: &Workers,
    control: &ExecutionControl,
    left: impl FnOnce() -> L,
    right: impl FnOnce() -> R + Send,
) -> Result<Pair<L, R>, ExecutionError> {
    control.check()?;
    let Some(permit) = workers.try_acquire() else {
        return Ok(Pair {
            left: left(),
            right: right(),
            spawned: false,
        });
    };
    let result = std::thread::scope(|scope| {
        let mut caller_guard = CancelOnUnwind {
            control,
            armed: true,
        };
        let worker = std::thread::Builder::new()
            .name("lkjscript-structured".to_owned())
            .spawn_scoped(scope, move || {
                let mut guard = CancelOnUnwind {
                    control,
                    armed: true,
                };
                let result = right();
                guard.armed = false;
                result
            })
            .map_err(|_| {
                ExecutionError::resource(
                    "normalized_parallel_spawn",
                    "cannot allocate a structured worker",
                )
            })?;
        let left = left();
        let right = worker.join().map_err(|_| {
            control.cancel();
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "normalized_parallel_worker",
                "structured worker terminated unexpectedly",
            )
        })?;
        caller_guard.armed = false;
        Ok(Pair {
            left,
            right,
            spawned: true,
        })
    });
    drop(permit);
    result
}

#[cfg(test)]
#[allow(
    clippy::panic,
    reason = "caught unwind verifies joined worker ownership"
)]
mod tests {
    use super::*;
    #[test]
    fn one_lane_runs_both_children_concurrently_and_joins_before_release() {
        let workers = Workers {
            active: AtomicUsize::new(0),
            maximum: 1,
        };
        let arrived = std::sync::Mutex::new(0);
        let changed = std::sync::Condvar::new();
        let rendezvous = || {
            let mut count = arrived
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *count += 1;
            changed.notify_all();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while *count < 2 {
                let (next, timeout) = changed
                    .wait_timeout(
                        count,
                        deadline.saturating_duration_since(std::time::Instant::now()),
                    )
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                count = next;
                assert!(
                    !timeout.timed_out() || *count == 2,
                    "structured children did not overlap"
                );
            }
        };
        let control = ExecutionControl::uncancelled();
        let result = run_with_workers(
            &workers,
            &control,
            || {
                rendezvous();
                std::thread::current().id()
            },
            || {
                rendezvous();
                std::thread::current().id()
            },
        );
        match result {
            Ok(pair) => {
                assert!(pair.spawned);
                assert_ne!(pair.left, pair.right);
            }
            Err(error) => unreachable!("{error:?}"),
        }
        assert_eq!(workers.active.load(Ordering::Acquire), 0);
    }
    #[test]
    fn occupied_lane_falls_back_without_waiting_for_its_owner() {
        let workers = Workers {
            active: AtomicUsize::new(1),
            maximum: 1,
        };
        let result = run_with_workers(&workers, &ExecutionControl::uncancelled(), || 17, || 29);
        match result {
            Ok(pair) => {
                assert!(!pair.spawned);
                assert_eq!((pair.left, pair.right), (17, 29));
            }
            Err(error) => unreachable!("{error:?}"),
        }
        assert_eq!(workers.active.load(Ordering::Acquire), 1);
    }
    #[test]
    fn caller_unwind_cancels_and_joins_before_releasing_worker_capacity() {
        let workers = Workers {
            active: AtomicUsize::new(0),
            maximum: 1,
        };
        let control = ExecutionControl::uncancelled();
        let observed = AtomicUsize::new(0);
        let (arrived, ready) = std::sync::mpsc::sync_channel(1);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = run_with_workers(
                &workers,
                &control,
                || {
                    assert!(
                        ready
                            .recv_timeout(std::time::Duration::from_secs(5))
                            .is_ok()
                    );
                    panic!("controlled caller unwind");
                },
                || {
                    assert!(arrived.send(()).is_ok());
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                    while !control.is_cancelled() && std::time::Instant::now() < deadline {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                    assert!(control.is_cancelled());
                    observed.store(workers.active.load(Ordering::Acquire), Ordering::Release);
                },
            );
        }));
        assert!(panic.is_err());
        assert_eq!(observed.load(Ordering::Acquire), 1);
        assert_eq!(workers.active.load(Ordering::Acquire), 0);
    }
}
