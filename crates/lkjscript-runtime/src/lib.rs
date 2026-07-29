//! Daemon-owned runtime-system foundation for supervised validated applications.

mod cache;
mod control;
mod control_store;
mod coordinator;
mod error;
mod execution;
mod ids;
mod invoke;
mod model;
mod providers;
mod service;
mod state;
mod system;

pub use control::{
    decode_request_frame, decode_response_frame, encode_request_frame, encode_response_frame,
    ControlError, ControlFailure, ControlIdentity, ControlOperation, ControlRequest,
    ControlResponse, ControlSuccess, MAX_CONTROL_FRAME_BYTES,
};
#[cfg(target_os = "linux")]
pub use control::{UnixControlClient, UnixControlServer};
pub use control_store::{ControlStore, ControlStoreError, RecoveryReport};
pub use coordinator::{CoordinatorError, CoordinatorLease, CoordinatorStatus, MachineCoordinator};
pub use error::{QuotaKind, RuntimeError};
pub use execution::protocol as process_cell_protocol;
pub use ids::{
    ApplicationId, ApplicationIncarnationId, CoordinatorIdentity, ExecutionCellId, PackageContentId,
};
pub use model::{
    ApplicationKind, ApplicationManifest, ApplicationStatus, DeploymentScope, ExecutionCellClass,
    InvocationMetrics, InvocationOutcome, InvocationRequest, Lifecycle, ProcessCellState,
    ResourceAccounting, ResourceQuota, RestartPolicy, MAX_LOG_ENTRIES, MAX_RESTART_ATTEMPTS,
};
pub use service::{ServiceBundle, ServiceConfiguration, ServiceError};
pub use system::RuntimeSystem;

#[cfg(test)]
mod tests;
