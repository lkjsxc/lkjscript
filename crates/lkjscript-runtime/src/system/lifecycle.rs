use std::num::NonZeroU64;
use std::sync::{Arc, Mutex};

use crate::execution::process::ProcessCell;
use crate::execution::protocol::{runtime_control_digest, ProcessBootstrap};
use crate::state::{private_inputs, InstanceRuntime};
use crate::{ApplicationId, ApplicationIncarnationId, Lifecycle, RuntimeError, RuntimeSystem};

impl RuntimeSystem {
    pub fn start(
        &self,
        application: ApplicationId,
    ) -> Result<ApplicationIncarnationId, RuntimeError> {
        let mut state = self.lock_state()?;
        let (incarnation, chunk, spec, package, capabilities, host, config) = {
            let app = state
                .apps
                .get_mut(&application)
                .ok_or(RuntimeError::ApplicationNotFound(application))?;
            let next = app
                .incarnation_counter
                .checked_add(1)
                .and_then(NonZeroU64::new)
                .ok_or(RuntimeError::IdentifierSpaceExhausted)?;
            app.lifecycle = app.lifecycle.transition(Lifecycle::Loading)?;
            app.lifecycle = app.lifecycle.transition(Lifecycle::Starting)?;
            app.incarnation_counter = next.get();
            (
                ApplicationIncarnationId::new(self.inner.identity, application, next),
                app.chunk.clone(),
                app.process_spec.clone(),
                app.package,
                app.manifest.capabilities.clone(),
                app.host.clone(),
                app.manifest.quota.execution.clone(),
            )
        };
        drop(state);
        let inputs = private_inputs(Vec::new(), capabilities.clone(), host);
        let process = match spec {
            Some(spec) => {
                let entry = spec.entry.to_str().ok_or_else(|| {
                    RuntimeError::ProcessCell("package entry is not UTF-8".into())
                })?;
                let bootstrap = ProcessBootstrap {
                    platform_revision: lkjscript_contracts::PLATFORM_REVISION,
                    contract: runtime_control_digest()
                        .map_err(|error| RuntimeError::ProcessCell(error.to_string()))?,
                    coordinator: incarnation.coordinator().get(),
                    application: application.get(),
                    incarnation: incarnation.incarnation(),
                    package: package.bytes(),
                    entry: entry.to_owned(),
                    expected_entry: spec.prepared.entry,
                    expected_prepared: spec.prepared.prepared,
                    expected_return_semantic: spec.prepared.return_semantic,
                    expected_root_witness_group: spec.prepared.root_witness_group,
                    expected_root_witness_member: spec.prepared.root_witness_member,
                    capabilities,
                    execution: config,
                };
                let parent_chunk = chunk.clone().ok_or_else(|| {
                    RuntimeError::ProcessCell("isolated parent prepared chunk is absent".into())
                })?;
                match ProcessCell::start(&spec, &bootstrap, parent_chunk) {
                    Ok(process) => Some(process),
                    Err(error) => {
                        self.record_start_failure(incarnation, inputs, &error)?;
                        return Err(RuntimeError::ProcessCell(error));
                    }
                }
            }
            None if chunk.is_some() => None,
            None => {
                let error = "application has no installed execution code";
                self.record_start_failure(incarnation, inputs, error)?;
                return Err(RuntimeError::ProcessCell(error.into()));
            }
        };
        let process_id = process.as_ref().map(ProcessCell::process);
        let process = process.map(|value| Arc::new(Mutex::new(value)));
        let mut state = self.lock_state()?;
        let app = state
            .apps
            .get_mut(&application)
            .ok_or(RuntimeError::ApplicationNotFound(application))?;
        if app.incarnation(self.inner.identity, application) != Some(incarnation)
            || app.lifecycle != Lifecycle::Starting
        {
            return Err(RuntimeError::StaleIncarnation {
                requested: incarnation,
                current: app.incarnation(self.inner.identity, application),
            });
        }
        app.instance = Some(InstanceRuntime::new(
            incarnation,
            inputs,
            process,
            process_id,
        ));
        app.lifecycle = app.lifecycle.transition(Lifecycle::Running)?;
        Ok(incarnation)
    }

    fn record_start_failure(
        &self,
        incarnation: ApplicationIncarnationId,
        inputs: lkjscript_vm::ExecutionInputs,
        error: &str,
    ) -> Result<(), RuntimeError> {
        let application = incarnation.application();
        let mut state = self.lock_state()?;
        let app = state
            .apps
            .get_mut(&application)
            .ok_or(RuntimeError::ApplicationNotFound(application))?;
        if app.incarnation(self.inner.identity, application) == Some(incarnation) {
            let mut instance = InstanceRuntime::new(incarnation, inputs, None, None);
            instance.logs.push_back(error.chars().take(4_096).collect());
            app.instance = Some(instance);
            app.lifecycle = Lifecycle::Failed;
        }
        self.inner.admission_changed.notify_all();
        Ok(())
    }
}
