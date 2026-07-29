use crate::{ApplicationId, ApplicationIncarnationId, Lifecycle, RuntimeError, RuntimeSystem};

impl RuntimeSystem {
    pub fn stop(&self, incarnation: ApplicationIncarnationId) -> Result<(), RuntimeError> {
        let application = incarnation.application();
        let process = {
            let mut state = self.lock_state()?;
            loop {
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
                if app.lifecycle == Lifecycle::Running {
                    app.lifecycle = app.lifecycle.transition(Lifecycle::Quiescing)?;
                    if let Some(instance) = &mut app.instance {
                        instance.cancelled = true;
                    }
                    self.inner.admission_changed.notify_all();
                }
                let active = app.instance.as_ref().map_or(0, |instance| instance.active);
                if active == 0 {
                    if matches!(app.lifecycle, Lifecycle::Quiescing | Lifecycle::Failed) {
                        app.lifecycle = app.lifecycle.transition(Lifecycle::Stopping)?;
                    }
                    break app
                        .instance
                        .as_ref()
                        .and_then(|instance| instance.process.clone());
                }
                state = self
                    .inner
                    .admission_changed
                    .wait(state)
                    .map_err(|_| RuntimeError::StateUnavailable)?;
            }
        };
        let stop_error = process.and_then(|process| match process.lock() {
            Ok(mut process) => process.stop().err(),
            Err(_) => Some("process cell lock is unavailable".into()),
        });
        let mut state = self.lock_state()?;
        let app = state
            .apps
            .get_mut(&application)
            .ok_or(RuntimeError::ApplicationNotFound(application))?;
        if let Some(instance) = &mut app.instance {
            instance.process = None;
            instance.process_id = None;
        }
        if let Some(error) = stop_error {
            app.lifecycle = Lifecycle::Failed;
            self.inner.admission_changed.notify_all();
            return Err(RuntimeError::ProcessCell(error));
        }
        app.lifecycle = app.lifecycle.transition(Lifecycle::Stopped)?;
        Ok(())
    }

    pub fn restart(
        &self,
        incarnation: ApplicationIncarnationId,
    ) -> Result<ApplicationIncarnationId, RuntimeError> {
        self.stop(incarnation)?;
        self.start(incarnation.application())
    }

    pub fn remove(&self, application: ApplicationId) -> Result<(), RuntimeError> {
        let mut state = self.lock_state()?;
        let app = state
            .apps
            .get_mut(&application)
            .ok_or(RuntimeError::ApplicationNotFound(application))?;
        app.lifecycle = app.lifecycle.transition(Lifecycle::Uninstalled)?;
        app.instance = None;
        app.chunk = None;
        app.process_spec = None;
        self.inner.admission_changed.notify_all();
        Ok(())
    }
}
