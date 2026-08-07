use crate::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JitConfig {
    pub backend_limits: BackendLimits,
    pub executable_limits: ExecutableLimits,
    pub max_object_compile_time: Duration,
    pub max_total_compile_time: Duration,
    pub retain_machine_code_diagnostics: bool,
    pub collect_metrics: bool,
    pub max_diagnostic_bytes: u64,
    pub optimization_limits: OptimizationLimits,
    /// Force bounded scheduled proof discovery; zero or one remains sequential.
    pub proof_discovery_workers: u16,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self {
            backend_limits: BackendLimits::default(),
            executable_limits: ExecutableLimits::default(),
            max_object_compile_time: Duration::from_millis(250),
            max_total_compile_time: Duration::from_secs(2),
            retain_machine_code_diagnostics: false,
            collect_metrics: false,
            max_diagnostic_bytes: 16 * 1024 * 1024,
            optimization_limits: OptimizationLimits::default(),
            proof_discovery_workers: 0,
        }
    }
}
