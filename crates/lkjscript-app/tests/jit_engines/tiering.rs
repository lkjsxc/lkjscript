use crate::canonical::{compile, evaluator, execution, f64_loop, Scalar};
use lkjscript_core::ExecutionConfig;
use lkjscript_ir::{evaluate, EvalConfig, OptimizationLimits};
use lkjscript_jit::{execute_forced, execute_optimizing, FailureCode, JitConfig, Tier, TierState};
use lkjscript_vm::run_chunk;

#[test]
fn canonical_source_ssa_installs_and_calls_main_callee_poll_without_fallback() {
    let program = compile(&f64_loop(), "f64-loop.lkjscript");
    let vm = execution(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ));
    let evaluated = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    let native = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("source-derived forced baseline JIT");
    assert_eq!(execution(native.outcome.clone()), vm);
    assert_eq!(execution(native.outcome), evaluated);
    assert_eq!(native.stats.code_objects.len(), 1);
    assert!(native.stats.code_objects[0].wx_transition_verified);
    assert!(native.stats.code_objects[0].code_bytes > 0);
    assert!(native.stats.code_objects[0].relocation_count >= 2);
    assert!(native.stats.code_objects[0]
        .runtime_calls
        .contains(&lkjscript_native::RuntimeCallSlot::Poll));
    assert!(native.stats.code_objects[0]
        .runtime_calls
        .contains(&lkjscript_native::RuntimeCallSlot::PublishSafepoint));
    assert!(native.stats.code_objects[0]
        .runtime_calls
        .contains(&lkjscript_native::RuntimeCallSlot::HeapDispatch));
    assert!(!native.stats.code_objects[0]
        .runtime_calls
        .contains(&lkjscript_native::RuntimeCallSlot::EnterFunction));
    assert!(native.stats.poll_calls > 0);
    assert!(native.stats.direct_native_calls > 0);
    assert_eq!(native.stats.vm_fallbacks, 0);
    assert_eq!(native.stats.functions.len(), 2);
    assert!(native
        .stats
        .functions
        .iter()
        .all(|function| function.native_entries() > 0));

    let optimized = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("source-derived forced optimizing JIT");
    assert_eq!(execution(optimized.outcome), vm);
    assert!(optimized.stats.optimizing_native_entries > 0);
    assert_eq!(optimized.stats.baseline_native_entries, 0);
    assert_eq!(optimized.stats.vm_fallbacks, 0);
}

#[test]
fn proof_optimizing_engine_executes_fewer_generated_operations_without_downgrade() {
    let source = include_str!("../fixtures/optimizing-loop.lkjscript");
    let program = compile(source, "optimizing-loop.lkjscript");
    let evaluated = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    let vm = execution(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ));
    let baseline = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("forced baseline execution");
    let optimizing = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("forced proof optimizing execution");

    assert_eq!(evaluated, Scalar::I64(0));
    assert_eq!(vm, evaluated);
    assert_eq!(execution(baseline.outcome), evaluated);
    assert_eq!(execution(optimizing.outcome), evaluated);
    assert_eq!(optimizing.stats.vm_fallbacks, 0);
    assert_eq!(optimizing.stats.baseline_code_objects, 0);
    assert_eq!(optimizing.stats.baseline_native_entries, 0);
    assert_eq!(optimizing.stats.optimizing_code_objects, 1);
    assert!(optimizing.stats.optimizing_native_entries > 0);
    assert!(optimizing.stats.algebraic_rewrites >= 3);
    assert!(optimizing.stats.gvn_rewrites >= 1);
    assert!(optimizing.stats.checked_i64_rewrites >= 1);
    assert!(optimizing
        .stats
        .functions
        .iter()
        .all(|function| function.state() == TierState::OptimizedNative));
    let baseline_object = &baseline.stats.code_objects[0];
    let optimized_object = &optimizing.stats.code_objects[0];
    assert_eq!(optimized_object.tier, Tier::Optimizing);
    assert!(optimized_object.wx_transition_verified);
    assert!(optimized_object.native_entry_count > 0);
    assert!(optimized_object.optimization_certificate.is_some());
    assert!(optimized_object.optimization_metadata_bytes_estimate > 0);
    assert!(optimized_object.code_bytes < baseline_object.code_bytes);
    let optimization = optimized_object
        .optimization_stats
        .expect("retained optimization accounting");
    assert!(optimization.output_instructions < optimization.input_instructions);
    assert_eq!(
        optimizing.stats.optimizing_passes,
        optimization.optimizing_passes
    );
    assert_eq!(optimizing.stats.optimization_discovery_passes, 1);
    assert_eq!(optimizing.stats.optimization_checker_passes, 1);
    assert_eq!(optimizing.stats.optimization_reconstruction_passes, 2);
    assert_eq!(optimizing.stats.optimization_cleanup_passes, 14);
    assert_eq!(optimizing.stats.optimization_validation_passes, 17);
    assert_eq!(optimization.instruction_growth, 0);
    assert!(optimizing.stats.optimization_certificate_bytes_estimate > 0);
}

#[test]
fn forced_native_tiers_reject_path_entries_without_fallback() {
    let path = compile(
        concat!(
            "main/\nsig/\n->\nPath\n/sig\nunwrap-ok/\npath-from-str/\n",
            "str/\n/tmp/native-path\n/str\n/path-from-str\n/unwrap-ok\n/main\n",
        ),
        "native-path.lkjscript",
    );
    let baseline = execute_forced(
        path.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect_err("forced baseline Path entry must not fall back");
    assert_eq!(baseline.code(), FailureCode::UnsupportedType);
    let optimizing = execute_optimizing(
        path.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect_err("forced optimizing Path entry must not fall back");
    assert_eq!(optimizing.code(), FailureCode::UnsupportedType);
}

#[test]
fn forced_optimizing_rejects_unsupported_and_budget_failure_without_downgrade() {
    let unsupported = compile(
        concat!(
            "main/\nsig/\nCapability/\nStdio\n/Capability\n->\nUnit\n/sig\n",
            "params/\nstdio\nCapability/\nStdio\n/Capability\n/params\n",
            "flush/\nstdio\n/flush\n/main\n"
        ),
        "optimizing-unsupported.lkjscript",
    );
    let error = execute_optimizing(
        unsupported.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect_err("forced optimizing host operation must not fall back");
    assert_eq!(error.code(), FailureCode::UnsupportedType);

    let source = include_str!("../fixtures/optimizing-loop.lkjscript");
    let program = compile(source, "optimizing-budget.lkjscript");
    let mut config = JitConfig::default();
    config.optimization_limits = OptimizationLimits {
        max_work_units: 0,
        ..OptimizationLimits::default()
    };
    let error = execute_optimizing(program.ssa(), &ExecutionConfig::default(), config)
        .expect_err("forced optimizing budget must not run baseline or VM");
    assert_eq!(error.code(), FailureCode::OptimizationBudget);
}
