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
    pub max_heap_bytes: usize,
    pub max_resources: usize,
    pub resource_policy: EvalResourcePolicy,
    pub cleanup_failure_limits: lkjscript_core::CleanupFailureLimits,
    pub structural_limits: lkjscript_core::StructuralValueRuntimeLimits,
    pub args: Vec<String>,
    pub capabilities: Vec<lkjscript_contracts::CapabilityKind>,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            fuel: 1_000_000,
            max_frames: 1_024,
            max_allocations: 1_000_000,
            max_heap_bytes: usize::MAX,
            max_resources: 4_096,
            resource_policy: EvalResourcePolicy::default(),
            cleanup_failure_limits: lkjscript_core::CleanupFailureLimits::default(),
            structural_limits: lkjscript_core::StructuralValueRuntimeLimits::default(),
            args: Vec::new(),
            capabilities: Vec::new(),
        }
    }
}
