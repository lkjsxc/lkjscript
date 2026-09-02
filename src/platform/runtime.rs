//! Bounded resident execution shared by service, worker, foreground, and test runners.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, watch};

pub const RESIDENT_RUNTIME_CONTRACT_VERSION: u16 = 3;
pub const MAXIMUM_CONCURRENT_TASKS: usize = 4_096;
pub const MAXIMUM_QUEUED_TASKS: usize = 65_536;
pub const MAXIMUM_OPERATIONAL_MILLISECONDS: u64 = 3_600_000;

const fn resident_runtime_contract_version() -> u16 {
    RESIDENT_RUNTIME_CONTRACT_VERSION
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResidentLimits {
    #[serde(skip, default = "resident_runtime_contract_version")]
    pub contract_version: u16,
    pub maximum_concurrent_tasks: usize,
    pub maximum_queued_tasks: usize,
    pub request_deadline_milliseconds: u64,
    pub shutdown_grace_milliseconds: u64,
    pub cancellation_grace_milliseconds: u64,
}

impl Default for ResidentLimits {
    fn default() -> Self {
        Self {
            contract_version: RESIDENT_RUNTIME_CONTRACT_VERSION,
            maximum_concurrent_tasks: 4,
            maximum_queued_tasks: 64,
            request_deadline_milliseconds: 30_000,
            shutdown_grace_milliseconds: 30_000,
            cancellation_grace_milliseconds: 5_000,
        }
    }
}

impl ResidentLimits {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != RESIDENT_RUNTIME_CONTRACT_VERSION {
            return Err(runtime_diagnostic(
                "resident_contract",
                "resident runtime limits use a predecessor or foreign contract",
            ));
        }
        if self.maximum_concurrent_tasks == 0
            || self.maximum_concurrent_tasks > MAXIMUM_CONCURRENT_TASKS
        {
            return Err(runtime_diagnostic(
                "resident_concurrency_limit",
                format!("maximum_concurrent_tasks must be 1 through {MAXIMUM_CONCURRENT_TASKS}"),
            ));
        }
        if self.maximum_queued_tasks > MAXIMUM_QUEUED_TASKS {
            return Err(runtime_diagnostic(
                "resident_queue_limit",
                format!("maximum_queued_tasks may not exceed {MAXIMUM_QUEUED_TASKS}"),
            ));
        }
        for (name, milliseconds) in [
            (
                "request_deadline_milliseconds",
                self.request_deadline_milliseconds,
            ),
            (
                "shutdown_grace_milliseconds",
                self.shutdown_grace_milliseconds,
            ),
            (
                "cancellation_grace_milliseconds",
                self.cancellation_grace_milliseconds,
            ),
        ] {
            if milliseconds == 0 || milliseconds > MAXIMUM_OPERATIONAL_MILLISECONDS {
                return Err(runtime_diagnostic(
                    "resident_time_limit",
                    format!("{name} must be 1 through {MAXIMUM_OPERATIONAL_MILLISECONDS}"),
                ));
            }
        }
        self.maximum_concurrent_tasks
            .checked_add(self.maximum_queued_tasks)
            .ok_or_else(|| {
                runtime_diagnostic(
                    "resident_admission_limit",
                    "resident admission capacity overflowed",
                )
            })?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResidentObservation {
    pub accepting: bool,
    pub queued: usize,
    pub active: usize,
    pub admitted: u64,
    pub completed: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub overloaded: u64,
    pub rejected_after_shutdown: u64,
    pub maximum_queued: usize,
    pub maximum_active: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShutdownReceipt {
    #[serde(skip_serializing)]
    pub contract_version: u16,
    pub admission_stopped: bool,
    pub drained_before_cancellation: bool,
    pub cancellation_requested: usize,
    pub remaining_tasks: usize,
    pub cleanup_failures: Vec<ExecutionError>,
    pub elapsed_nanoseconds: u64,
}

#[derive(Clone)]
pub(crate) struct ResidentKernel {
    inner: Arc<ResidentKernelInner>,
}

struct ResidentKernelInner {
    limits: ResidentLimits,
    accepting: AtomicBool,
    admission: Arc<Semaphore>,
    workers: Arc<Semaphore>,
    shutdown: watch::Sender<bool>,
    idle: Notify,
    queued: AtomicUsize,
    active: AtomicUsize,
    next_task: AtomicU64,
    controls: Mutex<BTreeMap<u64, ExecutionControl>>,
    cleanup_failures: Mutex<Option<Vec<ExecutionError>>>,
    counters: RuntimeCounters,
}

pub(crate) struct ResidentKernelReceipt<T> {
    pub value: T,
    pub queue_nanoseconds: u64,
    pub execution_nanoseconds: u64,
    pub task_id: u64,
}

#[derive(Default)]
struct RuntimeCounters {
    admitted: AtomicU64,
    completed: AtomicU64,
    failed: AtomicU64,
    cancelled: AtomicU64,
    overloaded: AtomicU64,
    rejected_after_shutdown: AtomicU64,
    maximum_queued: AtomicUsize,
    maximum_active: AtomicUsize,
}

impl ResidentKernel {
    pub(crate) fn new(limits: ResidentLimits) -> Result<Self, Diagnostic> {
        limits.validate()?;
        let admission_capacity = limits
            .maximum_concurrent_tasks
            .checked_add(limits.maximum_queued_tasks)
            .ok_or_else(|| {
                runtime_diagnostic(
                    "resident_admission_limit",
                    "resident admission capacity overflowed",
                )
            })?;
        let maximum_concurrent_tasks = limits.maximum_concurrent_tasks;
        let (shutdown, _) = watch::channel(false);
        Ok(Self {
            inner: Arc::new(ResidentKernelInner {
                limits,
                accepting: AtomicBool::new(true),
                admission: Arc::new(Semaphore::new(admission_capacity)),
                workers: Arc::new(Semaphore::new(maximum_concurrent_tasks)),
                shutdown,
                idle: Notify::new(),
                queued: AtomicUsize::new(0),
                active: AtomicUsize::new(0),
                next_task: AtomicU64::new(1),
                controls: Mutex::new(BTreeMap::new()),
                cleanup_failures: Mutex::new(None),
                counters: RuntimeCounters::default(),
            }),
        })
    }

    pub(crate) fn limits(&self) -> &ResidentLimits {
        &self.inner.limits
    }

    pub(crate) fn observe(&self) -> ResidentObservation {
        ResidentObservation {
            accepting: self.inner.accepting.load(Ordering::Acquire),
            queued: self.inner.queued.load(Ordering::Acquire),
            active: self.inner.active.load(Ordering::Acquire),
            admitted: self.inner.counters.admitted.load(Ordering::Acquire),
            completed: self.inner.counters.completed.load(Ordering::Acquire),
            failed: self.inner.counters.failed.load(Ordering::Acquire),
            cancelled: self.inner.counters.cancelled.load(Ordering::Acquire),
            overloaded: self.inner.counters.overloaded.load(Ordering::Acquire),
            rejected_after_shutdown: self
                .inner
                .counters
                .rejected_after_shutdown
                .load(Ordering::Acquire),
            maximum_queued: self.inner.counters.maximum_queued.load(Ordering::Acquire),
            maximum_active: self.inner.counters.maximum_active.load(Ordering::Acquire),
        }
    }

    pub(crate) async fn invoke<T, F>(
        &self,
        operation: F,
    ) -> Result<ResidentKernelReceipt<T>, ExecutionError>
    where
        T: Send + 'static,
        F: FnOnce(ExecutionControl) -> Result<T, ExecutionError> + Send + 'static,
    {
        if !self.inner.accepting.load(Ordering::Acquire) {
            self.inner
                .counters
                .rejected_after_shutdown
                .fetch_add(1, Ordering::AcqRel);
            return Err(shutdown_error());
        }
        let admission = match self.inner.admission.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) if !self.inner.accepting.load(Ordering::Acquire) => {
                self.inner
                    .counters
                    .rejected_after_shutdown
                    .fetch_add(1, Ordering::AcqRel);
                return Err(shutdown_error());
            }
            Err(_) => {
                self.inner
                    .counters
                    .overloaded
                    .fetch_add(1, Ordering::AcqRel);
                return Err(ExecutionError::resource(
                    "resident_overloaded",
                    "resident deployment admission queue is full",
                ));
            }
        };
        if !self.inner.accepting.load(Ordering::Acquire) {
            drop(admission);
            self.inner
                .counters
                .rejected_after_shutdown
                .fetch_add(1, Ordering::AcqRel);
            return Err(shutdown_error());
        }

        self.inner.counters.admitted.fetch_add(1, Ordering::AcqRel);
        let queued = self.inner.queued.fetch_add(1, Ordering::AcqRel) + 1;
        update_maximum(&self.inner.counters.maximum_queued, queued);
        let queue_started = Instant::now();
        let mut shutdown = self.inner.shutdown.subscribe();
        let workers = self.inner.workers.clone();
        let worker = tokio::select! {
            biased;
            changed = shutdown.changed() => {
                let _ = changed;
                None
            }
            permit = workers.acquire_owned() => permit.ok(),
        };
        self.inner.queued.fetch_sub(1, Ordering::AcqRel);
        self.inner.idle.notify_waiters();
        let Some(worker) = worker else {
            drop(admission);
            self.inner
                .counters
                .rejected_after_shutdown
                .fetch_add(1, Ordering::AcqRel);
            return Err(shutdown_error());
        };
        if !self.inner.accepting.load(Ordering::Acquire) {
            drop(worker);
            drop(admission);
            self.inner
                .counters
                .rejected_after_shutdown
                .fetch_add(1, Ordering::AcqRel);
            return Err(shutdown_error());
        }

        let queue_nanoseconds = duration_nanoseconds(queue_started.elapsed());
        let task_id = self
            .inner
            .next_task
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_add(1)
            })
            .map_err(|_| {
                ExecutionError::resource(
                    "resident_task_identity_exhausted",
                    "resident task identity domain was exhausted",
                )
            })?;
        let deadline_duration =
            Duration::from_millis(self.inner.limits.request_deadline_milliseconds);
        let control = ExecutionControl::with_deadline(Instant::now() + deadline_duration);
        lock_unpoisoned(&self.inner.controls).insert(task_id, control.clone());
        let active = self.inner.active.fetch_add(1, Ordering::AcqRel) + 1;
        update_maximum(&self.inner.counters.maximum_active, active);

        let inner = self.inner.clone();
        let execution_started = Instant::now();
        let closure_control = control.clone();
        let mut cancellation = CancelOnDrop::new(control.clone());
        let guard = ActiveGuard {
            inner: self.inner.clone(),
            task_id,
            _worker: worker,
            _admission: admission,
        };
        let mut task = tokio::task::spawn_blocking(move || {
            let _guard = guard;
            let outcome = operation(closure_control);
            inner.record_outcome(&outcome);
            outcome
        });
        let outcome = match tokio::time::timeout(deadline_duration, &mut task).await {
            Ok(outcome) => join_outcome(outcome),
            Err(_) => {
                control.cancel();
                let cancellation_grace =
                    Duration::from_millis(self.inner.limits.cancellation_grace_milliseconds);
                match tokio::time::timeout(cancellation_grace, &mut task).await {
                    Ok(outcome) => join_outcome(outcome),
                    Err(_) => Err(ExecutionError::new(
                        ExecutionFailureClass::Infrastructure,
                        "resident_cancellation_stalled",
                        "cancelled execution did not close within its cancellation grace",
                    )),
                }
            }
        };
        cancellation.disarm();
        Ok(ResidentKernelReceipt {
            value: outcome?,
            queue_nanoseconds,
            execution_nanoseconds: duration_nanoseconds(execution_started.elapsed()),
            task_id,
        })
    }

    pub(crate) async fn shutdown(
        &self,
        cleanup: impl FnOnce() -> Vec<ExecutionError>,
    ) -> ShutdownReceipt {
        let started = Instant::now();
        self.inner.accepting.store(false, Ordering::Release);
        self.inner.admission.close();
        self.inner.workers.close();
        let _ = self.inner.shutdown.send(true);
        let shutdown_grace = Duration::from_millis(self.inner.limits.shutdown_grace_milliseconds);
        let drained_before_cancellation =
            tokio::time::timeout(shutdown_grace, self.inner.wait_idle())
                .await
                .is_ok();
        let cancellation_requested = if drained_before_cancellation {
            0
        } else {
            let controls = lock_unpoisoned(&self.inner.controls);
            for control in controls.values() {
                control.cancel();
            }
            controls.len()
        };
        if !drained_before_cancellation {
            let cancellation_grace =
                Duration::from_millis(self.inner.limits.cancellation_grace_milliseconds);
            let _ = tokio::time::timeout(cancellation_grace, self.inner.wait_idle()).await;
        }
        let remaining_tasks = self
            .inner
            .queued
            .load(Ordering::Acquire)
            .saturating_add(self.inner.active.load(Ordering::Acquire));
        let cleanup_failures = {
            let mut completed = lock_unpoisoned(&self.inner.cleanup_failures);
            match completed.as_ref() {
                Some(failures) => failures.clone(),
                None => {
                    let failures = cleanup();
                    *completed = Some(failures.clone());
                    failures
                }
            }
        };
        ShutdownReceipt {
            contract_version: RESIDENT_RUNTIME_CONTRACT_VERSION,
            admission_stopped: true,
            drained_before_cancellation,
            cancellation_requested,
            remaining_tasks,
            cleanup_failures,
            elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        }
    }
}

