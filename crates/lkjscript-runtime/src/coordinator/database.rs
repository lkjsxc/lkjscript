use std::sync::Arc;

use lkjscript_host::{DatabaseProvider, DatabaseTenantFactory, DurableStorage};

use super::{CoordinatorError, MachineCoordinator};

impl<S: DurableStorage> MachineCoordinator<S> {
    pub fn attach_database(
        &mut self,
        factory: Arc<dyn DatabaseTenantFactory>,
    ) -> Result<(), CoordinatorError> {
        if self.database.is_some() {
            return Err(CoordinatorError::InvalidBootstrap);
        }
        let running = self
            .applications
            .iter()
            .filter_map(|(id, application)| application.incarnation.map(|value| (*id, value)))
            .collect::<Vec<_>>();
        let mut attached: Vec<(u64, Arc<dyn DatabaseProvider>)> = Vec::new();
        for (id, incarnation) in running {
            let provider = factory
                .attach(&tenant(id), incarnation.incarnation())
                .map_err(CoordinatorError::Host)?;
            attached.push((id, provider));
        }
        self.database = Some(factory);
        for (id, provider) in attached {
            self.applications
                .get_mut(&id)
                .ok_or(CoordinatorError::InvalidApplicationRegistry)?
                .database = Some(provider);
        }
        Ok(())
    }

    pub(super) fn attach_application_database(
        &mut self,
        id: u64,
        incarnation: crate::ApplicationIncarnationId,
    ) -> Result<(), CoordinatorError> {
        let factory = self
            .database
            .as_ref()
            .ok_or(CoordinatorError::DatabaseUnavailable)?;
        let provider = factory
            .attach(&tenant(id), incarnation.incarnation())
            .map_err(CoordinatorError::Host)?;
        self.applications
            .get_mut(&id)
            .ok_or(CoordinatorError::InvalidApplicationRegistry)?
            .database = Some(provider);
        Ok(())
    }

    pub(super) fn abort_application_database(&mut self, id: u64) -> Result<(), CoordinatorError> {
        if let Some(provider) = self
            .applications
            .get_mut(&id)
            .ok_or(CoordinatorError::InvalidApplicationRegistry)?
            .database
            .take()
        {
            provider.abort_all().map_err(CoordinatorError::Host)?;
        }
        Ok(())
    }

    pub(super) fn abort_all_databases(&mut self) -> Result<(), CoordinatorError> {
        let ids = self.applications.keys().copied().collect::<Vec<_>>();
        for id in ids {
            self.abort_application_database(id)?;
        }
        if let Some(database) = &self.database {
            database.checkpoint().map_err(CoordinatorError::Host)?;
        }
        Ok(())
    }
}

fn tenant(id: u64) -> String {
    format!("application-{id}")
}
