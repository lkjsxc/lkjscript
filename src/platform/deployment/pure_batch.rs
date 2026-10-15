//! One immutable admission with a fresh ordinary foreground lifetime for each pure row.

use super::*;
use crate::platform::execution::normalized::{
    PreparedCommandInvocation, execution_diagnostic, prepare_command_invocation,
    require_pure_artifact_target, require_pure_invocation,
};
use crate::platform::json::JsonLimits;

pub(crate) struct PreparedPureRunBatch {
    admitted: AdmittedDeployment,
    pub artifact_sha256: String,
}

impl PreparedPureRunBatch {
    pub fn deadline_milliseconds(&self) -> Option<u64> {
        self.admitted
            .descriptor
            .runtime
            .as_ref()
            .map(|limits| limits.request_deadline_milliseconds)
    }

    pub fn load(path: &Path, control: &ExecutionControl) -> Result<Self, Diagnostic> {
        let mut artifact_sha256 = String::new();
        let admitted = AdmittedDeployment::load_shared_digest(
            path,
            control,
            &mut BTreeMap::new(),
            Some(&mut artifact_sha256),
        )?;
        let descriptor = &admitted.descriptor;
        if !descriptor.grants.is_empty()
            || !descriptor.configuration.is_empty()
            || !descriptor.secrets.is_empty()
            || descriptor.listen.is_some()
            || descriptor.http.is_some()
            || descriptor.session.is_some()
            || descriptor.worker.is_some()
        {
            return Err(Diagnostic::new(
                DiagnosticClass::Capability,
                "run_batch_pure",
                "pure batches reject grants, configuration, secrets and service adapters",
            ));
        }
        require_pure_artifact_target(&admitted.program, &Name::new(descriptor.target.clone())?)?;
        Ok(Self {
            admitted,
            artifact_sha256,
        })
    }

    pub fn prepare_row(
        &self,
        target: Option<&str>,
        arguments: &[u8],
        execution: Option<RunPolicy>,
        runtime: Handle,
        control: &ExecutionControl,
    ) -> Result<(PreparedDeployment, PreparedCommandInvocation), Diagnostic> {
        control.check().map_err(execution_diagnostic)?;
        let mut descriptor = self.admitted.descriptor.clone();
        if let Some(target) = target {
            descriptor.target = target.to_owned();
        }
        if let Some(execution) = execution {
            descriptor.execution = Some(execution);
        }
        validate_descriptor(&descriptor)?;
        validate_program_descriptor(&descriptor, &self.admitted.program)?;
        let name = Name::new(descriptor.target.clone())?;
        require_pure_artifact_target(&self.admitted.program, &name)?;
        // Complete ordinary typed ingress precedes adapters and all mutable runtime owners.
        let invocation = prepare_command_invocation(
            &self.admitted.program,
            &name,
            arguments,
            JsonLimits::default(),
            control,
        )?;
        require_pure_invocation(&self.admitted.program, &invocation)?;
        control.check().map_err(execution_diagnostic)?;
        if let Some(deadline) = control.deadline() {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let mut limits = descriptor.runtime.take().unwrap_or_else(|| ResidentLimits {
                maximum_concurrent_tasks: 1,
                maximum_queued_tasks: 0,
                ..ResidentLimits::default()
            });
            limits.request_deadline_milliseconds =
                remaining.as_millis().max(1).min(u128::from(u64::MAX)) as u64;
            descriptor.runtime = Some(limits);
            validate_descriptor(&descriptor)?;
        }
        let mut executor = StructuredExecutor::new();
        let prepared = PreparedDeployment::prepare(
            descriptor,
            Arc::clone(&self.admitted.program),
            self.admitted.artifact_digest.clone(),
            &self.admitted.directory,
            runtime,
            SecretCatalog::from_environment(&[])?,
            control,
            executor.handle(),
        );
        match prepared {
            Ok(mut prepared) => {
                prepared.executor_owner = Some(executor);
                match control.check() {
                    Ok(()) => Ok((prepared, invocation)),
                    Err(error) => Err(close_uninvoked_row(prepared, execution_diagnostic(error))),
                }
            }
            Err(mut error) => {
                if let Err(cleanup) = executor.shutdown() {
                    error.notes.push(format!(
                        "structured worker cleanup failed with safe code '{}'",
                        cleanup.code
                    ));
                }
                Err(error)
            }
        }
    }
}

pub(crate) fn close_uninvoked_row(
    mut prepared: PreparedDeployment,
    mut error: Diagnostic,
) -> Diagnostic {
    prepared.close_uninvoked(&mut error);
    if let Some(mut executor) = prepared.take_executor()
        && let Err(cleanup) = executor.shutdown()
    {
        error.notes.push(format!(
            "structured worker cleanup failed with safe code '{}'",
            cleanup.code
        ));
    }
    error
}
