//! In-process runtime-node foundation for capability-free validated VM applications.

mod cache;
mod error;
mod execution;
mod ids;
mod invoke;
mod model;
mod node;
mod state;

pub use error::{QuotaKind, RuntimeError};
pub use ids::{
    ApplicationGenerationId, ApplicationId, ApplicationInstanceId, ExecutionCellId, NodeIdentity,
    PackageContentId,
};
pub use model::{
    ApplicationKind, ApplicationManifest, ApplicationStatus, DeploymentScope, InvocationMetrics,
    InvocationOutcome, InvocationRequest, Lifecycle, ProcessCellState, ResourceAccounting,
    ResourceQuota, RestartPolicy, MAX_LOG_ENTRIES, MAX_RESTART_ATTEMPTS,
};
pub use node::Node;

#[cfg(test)]
mod tests;
