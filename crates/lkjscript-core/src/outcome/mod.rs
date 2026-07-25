//! Process-safe execution outcomes and owned returned values.

mod errors;
mod execution;
mod owned_value;

pub use errors::{HostError, ResourceLimitKind, Trap};
pub use execution::ExecutionOutcome;
pub use owned_value::OwnedValue;
