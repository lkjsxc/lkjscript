//! Representation-neutral execution controls and normalized Graph 8 execution.

mod control;
pub(crate) mod normalized;

pub use control::{ExecutionControl, ExecutionError, ExecutionFailureClass, RunPolicy};
