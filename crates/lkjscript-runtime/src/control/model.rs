use lkjscript_contracts::ContractDigest;
use lkjscript_core::{CapabilityKind, ExecutionOutcome};

use super::{ControlError, ControlIdentity};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationInstallRequest {
    pub name: String,
    pub package: [u8; 32],
    pub package_root: String,
    pub entry: String,
    pub capabilities: Vec<CapabilityKind>,
    pub max_concurrent_invocations: u16,
    pub max_total_invocations: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlOperation {
    Describe,
    Status,
    Shutdown,
    ApplicationInstall(ApplicationInstallRequest),
    ApplicationList,
    ApplicationStart {
        application: u64,
    },
    ApplicationStop {
        application: u64,
    },
    ApplicationRestart {
        application: u64,
    },
    ApplicationRemove {
        application: u64,
    },
    ApplicationInvoke {
        application: u64,
        arguments: Vec<String>,
    },
}

impl ControlOperation {
    pub const fn modifies(&self) -> bool {
        !matches!(Self::kind(self), 1 | 2 | 11)
    }

    pub const fn kind(&self) -> u8 {
        match self {
            Self::Describe => 1,
            Self::Status => 2,
            Self::Shutdown => 3,
            Self::ApplicationInstall(_) => 10,
            Self::ApplicationList => 11,
            Self::ApplicationStart { .. } => 12,
            Self::ApplicationStop { .. } => 13,
            Self::ApplicationRestart { .. } => 14,
            Self::ApplicationRemove { .. } => 15,
            Self::ApplicationInvoke { .. } => 16,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlRequest {
    pub identity: ControlIdentity,
    pub request_id: u64,
    pub idempotency_id: [u8; 32],
    pub operation: ControlOperation,
}

impl ControlRequest {
    pub fn current(
        request_id: u64,
        idempotency_id: [u8; 32],
        operation: ControlOperation,
    ) -> Result<Self, ControlError> {
        if request_id == 0 || (operation.modifies() && idempotency_id == [0; 32]) {
            return Err(ControlError::InvalidIdentity);
        }
        Ok(Self {
            identity: ControlIdentity::current()?,
            request_id,
            idempotency_id,
            operation,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlledApplicationState {
    Installed,
    Running,
    Stopped,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlledApplication {
    pub application: u64,
    pub name: String,
    pub desired_running: bool,
    pub state: ControlledApplicationState,
    pub incarnation: Option<u64>,
    pub process: Option<u32>,
    pub database_attached: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ControlSuccess {
    Description {
        platform_revision: u64,
        contract_digest: ContractDigest,
        product: String,
    },
    Status {
        coordinator: u64,
        clean_shutdown: bool,
        control_sequence: u64,
        applications: u32,
    },
    ShutdownAccepted,
    Application(ControlledApplication),
    Applications(Vec<ControlledApplication>),
    ApplicationRemoved {
        application: u64,
    },
    ApplicationInvoked {
        application: u64,
        outcome: ExecutionOutcome,
        output: Vec<u8>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlFailure {
    Unauthorized,
    StaleRevision { expected: u64, found: u64 },
    ContractMismatch,
    ReplayConflict,
    Malformed,
    NotFound,
    Rejected(String),
    Internal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlResponse {
    pub request_id: u64,
    pub result: Result<ControlSuccess, ControlFailure>,
}
