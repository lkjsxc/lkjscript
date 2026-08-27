//! Representation-neutral execution controls and normalized Graph 5 execution.

mod control;
pub(crate) mod normalized;

pub use control::{ExecutionControl, ExecutionError, ExecutionFailureClass, RunPolicy};
