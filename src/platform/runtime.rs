//! Bounded resident execution shared by service, worker, foreground, and test runners.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{
    BoundCapabilities, CapabilityGrant, ExecutionControl, ExecutionError, ExecutionFailureClass,
    PreparedProgram, PreparedTarget, RunObservation, RunPolicy, Vm,
};
use super::value::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, watch};

pub const RESIDENT_RUNTIME_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_CONCURRENT_TASKS: usize = 4_096;
pub const MAXIMUM_QUEUED_TASKS: usize = 65_536;
pub const MAXIMUM_OPERATIONAL_MILLISECONDS: u64 = 3_600_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResidentLimits {
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

#[derive(Clone, Debug)]
pub struct InvocationReceipt {
    pub value: Value,
    pub execution: RunObservation,
    pub queue_nanoseconds: u64,
    pub execution_nanoseconds: u64,
    pub task_id: u64,
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
    pub contract_version: u16,
    pub admission_stopped: bool,
    pub drained_before_cancellation: bool,
    pub cancellation_requested: usize,
    pub remaining_tasks: usize,
    pub cleanup_failures: Vec<ExecutionError>,
    pub elapsed_nanoseconds: u64,
}

#[derive(Clone)]
pub struct ResidentDeployment {
    inner: Arc<RuntimeInner>,
}

struct RuntimeInner {
    program: Arc<PreparedProgram>,
    target: PreparedTarget,
    capabilities: BoundCapabilities,
    limits: ResidentLimits,
    run_policy: RunPolicy,
    accepting: AtomicBool,
    admission: Arc<Semaphore>,
    workers: Arc<Semaphore>,
    shutdown: watch::Sender<bool>,
    idle: Notify,
    queued: AtomicUsize,
    active: AtomicUsize,
    next_task: AtomicU64,
    controls: Mutex<BTreeMap<u64, ExecutionControl>>,
    counters: RuntimeCounters,
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

impl ResidentDeployment {
    pub fn prepare(
        program: Arc<PreparedProgram>,
        target_name: &str,
        grants: Vec<CapabilityGrant>,
        limits: ResidentLimits,
        run_policy: RunPolicy,
    ) -> Result<Self, Diagnostic> {
        limits.validate()?;
        let target = program.target(target_name)?.clone();
        let component = program.components().get(&target.component).ok_or_else(|| {
            runtime_diagnostic(
                "resident_component_missing",
                "prepared target lost its component",
            )
        })?;
        let capabilities = BoundCapabilities::bind(component, grants)?;
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
            inner: Arc::new(RuntimeInner {
                program,
                target,
                capabilities,
                limits,
                run_policy,
                accepting: AtomicBool::new(true),
                admission: Arc::new(Semaphore::new(admission_capacity)),
                workers: Arc::new(Semaphore::new(maximum_concurrent_tasks)),
                shutdown,
                idle: Notify::new(),
                queued: AtomicUsize::new(0),
                active: AtomicUsize::new(0),
                next_task: AtomicU64::new(1),
                controls: Mutex::new(BTreeMap::new()),
                counters: RuntimeCounters::default(),
            }),
        })
    }

    pub fn target(&self) -> &PreparedTarget {
        &self.inner.target
    }

    pub fn limits(&self) -> &ResidentLimits {
        &self.inner.limits
    }

    pub fn observe(&self) -> ResidentObservation {
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

    pub async fn invoke(&self, arguments: Vec<Value>) -> Result<InvocationReceipt, ExecutionError> {
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
        let task_id = self.inner.next_task.fetch_add(1, Ordering::AcqRel);
        if task_id == u64::MAX {
            drop(worker);
            drop(admission);
            return Err(ExecutionError::resource(
                "resident_task_identity_exhausted",
                "resident task identity domain was exhausted",
            ));
        }
        let deadline_duration =
            Duration::from_millis(self.inner.limits.request_deadline_milliseconds);
        let control = ExecutionControl::with_deadline(Instant::now() + deadline_duration);
        lock_unpoisoned(&self.inner.controls).insert(task_id, control.clone());
        let active = self.inner.active.fetch_add(1, Ordering::AcqRel) + 1;
        update_maximum(&self.inner.counters.maximum_active, active);

        let program = self.inner.program.clone();
        let function = self.inner.target.port.function.clone();
        let capabilities = self.inner.capabilities.task_scope();
        let policy = self.inner.run_policy;
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
            let outcome = Vm::new(&program, policy).invoke_controlled(
                &function,
                arguments,
                Some(&capabilities),
                &closure_control,
            );
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
        let (value, execution) = outcome?;
        Ok(InvocationReceipt {
            value,
            execution,
            queue_nanoseconds,
            execution_nanoseconds: duration_nanoseconds(execution_started.elapsed()),
            task_id,
        })
    }

    pub async fn shutdown(&self) -> ShutdownReceipt {
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
        let cleanup_failures = self.inner.capabilities.shutdown();
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

impl RuntimeInner {
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

    fn record_outcome(&self, outcome: &Result<(Value, RunObservation), ExecutionError>) {
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
    inner: Arc<RuntimeInner>,
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

fn join_outcome(
    outcome: Result<Result<(Value, RunObservation), ExecutionError>, tokio::task::JoinError>,
) -> Result<(Value, RunObservation), ExecutionError> {
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
    use crate::platform::{
        SourceLimits, build_artifact, decode_package, load_artifact, parse_source,
        validate_package_documents,
    };

    fn program(source: &[u8]) -> Arc<PreparedProgram> {
        let descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"resident","modules":[{"name":"main","path":"src/main.lkj"}],"dependencies":[],"targets":[{"name":"run","component":"main.App","port":"main","runner":"http"}]}"#,
        )
        .expect("descriptor");
        let document =
            parse_source("src/main.lkj", source, SourceLimits::default()).expect("source");
        let package = validate_package_documents(descriptor, vec![document], &[]).expect("package");
        let (bytes, _) = build_artifact(&package, &[&package]).expect("artifact");
        Arc::new(PreparedProgram::prepare(load_artifact(&bytes).expect("load")).expect("prepare"))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn resident_reuses_one_program_and_drains_cleanly() {
        let program = program(
            br#"(module main
  (export App)
  (extern add ((left I64) (right I64)) I64 core.i64.add)
  (fn main ((left I64) (right I64)) I64 (call add left right))
  (component App (port main (Function (I64 I64) I64) (function main))))
"#,
        );
        let deployment = ResidentDeployment::prepare(
            program,
            "run",
            Vec::new(),
            ResidentLimits {
                maximum_concurrent_tasks: 2,
                ..ResidentLimits::default()
            },
            RunPolicy::default(),
        )
        .expect("deployment");
        let (left, right) = tokio::join!(
            deployment.invoke(vec![Value::I64(20), Value::I64(22)]),
            deployment.invoke(vec![Value::I64(7), Value::I64(8)])
        );
        assert_eq!(
            left.expect("left").value.canonical_json(),
            serde_json::json!(42)
        );
        assert_eq!(
            right.expect("right").value.canonical_json(),
            serde_json::json!(15)
        );
        let shutdown = deployment.shutdown().await;
        assert!(shutdown.drained_before_cancellation);
        assert_eq!(shutdown.remaining_tasks, 0);
        assert_eq!(deployment.observe().completed, 2);
        let error = deployment
            .invoke(vec![Value::I64(1), Value::I64(2)])
            .await
            .expect_err("admission stopped");
        assert_eq!(error.code, "resident_shutting_down");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn deadline_cancels_computation_cooperatively() {
        let program = program(
            br#"(module main
  (export App)
  (fn forever ((value I64)) I64 (call forever value))
  (component App (port main (Function (I64) I64) (function forever))))
"#,
        );
        let deployment = ResidentDeployment::prepare(
            program,
            "run",
            Vec::new(),
            ResidentLimits {
                request_deadline_milliseconds: 5,
                cancellation_grace_milliseconds: 500,
                ..ResidentLimits::default()
            },
            RunPolicy {
                instruction_fuel: u64::MAX,
                maximum_call_depth: 1_000_000,
                maximum_value_stack: 2_000_000,
            },
        )
        .expect("deployment");
        let error = deployment
            .invoke(vec![Value::I64(1)])
            .await
            .expect_err("deadline");
        assert_eq!(error.class, ExecutionFailureClass::Cancelled);
        assert_eq!(deployment.shutdown().await.remaining_tasks, 0);
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
