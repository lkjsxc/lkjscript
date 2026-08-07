//! In-process execution outcomes and owned returned values.

mod cleanup;
mod cleanup_limits;
mod errors;
mod execution;
mod owned_value;
mod semantic_dag;
mod structural;

pub use cleanup::{CleanupFailure, CleanupFailures, CleanupPhase, CleanupSubject};
pub use cleanup_limits::{
    CleanupFailureLimits, CleanupRetentionPolicy, DEFAULT_MAX_CLEANUP_FAILURES,
    DEFAULT_MAX_CLEANUP_FAILURE_BYTES, MAX_CLEANUP_FAILURES, MAX_CLEANUP_FAILURE_BYTES,
};
pub use errors::{HostError, ResourceLimitKind, Trap};
pub use execution::ExecutionOutcome;
#[cfg(test)]
pub(crate) use owned_value::OwnedListNode;
pub use owned_value::OwnedValue;
pub use semantic_dag::{
    SealedSemanticDagBorrow, SealedSemanticDagBorrowFailure, SealedSemanticDagError,
    SealedSemanticDagFailure, SealedSemanticDagMetrics, SealedSemanticDagOwner,
    SealedSemanticDagReleaseFailure, SealedSemanticDagReleaseReport, SealedSemanticDagRuntime,
    SemanticDagKind, SemanticDagNode, SemanticDagNodeId, SemanticDagPayload, SemanticDagSnapshot,
    SemanticDagType,
};
pub use structural::StructuralSnapshotMetrics;

#[cfg(test)]
mod tests;
