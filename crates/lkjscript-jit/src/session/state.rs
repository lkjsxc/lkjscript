use crate::*;

fn automatic_cycle_reachable(program: &lkjscript_ir::Program) -> Vec<bool> {
    let mut edges = vec![Vec::new(); program.functions.len()];
    let mut reverse = vec![Vec::new(); program.functions.len()];
    for (index, function) in program.functions.iter().enumerate() {
        for callee in function
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .filter_map(|instruction| match &instruction.kind {
                lkjscript_ir::InstructionKind::Call {
                    target: lkjscript_ir::CallTarget::Direct(callee),
                    ..
                } => callee.index(),
                _ => None,
            })
            .filter(|callee| *callee < program.functions.len())
        {
            edges[index].push(callee);
            reverse[callee].push(index);
        }
    }

    let mut visited = vec![false; program.functions.len()];
    let mut finish_order = Vec::with_capacity(program.functions.len());
    for start in 0..program.functions.len() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut work = vec![(start, 0_usize)];
        while let Some((node, next_edge)) = work.last_mut() {
            if let Some(callee) = edges[*node].get(*next_edge).copied() {
                *next_edge += 1;
                if !visited[callee] {
                    visited[callee] = true;
                    work.push((callee, 0));
                }
            } else {
                finish_order.push(*node);
                work.pop();
            }
        }
    }

    let mut component = vec![usize::MAX; program.functions.len()];
    let mut cyclic_nodes = Vec::new();
    let mut component_id = 0_usize;
    for start in finish_order.into_iter().rev() {
        if component[start] != usize::MAX {
            continue;
        }
        component[start] = component_id;
        let mut members = Vec::new();
        let mut work = vec![start];
        while let Some(node) = work.pop() {
            members.push(node);
            for predecessor in &reverse[node] {
                if component[*predecessor] == usize::MAX {
                    component[*predecessor] = component_id;
                    work.push(*predecessor);
                }
            }
        }
        if members.len() > 1
            || members
                .first()
                .is_some_and(|node| edges[*node].contains(node))
        {
            cyclic_nodes.extend(members);
        }
        component_id += 1;
    }

    let mut reaches_cycle = vec![false; program.functions.len()];
    let mut work = cyclic_nodes;
    for node in &work {
        reaches_cycle[*node] = true;
    }
    while let Some(node) = work.pop() {
        for predecessor in &reverse[node] {
            if !reaches_cycle[*predecessor] {
                reaches_cycle[*predecessor] = true;
                work.push(*predecessor);
            }
        }
    }
    reaches_cycle
}

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

    pub(crate) fn new_baseline_attempt(
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
        let cycle_reachable = automatic_cycle_reachable(program.program());
        let functions = program
            .program()
            .functions
            .iter()
            .enumerate()
            .map(|(index, function)| FunctionTierRecord {
                function: function.id,
                name: source_name(&function.name).to_string(),
                state: initial_state,
                call_count: 0,
                attempts: 0,
                last_failure: None,
                code_object: None,
                epoch: config.epoch,
                native_entries: 0,
                auto_entry_eligible: !cycle_reachable[index]
                    && function.signature.memory_witness_parameters.is_empty()
                    && function.signature.parameters.len() <= 2
                    && function
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
            peak_native_stack_bytes: 0,
            vm_to_native_transitions: 0,
            native_to_vm_transitions: 0,
            last_runtime_trap: None,
            last_runtime_resource: None,
            last_runtime_failure: None,
            last_lowering_and_encoding: Duration::ZERO,
            last_installation: Duration::ZERO,
        }
    }
}
