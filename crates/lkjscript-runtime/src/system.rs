use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::{Arc, MutexGuard};

use lkjscript_core::ValidatedChunk;

use crate::model::private_inputs;
use crate::state::{AppRecord, Inner, InstanceRuntime, State};
use crate::{
    ApplicationId, ApplicationIncarnationId, ApplicationManifest, ApplicationStatus,
    CoordinatorIdentity, Lifecycle, PackageContentId, RuntimeError,
};

#[derive(Clone)]
pub struct RuntimeSystem {
    pub(crate) inner: Arc<Inner>,
}

impl RuntimeSystem {
    pub fn new(identity: CoordinatorIdentity, max_cache_entries: NonZeroUsize) -> Self {
        Self {
            inner: Arc::new(Inner {
                identity,
                state: std::sync::Mutex::new(State::new(max_cache_entries)),
                admission_changed: std::sync::Condvar::new(),
            }),
        }
    }

    pub fn identity(&self) -> CoordinatorIdentity {
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
                incarnation_counter: 0,
                instance: None,
            },
        );
        Ok(application)
    }

    pub fn start(
        &self,
        application: ApplicationId,
    ) -> Result<ApplicationIncarnationId, RuntimeError> {
        let mut state = self.lock_state()?;
        let (lifecycle, next_incarnation) = {
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
                .incarnation_counter
                .checked_add(1)
                .and_then(NonZeroU64::new)
                .ok_or(RuntimeError::IdentifierSpaceExhausted)?;
            (app.lifecycle, next)
        };
        let incarnation =
            ApplicationIncarnationId::new(self.inner.identity, application, next_incarnation);
        let app = state
            .apps
            .get_mut(&application)
            .ok_or(RuntimeError::ApplicationNotFound(application))?;
        app.lifecycle = lifecycle.transition(Lifecycle::Loading)?;
        app.lifecycle = app.lifecycle.transition(Lifecycle::Starting)?;
        app.incarnation_counter = next_incarnation.get();
        app.instance = Some(InstanceRuntime::new(
            incarnation,
            private_inputs(Vec::new()),
        ));
        app.lifecycle = app.lifecycle.transition(Lifecycle::Running)?;
        Ok(incarnation)
    }

    pub fn stop(&self, incarnation: ApplicationIncarnationId) -> Result<(), RuntimeError> {
        let application = incarnation.application();
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
        self.inner.admission_changed.notify_all();
        Ok(())
    }

    pub fn status(&self, application: ApplicationId) -> Result<ApplicationStatus, RuntimeError> {
        let state = self.lock_state()?;
        state
            .apps
            .get(&application)
            .map(|app| app.status(self.inner.identity, application))
            .ok_or(RuntimeError::ApplicationNotFound(application))
    }

    pub fn list(&self) -> Result<Vec<ApplicationStatus>, RuntimeError> {
        let state = self.lock_state()?;
        Ok(state
            .apps
            .iter()
            .map(|(id, app)| app.status(self.inner.identity, *id))
            .collect())
    }

    pub fn cache_contains(&self, package: PackageContentId) -> Result<bool, RuntimeError> {
        Ok(self.lock_state()?.cache.contains(package))
    }

    pub fn cache_len(&self) -> Result<usize, RuntimeError> {
        Ok(self.lock_state()?.cache.len())
    }
}
