use crate::*;

pub(crate) fn optimization_metadata_bytes_estimate(stats: Option<&OptimizationStats>) -> u64 {
    stats.map_or(0, |stats| {
        stats
            .certificate_bytes_estimate
            .saturating_add(17_u64.saturating_mul(8))
    })
}

pub fn execute_forced(
    program: &VerifiedProgram,
    execution: &ExecutionConfig,
    config: JitConfig,
) -> Result<JitExecution, EngineError> {
    execute_forced_with_capabilities(program, &[], execution, config)
}

pub fn execute_forced_with_capabilities(
    program: &VerifiedProgram,
    capabilities: &[lkjscript_core::CapabilityKind],
    execution: &ExecutionConfig,
    config: JitConfig,
) -> Result<JitExecution, EngineError> {
    let main = program.program().main;
    let mut session = JitSession::new_baseline(program, config);
    session.compile_group(main)?;
    let arguments = capability_arguments(program, capabilities)?;
    let invocation = session.invoke_scalar(main, &arguments, execution)?;
    let outcome = scalar_to_execution(&mut session, main, invocation.outcome)?
        .with_cleanup_failures(invocation.cleanup_failures);
    let stats = session.stats();
    verify_forced_entry(&outcome, &stats, main, TierState::BaselineNative)?;
    if stats.optimizing_code_objects != 0 || stats.optimizing_native_entries != 0 {
        return Err(EngineError::new(
            FailureCode::InvocationFailure,
            Some(main),
            "forced baseline engine installed or entered optimizing code",
        ));
    }
    Ok(JitExecution { outcome, stats })
}

/// Proof-optimize the complete verified program, install only optimizing-tier
/// code for the required reachable group, and enter optimized main.
pub fn execute_optimizing(
    program: &VerifiedProgram,
    execution: &ExecutionConfig,
    config: JitConfig,
) -> Result<JitExecution, EngineError> {
    execute_optimizing_with_capabilities(program, &[], execution, config)
}

pub fn execute_optimizing_with_capabilities(
    program: &VerifiedProgram,
    capabilities: &[lkjscript_core::CapabilityKind],
    execution: &ExecutionConfig,
    config: JitConfig,
) -> Result<JitExecution, EngineError> {
    let started = Instant::now();
    let optimized = optimize(program, config.optimization_limits).map_err(optimization_error)?;
    let optimization_time = started.elapsed();
    if optimization_time > config.max_object_compile_time
        || optimization_time > config.max_total_compile_time
    {
        return Err(EngineError::new(
            FailureCode::CompileWallTime,
            Some(program.program().main),
            "optimizing pass wall-time budget exceeded",
        ));
    }
    let main = optimized.program().main;
    let mut session = JitSession::new_optimizing(optimized, config, optimization_time);
    session.compile_group(main)?;
    let arguments = capability_arguments(program, capabilities)?;
    let invocation = session.invoke_scalar(main, &arguments, execution)?;
    let outcome = scalar_to_execution(&mut session, main, invocation.outcome)?
        .with_cleanup_failures(invocation.cleanup_failures);
    let stats = session.stats();
    verify_forced_entry(&outcome, &stats, main, TierState::OptimizedNative)?;
    if stats.baseline_code_objects != 0
        || stats.baseline_native_entries != 0
        || stats.optimizing_code_objects == 0
        || stats.optimizing_native_entries == 0
        || stats.vm_fallbacks != 0
    {
        return Err(EngineError::new(
            FailureCode::InvocationFailure,
            Some(main),
            "forced optimizing engine did not remain optimizing-only",
        ));
    }
    Ok(JitExecution { outcome, stats })
}

fn verify_forced_entry(
    _outcome: &ExecutionOutcome,
    stats: &JitStats,
    main: FunctionId,
    expected_state: TierState,
) -> Result<(), EngineError> {
    if stats.native_entries == 0
        || stats.vm_fallbacks != 0
        || stats.vm_to_native_transitions != 0
        || stats.native_to_vm_transitions != 0
        || stats
            .functions
            .iter()
            .filter(|record| record.code_object.is_some())
            .any(|record| record.state != expected_state)
    {
        return Err(EngineError::new(
            FailureCode::InvocationFailure,
            Some(main),
            "forced engine did not remain generated-only in the selected tier",
        ));
    }
    Ok(())
}

fn capability_arguments(
    program: &VerifiedProgram,
    capabilities: &[lkjscript_core::CapabilityKind],
) -> Result<Vec<NativeValue>, EngineError> {
    let Some(index) = program.program().main.index() else {
        return Err(EngineError::new(
            FailureCode::InvocationFailure,
            Some(program.program().main),
            "native main identity is invalid",
        ));
    };
    let Some(main) = program.program().functions.get(index) else {
        return Err(EngineError::new(
            FailureCode::InvocationFailure,
            Some(program.program().main),
            "native main is absent",
        ));
    };
    if main.signature.parameters.len() != capabilities.len()
        || main
            .signature
            .parameters
            .iter()
            .zip(capabilities)
            .any(|(parameter, capability)| parameter != &SsaType::Capability(*capability))
    {
        return Err(EngineError::new(
            FailureCode::InvocationFailure,
            Some(program.program().main),
            "native main capability arguments do not exactly match verified SSA",
        ));
    }
    Ok(capabilities
        .iter()
        .copied()
        .map(NativeValue::Capability)
        .collect())
}

fn optimization_error(error: lkjscript_ir::OptimizationError) -> EngineError {
    let code = match error.code() {
        OptimizationFailureCode::BudgetExceeded => FailureCode::OptimizationBudget,
        OptimizationFailureCode::InputVerification
        | OptimizationFailureCode::CertificateMismatch
        | OptimizationFailureCode::IllegalEdit
        | OptimizationFailureCode::CandidateMismatch
        | OptimizationFailureCode::OutputVerification => FailureCode::CertificateVerification,
    };
    EngineError::new(code, None, error.to_string())
}
