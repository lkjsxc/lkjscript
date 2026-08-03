mod access;
mod config;
mod error;
mod lifecycle;
mod loans;
mod metrics;
mod model;
mod publish;
mod sealed;
mod validation;

pub use config::StructuralRootTableLimits;
pub use error::{StructuralRootTableError, StructuralRootTableLimit};
pub use metrics::StructuralRootTableStats;
pub use model::{
    StructuralBorrow, StructuralBorrowKey, StructuralRootOwnership, StructuralRootState,
    StructuralRootTable, StructuralValueKey,
};

use model::{LiveLoan, LiveRoot, LoanSlot, RootSlot, TerminalState};
