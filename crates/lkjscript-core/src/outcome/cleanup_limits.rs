pub const DEFAULT_MAX_CLEANUP_FAILURES: usize = 32;
pub const DEFAULT_MAX_CLEANUP_FAILURE_BYTES: usize = 8 * 1024;
pub const MAX_CLEANUP_FAILURES: usize = 4_096;
pub const MAX_CLEANUP_FAILURE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CleanupFailureLimits {
    pub(super) max_failures: usize,
    pub(super) max_message_bytes: usize,
}

impl CleanupFailureLimits {
    pub const fn new(max_failures: usize, max_message_bytes: usize) -> Option<Self> {
        if max_failures <= MAX_CLEANUP_FAILURES && max_message_bytes <= MAX_CLEANUP_FAILURE_BYTES {
            Some(Self {
                max_failures,
                max_message_bytes,
            })
        } else {
            None
        }
    }

    pub const fn max_failures(self) -> usize {
        self.max_failures
    }

    pub const fn max_message_bytes(self) -> usize {
        self.max_message_bytes
    }
}

impl Default for CleanupFailureLimits {
    fn default() -> Self {
        Self::new(
            DEFAULT_MAX_CLEANUP_FAILURES,
            DEFAULT_MAX_CLEANUP_FAILURE_BYTES,
        )
        .unwrap_or_else(|| std::process::abort())
    }
}
