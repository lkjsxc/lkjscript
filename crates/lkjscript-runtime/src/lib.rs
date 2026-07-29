//! Daemon-owned runtime-system foundation for supervised validated applications.

mod cache;
mod error;
mod execution;
mod ids;
mod invoke;
mod model;
mod state;
mod system;

pub use error::{QuotaKind, RuntimeError};
pub use ids::{
    ApplicationId, ApplicationIncarnationId, CoordinatorIdentity, ExecutionCellId, PackageContentId,
};
pub use model::{
    ApplicationKind, ApplicationManifest, ApplicationStatus, DeploymentScope, InvocationMetrics,
    InvocationOutcome, InvocationRequest, Lifecycle, ProcessCellState, ResourceAccounting,
    ResourceQuota, RestartPolicy, MAX_LOG_ENTRIES, MAX_RESTART_ATTEMPTS,
};
pub use system::RuntimeSystem;

#[cfg(test)]
mod tests;
