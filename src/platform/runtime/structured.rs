//! Explicitly owned reusable workers for structured, effect-free child execution.
//!
//! A worker accepts only one reserved child at a time. Its slot remains reserved
//! until the caller receives the joined result; nested scopes never await a slot.

use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, mpsc};
use std::thread::{self, JoinHandle};

#[path = "structured_lending.rs"]
mod lending;
use lending::IdleWorker;

#[path = "structured_receipt.rs"]
mod receipt;

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Command {
    Run(Job),
    Stop,
}

struct Capacity {
    maximum: usize,
    reserved: AtomicUsize,
    // Bounded by physical workers; entries never retain their runtime owner.
    idle: Mutex<Vec<IdleWorker>>,
}

impl Capacity {
    fn reserve(self: &Arc<Self>) -> Option<ThreadReservation> {
        self.reserved
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |reserved| {
                (reserved < self.maximum).then(|| reserved + 1)
            })
            .ok()
            .map(|_| ThreadReservation(Arc::clone(self)))
    }
}

struct ThreadReservation(Arc<Capacity>);

impl Drop for ThreadReservation {
    fn drop(&mut self) {
        self.0.reserved.fetch_sub(1, Ordering::AcqRel);
    }
}

fn process_capacity() -> Arc<Capacity> {
    static CAPACITY: OnceLock<Arc<Capacity>> = OnceLock::new();
    Arc::clone(CAPACITY.get_or_init(|| {
        Arc::new(Capacity {
            maximum: thread::available_parallelism().map_or(0, |n| n.get().saturating_sub(1)),
            reserved: AtomicUsize::new(0),
            idle: Mutex::new(Vec::new()),
        })
    }))
}

/// Per-owner scheduling observations. These exclude resident root invocation threads.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredExecutorObservation {
    pub dispatch_open: bool,
    /// The process-wide ceiling, not a privately reserved worker entitlement.
    pub worker_limit: usize,
    pub workers_started: u64,
    /// Existing physical workers whose join custody arrived from another owner.
    pub workers_received: u64,
    /// Idle workers whose join custody moved to another owner.
    pub workers_handed_off: u64,
    pub active_dispatches: usize,
    pub maximum_active_dispatches: usize,
    pub completed_dispatches: u64,
    pub inline_fallbacks: u64,
    /// Currently owned workers whose thread handles have not yet been joined.
    pub remaining_workers: usize,
    pub joined_workers: u64,
}

struct Worker {
    sender: mpsc::SyncSender<Command>,
    thread: JoinHandle<()>,
    // Physical capacity is retained through joining the thread, including failure.
    _reservation: ThreadReservation,
}

struct State {
    observation: StructuredExecutorObservation,
    workers: Vec<Option<Worker>>,
    shutdown_error: Option<ExecutionError>,
    #[cfg(test)]
    refuse_next_worker_start: bool,
    #[cfg(test)]
    refuse_next_submission: bool,
}

struct Inner {
    capacity: Arc<Capacity>,
    state: Mutex<State>,
    joined: Condvar,
}

/// Sole lifecycle owner. Jobs and application clones carry only its handle.
pub struct StructuredExecutor {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for StructuredExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StructuredExecutor")
            .field("observation", &self.observe())
            .finish()
    }
}

/// Cloneable dispatch authority, without authority to join or unload its workers.
#[derive(Clone)]
pub struct StructuredExecutorHandle {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for StructuredExecutorHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StructuredExecutorHandle")
            .field("observation", &self.observe())
            .finish()
    }
}

#[derive(Debug)]
pub(crate) struct StructuredPair<L, R> {
    pub left: L,
    pub right: R,
    pub dispatched: bool,
}

impl Default for StructuredExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl StructuredExecutor {
    /// No threads or program storage are retained until an actual dispatch.
    pub fn new() -> Self {
        Self::with_capacity(process_capacity())
    }

