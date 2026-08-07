use crate::*;

impl NativeRun {
    pub(crate) fn new_baseline_attempt(program: &VerifiedProgram, config: JitConfig) -> Self {
        Self {
            program: program.clone(),
            installer: ExecutableInstaller::new(config.executable_limits),
            config,
            object: None,
            require_pre_entry_stack_check: true,
            total_compile_time: Duration::ZERO,
            native_entries: 0,
            direct_native_calls: 0,
            poll_calls: 0,
            native_invocations: 0,
            metrics_started: config.collect_metrics.then(Instant::now),
            time_to_first_native_entry: None,
            first_native_call: None,
            native_execution: Duration::ZERO,
            diagnostic_bytes: 0,
            lists: None,
            region_products: None,
            runtime_heap_attempts: 0,
            runtime_heap_successes: 0,
            resource_runtime_calls: 0,
            unique_runtime_calls: 0,
            structural_runtime_calls: 0,
            native_resources: NativeResourceStats::default(),
            native_unique: NativeUniqueStats::default(),
            native_structural: NativeStructuralStats::default(),
            returned_unique: None,
            returned_structural: None,
            next_resource_scope: 1,
            peak_native_frame_depth: 0,
            peak_native_stack_bytes: 0,
            last_runtime_trap: None,
            last_runtime_resource: None,
            last_runtime_failure: None,
            last_lowering_and_encoding: Duration::ZERO,
            last_installation: Duration::ZERO,
        }
    }
}
