use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;

use lkjscript_contracts::PLATFORM_REVISION;
use lkjscript_host::{DurableStorage, HostError};

use crate::{
    ControlFailure, ControlIdentity, ControlOperation, ControlRequest, ControlStore,
    ControlStoreError, ControlSuccess, CoordinatorIdentity, RuntimeError, RuntimeSystem,
};

mod application_registry;
mod database;
mod lease;
#[cfg(test)]
mod tests;

use application_registry::ManagedApplication;
pub use lease::CoordinatorLease;

const BOOTSTRAP_KEY: &str = "system/bootstrap";
const CLEAN_KEY: &str = "system/clean-shutdown";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoordinatorError {
    ControlStore(ControlStoreError),
    Runtime(RuntimeError),
    Host(HostError),
    IdentityMismatch,
    InvalidBootstrap,
    InvalidApplicationRegistry,
    WorkerUnavailable,
    DatabaseUnavailable,
}

impl fmt::Display for CoordinatorError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ControlStore(error) => write!(output, "coordinator control store: {error}"),
            Self::Runtime(error) => write!(output, "coordinator runtime: {error}"),
            Self::Host(error) => write!(output, "coordinator host: {error}"),
            Self::IdentityMismatch => output.write_str("durable coordinator identity mismatch"),
            Self::InvalidBootstrap => output.write_str("invalid durable coordinator bootstrap"),
            Self::InvalidApplicationRegistry => {
                output.write_str("invalid durable application registry")
            }
            Self::WorkerUnavailable => output.write_str("process-cell worker is unavailable"),
            Self::DatabaseUnavailable => output.write_str("database tenant service is unavailable"),
        }
    }
}

impl std::error::Error for CoordinatorError {}

impl From<ControlStoreError> for CoordinatorError {
    fn from(value: ControlStoreError) -> Self {
        Self::ControlStore(value)
    }
}

impl From<RuntimeError> for CoordinatorError {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoordinatorStatus {
    pub identity: CoordinatorIdentity,
    pub principal: u32,
    pub previous_shutdown_clean: bool,
    pub control_sequence: u64,
    pub applications: usize,
}

pub struct MachineCoordinator<S> {
    identity: CoordinatorIdentity,
    principal: u32,
    previous_shutdown_clean: bool,
    store: ControlStore<S>,
    runtime: RuntimeSystem,
    worker: Option<PathBuf>,
    applications: BTreeMap<u64, ManagedApplication>,
    next_application: u64,
    database: Option<Arc<dyn lkjscript_host::DatabaseTenantFactory>>,
}

impl<S: DurableStorage> MachineCoordinator<S> {
    pub fn start(
        identity: CoordinatorIdentity,
        principal: u32,
        storage: S,
        max_cache_entries: NonZeroUsize,
        worker: Option<PathBuf>,
    ) -> Result<Self, CoordinatorError> {
        let mut store = ControlStore::open(storage)?;
        let bootstrap = bootstrap(identity)?;
        match store.get(BOOTSTRAP_KEY) {
            None => {
                store.put(BOOTSTRAP_KEY.to_string(), bootstrap)?;
            }
            Some(found) if found == bootstrap => {}
            Some(_) => return Err(CoordinatorError::IdentityMismatch),
        }
        let previous_shutdown_clean = match store.get(CLEAN_KEY) {
            None => true,
            Some(b"true") => true,
            Some(b"false") => false,
            Some(_) => return Err(CoordinatorError::InvalidBootstrap),
        };
        store.put(CLEAN_KEY.to_string(), b"false".to_vec())?;
        let mut coordinator = Self {
            identity,
            principal,
            previous_shutdown_clean,
            store,
            runtime: RuntimeSystem::new(identity, max_cache_entries),
            worker,
            applications: BTreeMap::new(),
            next_application: 1,
            database: None,
        };
        coordinator.recover_applications()?;
        Ok(coordinator)
    }

    pub fn runtime(&self) -> &RuntimeSystem {
        &self.runtime
    }

    pub fn status(&self) -> Result<CoordinatorStatus, CoordinatorError> {
        Ok(CoordinatorStatus {
            identity: self.identity,
            principal: self.principal,
            previous_shutdown_clean: self.previous_shutdown_clean,
            control_sequence: self.store.sequence(),
            applications: self.applications.len(),
        })
    }

    pub fn handle_control(
        &mut self,
        request: &ControlRequest,
    ) -> Result<ControlSuccess, ControlFailure> {
        match &request.operation {
            ControlOperation::Describe => {
                let identity = ControlIdentity::current().map_err(|_| ControlFailure::Internal)?;
                Ok(ControlSuccess::Description {
                    platform_revision: identity.platform_revision,
                    contract_digest: identity.contract_digest,
                    product: "lkjscript runtime".to_string(),
                })
            }
            ControlOperation::Status => {
                let status = self.status().map_err(|_| ControlFailure::Internal)?;
                Ok(ControlSuccess::Status {
                    coordinator: status.identity.get(),
                    clean_shutdown: status.previous_shutdown_clean,
                    control_sequence: status.control_sequence,
                    applications: u32::try_from(status.applications)
                        .map_err(|_| ControlFailure::Internal)?,
                })
            }
            ControlOperation::Shutdown => Ok(ControlSuccess::ShutdownAccepted),
            operation => self.application_control(operation),
        }
    }

    pub fn shutdown(&mut self) -> Result<(), CoordinatorError> {
        self.abort_all_databases()?;
        for application in self.applications.values_mut() {
            if let Some(incarnation) = application.incarnation.take() {
                self.runtime.stop(incarnation)?;
            }
        }
        self.store.put(CLEAN_KEY.to_string(), b"true".to_vec())?;
        self.store.checkpoint()?;
        Ok(())
    }
}

fn bootstrap(identity: CoordinatorIdentity) -> Result<Vec<u8>, CoordinatorError> {
    let control = ControlIdentity::current().map_err(|error| match error {
        crate::ControlError::Host(host) => CoordinatorError::Host(host),
        _ => CoordinatorError::InvalidBootstrap,
    })?;
    let mut bytes = Vec::with_capacity(48);
    bytes.extend_from_slice(&identity.get().to_le_bytes());
    bytes.extend_from_slice(&PLATFORM_REVISION.to_le_bytes());
    bytes.extend_from_slice(&control.contract_digest.as_bytes());
    Ok(bytes)
}
