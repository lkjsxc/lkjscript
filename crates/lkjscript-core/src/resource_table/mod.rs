mod access;
mod cleanup;
mod close;
mod error;
mod identity;
mod limits;
mod reservation;
mod reserve;
mod slot;
mod state;
mod stats;
mod table;

pub use cleanup::{ResourceCleanupAttempt, ResourceCleanupReport};
pub use error::{ResourceTableError, ResourceTableLimit};
pub use identity::{ProviderId, ResourceKey, ScopeId};
pub use limits::{ResourceTableConfigError, ResourceTableLimits};
pub use reservation::{BorrowedReservation, OwnedReservation};
pub use state::{ResourceObservation, ResourceOwnership, ResourceState};
pub use stats::{EmergencyObligations, ResourceTableStats};
pub use table::ResourceTable;

#[cfg(test)]
mod tests;
