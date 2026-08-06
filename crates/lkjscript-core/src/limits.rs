//! Language, bytecode-validation, and execution budgets.

use std::time::Duration;

/// Maximum fields in one nominal product declaration.
pub const MAX_PRODUCT_FIELDS: usize = 15;
/// Maximum entry comparisons performed by one structural list equality.
pub const MAX_LIST_EQUAL_STEPS: usize = 1_000_000;
/// Maximum bytes owned by one language buffer.
pub const MAX_BYTE_STORAGE_BYTES: usize = 1_000_000;
/// Maximum bytes transferred by one bulk file or socket operation.
pub const MAX_BULK_IO_BYTES: usize = 64 * 1024;

pub const MAX_CHUNK_ENCODED_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_FUNCTION_CODE_BYTES: usize = 65_535;
pub const MAX_BYTECODE_TABLE_ENTRIES: usize = 65_535;
pub const MAX_BYTECODE_METADATA_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_CONSTANT_DATA_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationLimits {
    pub max_encoded_bytes: usize,
    pub max_function_code_bytes: usize,
    pub max_table_entries: usize,
    pub max_metadata_bytes: usize,
    pub max_constant_data_bytes: usize,
}

impl Default for ValidationLimits {
    fn default() -> Self {
        Self {
            max_encoded_bytes: MAX_CHUNK_ENCODED_BYTES,
            max_function_code_bytes: MAX_FUNCTION_CODE_BYTES,
            max_table_entries: MAX_BYTECODE_TABLE_ENTRIES,
            max_metadata_bytes: MAX_BYTECODE_METADATA_BYTES,
            max_constant_data_bytes: MAX_CONSTANT_DATA_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Limits {
    pub validation: ValidationLimits,
}

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
    fn defaults_match_spec_consts() {
        let lim = Limits::default();
        assert_eq!(lim.validation, ValidationLimits::default());
        assert_eq!(MAX_PRODUCT_FIELDS, 15);
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
