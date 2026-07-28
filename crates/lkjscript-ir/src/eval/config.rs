#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EvalResourcePolicy {
    pub fail_acquisition: Option<lkjscript_core::ResourceKind>,
    pub fail_close: Option<lkjscript_core::ResourceKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalConfig {
    pub fuel: u64,
    pub max_frames: usize,
    pub max_allocations: u64,
    pub max_logical_aggregate_constructions: u64,
    pub max_heap_bytes: usize,
    pub max_buffer_bytes: usize,
    pub max_list_equal_steps: usize,
    pub max_resources: usize,
    pub resource_policy: EvalResourcePolicy,
    pub cleanup_failure_limits: lkjscript_core::CleanupFailureLimits,
    pub args: Vec<String>,
    pub capabilities: Vec<lkjscript_contracts::CapabilityKind>,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            fuel: 1_000_000,
            max_frames: 1_024,
            max_allocations: 1_000_000,
            max_logical_aggregate_constructions: 1_000_000,
            max_heap_bytes: usize::MAX,
            max_buffer_bytes: 1_000_000,
            max_list_equal_steps: 1_000_000,
            max_resources: 4_096,
            resource_policy: EvalResourcePolicy::default(),
            cleanup_failure_limits: lkjscript_core::CleanupFailureLimits::default(),
            args: Vec::new(),
            capabilities: Vec::new(),
        }
    }
}