    fn with_capacity(capacity: Arc<Capacity>) -> Self {
        let worker_limit = capacity.maximum;
        Self {
            inner: Arc::new(Inner {
                capacity,
                state: Mutex::new(State {
                    observation: StructuredExecutorObservation {
                        dispatch_open: true,
                        worker_limit,
                        workers_started: 0,
                        workers_received: 0,
                        workers_handed_off: 0,
                        active_dispatches: 0,
                        maximum_active_dispatches: 0,
                        completed_dispatches: 0,
                        inline_fallbacks: 0,
                        remaining_workers: 0,
                        joined_workers: 0,
                    },
                    workers: Vec::new(),
                    shutdown_error: None,
                    #[cfg(test)]
                    refuse_next_worker_start: false,
                    #[cfg(test)]
                    refuse_next_submission: false,
                }),
                joined: Condvar::new(),
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(worker_limit: usize) -> Self {
        Self::with_capacity(Arc::new(Capacity {
            maximum: worker_limit,
            reserved: AtomicUsize::new(0),
            idle: Mutex::new(Vec::new()),
        }))
    }

    #[cfg(test)]
    pub(crate) fn sharing_capacity_for_test(&self) -> Self {
        Self::with_capacity(Arc::clone(&self.inner.capacity))
    }

    pub fn handle(&self) -> StructuredExecutorHandle {
        StructuredExecutorHandle {
            inner: Arc::clone(&self.inner),
        }
    }

    pub fn observe(&self) -> StructuredExecutorObservation {
        lock(&self.inner.state).observation.clone()
    }

    /// Future child work executes inline. This never cancels another invocation.
    pub fn close_dispatch(&self) {
        // All operations spanning owners acquire the catalogue before any state.
        let mut idle = lock(&self.inner.capacity.idle);
        let mut state = lock(&self.inner.state);
        state.observation.dispatch_open = false;
        idle.retain(|entry| !entry.belongs_to(&self.inner));
    }

    /// Close dispatch, retain active receipts, then join every still-owned thread.
    /// Application owners must separately stop and drain their resident invocations.
    pub fn shutdown(&mut self) -> Result<StructuredExecutorObservation, ExecutionError> {
        self.close_dispatch();
        let workers = {
            let mut state = lock(&self.inner.state);
            while state.observation.active_dispatches != 0 {
                state = self
                    .inner
                    .joined
                    .wait(state)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
            std::mem::take(&mut state.workers)
        };
        // All accepted jobs have returned their receipts. No slot is reusable after
        // dispatch closes, so each mailbox can accept its termination message.
        for worker in workers.iter().flatten() {
            let _ = worker.sender.send(Command::Stop);
        }
        for worker in workers.into_iter().flatten() {
            let Worker {
                sender,
                thread,
                _reservation,
            } = worker;
            drop(sender);
            let result = thread.join();
            let mut state = lock(&self.inner.state);
            state.observation.remaining_workers -= 1;
            state.observation.joined_workers = state.observation.joined_workers.saturating_add(1);
            if result.is_err() && state.shutdown_error.is_none() {
                state.shutdown_error = Some(worker_failure());
            }
            drop(state);
            drop(_reservation);
        }
        let state = lock(&self.inner.state);
        match &state.shutdown_error {
            Some(error) => Err(error.clone()),
            None => Ok(state.observation.clone()),
        }
    }
}

impl Drop for StructuredExecutor {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

enum Dispatch<R, F> {
    Inline(F),
    Accepted(JoinedChild<R>),
    /// The mailbox refused an intact, unexecuted closure. Run that same closure.
    Refused(Job, mpsc::Receiver<Result<R, ExecutionError>>),
}

impl StructuredExecutorHandle {
    pub fn observe(&self) -> StructuredExecutorObservation {
        lock(&self.inner.state).observation.clone()
    }

    pub(crate) fn run<L, R: Send + 'static>(
        &self,
        control: &ExecutionControl,
        left: impl FnOnce() -> L,
        right: impl FnOnce() -> R + Send + 'static,
    ) -> Result<StructuredPair<L, R>, ExecutionError> {
        control.check()?;
        match self.dispatch(control, right) {
            Dispatch::Inline(right) => Ok(StructuredPair {
                left: left(),
                right: right(),
                dispatched: false,
            }),
            Dispatch::Accepted(mut child) => {
                let left = left();
                let right = child.receive()?;
                Ok(StructuredPair {
                    left,
                    right,
                    dispatched: true,
                })
            }
            Dispatch::Refused(job, receiver) => {
                let left = left();
                job();
                let right = receiver.recv().map_err(|_| worker_failure())??;
                Ok(StructuredPair {
                    left,
                    right,
                    dispatched: false,
                })
            }
        }
    }

    fn dispatch<R: Send + 'static, F: FnOnce() -> R + Send + 'static>(
        &self,
        control: &ExecutionControl,
        right: F,
    ) -> Dispatch<R, F> {
        let mut idle = lock(&self.inner.capacity.idle);
        let mut state = lock(&self.inner.state);
        let slot = if state.observation.dispatch_open {
            self.claim_idle(&mut state, &mut idle)
                .or_else(|| self.start_worker(&mut state))
        } else {
            None
        };
        let Some(slot) = slot else {
            state.observation.inline_fallbacks =
                state.observation.inline_fallbacks.saturating_add(1);
            return Dispatch::Inline(right);
        };
        let (sender, receiver) = mpsc::sync_channel(1);
        let job_control = control.clone();
        let job: Job = Box::new(move || {
            let mut guard = CancelOnUnwind::new(job_control.clone());
            let outcome = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(right)) {
                Ok(value) => Ok(value),
                Err(_) => {
                    job_control.cancel();
                    Err(worker_failure())
                }
            };
            let _ = sender.send(outcome);
            guard.armed = false;
        });
        let command = Command::Run(job);
        #[cfg(test)]
        let refuse = std::mem::take(&mut state.refuse_next_submission);
        #[cfg(not(test))]
        let refuse = false;
        let submitted = if refuse {
            Err(mpsc::TrySendError::Full(command))
        } else {
            match &state.workers[slot] {
                Some(worker) => worker.sender.try_send(command),
                None => Err(mpsc::TrySendError::Disconnected(command)),
            }
        };
        match submitted {
            Ok(()) => {
                let observation = &mut state.observation;
                observation.active_dispatches += 1;
                observation.maximum_active_dispatches = observation
                    .maximum_active_dispatches
                    .max(observation.active_dispatches);
                Dispatch::Accepted(JoinedChild {
                    inner: Arc::clone(&self.inner),
                    slot: Some(slot),
                    receiver,
                    control: control.clone(),
                })
            }
            Err(mpsc::TrySendError::Full(Command::Run(job))) => {
                idle.push(IdleWorker::new(&self.inner, slot));
                state.observation.inline_fallbacks =
                    state.observation.inline_fallbacks.saturating_add(1);
                Dispatch::Refused(job, receiver)
            }
            Err(mpsc::TrySendError::Disconnected(Command::Run(job))) => {
                // A failure discovered after handoff stays with the current join
                // owner. Never recirculate the worker or return it to the donor.
                state.observation.inline_fallbacks =
                    state.observation.inline_fallbacks.saturating_add(1);
                Dispatch::Refused(job, receiver)
            }
            // Only Run is submitted here; retaining exhaustive command handling
            // makes this refusal path safe if another internal command is added.
            Err(mpsc::TrySendError::Full(Command::Stop))
            | Err(mpsc::TrySendError::Disconnected(Command::Stop)) => {
                unreachable!("dispatch submits only child jobs")
            }
        }
    }

    fn start_worker(&self, state: &mut State) -> Option<usize> {
        let reservation = self.inner.capacity.reserve()?;
        #[cfg(test)]
        if std::mem::take(&mut state.refuse_next_worker_start) {
            return None;
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("lkjscript-structured".to_owned())
            .spawn(move || {
                while let Ok(Command::Run(job)) = receiver.recv() {
                    // The typed wrapper reports child panics. This outer boundary
                    // also retains the worker if a panic payload's destructor fails.
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
                }
            })
            .ok()?;
        let slot = state.retain_worker(Worker {
            sender,
            thread,
            _reservation: reservation,
        });
        state.observation.workers_started = state.observation.workers_started.saturating_add(1);
        Some(slot)
    }
}

struct CancelOnUnwind {
    control: ExecutionControl,
    armed: bool,
}

impl CancelOnUnwind {
    fn new(control: ExecutionControl) -> Self {
        Self {
            control,
            armed: true,
        }
    }
}

impl Drop for CancelOnUnwind {
    fn drop(&mut self) {
        if self.armed {
            self.control.cancel();
        }
    }
}

struct JoinedChild<R> {
    inner: Arc<Inner>,
    slot: Option<usize>,
    receiver: mpsc::Receiver<Result<R, ExecutionError>>,
    control: ExecutionControl,
}

impl<R> JoinedChild<R> {
    fn receive(&mut self) -> Result<R, ExecutionError> {
        let result = self.receiver.recv().map_err(|_| worker_failure());
        self.release(result.is_ok());
        result?
    }

    fn release(&mut self, reusable: bool) {
        if let Some(slot) = self.slot.take() {
            let mut idle = lock(&self.inner.capacity.idle);
            let mut state = lock(&self.inner.state);
            state.observation.active_dispatches -= 1;
            state.observation.completed_dispatches =
                state.observation.completed_dispatches.saturating_add(1);
            if reusable && state.observation.dispatch_open {
                idle.push(IdleWorker::new(&self.inner, slot));
            }
            drop(state);
            drop(idle);
            self.inner.joined.notify_all();
        }
    }
}

impl<R> Drop for JoinedChild<R> {
    fn drop(&mut self) {
        if self.slot.is_some() {
            self.control.cancel();
            self.discard();
        }
    }
}

fn worker_failure() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "normalized_parallel_worker",
        "structured worker terminated unexpectedly",
    )
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
#[path = "structured_tests.rs"]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "controlled scheduler proofs"
)]
mod tests;

#[cfg(test)]
#[path = "structured_handoff_tests.rs"]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "controlled ownership handoff proofs"
)]
mod handoff_tests;

#[cfg(test)]
#[path = "structured_handoff_lifecycle_tests.rs"]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "controlled ownership lifecycle proofs"
)]
mod handoff_lifecycle_tests;

#[cfg(test)]
#[path = "structured_handoff_race_tests.rs"]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "controlled ownership race proofs"
)]
mod handoff_race_tests;

#[cfg(test)]
#[path = "structured_handoff_failure_tests.rs"]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "controlled late mailbox failure"
)]
mod handoff_failure_tests;

#[cfg(test)]
#[path = "structured_disposal_tests.rs"]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "controlled result custody proofs"
)]
mod disposal_tests;
