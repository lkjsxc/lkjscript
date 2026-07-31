mod structural;
mod traits;
use super::*;
pub use structural::*;
pub use traits::*;

mod noop;
pub(super) use noop::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeInvocationConfig {
    pub(super) poll_fuel: u64,
    pub(super) wall_time: Option<Duration>,
    pub(super) max_active_frames: usize,
    pub(super) max_active_values: usize,
    pub(super) max_native_stack_bytes: usize,
    pub(super) max_native_frame_bytes: usize,
    pub(super) max_cleanup_failures: usize,
}

impl NativeInvocationConfig {
    #[must_use]
    pub const fn new(poll_fuel: u64, wall_time: Option<Duration>) -> Self {
        Self {
            poll_fuel,
            wall_time,
            max_active_frames: MAX_ACTIVE_FRAMES,
            max_active_values: usize::MAX,
            max_native_stack_bytes: DEFAULT_MAX_NATIVE_STACK_BYTES,
            max_native_frame_bytes: DEFAULT_MAX_NATIVE_FRAME_BYTES,
            max_cleanup_failures: 32,
        }
    }

    #[must_use]
    pub const fn with_max_active_frames(mut self, maximum: usize) -> Self {
        self.max_active_frames = maximum;
        self
    }

    #[must_use]
    pub const fn with_max_active_values(mut self, maximum: usize) -> Self {
        self.max_active_values = maximum;
        self
    }

    #[must_use]
    pub const fn with_max_cleanup_failures(mut self, maximum: usize) -> Self {
        self.max_cleanup_failures = maximum;
        self
    }

    #[must_use]
    pub const fn with_native_stack_limits(
        mut self,
        maximum_aggregate_bytes: usize,
        maximum_frame_bytes: usize,
    ) -> Self {
        self.max_native_stack_bytes = maximum_aggregate_bytes;
        self.max_native_frame_bytes = maximum_frame_bytes;
        self
    }
}

impl Default for NativeInvocationConfig {
    fn default() -> Self {
        Self::new(u64::MAX, None)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeEntryCount {
    pub(super) source_function: u32,
    pub(super) entries: u64,
}

impl NativeEntryCount {
    #[must_use]
    pub const fn source_function(self) -> u32 {
        self.source_function
    }

    #[must_use]
    pub const fn entries(self) -> u64 {
        self.entries
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeServiceError {
    Trap,
    ResourceLimitExceeded,
    HostFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeCleanupFailure {
    slot: RuntimeCallSlot,
    error: NativeServiceError,
}

impl NativeCleanupFailure {
    pub(super) const fn new(slot: RuntimeCallSlot, error: NativeServiceError) -> Self {
        Self { slot, error }
    }

    #[must_use]
    pub const fn slot(self) -> RuntimeCallSlot {
        self.slot
    }

    #[must_use]
    pub const fn error(self) -> NativeServiceError {
        self.error
    }
}
