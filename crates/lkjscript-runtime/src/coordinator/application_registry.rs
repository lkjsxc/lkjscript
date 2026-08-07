use std::num::{NonZeroU64, NonZeroUsize};
use std::path::PathBuf;
use std::sync::Arc;

use lkjscript_core::{CapabilityKind, ExecutionPolicy, LimitedExecutionPolicy};
use lkjscript_host::{ApplicationPath, BufferedStdio, DurableStorage, HostEnvironment};

use crate::{
    ApplicationId, ApplicationKind, ApplicationManifest, ApplicationStatus, DeploymentScope,
    ExecutionCellClass, Lifecycle, PackageContentId, ResourceQuota, RestartPolicy,
};

use super::{CoordinatorError, MachineCoordinator};

const RECORD_PREFIX: &str = "application/record/";
const NEXT_KEY: &str = "application/next";
pub(super) const MAX_REGISTERED_APPLICATIONS: usize = 1_024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DurableApplication {
    pub id: u64,
    pub name: String,
    pub package: [u8; 32],
    pub package_root: PathBuf,
    pub entry: ApplicationPath,
    pub capabilities: Vec<CapabilityKind>,
    pub max_concurrent: u16,
    pub max_total: u64,
    pub desired_running: bool,
}

pub(super) struct ManagedApplication {
    pub durable: DurableApplication,
    pub runtime: ApplicationId,
    pub incarnation: Option<crate::ApplicationIncarnationId>,
    pub stdio: BufferedStdio,
    pub database: Option<Arc<dyn lkjscript_host::DatabaseProvider>>,
}

impl<S: DurableStorage> MachineCoordinator<S> {
    pub(super) fn recover_applications(&mut self) -> Result<(), CoordinatorError> {
        let records = self
            .store
            .facts()
            .filter(|(key, _)| key.starts_with(RECORD_PREFIX))
            .map(|(key, value)| (key.to_owned(), value.to_vec()))
            .collect::<Vec<_>>();
        if records.len() > MAX_REGISTERED_APPLICATIONS {
            return Err(CoordinatorError::InvalidApplicationRegistry);
        }
        let mut names = std::collections::BTreeSet::new();
        for (key, bytes) in records {
            let durable = decode_record(&bytes)?;
            if key != record_key(durable.id) || !names.insert(durable.name.clone()) {
                return Err(CoordinatorError::InvalidApplicationRegistry);
            }
            let mut managed = self.install_record(durable)?;
            if managed.durable.desired_running {
                managed.incarnation = Some(self.runtime.start(managed.runtime)?);
            }
            self.applications.insert(managed.durable.id, managed);
        }
        let stored_next = match self.store.get(NEXT_KEY) {
            Some(bytes) if bytes.len() == 8 => u64::from_le_bytes(
                bytes
                    .try_into()
                    .map_err(|_| CoordinatorError::InvalidApplicationRegistry)?,
            ),
            None => 1,
            Some(_) => return Err(CoordinatorError::InvalidApplicationRegistry),
        };
        let minimum = self
            .applications
            .last_key_value()
            .and_then(|(id, _)| id.checked_add(1))
            .unwrap_or(1);
        if stored_next == 0 || stored_next < minimum {
            return Err(CoordinatorError::InvalidApplicationRegistry);
        }
        self.next_application = stored_next;
        Ok(())
    }

    pub(super) fn install_record(
        &self,
        durable: DurableApplication,
    ) -> Result<ManagedApplication, CoordinatorError> {
        let worker = self
            .worker
            .as_ref()
            .ok_or(CoordinatorError::WorkerUnavailable)?;
        let stdio = BufferedStdio::default();
        let runtime = self.runtime.install_isolated(
            manifest(&durable)?,
            PackageContentId::new(durable.package)
                .ok_or(CoordinatorError::InvalidApplicationRegistry)?,
            &durable.package_root,
            worker,
            HostEnvironment {
                stdio: Some(Arc::new(stdio.clone())),
                clock: Some(Arc::new(lkjscript_host::PortableClock::new())),
                logger: Some(Arc::new(lkjscript_host::PortableLogger)),
                cancellation: Some(Arc::new(lkjscript_host::CancellationToken::new())),
                directory: None,
                database: None,
            },
        )?;
        Ok(ManagedApplication {
            durable,
            runtime,
            incarnation: None,
            stdio,
            database: None,
        })
    }
}

fn manifest(record: &DurableApplication) -> Result<ApplicationManifest, CoordinatorError> {
    let max_concurrent_invocations = NonZeroUsize::new(usize::from(record.max_concurrent))
        .ok_or(CoordinatorError::InvalidApplicationRegistry)?;
    let max_total_invocations =
        NonZeroU64::new(record.max_total).ok_or(CoordinatorError::InvalidApplicationRegistry)?;
    let mut execution = LimitedExecutionPolicy::conservative();
    execution.max_heap_bytes = 64 * 1024;
    execution.max_output_bytes = 16 * 1024;
    Ok(ApplicationManifest {
        name: record.name.clone(),
        kind: ApplicationKind::Service,
        scope: DeploymentScope::Standalone,
        cell: ExecutionCellClass::IsolatedProcess {
            entry: record.entry.clone(),
        },
        capabilities: record.capabilities.clone(),
        quota: ResourceQuota {
            max_concurrent_invocations,
            max_total_invocations,
            execution: ExecutionPolicy::limited(execution),
        },
        restart: RestartPolicy::Never,
    })
}

pub(super) fn state(status: &ApplicationStatus) -> crate::ControlledApplicationState {
    match status.lifecycle {
        Lifecycle::Installed | Lifecycle::Loading | Lifecycle::Starting => {
            crate::ControlledApplicationState::Installed
        }
        Lifecycle::Running | Lifecycle::Quiescing | Lifecycle::Stopping => {
            crate::ControlledApplicationState::Running
        }
        Lifecycle::Stopped | Lifecycle::Uninstalled => crate::ControlledApplicationState::Stopped,
        Lifecycle::Failed => crate::ControlledApplicationState::Failed,
    }
}

fn record_key(id: u64) -> String {
    format!("{RECORD_PREFIX}{id:020}")
}

include!("application_registry/codec.rs");
include!("application_registry/operations.rs");
include!("application_registry/invoke.rs");
