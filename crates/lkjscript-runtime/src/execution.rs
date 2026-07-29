use lkjscript_core::ExecutionOutcome;
use lkjscript_vm::run_chunk;

use crate::invoke::Admission;
use crate::{
    ApplicationGenerationId, InvocationOutcome, InvocationRequest, Node, RuntimeError,
    MAX_LOG_ENTRIES,
};

impl Node {
    fn complete(
        &self,
        admission: &Admission,
        outcome: &ExecutionOutcome,
    ) -> Result<(), RuntimeError> {
        let mut state = self.lock_state()?;
        if let Some(instance) = state
            .apps
            .get_mut(&admission.generation.application())
            .filter(|app| {
                app.generation(admission.generation.application()) == Some(admission.generation)
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
        generation: ApplicationGenerationId,
        arguments: Vec<String>,
    ) -> Result<InvocationOutcome, RuntimeError> {
        let admission = self.admit(generation, arguments)?;
        let outcome = run_chunk(&admission.chunk, &admission.inputs, &admission.config);
        self.complete(&admission, &outcome)?;
        Ok(InvocationOutcome {
            execution_cell: admission.cell,
            generation,
            outcome,
        })
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
                    scope.spawn(move || self.invoke(request.generation, request.arguments))
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().map_err(|_| RuntimeError::WorkerPanicked))
                .collect::<Result<Vec<_>, _>>()
        })
    }
}
