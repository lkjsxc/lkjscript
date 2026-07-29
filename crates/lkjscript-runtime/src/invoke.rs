use std::sync::{Arc, Mutex};

use lkjscript_core::{ExecutionConfig, ValidatedChunk};
use lkjscript_vm::ExecutionInputs;

use crate::state::State;
use crate::{
    ApplicationId, ApplicationIncarnationId, ExecutionCellId, Lifecycle, QuotaKind, RuntimeError,
    RuntimeSystem,
};

pub(crate) struct Admission {
    pub(crate) incarnation: ApplicationIncarnationId,
    pub(crate) cell: ExecutionCellId,
    pub(crate) chunk: Option<Arc<ValidatedChunk>>,
    pub(crate) process: Option<Arc<Mutex<crate::execution::process::ProcessCell>>>,
    pub(crate) inputs: ExecutionInputs,
    pub(crate) config: ExecutionConfig,
}

impl RuntimeSystem {
    fn advance_ticket(state: &mut State, application: ApplicationId) {
        if let Some(instance) = state
            .apps
            .get_mut(&application)
            .and_then(|app| app.instance.as_mut())
        {
            instance.serving_ticket = instance.serving_ticket.saturating_add(1);
        }
    }

    pub(crate) fn admit(
        &self,
        incarnation: ApplicationIncarnationId,
        arguments: Vec<String>,
    ) -> Result<Admission, RuntimeError> {
        let application = incarnation.application();
        let mut state = self.lock_state()?;
        let ticket = {
            let app = state
                .apps
                .get_mut(&application)
                .ok_or(RuntimeError::ApplicationNotFound(application))?;
            let current = app.incarnation(self.inner.identity, application);
            if current != Some(incarnation) {
                return Err(RuntimeError::StaleIncarnation {
                    requested: incarnation,
                    current,
                });
            }
            let instance = app
                .instance
                .as_mut()
                .ok_or(RuntimeError::IllegalTransition {
                    from: app.lifecycle,
                    to: Lifecycle::Running,
                })?;
            let ticket = instance.next_ticket;
            instance.next_ticket = ticket
                .checked_add(1)
                .ok_or(RuntimeError::IdentifierSpaceExhausted)?;
            ticket
        };
        loop {
            let (current, lifecycle, serving, total, max_total, active, max_concurrent) = {
                let app = state
                    .apps
                    .get(&application)
                    .ok_or(RuntimeError::ApplicationNotFound(application))?;
                let instance = app
                    .instance
                    .as_ref()
                    .ok_or(RuntimeError::IllegalTransition {
                        from: app.lifecycle,
                        to: Lifecycle::Running,
                    })?;
                (
                    app.incarnation(self.inner.identity, application),
                    app.lifecycle,
                    instance.serving_ticket == ticket,
                    instance.total,
                    app.manifest.quota.max_total_invocations.get(),
                    instance.active,
                    app.manifest.quota.max_concurrent_invocations.get(),
                )
            };
            if current != Some(incarnation) {
                return Err(RuntimeError::StaleIncarnation {
                    requested: incarnation,
                    current,
                });
            }
            if lifecycle != Lifecycle::Running && serving {
                Self::advance_ticket(&mut state, application);
                self.inner.admission_changed.notify_all();
                return Err(RuntimeError::IllegalTransition {
                    from: lifecycle,
                    to: Lifecycle::Running,
                });
            }
            if lifecycle == Lifecycle::Running && serving && total >= max_total {
                Self::advance_ticket(&mut state, application);
                self.inner.admission_changed.notify_all();
                return Err(RuntimeError::QuotaExceeded(QuotaKind::TotalInvocations));
            }
            if lifecycle == Lifecycle::Running && serving && active < max_concurrent {
                break;
            }
            state = self
                .inner
                .admission_changed
                .wait(state)
                .map_err(|_| RuntimeError::StateUnavailable)?;
        }
        let serial = state.allocate()?;
        let (instance_id, chunk, process, mut inputs, config) = {
            let app = state
                .apps
                .get(&application)
                .ok_or(RuntimeError::ApplicationNotFound(application))?;
            let instance = app
                .instance
                .as_ref()
                .ok_or(RuntimeError::IllegalTransition {
                    from: app.lifecycle,
                    to: Lifecycle::Running,
                })?;
            (
                instance.id,
                app.chunk.clone(),
                instance.process.clone(),
                instance.inputs.clone(),
                app.manifest.quota.execution.clone(),
            )
        };
        let cell = ExecutionCellId::new(instance_id, serial);
        inputs.arguments = arguments;
        if let Some(instance) = state
            .apps
            .get_mut(&application)
            .and_then(|app| app.instance.as_mut())
        {
            instance.serving_ticket = instance.serving_ticket.saturating_add(1);
            instance.active += 1;
            instance.total += 1;
            instance.metrics.admitted += 1;
            instance.metrics.peak_concurrent =
                instance.metrics.peak_concurrent.max(instance.active);
        }
        self.inner.admission_changed.notify_all();
        Ok(Admission {
            incarnation,
            cell,
            chunk,
            process,
            inputs,
            config,
        })
    }
}
