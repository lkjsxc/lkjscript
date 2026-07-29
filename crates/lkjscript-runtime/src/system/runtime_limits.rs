#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeLimits {
    pub max_concurrent_invocations: NonZeroUsize,
    pub max_total_invocations: NonZeroU64,
}

impl Default for RuntimeLimits {
    fn default() -> Self {
        Self {
            max_concurrent_invocations: NonZeroUsize::new(1_024).unwrap_or(NonZeroUsize::MIN),
            max_total_invocations: NonZeroU64::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeAccounting {
    pub active_invocations: usize,
    pub total_invocations: u64,
    pub peak_concurrent: usize,
    pub limits: RuntimeLimits,
}
