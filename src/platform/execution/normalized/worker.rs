//! Structured worker topology for exact Graph 8 resident deployments.

use super::resident::NormalizedResidentDeployment;
use super::value::NormalizedValue;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::ExecutionError;
use crate::platform::kernel::{TypeForm, TypeObjectInterner};
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
        let port = resident
            .program()
            .ports
            .get(resident.target().port.0 as usize)
            .ok_or_else(|| {
                worker_diagnostic(
                    DiagnosticClass::Corrupt,
                    "normalized_worker_port",
                    "selected worker target port escaped the exact runtime table",
                )
            })?;
        if port.function_type != worker_function_type()? {
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

fn worker_function_type() -> Result<crate::platform::kernel::TypeObjectDigest, Diagnostic> {
    let mut types = TypeObjectInterner::default();
    let result = types.intern(TypeForm::Bool)?;
    types.intern(TypeForm::Function {
        parameters: Vec::new(),
        result,
    })
}

fn worker_diagnostic(
    class: DiagnosticClass,
    code: &'static str,
    message: &'static str,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
