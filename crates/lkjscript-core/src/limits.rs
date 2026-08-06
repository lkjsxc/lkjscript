//! Language representation constants and execution policy.

use std::time::Duration;

/// Maximum fields in one nominal product declaration.
/// Maximum entry comparisons performed by one structural list equality.
pub const MAX_LIST_EQUAL_STEPS: usize = 1_000_000;
/// Maximum bytes owned by one language buffer.
pub const MAX_BYTE_STORAGE_BYTES: usize = 1_000_000;
/// Maximum bytes transferred by one bulk file or socket operation.
pub const MAX_BULK_IO_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionConfig {
    pub instruction_fuel: u64,
    pub max_stack_values: usize,
    pub max_frames: usize,
    pub max_heap_bytes: usize,
    pub max_allocations: u64,
    pub max_logical_aggregate_constructions: u64,
    pub max_handles: usize,
    pub max_output_bytes: usize,
    pub cleanup_failure_limits: crate::CleanupFailureLimits,
    /// A cooperative monotonic wall limit. Read/poll/wait operations are
    /// shortened to the remaining duration. Other host calls are checked
    /// immediately before and after because their current Linux wrappers do
    /// not all expose cancellable variants.
    pub wall_time: Option<Duration>,
    /// Reject a host operation with `HostFailure` before effects when the
    /// current host wrapper cannot provide a hard cancellable deadline.
    pub require_hard_deadline: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            instruction_fuel: 1_000_000_000,
            max_stack_values: 1_000_000,
            max_frames: 65_536,
            max_heap_bytes: 256 * 1024 * 1024,
            max_allocations: 10_000_000,
            max_logical_aggregate_constructions: 1_000_000,
            max_handles: 4_096,
            max_output_bytes: 64 * 1024 * 1024,
            cleanup_failure_limits: crate::CleanupFailureLimits::default(),
            wall_time: Some(Duration::from_secs(30 * 60)),
            require_hard_deadline: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_constants_match_implemented_representations() {
        assert_eq!(MAX_LIST_EQUAL_STEPS, 1_000_000);
        assert_eq!(MAX_BYTE_STORAGE_BYTES, 1_000_000);
        assert_eq!(MAX_BULK_IO_BYTES, 64 * 1024);
    }

    #[test]
    fn execution_defaults_are_bounded() {
        let limits = ExecutionConfig::default();
        assert!(limits.instruction_fuel > 0);
        assert!(limits.max_stack_values > 0);
        assert!(limits.max_frames > 0);
        assert!(limits.max_heap_bytes > 0);
        assert!(limits.max_allocations > 0);
        assert!(limits.max_logical_aggregate_constructions > 0);
        assert!(limits.max_handles > 0);
        assert!(limits.max_output_bytes > 0);
        assert!(limits.cleanup_failure_limits.max_failures() > 0);
        assert!(limits.cleanup_failure_limits.max_message_bytes() > 0);
        assert!(limits.wall_time.is_some());
    }
}
