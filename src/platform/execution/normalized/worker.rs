//! Structured worker topology for exact Graph 10 resident deployments.

use super::resident::NormalizedResidentDeployment;
use super::value::NormalizedValue;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::ExecutionError;
use crate::platform::kernel::TypeForm;
use crate::platform::package::RunnerKind;
use crate::platform::runtime::ShutdownReceipt;
use crate::platform::worker::{
    ResidentWorker, WorkerLimits, WorkerReceipt, run_worker_topology, worker_result_type,
};
use std::future::Future;

#[derive(Clone)]
pub(crate) struct NormalizedWorkerApplication {
    resident: NormalizedResidentDeployment,
    limits: WorkerLimits,
}

impl NormalizedWorkerApplication {
    pub(crate) fn new(
        resident: NormalizedResidentDeployment,
        limits: WorkerLimits,
    ) -> Result<Self, Diagnostic> {
        limits.validate(resident.limits().maximum_concurrent_tasks)?;
        if resident.target().runner != RunnerKind::Worker {
            return Err(worker_diagnostic(
                DiagnosticClass::Source,
                "normalized_worker_runner_kind",
                "normalized worker topology requires a worker runner target",
            ));
        }
        let port_index = resident.target().port.ok_or_else(|| {
            worker_diagnostic(
                DiagnosticClass::Corrupt,
                "normalized_worker_target_port",
                "normalized worker target has no exact port",
            )
        })?;
        let port = resident
            .program()
            .ports
            .get(port_index.0 as usize)
            .ok_or_else(|| {
                worker_diagnostic(
                    DiagnosticClass::Corrupt,
                    "normalized_worker_port",
                    "selected worker target port escaped the exact runtime table",
                )
            })?;
        let shape = resident
            .program()
            .types
            .get(&port.function_type)
            .map(|ty| &ty.form);
        let valid = match shape {
            Some(TypeForm::Function { parameters, result }) => {
                parameters.is_empty()
                    && resident
                        .program()
                        .types
                        .get(result)
                        .is_some_and(|ty| matches!(ty.form, TypeForm::Bool))
            }
            Some(TypeForm::TaskFunction {
                parameters,
                result,
                effect,
            }) => {
                effect.is_closed()
                    && parameters.is_empty()
                    && resident
                        .program()
                        .types
                        .get(result)
                        .is_some_and(|ty| matches!(ty.form, TypeForm::Bool))
            }
            _ => false,
        };
        if !valid {
            return Err(worker_diagnostic(
                DiagnosticClass::Semantic,
                "normalized_worker_port_signature",
                "normalized worker port must have the exact signature () -> Bool",
            ));
        }
        Ok(Self { resident, limits })
    }

    pub(crate) async fn run(
        self,
        shutdown: impl Future<Output = ()> + Send,
    ) -> Result<WorkerReceipt, Diagnostic> {
        run_worker_topology(self.resident, self.limits, shutdown).await
    }

    pub(crate) fn resident(&self) -> &NormalizedResidentDeployment {
        &self.resident
    }
}

impl ResidentWorker for NormalizedResidentDeployment {
    async fn invoke_worker(&self) -> Result<bool, ExecutionError> {
        let receipt = self.invoke(Vec::new()).await?;
        match receipt.value {
            NormalizedValue::Bool(value) => Ok(value),
            _ => Err(worker_result_type()),
        }
    }

    async fn shutdown_worker(&self) -> ShutdownReceipt {
        self.shutdown().await
    }
}

fn worker_diagnostic(
    class: DiagnosticClass,
    code: &'static str,
    message: &'static str,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
