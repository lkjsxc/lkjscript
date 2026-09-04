//! Representation-neutral execution controls and normalized Graph 10 execution.

mod control;
pub(crate) mod normalized;

pub use control::{ExecutionControl, ExecutionError, ExecutionFailureClass, RunPolicy};
