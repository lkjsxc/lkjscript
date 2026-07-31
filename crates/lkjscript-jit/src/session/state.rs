use crate::*;

fn source_name(name: &str) -> &str {
    let Some(encoded) = name.strip_prefix("__module_") else {
        return name;
    };
    let Some((digest, source)) = encoded.split_once(':') else {
        return name;
    };
    if digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        source
    } else {
        name
    }
}

impl JitSession {
    pub fn new_auto(
        program: &VerifiedProgram,
        links: &BytecodeLinkMetadata,
        config: JitConfig,
    ) -> Self {
        Self::new(
            ProgramAuthority::Baseline(program.clone()),
            Some(links.clone()),
            config,
            Duration::ZERO,
        )
    }

    pub(crate) fn new_baseline(program: &VerifiedProgram, config: JitConfig) -> Self {
        Self::new(
            ProgramAuthority::Baseline(program.clone()),
            None,
            config,
            Duration::ZERO,
        )
    }

    pub(crate) fn new_optimizing(
        program: VerifiedOptimizedProgram,
        config: JitConfig,
        optimization_time: Duration,
    ) -> Self {
        Self::new(
            ProgramAuthority::Optimizing(program),
            None,
            config,
            optimization_time,
        )
    }

    pub(crate) fn new(
        program: ProgramAuthority,
        links: Option<BytecodeLinkMetadata>,
        config: JitConfig,
        optimization_time: Duration,
    ) -> Self {
        let initial_state = match program.tier() {
            Tier::Baseline => TierState::VmOnly,
            Tier::Optimizing => TierState::OptimizingCandidate,
        };
        let functions = program
            .program()
            .functions
            .iter()
            .map(|function| FunctionTierRecord {
                function: function.id,
                name: source_name(&function.name).to_string(),
                state: initial_state,
                call_count: 0,
                attempts: 0,
                last_failure: None,
                code_object: None,
                epoch: config.epoch,
                native_entries: 0,
                auto_entry_eligible: function
                    .signature
                    .parameters
                    .iter()
                    .chain(std::iter::once(function.signature.result.as_ref()))
                    .all(|ty| native_type(ty).is_some()),
            })
            .collect();
        Self {
            program,
            links,
            installer: ExecutableInstaller::new(config.executable_limits),
            config,
            functions,
            objects: Vec::new(),
            next_object: 1,
            total_compile_time: optimization_time,
            optimization_time,
            native_entries: 0,
            direct_native_calls: 0,
            poll_calls: 0,
            vm_fallbacks: 0,
            compile_failures: 0,
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
            vm_to_native_transitions: 0,
            native_to_vm_transitions: 0,
            last_runtime_trap: None,
            last_runtime_resource: None,
        }
    }
}
