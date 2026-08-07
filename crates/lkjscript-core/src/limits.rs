//! Explicit execution resource policy.

use std::time::Duration;

use crate::{CleanupFailureLimits, CleanupRetentionPolicy};

/// Host policy for one execution.
///
/// Trusted local callers deliberately select [`Self::Unrestricted`]. Callers
/// handling untrusted work must construct [`Self::Limited`] explicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionPolicy {
    Unrestricted,
    Limited(LimitedExecutionPolicy),
}

impl ExecutionPolicy {
    pub const fn unrestricted() -> Self {
        Self::Unrestricted
    }

    pub const fn limited(policy: LimitedExecutionPolicy) -> Self {
        Self::Limited(policy)
    }

    pub const fn limited_policy(&self) -> Option<&LimitedExecutionPolicy> {
        match self {
            Self::Unrestricted => None,
            Self::Limited(policy) => Some(policy),
        }
    }

    pub fn limited_policy_mut(&mut self) -> Option<&mut LimitedExecutionPolicy> {
        match self {
            Self::Unrestricted => None,
            Self::Limited(policy) => Some(policy),
        }
    }

    pub const fn instruction_fuel(&self) -> Option<u64> {
        match self.limited_policy() {
            Some(policy) => Some(policy.instruction_fuel),
            None => None,
        }
    }

    pub const fn max_stack_values(&self) -> Option<usize> {
        match self.limited_policy() {
            Some(policy) => Some(policy.max_stack_values),
            None => None,
        }
    }

    pub const fn max_frames(&self) -> Option<usize> {
        match self.limited_policy() {
            Some(policy) => Some(policy.max_frames),
            None => None,
        }
    }

    pub const fn max_heap_bytes(&self) -> Option<usize> {
        match self.limited_policy() {
            Some(policy) => Some(policy.max_heap_bytes),
            None => None,
        }
    }

    pub const fn max_allocations(&self) -> Option<u64> {
        match self.limited_policy() {
            Some(policy) => Some(policy.max_allocations),
            None => None,
        }
    }

    pub const fn max_handles(&self) -> Option<usize> {
        match self.limited_policy() {
            Some(policy) => Some(policy.max_handles),
            None => None,
        }
    }

    pub const fn max_output_bytes(&self) -> Option<usize> {
        match self.limited_policy() {
            Some(policy) => Some(policy.max_output_bytes),
            None => None,
        }
    }

    pub const fn cleanup_retention(&self) -> CleanupRetentionPolicy {
        match self.limited_policy() {
            Some(policy) => CleanupRetentionPolicy::Limited(policy.cleanup_retention),
            None => CleanupRetentionPolicy::Unrestricted,
        }
    }

    pub const fn wall_time(&self) -> Option<Duration> {
        match self.limited_policy() {
            Some(policy) => policy.wall_time,
            None => None,
        }
    }

    pub const fn require_hard_deadline(&self) -> bool {
        match self.limited_policy() {
            Some(policy) => policy.require_hard_deadline,
            None => false,
        }
    }
}

/// Coarse host resources available to an untrusted execution.
///
/// This type intentionally has no `Default`: every trust boundary must state
/// that it is choosing a limited policy. [`Self::conservative`] preserves the
/// operational values used by process workers before the policy cutover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitedExecutionPolicy {
    pub instruction_fuel: u64,
    pub max_stack_values: usize,
    pub max_frames: usize,
    pub max_heap_bytes: usize,
    pub max_allocations: u64,
    pub max_handles: usize,
    pub max_output_bytes: usize,
    pub cleanup_retention: CleanupFailureLimits,
    /// A cooperative monotonic wall limit. Read/poll/wait operations are
    /// shortened to the remaining duration. Other host calls are checked
    /// immediately before and after because their current Linux wrappers do
    /// not all expose cancellable variants.
    pub wall_time: Option<Duration>,
    /// Reject a host operation with `HostFailure` before effects when the
    /// current host wrapper cannot provide a hard cancellable deadline.
    pub require_hard_deadline: bool,
}

impl LimitedExecutionPolicy {
    pub fn conservative() -> Self {
        Self {
            instruction_fuel: 1_000_000_000,
            max_stack_values: 1_000_000,
            max_frames: 65_536,
            max_heap_bytes: 256 * 1024 * 1024,
            max_allocations: 10_000_000,
            max_handles: 4_096,
            max_output_bytes: 64 * 1024 * 1024,
            cleanup_retention: CleanupFailureLimits::default(),
            wall_time: Some(Duration::from_secs(30 * 60)),
            require_hard_deadline: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrestricted_has_no_finite_execution_resources() {
        let policy = ExecutionPolicy::unrestricted();
        assert_eq!(policy.instruction_fuel(), None);
        assert_eq!(policy.max_stack_values(), None);
        assert_eq!(policy.max_frames(), None);
        assert_eq!(policy.max_heap_bytes(), None);
        assert_eq!(policy.max_allocations(), None);
        assert_eq!(policy.max_handles(), None);
        assert_eq!(policy.max_output_bytes(), None);
        assert_eq!(
            policy.cleanup_retention(),
            CleanupRetentionPolicy::Unrestricted
        );
        assert_eq!(policy.wall_time(), None);
        assert!(!policy.require_hard_deadline());
    }
}
