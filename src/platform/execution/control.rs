//! Representation-neutral execution failure, policy, cancellation, and deadline controls.

use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionFailureClass {
    Trap,
    Capability,
    PossibleVisibility,
    Resource,
    Cancelled,
    Infrastructure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionError {
    pub class: ExecutionFailureClass,
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub possibly_visible: bool,
}

impl ExecutionError {
    pub fn new(
        class: ExecutionFailureClass,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            class,
            code: code.into(),
            message: message.into(),
            retryable: false,
            possibly_visible: class == ExecutionFailureClass::PossibleVisibility,
        }
    }

    pub fn resource(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(ExecutionFailureClass::Resource, code, message)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunPolicy {
    pub instruction_fuel: u64,
    pub maximum_call_depth: usize,
    pub maximum_value_stack: usize,
}

impl Default for RunPolicy {
    fn default() -> Self {
        Self {
            instruction_fuel: 10_000_000,
            maximum_call_depth: 4_096,
            maximum_value_stack: 1_000_000,
        }
    }
}

/// Runtime-owned cancellation and deadline state. It is never representable as a language value
/// and therefore cannot cross a durable boundary.
#[derive(Clone, Debug)]
pub struct ExecutionControl {
    cancelled: Arc<AtomicBool>,
    deadline: Option<Instant>,
}

impl ExecutionControl {
    pub fn uncancelled() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: None,
        }
    }

    pub fn with_deadline(deadline: Instant) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: Some(deadline),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    pub fn check(&self) -> Result<(), ExecutionError> {
        if self.is_cancelled() {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Cancelled,
                "execution_cancelled",
                "execution was cancelled by its owning task scope",
            ));
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Cancelled,
                "execution_deadline",
                "execution exceeded its operational deadline",
            ));
        }
        Ok(())
    }
}

impl Default for ExecutionControl {
    fn default() -> Self {
        Self::uncancelled()
    }
}
