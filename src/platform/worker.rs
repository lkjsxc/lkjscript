//! Structured resident worker topology over an authored zero-argument component port.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{ExecutionError, ExecutionFailureClass};
use super::package::RunnerKind;
use super::runtime::{ResidentDeployment, ShutdownReceipt};
use super::semantic::ResolvedType;
use super::value::Value;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinSet;

pub const WORKER_RUNNER_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_RESIDENT_WORKERS: usize = 4_096;
pub const MAXIMUM_IDLE_WAIT_MILLISECONDS: u64 = 60_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerLimits {
    pub contract_version: u16,
    pub maximum_workers: usize,
    pub idle_wait_milliseconds: u64,
}

impl Default for WorkerLimits {
    fn default() -> Self {
        Self {
            contract_version: WORKER_RUNNER_CONTRACT_VERSION,
            maximum_workers: 1,
            idle_wait_milliseconds: 100,
        }
    }
}

impl WorkerLimits {
    pub fn validate(&self, maximum_runtime_tasks: usize) -> Result<(), Diagnostic> {
        if self.contract_version != WORKER_RUNNER_CONTRACT_VERSION {
            return Err(worker_diagnostic(
                DiagnosticClass::Source,
                "worker_contract",
                "worker limits use a predecessor or foreign contract",
            ));
        }
        if self.maximum_workers == 0
            || self.maximum_workers > MAXIMUM_RESIDENT_WORKERS
            || self.maximum_workers > maximum_runtime_tasks
        {
            return Err(worker_diagnostic(
                DiagnosticClass::Resource,
                "worker_count_limit",
                format!(
                    "maximum_workers must be 1 through the resident concurrency limit {maximum_runtime_tasks}"
                ),
            ));
        }
        if self.idle_wait_milliseconds == 0
            || self.idle_wait_milliseconds > MAXIMUM_IDLE_WAIT_MILLISECONDS
        {
            return Err(worker_diagnostic(
                DiagnosticClass::Resource,
                "worker_idle_wait_limit",
                format!(
                    "idle_wait_milliseconds must be 1 through {MAXIMUM_IDLE_WAIT_MILLISECONDS}"
                ),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerReceipt {
    pub contract_version: u16,
    pub iterations: u64,
    pub productive_iterations: u64,
    pub idle_iterations: u64,
    pub shutdown: ShutdownReceipt,
}

#[derive(Clone)]
pub struct WorkerApplication {
    deployment: ResidentDeployment,
    limits: WorkerLimits,
}

impl WorkerApplication {
    pub fn new(deployment: ResidentDeployment, limits: WorkerLimits) -> Result<Self, Diagnostic> {
        limits.validate(deployment.limits().maximum_concurrent_tasks)?;
        if deployment.target().runner != RunnerKind::Worker {
            return Err(worker_diagnostic(
                DiagnosticClass::Source,
                "worker_runner_kind",
                "worker topology requires a worker runner target",
            ));
        }
        let signature = &deployment.target().port.signature;
        if !signature.parameters.is_empty() || signature.result != ResolvedType::Bool {
            return Err(worker_diagnostic(
                DiagnosticClass::Semantic,
                "worker_port_signature",
                "worker port must have the exact signature () -> Bool",
            ));
        }
        Ok(Self { deployment, limits })
    }

    pub async fn run(
        self,
        shutdown: impl Future<Output = ()> + Send,
    ) -> Result<WorkerReceipt, Diagnostic> {
        let counters = Arc::new(WorkerCounters::default());
        let (stop, _) = watch::channel(false);
        let mut workers = JoinSet::new();
        for _ in 0..self.limits.maximum_workers {
            workers.spawn(worker_loop(
                self.deployment.clone(),
                stop.subscribe(),
                Duration::from_millis(self.limits.idle_wait_milliseconds),
                counters.clone(),
            ));
        }

        tokio::pin!(shutdown);
        let failure = tokio::select! {
            () = &mut shutdown => None,
            joined = workers.join_next() => match joined {
                Some(Ok(Err(error))) => Some(execution_diagnostic(error)),
                Some(Err(_)) => Some(worker_diagnostic(
                    DiagnosticClass::Infrastructure,
                    "worker_task_panic",
                    "a resident worker task terminated unexpectedly",
                )),
                Some(Ok(Ok(()))) | None => Some(worker_diagnostic(
                    DiagnosticClass::Infrastructure,
                    "worker_task_unowned_exit",
                    "a resident worker task exited before its owning shutdown scope",
                )),
            }
        };

        let _ = stop.send(true);
        let runtime_shutdown = self.deployment.shutdown().await;
        workers.abort_all();
        while workers.join_next().await.is_some() {}
        if let Some(error) = failure {
            return Err(error);
        }
        if runtime_shutdown.remaining_tasks != 0 || !runtime_shutdown.cleanup_failures.is_empty() {
            return Err(worker_diagnostic(
                DiagnosticClass::Infrastructure,
                "worker_shutdown_incomplete",
                format!(
                    "{} resident tasks and {} cleanup failures remained after worker shutdown",
                    runtime_shutdown.remaining_tasks,
                    runtime_shutdown.cleanup_failures.len()
                ),
            ));
        }
        Ok(WorkerReceipt {
            contract_version: WORKER_RUNNER_CONTRACT_VERSION,
            iterations: counters.iterations.load(Ordering::Acquire),
            productive_iterations: counters.productive.load(Ordering::Acquire),
            idle_iterations: counters.idle.load(Ordering::Acquire),
            shutdown: runtime_shutdown,
        })
    }
}

#[derive(Default)]
struct WorkerCounters {
    iterations: AtomicU64,
    productive: AtomicU64,
    idle: AtomicU64,
}

async fn worker_loop(
    deployment: ResidentDeployment,
    mut stop: watch::Receiver<bool>,
    idle_wait: Duration,
    counters: Arc<WorkerCounters>,
) -> Result<(), ExecutionError> {
    loop {
        if *stop.borrow() {
            return Ok(());
        }
        let receipt = deployment.invoke(Vec::new()).await?;
        increment(&counters.iterations)?;
        match receipt.value {
            Value::Bool(true) => increment(&counters.productive)?,
            Value::Bool(false) => {
                increment(&counters.idle)?;
                tokio::select! {
                    biased;
                    changed = stop.changed() => {
                        let _ = changed;
                    }
                    () = tokio::time::sleep(idle_wait) => {}
                }
            }
            _ => {
                return Err(ExecutionError::new(
                    ExecutionFailureClass::Infrastructure,
                    "worker_result_type",
                    "validated worker port returned a non-Bool value",
                ));
            }
        }
    }
}

fn increment(counter: &AtomicU64) -> Result<(), ExecutionError> {
    counter
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
            value.checked_add(1)
        })
        .map(|_| ())
        .map_err(|_| {
            ExecutionError::resource(
                "worker_counter_exhausted",
                "worker observation counter was exhausted",
            )
        })
}

fn execution_diagnostic(error: ExecutionError) -> Diagnostic {
    let class = match error.class {
        ExecutionFailureClass::Resource => DiagnosticClass::Resource,
        ExecutionFailureClass::Cancelled => DiagnosticClass::Cancelled,
        ExecutionFailureClass::Capability | ExecutionFailureClass::PossibleVisibility => {
            DiagnosticClass::Capability
        }
        ExecutionFailureClass::Trap => DiagnosticClass::Semantic,
        ExecutionFailureClass::Infrastructure => DiagnosticClass::Infrastructure,
    };
    worker_diagnostic(class, error.code, error.message)
}

fn worker_diagnostic(
    class: DiagnosticClass,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_limits_are_exact_and_bounded_by_runtime_concurrency() {
        assert!(WorkerLimits::default().validate(1).is_ok());
        let error = WorkerLimits {
            contract_version: 0,
            ..WorkerLimits::default()
        }
        .validate(1)
        .expect_err("predecessor worker contract must reject");
        assert_eq!(error.code, "worker_contract");
        let error = WorkerLimits {
            maximum_workers: 2,
            ..WorkerLimits::default()
        }
        .validate(1)
        .expect_err("worker count above runtime concurrency must reject");
        assert_eq!(error.code, "worker_count_limit");
    }
}
