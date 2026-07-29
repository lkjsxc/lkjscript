//! Process-safe execution outcomes and owned returned values.

mod cleanup;
mod cleanup_limits;
mod codec;
mod errors;
mod execution;
mod owned_value;

pub use cleanup::{CleanupFailure, CleanupFailures, CleanupPhase, CleanupSubject};
pub use cleanup_limits::{
    CleanupFailureLimits, DEFAULT_MAX_CLEANUP_FAILURES, DEFAULT_MAX_CLEANUP_FAILURE_BYTES,
    MAX_CLEANUP_FAILURES, MAX_CLEANUP_FAILURE_BYTES,
};
pub use codec::{decode_execution_outcome, encode_execution_outcome};
pub use errors::{HostError, ResourceLimitKind, Trap};
pub use execution::ExecutionOutcome;
pub use owned_value::OwnedValue;

#[cfg(test)]
mod tests;
