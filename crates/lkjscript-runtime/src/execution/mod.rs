pub(crate) mod process;
pub mod protocol;
pub mod rehydration;

use lkjscript_core::ExecutionOutcome;
use lkjscript_vm::run_chunk;

use crate::invoke::Admission;
use crate::{
    ApplicationIncarnationId, InvocationOutcome, InvocationRequest, RuntimeError, RuntimeSystem,
    MAX_LOG_ENTRIES,
};

impl RuntimeSystem {
    fn complete(
        &self,
        admission: &Admission,
        outcome: &ExecutionOutcome,
    ) -> Result<(), RuntimeError> {
        let mut state = self.lock_state()?;
        state.global.complete();
        if let Some(instance) = state
            .apps
            .get_mut(&admission.incarnation.application())
            .filter(|app| {
                app.incarnation(self.inner.identity, admission.incarnation.application())
                    == Some(admission.incarnation)
            })
            .and_then(|app| app.instance.as_mut())
        {
            instance.active = instance.active.saturating_sub(1);
            instance.metrics.completed += 1;
            if matches!(outcome.primary(), ExecutionOutcome::Trapped(_)) {
                instance.metrics.trapped += 1;
            }
            if instance.logs.len() == MAX_LOG_ENTRIES {
                instance.logs.pop_front();
            }
            instance.logs.push_back(format!(
                "cell {}: {}",
                admission.cell.serial(),
                outcome.summary()
            ));
        }
        self.inner.admission_changed.notify_all();
        Ok(())
    }

    pub fn invoke(
        &self,
        incarnation: ApplicationIncarnationId,
        arguments: Vec<String>,
    ) -> Result<InvocationOutcome, RuntimeError> {
        let admission = self.admit(incarnation, arguments)?;
        let execution = if let Some(process) = &admission.process {
            let result = process
                .lock()
                .map_err(|_| (RuntimeError::StateUnavailable, true))
                .and_then(|mut process| {
                    process
                        .invoke(admission.cell.serial(), admission.inputs.arguments.clone())
                        .map_err(|failure| {
                            let fatal = failure.is_fatal();
                            (RuntimeError::ProcessCell(failure.into_message()), fatal)
                        })
                });
            result.and_then(|(outcome, output, flushes, report)| {
                relay_process_output(&admission, &output, flushes)
                    .map_err(|error| (error, true))?;
                Ok((outcome, report))
            })
        } else if let Some(chunk) = &admission.chunk {
            Ok((run_chunk(chunk, &admission.inputs, &admission.config), None))
        } else {
            Err((
                RuntimeError::ProcessCell("running application has no execution cell".into()),
                true,
            ))
        };
        let (outcome, rehydration) = match execution {
            Ok(outcome) => outcome,
            Err((error, true)) => {
                self.fail_cell(&admission, &error.to_string())?;
                return Err(error);
            }
            Err((error, false)) => {
                self.reject_invocation(&admission, &error.to_string())?;
                return Err(error);
            }
        };
        self.complete(&admission, &outcome)?;
        Ok(InvocationOutcome {
            execution_cell: admission.cell,
            incarnation,
            outcome,
            rehydration,
        })
    }

    fn reject_invocation(&self, admission: &Admission, error: &str) -> Result<(), RuntimeError> {
        let mut state = self.lock_state()?;
        state.global.complete();
        if let Some(instance) = state
            .apps
            .get_mut(&admission.incarnation.application())
            .filter(|app| {
                app.incarnation(self.inner.identity, admission.incarnation.application())
                    == Some(admission.incarnation)
            })
            .and_then(|app| app.instance.as_mut())
        {
            instance.active = instance.active.saturating_sub(1);
            instance.metrics.completed += 1;
            if instance.logs.len() == MAX_LOG_ENTRIES {
                instance.logs.pop_front();
            }
            instance.logs.push_back(error.chars().take(4_096).collect());
        }
        self.inner.admission_changed.notify_all();
        Ok(())
    }

    fn fail_cell(&self, admission: &Admission, error: &str) -> Result<(), RuntimeError> {
        let mut state = self.lock_state()?;
        state.global.complete();
        if let Some(app) = state
            .apps
            .get_mut(&admission.incarnation.application())
            .filter(|app| {
                app.incarnation(self.inner.identity, admission.incarnation.application())
                    == Some(admission.incarnation)
            })
        {
            app.lifecycle = crate::Lifecycle::Failed;
            if let Some(instance) = &mut app.instance {
                instance.active = instance.active.saturating_sub(1);
                instance.metrics.completed += 1;
                instance.process = None;
                instance.process_id = None;
                if instance.logs.len() == MAX_LOG_ENTRIES {
                    instance.logs.pop_front();
                }
                instance.logs.push_back(error.chars().take(4_096).collect());
            }
        }
        self.inner.admission_changed.notify_all();
        Ok(())
    }

    pub fn invoke_concurrent(
        &self,
        requests: Vec<InvocationRequest>,
    ) -> Result<Vec<Result<InvocationOutcome, RuntimeError>>, RuntimeError> {
        if requests.len() < 2 {
            return Err(RuntimeError::AtLeastTwoInvocationsRequired);
        }
        std::thread::scope(|scope| {
            let handles: Vec<_> = requests
                .into_iter()
                .map(|request| {
                    scope.spawn(move || self.invoke(request.incarnation, request.arguments))
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().map_err(|_| RuntimeError::WorkerPanicked))
                .collect::<Result<Vec<_>, _>>()
        })
    }
}

fn relay_process_output(
    admission: &Admission,
    output: &[u8],
    flushes: u64,
) -> Result<(), RuntimeError> {
    if output.is_empty() && flushes == 0 {
        return Ok(());
    }
    let provider = admission
        .inputs
        .host
        .stdio
        .as_ref()
        .ok_or_else(|| RuntimeError::ProcessCell("stdio provider is unavailable".into()))?;
    if !output.is_empty() {
        provider
            .write(output)
            .map_err(|error| RuntimeError::ProcessCell(error.to_string()))?;
    }
    for _ in 0..flushes {
        provider
            .flush()
            .map_err(|error| RuntimeError::ProcessCell(error.to_string()))?;
    }
    Ok(())
}
