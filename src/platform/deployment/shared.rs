//! Exact in-process code sharing; operational authority remains per deployment.

use super::*;

pub const MAXIMUM_SHARED_DEPLOYMENTS: usize = 64;
pub const SHARED_RUNTIME_CONTRACT_VERSION: u16 = 3;

/// These are retained encoded object bytes and table counts, not heap/RSS estimates.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SharedProgramObservation {
    pub artifact_digest: String,
    pub instances: usize,
    pub artifact_object_bytes: u64,
    pub functions: usize,
    pub types: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SharedRuntimeObservation {
    pub contract_version: u16,
    pub runtime_version: String,
    pub instances: usize,
    pub programs: Vec<SharedProgramObservation>,
    pub maximum_concurrent_tasks: usize,
    pub maximum_queued_tasks: usize,
    pub private_configuration_fields: usize,
    pub executor: StructuredExecutorObservation,
}

/// A finite service group, not a global deployment selector or mutable code cache.
/// Preparation opens independent adapters only after every member is admitted.
#[derive(Debug)]
pub struct PreparedServiceSet {
    deployments: Vec<PreparedDeployment>,
    observation: SharedRuntimeObservation,
    executor: StructuredExecutor,
}

impl PreparedServiceSet {
    pub fn load(
        paths: &[PathBuf],
        runtime: Handle,
        control: &ExecutionControl,
    ) -> Result<Self, Diagnostic> {
        if paths.is_empty() || paths.len() > MAXIMUM_SHARED_DEPLOYMENTS {
            return Err(shared_limit(format!(
                "a shared service group requires 1 through {MAXIMUM_SHARED_DEPLOYMENTS} deployments"
            )));
        }
        let mut programs = BTreeMap::new();
        let mut admitted = Vec::with_capacity(paths.len());
        let mut concurrent = 0_usize;
        let mut queued = 0_usize;
        let mut configuration_fields = 0_usize;
        for path in paths {
            let item = AdmittedDeployment::load_shared(path, control, &mut programs)?;
            let limits = item
                .descriptor
                .runtime
                .as_ref()
                .ok_or_else(missing_resident_policy)?;
            if item.descriptor.execution.is_none() {
                return Err(missing_resident_policy());
            }
            let target = item
                .program
                .root_target(&Name::new(item.descriptor.target.clone())?)
                .ok_or_else(|| {
                    deployment_error("shared_serve_runner", "shared service target is absent")
                })?;
            if !matches!(target.runner, RunnerKind::Http | RunnerKind::Interactive)
                || item.descriptor.listen.is_none()
            {
                return Err(deployment_error(
                    "shared_serve_runner",
                    "shared serve requires HTTP or interactive targets with listen addresses",
                ));
            }
            concurrent = concurrent
                .checked_add(limits.maximum_concurrent_tasks)
                .ok_or_else(|| shared_limit("shared task capacity overflowed"))?;
            queued = queued
                .checked_add(limits.maximum_queued_tasks)
                .ok_or_else(|| shared_limit("shared queue capacity overflowed"))?;
            if concurrent > MAXIMUM_CONCURRENT_TASKS || queued > MAXIMUM_QUEUED_TASKS {
                return Err(shared_limit(format!(
                    "the group may declare at most {MAXIMUM_CONCURRENT_TASKS} concurrent and \
                     {MAXIMUM_QUEUED_TASKS} queued resident tasks in total"
                )));
            }
            configuration_fields = configuration_fields
                .checked_add(item.descriptor.configuration.len())
                .ok_or_else(|| shared_limit("shared configuration accounting overflowed"))?;
            admitted.push(item);
        }

        let mut executor = StructuredExecutor::new();
        let observation = SharedRuntimeObservation {
            contract_version: SHARED_RUNTIME_CONTRACT_VERSION,
            runtime_version: crate::PRODUCT_VERSION.to_owned(),
            instances: admitted.len(),
            programs: programs
                .iter()
                .map(|(digest, program)| SharedProgramObservation {
                    artifact_digest: digest.clone(),
                    instances: admitted
                        .iter()
                        .filter(|item| item.artifact_digest == *digest)
                        .count(),
                    artifact_object_bytes: program.artifact().work.object_bytes,
                    functions: program.functions.len(),
                    types: program.types.len(),
                })
                .collect(),
            maximum_concurrent_tasks: concurrent,
            maximum_queued_tasks: queued,
            private_configuration_fields: configuration_fields,
            executor: executor.observe(),
        };
        // No process-global cache keeps a stopped group's programs alive.
        drop(programs);
        let mut deployments: Vec<PreparedDeployment> = Vec::with_capacity(admitted.len());
        for item in admitted {
            match item.prepare(runtime.clone(), control, executor.handle()) {
                Ok(deployment) => deployments.push(deployment),
                Err(mut error) => {
                    close_uninvoked(&deployments, &mut error);
                    shutdown_executor(&mut executor, &mut error);
                    return Err(error);
                }
            }
        }
        if let Err(failure) = control.check() {
            let mut error = super::super::execution::normalized::execution_diagnostic(failure);
            close_uninvoked(&deployments, &mut error);
            shutdown_executor(&mut executor, &mut error);
            return Err(error);
        }
        Ok(Self {
            deployments,
            observation,
            executor,
        })
    }

    pub fn observe(&self) -> SharedRuntimeObservation {
        let mut observation = self.observation.clone();
        observation.executor = self.executor.observe();
        observation
    }

    pub fn deployments(&self) -> &[PreparedDeployment] {
        &self.deployments
    }

    /// Transfer prepared members and their sole worker owner independently.
    /// The owner retains no program or application authority.
    pub fn into_parts(self) -> (Vec<PreparedDeployment>, StructuredExecutor) {
        (self.deployments, self.executor)
    }

    /// Only before invoking any member. Started members require joined shutdown.
    pub fn close_uninvoked(&mut self, error: &mut Diagnostic) {
        self.executor.close_dispatch();
        close_uninvoked(&self.deployments, error);
        shutdown_executor(&mut self.executor, error);
    }
}

fn shutdown_executor(executor: &mut StructuredExecutor, error: &mut Diagnostic) {
    executor.close_dispatch();
    if let Err(failure) = executor.shutdown() {
        error.notes.push(format!(
            "structured worker cleanup failed with safe code '{}'",
            failure.code
        ));
    }
}

fn close_uninvoked(deployments: &[PreparedDeployment], error: &mut Diagnostic) {
    for deployment in deployments.iter().rev() {
        deployment.close_uninvoked(error);
    }
}

fn shared_limit(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Resource, "shared_serve_limit", message)
}

#[cfg(test)]
mod tests;
