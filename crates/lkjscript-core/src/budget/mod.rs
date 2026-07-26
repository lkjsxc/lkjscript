mod authority;
mod category;
mod category_names;
mod category_units;
mod diagnostic;
mod journal;
mod ledger;
mod legacy;
mod path;
mod reservation;
mod scope;

pub use authority::BudgetAuthority;
pub use category::ResourceCategory;
pub(crate) use category::RESOURCE_CATEGORY_COUNT;
pub use diagnostic::{BudgetCause, BudgetError, BudgetErrorKind};
pub(crate) use journal::BudgetJournal;
pub use journal::{
    BudgetPrefix, BudgetRejectedEvent, ReservationJournalRecord, MAX_BUDGET_JOURNAL_ENTRIES,
};
pub use ledger::{BudgetLedger, ResourceUsage};
pub use legacy::ResourceDiagnostic;
pub use path::{BudgetPath, MAX_BUDGET_PATH_DEPTH};
pub use reservation::{Reservation, ReservationId, ReservationState};
pub use scope::BudgetScope;

#[cfg(test)]
use crate::ResourceProfile;
#[cfg(test)]
mod hierarchy_tests;
#[cfg(test)]
mod journal_tests;
#[cfg(test)]
mod segment_tests;
#[cfg(test)]
mod tests;
