//! Bounded resident execution for exact Graph 7 deployments.

use super::deployment::NormalizedPreparedDeployment;
use super::prepare::{NormalizedProgram, NormalizedTarget};
use super::resource::NormalizedResourceScope;
use super::value::NormalizedValue;
use super::vm::{NormalizedRunObservation, NormalizedRunPolicy, NormalizedVm};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::ExecutionError;
use crate::platform::runtime::{
    ResidentKernel, ResidentLimits, ResidentObservation, ShutdownReceipt,
};
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NormalizedResidentInvocationReceipt {
    pub value: NormalizedValue,
    pub execution: NormalizedRunObservation,
    pub queue_nanoseconds: u64,
    pub execution_nanoseconds: u64,
    pub task_id: u64,
}

#[derive(Clone)]
pub(crate) struct NormalizedResidentDeployment {
    program: Arc<NormalizedProgram>,
    deployment: NormalizedPreparedDeployment,
    target: NormalizedTarget,
    policy: NormalizedRunPolicy,
    kernel: ResidentKernel,
}

impl NormalizedResidentDeployment {
    pub(crate) fn prepare(
        program: Arc<NormalizedProgram>,
        deployment: NormalizedPreparedDeployment,
        limits: ResidentLimits,
        policy: NormalizedRunPolicy,
    ) -> Result<Self, Diagnostic> {
        let observation = deployment.observation();
        if observation.artifact_manifest != program.artifact().manifest_digest
            || observation.repository != program.root_repository
            || observation.package != program.root_package
            || observation.revision != program.root_revision
            || observation.semantic_state != program.root_semantic_state
        {
            return Err(resident_diagnostic(
                "normalized_resident_deployment_binding",
                "prepared deployment and executable artifact do not bind one exact accepted root",
            ));
        }
        let target = program
            .root_target(deployment.target())
            .cloned()
            .ok_or_else(|| {
                resident_diagnostic(
                    "normalized_resident_target",
                    "prepared deployment target is absent from its exact executable artifact",
                )
            })?;
        if deployment.capabilities().component() != target.component {
            return Err(resident_diagnostic(
                "normalized_resident_component",
                "prepared deployment grants are bound to another exact target component",
            ));
        }
        Ok(Self {
            program,
            deployment,
            target,
            policy,
            kernel: ResidentKernel::new(limits)?,
        })
    }

    pub(crate) fn target(&self) -> &NormalizedTarget {
        &self.target
    }

    pub(crate) fn deployment(&self) -> &NormalizedPreparedDeployment {
        &self.deployment
    }

    pub(crate) fn program(&self) -> &NormalizedProgram {
        &self.program
    }

    pub(crate) fn limits(&self) -> &ResidentLimits {
        self.kernel.limits()
    }

    pub(crate) fn observe(&self) -> ResidentObservation {
        self.kernel.observe()
    }

    pub(crate) async fn invoke(
        &self,
        arguments: Vec<NormalizedValue>,
    ) -> Result<NormalizedResidentInvocationReceipt, ExecutionError> {
        self.invoke_scoped(NormalizedResourceScope::new()?, arguments)
            .await
    }

    pub(crate) async fn invoke_scoped(
        &self,
        resources: NormalizedResourceScope,
        arguments: Vec<NormalizedValue>,
    ) -> Result<NormalizedResidentInvocationReceipt, ExecutionError> {
        let program = Arc::clone(&self.program);
        let target = self.target.clone();
        let capabilities = self.deployment.capabilities().clone();
        let policy = self.policy;
        let receipt = self
            .kernel
            .invoke(move |control| {
                NormalizedVm::new(&program, policy).invoke_target_scoped(
                    &target,
                    arguments,
                    Some(&capabilities),
                    &resources,
                    &control,
                )
            })
            .await?;
        let (value, execution) = receipt.value;
        Ok(NormalizedResidentInvocationReceipt {
            value,
            execution,
            queue_nanoseconds: receipt.queue_nanoseconds,
            execution_nanoseconds: receipt.execution_nanoseconds,
            task_id: receipt.task_id,
        })
    }

    pub(crate) async fn shutdown(&self) -> ShutdownReceipt {
        let capabilities = self.deployment.capabilities().clone();
        self.kernel.shutdown(move || capabilities.shutdown()).await
    }
}

fn resident_diagnostic(code: &'static str, message: &'static str) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}
