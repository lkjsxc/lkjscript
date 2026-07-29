mod install;
mod lifecycle;
mod stop;

use std::num::NonZeroUsize;
use std::sync::{Arc, MutexGuard};

use crate::state::{Inner, State};
use crate::{
    ApplicationId, ApplicationStatus, CoordinatorIdentity, PackageContentId, RuntimeError,
};

#[derive(Clone)]
pub struct RuntimeSystem {
    pub(crate) inner: Arc<Inner>,
}

impl RuntimeSystem {
    pub fn new(identity: CoordinatorIdentity, max_cache_entries: NonZeroUsize) -> Self {
        Self::with_limits(identity, max_cache_entries, crate::RuntimeLimits::default())
    }

    pub fn with_limits(
        identity: CoordinatorIdentity,
        max_cache_entries: NonZeroUsize,
        limits: crate::RuntimeLimits,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                identity,
                state: std::sync::Mutex::new(State::new(max_cache_entries, limits)),
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

    pub fn accounting(&self) -> Result<crate::RuntimeAccounting, RuntimeError> {
        Ok(self.lock_state()?.global.accounting())
    }

    pub fn cache_contains(&self, package: PackageContentId) -> Result<bool, RuntimeError> {
        Ok(self.lock_state()?.cache.contains(package))
    }

    pub fn cache_len(&self) -> Result<usize, RuntimeError> {
        Ok(self.lock_state()?.cache.len())
    }
}
