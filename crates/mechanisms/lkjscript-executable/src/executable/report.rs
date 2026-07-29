use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct InvocationReport {
    pub(super) outcome: InvocationOutcome,
    pub(super) trap_site: Option<u32>,
    pub(super) poll_count: u64,
    pub(super) native_entries: Vec<NativeEntryCount>,
    pub(super) peak_active_frame_depth: usize,
    pub(super) active_frame_depth: usize,
    pub(super) collection_calls: u64,
    pub(super) maximum_roots: usize,
    pub(super) exact_root_counts: Vec<usize>,
    pub(super) peak_native_stack_bytes: usize,
    pub(super) reserved_native_stack_bytes: usize,
    pub(super) heap_operation_attempts: u64,
    pub(super) heap_operation_successes: u64,
    pub(super) barrier_count: u64,
    pub(super) peak_active_value_homes: usize,
    pub(super) active_value_homes: usize,
    pub(super) resource_calls: u64,
    pub(super) unique_calls: u64,
    pub(super) cleanup_failures: Vec<NativeCleanupFailure>,
    pub(super) omitted_cleanup_failures: usize,
    pub(super) collector_runtime: bool,
}

impl InvocationReport {
    #[must_use]
    pub const fn outcome(&self) -> InvocationOutcome {
        self.outcome
    }

    #[must_use]
    pub const fn trap_site(&self) -> Option<u32> {
        self.trap_site
    }

    #[must_use]
    pub const fn poll_count(&self) -> u64 {
        self.poll_count
    }

    #[must_use]
    pub fn native_entries(&self) -> &[NativeEntryCount] {
        &self.native_entries
    }

    #[must_use]
    pub const fn peak_active_frame_depth(&self) -> usize {
        self.peak_active_frame_depth
    }

    #[must_use]
    pub const fn active_frame_depth(&self) -> usize {
        self.active_frame_depth
    }

    #[must_use]
    pub const fn collection_calls(&self) -> u64 {
        self.collection_calls
    }

    #[must_use]
    pub const fn maximum_roots(&self) -> usize {
        self.maximum_roots
    }

    #[must_use]
    pub fn exact_root_counts(&self) -> &[usize] {
        &self.exact_root_counts
    }

    #[must_use]
    pub const fn peak_native_stack_bytes(&self) -> usize {
        self.peak_native_stack_bytes
    }

    #[must_use]
    pub const fn reserved_native_stack_bytes(&self) -> usize {
        self.reserved_native_stack_bytes
    }

    #[must_use]
    pub const fn heap_operation_attempts(&self) -> u64 {
        self.heap_operation_attempts
    }

    #[must_use]
    pub const fn heap_operation_successes(&self) -> u64 {
        self.heap_operation_successes
    }

    #[must_use]
    pub const fn barrier_count(&self) -> u64 {
        self.barrier_count
    }

    #[must_use]
    pub const fn peak_active_value_homes(&self) -> usize {
        self.peak_active_value_homes
    }

    #[must_use]
    pub const fn active_value_homes(&self) -> usize {
        self.active_value_homes
    }

    #[must_use]
    pub const fn resource_calls(&self) -> u64 {
        self.resource_calls
    }

    #[must_use]
    pub const fn unique_calls(&self) -> u64 {
        self.unique_calls
    }

    #[must_use]
    pub fn cleanup_failures(&self) -> &[NativeCleanupFailure] {
        &self.cleanup_failures
    }

    #[must_use]
    pub const fn omitted_cleanup_failures(&self) -> usize {
        self.omitted_cleanup_failures
    }

    #[must_use]
    pub const fn collector_runtime(&self) -> bool {
        self.collector_runtime
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MappingPermissions {
    pub(super) readable: bool,
    pub(super) writable: bool,
    pub(super) executable: bool,
}
