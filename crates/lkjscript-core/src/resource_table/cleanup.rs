use super::{ResourceObservation, ResourceTable, ResourceTableError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCleanupAttempt<R> {
    resource: ResourceObservation,
    outcome: R,
}

impl<R> ResourceCleanupAttempt<R> {
    pub const fn resource(&self) -> &ResourceObservation {
        &self.resource
    }

    pub const fn outcome(&self) -> &R {
        &self.outcome
    }

    pub fn into_outcome(self) -> R {
        self.outcome
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCleanupReport<R> {
    attempts: Vec<ResourceCleanupAttempt<R>>,
}

impl<R> ResourceCleanupReport<R> {
    pub fn attempts(&self) -> &[ResourceCleanupAttempt<R>] {
        &self.attempts
    }

    pub const fn count(&self) -> usize {
        self.attempts.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.attempts.is_empty()
    }

    pub fn into_attempts(self) -> Vec<ResourceCleanupAttempt<R>> {
        self.attempts
    }
}

impl<P> ResourceTable<P> {
    pub fn cleanup_owned_reverse<R>(
        &mut self,
        mut cleanup: impl FnMut(ResourceObservation, P) -> R,
    ) -> Result<ResourceCleanupReport<R>, ResourceTableError> {
        let mut attempts = Vec::with_capacity(self.owned_order.len());
        while let Some(index) = self.owned_order.last().copied() {
            let prepared = self.prepare_owned_close(index)?;
            self.remove_owned_order(index);
            let resource = prepared.observation.clone();
            let outcome = cleanup(prepared.observation, prepared.payload);
            self.finish_close(prepared.slot);
            attempts.push(ResourceCleanupAttempt { resource, outcome });
        }
        Ok(ResourceCleanupReport { attempts })
    }
}
