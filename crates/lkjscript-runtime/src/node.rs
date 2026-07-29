use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::{Arc, MutexGuard};

use lkjscript_core::ValidatedChunk;

use crate::model::private_inputs;
use crate::state::{AppRecord, Inner, InstanceRuntime, State};
use crate::{
    ApplicationGenerationId, ApplicationId, ApplicationInstanceId, ApplicationManifest,
    ApplicationStatus, Lifecycle, NodeIdentity, PackageContentId, RuntimeError,
};

#[derive(Clone)]
pub struct Node {
    pub(crate) inner: Arc<Inner>,
}

impl Node {
    pub fn new(identity: NodeIdentity, max_cache_entries: NonZeroUsize) -> Self {
        Self {
            inner: Arc::new(Inner {
                identity,
                state: std::sync::Mutex::new(State::new(max_cache_entries)),
                admission_changed: std::sync::Condvar::new(),
            }),
        }
    }

    pub fn identity(&self) -> NodeIdentity {
        self.inner.identity
    }

    pub(crate) fn lock_state(&self) -> Result<MutexGuard<'_, State>, RuntimeError> {
        self.inner
            .state
            .lock()
            .map_err(|_| RuntimeError::StateUnavailable)
    }

    pub fn install(
        &self,
        manifest: ApplicationManifest,
        package: PackageContentId,
        chunk: Arc<ValidatedChunk>,
    ) -> Result<ApplicationId, RuntimeError> {
        manifest.validate()?;
        if !chunk.required_capabilities().is_empty() {
            return Err(RuntimeError::UnsafeCapabilities);
        }
        let mut state = self.lock_state()?;
        let application = ApplicationId::from_nonzero(state.allocate()?);
        let lease = state.cache.lease(package, chunk)?;
        state.apps.insert(
            application,
            AppRecord {
                manifest,
                package,
                chunk: Some(lease),
                lifecycle: Lifecycle::Installed,
                generation_number: 0,
                instance: None,
            },
        );
        Ok(application)
    }

    pub fn start(
        &self,
        application: ApplicationId,
    ) -> Result<ApplicationGenerationId, RuntimeError> {
        let mut state = self.lock_state()?;
        let (lifecycle, next_generation) = {
            let app = state
                .apps
                .get(&application)
                .ok_or(RuntimeError::ApplicationNotFound(application))?;
            if app.chunk.is_none() {
                return Err(RuntimeError::IllegalTransition {
                    from: app.lifecycle,
                    to: Lifecycle::Loading,
                });
            }
            let next = app
                .generation_number
                .checked_add(1)
                .and_then(NonZeroU64::new)
                .ok_or(RuntimeError::IdentifierSpaceExhausted)?;
            (app.lifecycle, next)
        };
        let instance_serial = state.allocate()?;
        let generation = ApplicationGenerationId::new(application, next_generation);
        let instance_id = ApplicationInstanceId::new(generation, instance_serial);
        let app = state
            .apps
            .get_mut(&application)
            .ok_or(RuntimeError::ApplicationNotFound(application))?;
        app.lifecycle = lifecycle.transition(Lifecycle::Loading)?;
        app.lifecycle = app.lifecycle.transition(Lifecycle::Starting)?;
        app.generation_number = next_generation.get();
        app.instance = Some(InstanceRuntime::new(
            instance_id,
            private_inputs(Vec::new()),
        ));
        app.lifecycle = app.lifecycle.transition(Lifecycle::Running)?;
        Ok(generation)
    }

    pub fn stop(&self, generation: ApplicationGenerationId) -> Result<(), RuntimeError> {
        let application = generation.application();
        let mut state = self.lock_state()?;
        loop {
            let app = state
                .apps
                .get_mut(&application)
                .ok_or(RuntimeError::ApplicationNotFound(application))?;
            if app.generation(application) != Some(generation) {
                return Err(RuntimeError::StaleGeneration {
                    requested: generation,
                    current: app.generation(application),
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
                if app.lifecycle == Lifecycle::Quiescing || app.lifecycle == Lifecycle::Failed {
                    app.lifecycle = app.lifecycle.transition(Lifecycle::Stopping)?;
                }
                app.lifecycle = app.lifecycle.transition(Lifecycle::Stopped)?;
                return Ok(());
            }
            state = self
                .inner
                .admission_changed
                .wait(state)
                .map_err(|_| RuntimeError::StateUnavailable)?;
        }
    }

    pub fn restart(
        &self,
        generation: ApplicationGenerationId,
    ) -> Result<ApplicationGenerationId, RuntimeError> {
        self.stop(generation)?;
        self.start(generation.application())
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
        self.inner.admission_changed.notify_all();
        Ok(())
    }

    pub fn status(&self, application: ApplicationId) -> Result<ApplicationStatus, RuntimeError> {
        let state = self.lock_state()?;
        state
            .apps
            .get(&application)
            .map(|app| app.status(application))
            .ok_or(RuntimeError::ApplicationNotFound(application))
    }

    pub fn list(&self) -> Result<Vec<ApplicationStatus>, RuntimeError> {
        let state = self.lock_state()?;
        Ok(state.apps.iter().map(|(id, app)| app.status(*id)).collect())
    }

    pub fn cache_contains(&self, package: PackageContentId) -> Result<bool, RuntimeError> {
        Ok(self.lock_state()?.cache.contains(package))
    }

    pub fn cache_len(&self) -> Result<usize, RuntimeError> {
        Ok(self.lock_state()?.cache.len())
    }
}
