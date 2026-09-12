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

/// Precharge a cumulative quota, or observe work without imposing a lifetime limit.
/// Unbounded observations saturate: a value of u64::MAX is a lower bound, never
/// an exact count. Storage-size arithmetic must be checked separately before growth.
pub(crate) fn cumulative_charge(
    current: u64,
    additional: u64,
    maximum: Option<u64>,
    code: &'static str,
    message: &'static str,
) -> Result<u64, ExecutionError> {
    match maximum {
        Some(maximum) => current
            .checked_add(additional)
            .filter(|next| *next <= maximum)
            .ok_or_else(|| ExecutionError::resource(code, message)),
        None => Ok(current.saturating_add(additional)),
    }
}

/// Runtime-owned cancellation and deadline state. It is never representable as a language value
/// and therefore cannot cross a durable boundary.
#[derive(Clone, Debug)]
pub struct ExecutionControl {
    cancelled: Arc<AtomicBool>,
    deadline: Option<Instant>,
    remaining_checks: Option<Arc<std::sync::atomic::AtomicU64>>,
}

impl ExecutionControl {
    pub fn uncancelled() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: None,
            remaining_checks: None,
        }
    }

    pub fn with_deadline(deadline: Instant) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: Some(deadline),
            remaining_checks: None,
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

    /// Contributor-only deterministic cancellation. Public controls never install this probe.
    pub(crate) fn cancel_after_checks(count: u64) -> Self {
        Self {
            remaining_checks: Some(Arc::new(std::sync::atomic::AtomicU64::new(count))),
            ..Self::uncancelled()
        }
    }

    pub fn check(&self) -> Result<(), ExecutionError> {
        if let Some(remaining) = &self.remaining_checks
            && remaining
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                    count.checked_sub(1)
                })
                .is_err()
        {
            self.cancel();
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cumulative_quota_overflow_and_unbounded_telemetry_are_distinct() {
        for maximum in [None, Some(u64::MAX)] {
            assert_eq!(
                cumulative_charge(u64::MAX - 1, 1, maximum, "quota", "exhausted").ok(),
                Some(u64::MAX)
            );
        }
        assert_eq!(
            cumulative_charge(u64::MAX, 1, None, "quota", "exhausted").ok(),
            Some(u64::MAX)
        );
        assert!(cumulative_charge(u64::MAX, 1, Some(u64::MAX), "quota", "exhausted").is_err());
        assert_eq!(
            cumulative_charge(4, 1, Some(5), "quota", "exhausted").ok(),
            Some(5)
        );
        assert!(cumulative_charge(5, 1, Some(5), "quota", "exhausted").is_err());
    }
}