impl ResidentKernelInner {
    async fn wait_idle(&self) {
        loop {
            let notified = self.idle.notified();
            if self.queued.load(Ordering::Acquire) == 0 && self.active.load(Ordering::Acquire) == 0
            {
                return;
            }
            notified.await;
        }
    }

    fn record_outcome<T>(&self, outcome: &Result<T, ExecutionError>) {
        self.counters.completed.fetch_add(1, Ordering::AcqRel);
        if let Err(error) = outcome {
            self.counters.failed.fetch_add(1, Ordering::AcqRel);
            if error.class == ExecutionFailureClass::Cancelled {
                self.counters.cancelled.fetch_add(1, Ordering::AcqRel);
            }
        }
    }
}

struct ActiveGuard {
    inner: Arc<ResidentKernelInner>,
    task_id: u64,
    _worker: OwnedSemaphorePermit,
    _admission: OwnedSemaphorePermit,
}

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        lock_unpoisoned(&self.inner.controls).remove(&self.task_id);
        self.inner.active.fetch_sub(1, Ordering::AcqRel);
        self.inner.idle.notify_waiters();
    }
}

struct CancelOnDrop {
    control: ExecutionControl,
    armed: bool,
}

impl CancelOnDrop {
    fn new(control: ExecutionControl) -> Self {
        Self {
            control,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if self.armed {
            self.control.cancel();
        }
    }
}

fn join_outcome<T>(
    outcome: Result<Result<T, ExecutionError>, tokio::task::JoinError>,
) -> Result<T, ExecutionError> {
    outcome.map_err(|_| {
        ExecutionError::new(
            ExecutionFailureClass::Infrastructure,
            "resident_worker_panic",
            "resident execution worker terminated unexpectedly",
        )
    })?
}

fn update_maximum(maximum: &AtomicUsize, candidate: usize) {
    let _ = maximum.fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
        (candidate > current).then_some(candidate)
    });
}

fn duration_nanoseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn shutdown_error() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Cancelled,
        "resident_shutting_down",
        "resident deployment has stopped admission",
    )
}

fn runtime_diagnostic(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn resident_kernel_is_value_agnostic() {
        let kernel = ResidentKernel::new(ResidentLimits::default()).expect("kernel");
        let receipt = kernel
            .invoke(|control| {
                control.check()?;
                Ok(String::from("normalized"))
            })
            .await
            .expect("invoke");

        assert_eq!(receipt.value, "normalized");
        assert_eq!(receipt.task_id, 1);
        assert_eq!(kernel.observe().completed, 1);
        assert_eq!(kernel.shutdown(Vec::new).await.remaining_tasks, 0);
    }

    #[test]
    fn limits_reject_predecessors_and_unbounded_shapes() {
        assert!(
            ResidentLimits {
                contract_version: 0,
                ..ResidentLimits::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            ResidentLimits {
                maximum_concurrent_tasks: 0,
                ..ResidentLimits::default()
            }
            .validate()
            .is_err()
        );
    }
}
