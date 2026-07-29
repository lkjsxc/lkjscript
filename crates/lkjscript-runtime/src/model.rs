use std::num::{NonZeroU32, NonZeroU64, NonZeroUsize};

use lkjscript_core::{CapabilityKind, ExecutionConfig, ExecutionOutcome};
use lkjscript_vm::ExecutionInputs;

use crate::{
    ApplicationId, ApplicationIncarnationId, ExecutionCellId, PackageContentId, RuntimeError,
};

pub const MAX_RESTART_ATTEMPTS: u32 = 1_024;
pub const MAX_LOG_ENTRIES: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeploymentScope {
    System { principal: u32 },
    Container { principal: u32, container: String },
    Standalone,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationKind {
    Command,
    Service,
    Interactive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestartPolicy {
    Never,
    OnFailure { max_attempts: NonZeroU32 },
    Always { max_attempts: NonZeroU32 },
}

impl RestartPolicy {
    pub const fn max_attempts(self) -> u32 {
        match self {
            Self::Never => 0,
            Self::OnFailure { max_attempts } | Self::Always { max_attempts } => max_attempts.get(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceQuota {
    pub max_concurrent_invocations: NonZeroUsize,
    pub max_total_invocations: NonZeroU64,
    pub execution: ExecutionConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationManifest {
    pub name: String,
    pub kind: ApplicationKind,
    pub scope: DeploymentScope,
    pub capabilities: Vec<CapabilityKind>,
    pub quota: ResourceQuota,
    pub restart: RestartPolicy,
}

impl ApplicationManifest {
    pub(crate) fn validate(&self) -> Result<(), RuntimeError> {
        if self.name.is_empty() || self.name.len() > 64 {
            return Err(RuntimeError::InvalidManifest(
                "name must contain 1..=64 bytes",
            ));
        }
        if matches!(
            &self.scope,
            DeploymentScope::Container { container, .. }
                if container.is_empty() || container.len() > 128
        ) {
            return Err(RuntimeError::InvalidManifest(
                "container identity must contain 1..=128 bytes",
            ));
        }
        if !self.capabilities.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(RuntimeError::InvalidManifest(
                "capabilities must be sorted and unique",
            ));
        }
        if self.restart.max_attempts() > MAX_RESTART_ATTEMPTS {
            return Err(RuntimeError::InvalidManifest(
                "restart attempts exceed bound",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lifecycle {
    Installed,
    Loading,
    Starting,
    Running,
    Quiescing,
    Stopping,
    Stopped,
    Failed,
    Uninstalled,
}

impl Lifecycle {
    pub fn transition(self, next: Self) -> Result<Self, RuntimeError> {
        let legal = matches!(
            (self, next),
            (Self::Installed, Self::Loading | Self::Uninstalled)
                | (Self::Loading, Self::Starting | Self::Failed)
                | (Self::Starting, Self::Running | Self::Failed)
                | (Self::Running, Self::Quiescing | Self::Failed)
                | (Self::Quiescing, Self::Stopping | Self::Failed)
                | (Self::Stopping, Self::Stopped | Self::Failed)
                | (Self::Stopped, Self::Loading | Self::Uninstalled)
                | (
                    Self::Failed,
                    Self::Loading | Self::Stopping | Self::Uninstalled
                )
        );
        if legal {
            Ok(next)
        } else {
            Err(RuntimeError::IllegalTransition {
                from: self,
                to: next,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessCellState {
    DeferredUnavailable,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InvocationMetrics {
    pub admitted: u64,
    pub completed: u64,
    pub trapped: u64,
    pub peak_concurrent: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceAccounting {
    pub active_invocations: usize,
    pub total_invocations: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationStatus {
    pub application: ApplicationId,
    pub package: PackageContentId,
    pub lifecycle: Lifecycle,
    pub incarnation: Option<ApplicationIncarnationId>,
    pub cancelled: bool,
    pub metrics: InvocationMetrics,
    pub resources: ResourceAccounting,
    pub logs: Vec<String>,
    pub process_cell: ProcessCellState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvocationRequest {
    pub incarnation: ApplicationIncarnationId,
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvocationOutcome {
    pub execution_cell: ExecutionCellId,
    pub incarnation: ApplicationIncarnationId,
    pub outcome: ExecutionOutcome,
}

pub(crate) fn private_inputs(
    arguments: Vec<String>,
    capabilities: Vec<CapabilityKind>,
    host: lkjscript_host::HostEnvironment,
) -> ExecutionInputs {
    ExecutionInputs {
        arguments,
        capabilities,
        host,
    }
}
