//! Representation-neutral execution controls and normalized Graph 10 execution.

mod control;
pub(crate) mod mailbox;
pub(crate) mod normalized;
pub mod region;

pub use control::{ExecutionControl, ExecutionError, ExecutionFailureClass, RunPolicy};
pub(crate) use control::{cumulative_charge, cumulative_observe, finite_charge};
