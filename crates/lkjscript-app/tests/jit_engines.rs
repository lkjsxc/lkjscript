#![allow(clippy::expect_used, clippy::field_reassign_with_default)]

use std::time::Duration;

use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Limits, OwnedValue, ResourceLimitKind};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue, OptimizationLimits};
use lkjscript_jit::{
    execute_forced, execute_optimizing, FailureCode, JitConfig, JitSession, Tier, TierState,
};
use lkjscript_vm::{run_chunk, run_chunk_auto};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Scalar {
    Unit,
    Bool(bool),
    I64(i64),
    F64(u64),
    Exited(i64),
    Trapped,
    Deadline,
    Fuel,
    Other(String),
}

fn owned(value: &OwnedValue) -> Scalar {
    if value.is_unit() {
        Scalar::Unit
    } else if let Some(value) = value.as_bool() {
        Scalar::Bool(value)
    } else if let Some(value) = value.as_i64() {
        Scalar::I64(value)
    } else if let Some(value) = value.as_f64() {
        Scalar::F64(value.to_bits())
    } else {
        Scalar::Other(format!("{value:?}"))
    }
}

fn execution(outcome: ExecutionOutcome) -> Scalar {
    match outcome {
        ExecutionOutcome::Returned(value) => owned(&value),
        ExecutionOutcome::Exited(code) => Scalar::Exited(i64::from(code)),
        ExecutionOutcome::Trapped(_) => Scalar::Trapped,
        ExecutionOutcome::DeadlineExceeded => Scalar::Deadline,
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::InstructionFuel) => Scalar::Fuel,
        other => Scalar::Other(other.summary()),
    }
}

fn evaluator(outcome: EvalOutcome) -> Scalar {
    match outcome {
        EvalOutcome::Returned(EvalValue::Unit) => Scalar::Unit,
        EvalOutcome::Returned(EvalValue::Bool(value)) => Scalar::Bool(value),
        EvalOutcome::Returned(EvalValue::I64(value)) => Scalar::I64(value),
        EvalOutcome::Returned(EvalValue::F64(value)) => Scalar::F64(value.to_bits()),
        EvalOutcome::Exited(code) => Scalar::Exited(code),
        EvalOutcome::Trapped(_) => Scalar::Trapped,
        other => Scalar::Other(format!("{other:?}")),
    }
}

fn compile(source: &str, name: &str) -> lkjscript_compiler::ExecutableProgram {
    compile_source(source, name, &Limits::default()).expect("compile JIT source fixture")
}

fn forced(source: &str, name: &str) -> lkjscript_jit::JitExecution {
    let program = compile(source, name);
    execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("forced native execution")
}

const F64_LOOP: &str = "def/\nname/\nstep\n/name\nfn/\nsig/\nF64\nI64\n->\nF64\n/sig\nparams/\nacc\nF64\ni\nI64\n/params\n+/\nacc\ndiv/\n1.0\n+/\n*/\n2.0\ni\n/*\n1.0\n/+\n/div\n/+\n/fn\n/def\nmain/\nsig/\n->\nF64\n/sig\nvar/\nname/\ni\n/name\ntype/\nI64\n/type\n0\nvar/\nname/\nacc\n/name\ntype/\nF64\n/type\n0.0\ndo/\nwhile/\nlt/\ni\n1000\n/lt\ndo/\nset/\nacc\nstep/\nacc\ni\n/step\n/set\nset/\ni\n+/\ni\n1\n/+\n/set\n/do\n/while\nacc\n/do\n/var\n/var\n/main\n";

#[test]
fn canonical_source_ssa_installs_and_calls_main_callee_poll_v1_without_fallback() {
    let program = compile(F64_LOOP, "f64-loop.lkjscript");
    let vm = execution(run_chunk(program.bytecode(), &ExecutionConfig::default()));
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
        .contains(&lkjscript_native::RuntimeCallSlot::PollV1));
    assert!(!native.stats.code_objects[0]
        .runtime_calls
        .contains(&lkjscript_native::RuntimeCallSlot::PublishSafepointV1));
    assert!(!native.stats.code_objects[0]
        .runtime_calls
        .contains(&lkjscript_native::RuntimeCallSlot::EnterFunctionV1));
    assert!(native.stats.poll_v1_calls > 0);
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
    let source = include_str!("fixtures/optimizing-loop.lkjscript");
    let program = compile(source, "optimizing-loop.lkjscript");
    let evaluated = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    let vm = execution(run_chunk(program.bytecode(), &ExecutionConfig::default()));
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
fn forced_optimizing_rejects_unsupported_and_budget_failure_without_downgrade() {
    let unsupported = compile(
        "main/\nsig/\n->\nUnit\n/sig\nflush/\n/flush\n/main\n",
        "optimizing-unsupported.lkjscript",
    );
    let error = execute_optimizing(
        unsupported.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect_err("forced optimizing host operation must not fall back");
    assert_eq!(error.code(), FailureCode::UnsupportedOperation);

    let source = include_str!("fixtures/optimizing-loop.lkjscript");
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

#[test]
fn i64_multiblock_loop_and_direct_call_match_vm() {
    let source = "def/\nname/\nadd-if-even\n/name\nfn/\nsig/\nI64\nI64\n->\nI64\n/sig\nparams/\nacc\nI64\ni\nI64\n/params\nif/\nequal-value/\nbit-and/\ni\n1\n/bit-and\n0\n/equal-value\n+/\nacc\ni\n/+\nacc\n/if\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nvar/\nname/\ni\n/name\ntype/\nI64\n/type\n0\nvar/\nname/\nacc\n/name\ntype/\nI64\n/type\n0\ndo/\nwhile/\nlt/\ni\n100\n/lt\ndo/\nset/\nacc\nadd-if-even/\nacc\ni\n/add-if-even\n/set\nset/\ni\n+/\ni\n1\n/+\n/set\n/do\n/while\nacc\n/do\n/var\n/var\n/main\n";
    let program = compile(source, "i64-cfg.lkjscript");
    let vm = execution(run_chunk(program.bytecode(), &ExecutionConfig::default()));
    let native = forced(source, "i64-cfg.lkjscript");
    assert_eq!(vm, Scalar::I64(2450));
    assert_eq!(execution(native.outcome), vm);
    assert!(native.stats.direct_native_calls >= 100);
}

#[test]
fn checked_i64_traps_exit_and_explicit_trap_remain_structured() {
    for (name, expression) in [
        ("overflow.lkjscript", "+/\n9223372036854775807\n1\n/+"),
        ("divide-zero.lkjscript", "div/\n1\n0\n/div"),
        (
            "divide-overflow.lkjscript",
            "div/\n-9223372036854775808\n-1\n/div",
        ),
    ] {
        let source = format!("main/\nsig/\n->\nI64\n/sig\n{expression}\n/main\n");
        let program = compile(&source, name);
        assert_eq!(
            execution(run_chunk(program.bytecode(), &ExecutionConfig::default())),
            Scalar::Trapped
        );
        assert_eq!(execution(forced(&source, name).outcome), Scalar::Trapped);
        let optimized = execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("optimizing trap remains structured");
        assert_eq!(execution(optimized.outcome), Scalar::Trapped);
        assert_eq!(optimized.stats.baseline_native_entries, 0);
    }

    for (name, operation, left, right, expected_trap) in [
        (
            "duplicate-overflow.lkjscript",
            "+",
            "9223372036854775807",
            "1",
            "checked I64 overflow",
        ),
        (
            "duplicate-division.lkjscript",
            "div",
            "1",
            "0",
            "div: I64 division by zero",
        ),
    ] {
        let expression = format!("{operation}/\nz\none\n/{operation}");
        let source = format!(
            "main/\nsig/\n->\nI64\n/sig\nlet/\nbind/\nz\n{left}\n/bind\nlet/\nbind/\none\n{right}\n/bind\nlet/\nbind/\nfirst\n{expression}\n/bind\nlet/\nbind/\nsecond\n{expression}\n/bind\nsecond\n/let\n/let\n/let\n/let\n/main\n"
        );
        let program = compile(&source, name);
        let baseline = execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("duplicate checked baseline trap");
        let optimized = execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("duplicate checked optimizing trap");
        assert!(matches!(
            baseline.outcome,
            ExecutionOutcome::Trapped(ref trap) if trap.as_str() == expected_trap
        ));
        assert!(matches!(
            optimized.outcome,
            ExecutionOutcome::Trapped(ref trap) if trap.as_str() == expected_trap
        ));
        assert_eq!(optimized.stats.baseline_native_entries, 0);
        assert_eq!(optimized.stats.vm_fallbacks, 0);
        assert_eq!(optimized.stats.checked_i64_rewrites, 1);
    }

    let exit = "main/\nsig/\n->\nUnit\n/sig\nexit/\n17\n/exit\n/main\n";
    assert_eq!(
        execution(forced(exit, "exit.lkjscript").outcome),
        Scalar::Exited(17)
    );
    let exit_program = compile(exit, "optimizing-exit.lkjscript");
    assert_eq!(
        execution(
            execute_optimizing(
                exit_program.ssa(),
                &ExecutionConfig::default(),
                JitConfig::default(),
            )
            .expect("optimizing exit remains structured")
            .outcome,
        ),
        Scalar::Exited(17)
    );
}

#[test]
fn f64_bits_ieee_comparisons_and_mixed_conversion_are_exact() {
    let cases = [
        (
            "signed-zero.lkjscript",
            "Bool",
            "f64-bits-equal/\n0.0\n-0.0\n/f64-bits-equal",
            Scalar::Bool(false),
        ),
        (
            "nan-order.lkjscript",
            "Bool",
            "lt/\ndiv/\n0.0\n0.0\n/div\n1.0\n/lt",
            Scalar::Bool(false),
        ),
        (
            "mixed.lkjscript",
            "F64",
            "+/\n9007199254740993\n0.5\n/+",
            Scalar::F64((9_007_199_254_740_993_i64 as f64 + 0.5).to_bits()),
        ),
    ];
    for (name, ty, expression, expected) in cases {
        let source = format!("main/\nsig/\n->\n{ty}\n/sig\n{expression}\n/main\n");
        let program = compile(&source, name);
        let vm = execution(run_chunk(program.bytecode(), &ExecutionConfig::default()));
        let native = execution(forced(&source, name).outcome);
        assert_eq!(vm, expected, "VM oracle for {name}");
        assert_eq!(native, vm, "native result for {name}");
    }
}

#[test]
fn forced_mode_executes_host_independent_allocation_and_recursion_without_fallback() {
    let unsupported = [
        (
            "host.lkjscript",
            "main/\nsig/\n->\nUnit\n/sig\nflush/\n/flush\n/main\n",
        ),
        (
            "owned-buffer.lkjscript",
            "main/\nsig/\n->\nI64\n/sig\nlet/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nowned-buf-ref/\nborrow/\nb\n/borrow\n0\n/owned-buf-ref\n/let\n/main\n",
        ),
    ];
    for (name, source) in unsupported {
        let program = compile(source, name);
        let error = execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect_err("forced native must reject unsupported semantics");
        assert!(matches!(
            error.code(),
            FailureCode::UnsupportedType | FailureCode::UnsupportedOperation
        ));
        if name == "owned-buffer.lkjscript" {
            let diagnostic = error.to_string();
            assert!(
                diagnostic.contains("reference") || diagnostic.contains("ownership"),
                "forced owned-buffer rejection was not ownership-specific: {diagnostic}"
            );
        }
    }

    let recursion = "def/\nname/\nrecur\n/name\nfn/\nsig/\nI64\n->\nI64\n/sig\nparams/\nn\nI64\n/params\nif/\nlte/\nn\n0\n/lte\n0\nrecur/\n-/\nn\n1\n/-\n/recur\n/if\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nrecur/\n3\n/recur\n/main\n";
    let program = compile(recursion, "recursion.lkjscript");
    let executed = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("direct recursion is a bounded native SCC");
    assert!(
        matches!(executed.outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(0))
    );
    assert!(executed.stats.direct_native_calls >= 3);
    assert_eq!(executed.stats.vm_fallbacks, 0);

    let allocation = compile(
        "main/\nsig/\n->\nStr\n/sig\nempty-str/\n/empty-str\n/main\n",
        "allocation.lkjscript",
    );
    let mut config = JitConfig::default();
    config.force_gc_before_allocation = true;
    let executed = execute_forced(allocation.ssa(), &ExecutionConfig::default(), config)
        .expect("source allocation reaches generated heap dispatch");
    assert!(
        matches!(executed.outcome, ExecutionOutcome::Returned(value) if value.as_str() == Some(""))
    );
    assert_eq!(executed.stats.vm_fallbacks, 0);
    assert_eq!(executed.stats.allocations, 1);
    assert_eq!(executed.stats.collections, 1);
    assert_eq!(executed.stats.runtime_heap_attempts, 1);
    assert_eq!(executed.stats.runtime_heap_successes, 1);
    assert!(executed.stats.native_entries > 0);
}

#[test]
fn generated_buffer_result_boundaries_match_vm_and_evaluator_exactly() {
    let cases = [
        (
            "buf-slice-negative-offset.lkjscript",
            "equal-value/\nunwrap-err/\nbuf-slice/\nbuf-new/\n1\n/buf-new\n-1\n1\n/buf-slice\n/unwrap-err\nstr/\nbuf-slice offset out of range\n/str\n/equal-value",
        ),
        (
            "buf-slice-negative-length.lkjscript",
            "equal-value/\nunwrap-err/\nbuf-slice/\nbuf-new/\n1\n/buf-new\n0\n-1\n/buf-slice\n/unwrap-err\nstr/\nbuf-slice length out of range\n/str\n/equal-value",
        ),
        (
            "buf-slice-out-of-range.lkjscript",
            "equal-value/\nunwrap-err/\nbuf-slice/\nbuf-new/\n1\n/buf-new\n1\n1\n/buf-slice\n/unwrap-err\nstr/\nbuf-slice range out of bounds\n/str\n/equal-value",
        ),
    ];
    for (name, expression) in cases {
        let source = format!("main/\nsig/\n->\nBool\n/sig\n{expression}\n/main\n");
        let program = compile(&source, name);
        let expected = Scalar::Bool(true);
        assert_eq!(
            evaluator(evaluate(program.ssa(), &EvalConfig::default())),
            expected
        );
        assert_eq!(
            execution(run_chunk(program.bytecode(), &ExecutionConfig::default())),
            expected
        );
        assert_eq!(execution(forced(&source, name).outcome), expected);
    }

    let invalid_utf8 = "main/\nsig/\n->\nBool\n/sig\nvar/\nname/\nb\n/name\ntype/\nBuf\n/type\nbuf-new/\n1\n/buf-new\ndo/\nbuf-set/\nb\n0\n255\n/buf-set\nequal-value/\nunwrap-err/\nbuf-to-str/\nb\n/buf-to-str\n/unwrap-err\nstr/\nbuf-to-str: invalid UTF-8\n/str\n/equal-value\n/do\n/var\n/main\n";
    let program = compile(invalid_utf8, "buf-to-str-error.lkjscript");
    let expected = Scalar::Bool(true);
    assert_eq!(
        evaluator(evaluate(program.ssa(), &EvalConfig::default())),
        expected
    );
    assert_eq!(
        execution(run_chunk(program.bytecode(), &ExecutionConfig::default())),
        expected
    );
    assert_eq!(
        execution(forced(invalid_utf8, "buf-to-str-error.lkjscript").outcome),
        expected
    );
}

#[test]
fn evaluator_vm_and_native_buffer_results_share_tiny_resource_boundaries() {
    let cases = [
        (
            "buf-to-str-success-limits.lkjscript",
            "Result\nStr\nStr",
            "buf-to-str/\nbuf-new/\n0\n/buf-new\n/buf-to-str",
        ),
        (
            "buf-to-str-error-limits.lkjscript",
            "Result\nStr\nStr",
            "var/\nname/\nb\n/name\ntype/\nBuf\n/type\nbuf-new/\n1\n/buf-new\ndo/\nbuf-set/\nb\n0\n255\n/buf-set\nbuf-to-str/\nb\n/buf-to-str\n/do\n/var",
        ),
        (
            "buf-slice-success-limits.lkjscript",
            "Result\nBuf\nStr",
            "buf-slice/\nbuf-new/\n1\n/buf-new\n0\n1\n/buf-slice",
        ),
        (
            "buf-slice-error-limits.lkjscript",
            "Result\nBuf\nStr",
            "buf-slice/\nbuf-new/\n1\n/buf-new\n-1\n1\n/buf-slice",
        ),
    ];
    for (name, return_type, expression) in cases {
        let source = format!("main/\nsig/\n->\n{return_type}\n/sig\n{expression}\n/main\n");
        let program = compile(&source, name);

        let eval_allocations = EvalConfig {
            max_allocations: 2,
            ..EvalConfig::default()
        };
        assert!(matches!(
            evaluate(program.ssa(), &eval_allocations),
            EvalOutcome::ResourceLimitExceeded(ref kind) if kind == "allocations"
        ));
        let allocation_limits = ExecutionConfig {
            max_allocations: 2,
            ..ExecutionConfig::default()
        };
        assert!(matches!(
            run_chunk(program.bytecode(), &allocation_limits),
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations)
        ));
        assert!(matches!(
            execute_forced(program.ssa(), &allocation_limits, JitConfig::default())
                .expect("native allocation limit is structured")
                .outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations)
        ));

        let eval_heap = EvalConfig {
            max_heap_bytes: 1,
            ..EvalConfig::default()
        };
        assert!(matches!(
            evaluate(program.ssa(), &eval_heap),
            EvalOutcome::ResourceLimitExceeded(ref kind) if kind == "heap bytes"
        ));
        let heap_limits = ExecutionConfig {
            max_heap_bytes: 1,
            ..ExecutionConfig::default()
        };
        assert!(matches!(
            run_chunk(program.bytecode(), &heap_limits),
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::HeapBytes)
        ));
        assert!(matches!(
            execute_forced(program.ssa(), &heap_limits, JitConfig::default())
                .expect("native heap limit is structured")
                .outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::HeapBytes)
        ));
    }
}

#[test]
fn nested_product_option_result_list_string_buffer_graph_matches_all_engines() {
    let source = include_str!("fixtures/allocation-graph.lkjscript");
    let program = compile(source, "allocation-graph.lkjscript");
    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    let vm = execution(run_chunk(program.bytecode(), &ExecutionConfig::default()));
    let mut config = JitConfig::default();
    config.force_gc_before_allocation = true;
    let native = execute_forced(program.ssa(), &ExecutionConfig::default(), config)
        .expect("nested graph executes through generated heap sites");
    let optimized = execute_optimizing(program.ssa(), &ExecutionConfig::default(), config)
        .expect("nested graph executes through optimized generated heap sites");
    assert_eq!(expected, Scalar::I64(1));
    assert_eq!(vm, expected);
    assert_eq!(execution(native.outcome), expected);
    assert_eq!(execution(optimized.outcome), expected);
    assert_eq!(native.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.baseline_native_entries, 0);
    assert!(optimized.stats.optimizing_native_entries > 0);
    assert_eq!(optimized.stats.allocations, native.stats.allocations);
    assert_eq!(
        optimized.stats.allocation_bytes_estimate,
        native.stats.allocation_bytes_estimate
    );
    assert_eq!(optimized.stats.collections, native.stats.collections);
    assert_eq!(
        optimized.stats.runtime_heap_attempts,
        native.stats.runtime_heap_attempts
    );
    assert_eq!(
        optimized.stats.runtime_heap_successes,
        native.stats.runtime_heap_successes
    );
    assert!(native.stats.allocations >= 7);
    assert!(native.stats.collections >= 7);
    assert!(native.stats.maximum_roots > 0);
    assert!(native.stats.runtime_heap_attempts >= 14);
    assert_eq!(
        native.stats.runtime_heap_attempts,
        native.stats.runtime_heap_successes
    );
    assert!(native.stats.barrier_count >= 4);
}

#[test]
fn forced_collection_sees_live_reference_in_recursive_caller_and_callee_frames() {
    let source = "def/\nname/\nwalk\n/name\nfn/\nsig/\nStr\nI64\n->\nI64\n/sig\nparams/\ntext\nStr\ndepth\nI64\n/params\nif/\nlte/\ndepth\n0\n/lte\nstr-len/\ntext\n/str-len\nwalk/\ntext\n-/\ndepth\n1\n/-\n/walk\n/if\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nwalk/\nempty-str/\n/empty-str\n4\n/walk\n/main\n";
    let program = compile(source, "recursive-roots.lkjscript");
    let vm = run_chunk(program.bytecode(), &ExecutionConfig::default());
    let mut config = JitConfig::default();
    config.force_gc_before_allocation = true;
    let native = execute_forced(program.ssa(), &ExecutionConfig::default(), config)
        .expect("recursive generated reference execution");
    let mut optimizing_config = JitConfig::default();
    optimizing_config.force_gc_before_allocation = true;
    let optimized = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        optimizing_config,
    )
    .expect("optimizing recursive generated reference execution");
    assert_eq!(execution(native.outcome), execution(vm.clone()));
    assert_eq!(execution(optimized.outcome), execution(vm));
    assert_eq!(native.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.baseline_native_entries, 0);
    assert!(native.stats.native_entries >= 6);
    assert!(optimized.stats.optimizing_native_entries >= 6);
    assert!(native.stats.peak_native_frame_depth >= 6);
    assert!(optimized.stats.peak_native_frame_depth >= 6);
    assert!(native.stats.collections >= 2);
    assert!(optimized.stats.collections >= 2);
    assert!(native.stats.maximum_roots >= 5);
    assert!(optimized.stats.maximum_roots >= 5);
    assert!(native
        .stats
        .code_objects
        .iter()
        .any(|object| !object.exact_scalar_stack_maps));
    assert!(optimized
        .stats
        .code_objects
        .iter()
        .any(|object| !object.exact_scalar_stack_maps));

    let mutual = "def/\nname/\neven\n/name\nfn/\nsig/\nStr\nI64\n->\nI64\n/sig\nparams/\ntext\nStr\ndepth\nI64\n/params\nif/\nlte/\ndepth\n0\n/lte\nstr-len/\ntext\n/str-len\nodd/\ntext\n-/\ndepth\n1\n/-\n/odd\n/if\n/fn\n/def\ndef/\nname/\nodd\n/name\nfn/\nsig/\nStr\nI64\n->\nI64\n/sig\nparams/\ntext\nStr\ndepth\nI64\n/params\nif/\nlte/\ndepth\n0\n/lte\nstr-len/\ntext\n/str-len\neven/\ntext\n-/\ndepth\n1\n/-\n/even\n/if\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\neven/\nempty-str/\n/empty-str\n5\n/even\n/main\n";
    let program = compile(mutual, "mutual-recursive-roots.lkjscript");
    let mut config = JitConfig::default();
    config.force_gc_before_allocation = true;
    let native = execute_forced(program.ssa(), &ExecutionConfig::default(), config)
        .expect("mutual recursive SCC with live reference executes natively");
    let mut optimizing_config = JitConfig::default();
    optimizing_config.force_gc_before_allocation = true;
    let optimized = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        optimizing_config,
    )
    .expect("mutual recursive SCC with live reference optimizes natively");
    assert!(
        matches!(native.outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(0))
    );
    assert!(
        matches!(optimized.outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(0))
    );
    assert_eq!(native.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.baseline_native_entries, 0);
    assert!(native.stats.peak_native_frame_depth >= 7);
    assert!(optimized.stats.peak_native_frame_depth >= 7);
    assert!(native.stats.maximum_roots >= 6);
    assert!(optimized.stats.maximum_roots >= 6);
}

#[test]
fn auto_group_reference_helper_remains_vm_entry_ineligible() {
    let source = "def/\nname/\ntext\n/name\nfn/\nsig/\n->\nStr\n/sig\nparams/\n/params\nempty-str/\n/empty-str\n/fn\n/def\ndef/\nname/\nsize\n/name\nfn/\nsig/\n->\nI64\n/sig\nparams/\n/params\nstr-len/\ntext/\n/text\n/str-len\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\ndo/\nsize/\n/size\ntext/\n/text\nsize/\n/size\n/do\n/main\n";
    let program = compile(source, "auto-reference-helper.lkjscript");
    let mut config = JitConfig::default();
    config.auto_threshold = 1;
    let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), config);
    let (outcome, stats) = run_chunk_auto(
        program.bytecode(),
        &[],
        &ExecutionConfig::default(),
        session,
    );
    assert!(matches!(outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(0)));
    let helper = stats
        .functions
        .iter()
        .find(|function| function.name() == "text")
        .expect("reference helper tier record");
    assert!(!helper.auto_entry_eligible());
    assert_ne!(helper.state(), TierState::BaselineNative);
    assert!(
        helper.native_entries() > 0,
        "native direct calls remain supported"
    );
    assert_eq!(stats.compile_failures, 0);
}

#[test]
fn native_poll_deadline_fuel_and_code_work_limits_are_bounded() {
    let program = compile(F64_LOOP, "limits.lkjscript");
    let mut deadline = ExecutionConfig::default();
    deadline.wall_time = Some(Duration::ZERO);
    let outcome = execute_forced(program.ssa(), &deadline, JitConfig::default())
        .expect("deadline is a language outcome");
    assert_eq!(execution(outcome.outcome), Scalar::Deadline);
    let optimized = execute_optimizing(program.ssa(), &deadline, JitConfig::default())
        .expect("optimizing deadline is a language outcome");
    assert_eq!(execution(optimized.outcome), Scalar::Deadline);
    assert_eq!(optimized.stats.baseline_native_entries, 0);

    let mut fuel = ExecutionConfig::default();
    fuel.instruction_fuel = 0;
    let outcome = execute_forced(program.ssa(), &fuel, JitConfig::default())
        .expect("fuel is a language outcome");
    assert_eq!(execution(outcome.outcome), Scalar::Fuel);
    let optimized = execute_optimizing(program.ssa(), &fuel, JitConfig::default())
        .expect("optimizing fuel is a language outcome");
    assert_eq!(execution(optimized.outcome), Scalar::Fuel);
    assert_eq!(optimized.stats.baseline_native_entries, 0);

    for maximum in [0, 1] {
        let mut stack = ExecutionConfig::default();
        stack.max_stack_values = maximum;
        let outcome = execute_forced(program.ssa(), &stack, JitConfig::default())
            .expect("native active-value limit is a language outcome");
        assert_eq!(
            outcome.outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::StackValues)
        );
        assert_eq!(outcome.stats.peak_native_frame_depth, 0);
        let optimized = execute_optimizing(program.ssa(), &stack, JitConfig::default())
            .expect("optimizing active-value limit is a language outcome");
        assert_eq!(
            optimized.outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::StackValues)
        );
        assert!(optimized.stats.optimizing_native_entries > 0);
        assert_eq!(optimized.stats.baseline_native_entries, 0);
    }

    let mut no_frames = ExecutionConfig::default();
    no_frames.max_frames = 0;
    let optimized = execute_optimizing(program.ssa(), &no_frames, JitConfig::default())
        .expect("optimizing frame limit is a language outcome");
    assert_eq!(
        optimized.outcome,
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::FrameDepth)
    );
    assert!(optimized.stats.optimizing_native_entries > 0);
    assert_eq!(optimized.stats.baseline_native_entries, 0);

    let allocation = compile(
        "main/\nsig/\n->\nStr\n/sig\nempty-str/\n/empty-str\n/main\n",
        "tiny-allocation-limit.lkjscript",
    );
    let mut no_allocations = ExecutionConfig::default();
    no_allocations.max_allocations = 0;
    let outcome = execute_forced(allocation.ssa(), &no_allocations, JitConfig::default())
        .expect("allocation limit is structured");
    assert!(matches!(
        outcome.outcome,
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations)
    ));
    let mut tiny_heap = ExecutionConfig::default();
    tiny_heap.max_heap_bytes = 1;
    let outcome = execute_forced(allocation.ssa(), &tiny_heap, JitConfig::default())
        .expect("heap limit is structured");
    assert!(matches!(
        outcome.outcome,
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::HeapBytes)
    ));

    let mut limited = JitConfig::default();
    limited.backend_limits =
        lkjscript_native::BackendLimits::new(64, 1024, 16_384, 1024, 1, 4 * 1024 * 1024, 100_000);
    let error = execute_forced(program.ssa(), &ExecutionConfig::default(), limited)
        .expect_err("code byte limit must fail forced engine");
    assert_eq!(error.code(), FailureCode::BackendVerification);
}

#[test]
fn auto_compiles_for_later_calls_and_suppresses_unsupported_retry() {
    let program = compile(F64_LOOP, "auto.lkjscript");
    let mut config = JitConfig::default();
    config.auto_threshold = 2;
    let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), config);
    let (outcome, stats) = run_chunk_auto(
        program.bytecode(),
        &[],
        &ExecutionConfig::default(),
        session,
    );
    assert_eq!(
        execution(outcome),
        execution(run_chunk(program.bytecode(), &ExecutionConfig::default()))
    );
    let step = stats
        .functions
        .iter()
        .find(|function| function.name() == "step")
        .expect("step tier record");
    assert_eq!(step.state(), TierState::BaselineNative);
    assert_eq!(step.attempts(), 1);
    assert!(step.native_entries() > 0);
    assert!(stats.vm_fallbacks >= 2);

    let allocation = "def/\nname/\nallocate\n/name\nfn/\nsig/\n->\nStr\n/sig\nparams/\n/params\nempty-str/\n/empty-str\n/fn\n/def\nmain/\nsig/\n->\nStr\n/sig\ndo/\nallocate/\n/allocate\nallocate/\n/allocate\nallocate/\n/allocate\n/do\n/main\n";
    let program = compile(allocation, "auto-unsupported.lkjscript");
    let mut config = JitConfig::default();
    config.auto_threshold = 1;
    let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), config);
    let (outcome, stats) = run_chunk_auto(
        program.bytecode(),
        &[],
        &ExecutionConfig::default(),
        session,
    );
    assert!(matches!(outcome, ExecutionOutcome::Returned(value) if value.as_str() == Some("")));
    let allocate = stats
        .functions
        .iter()
        .find(|function| function.name() == "allocate")
        .expect("allocation tier record");
    assert_eq!(allocate.state(), TierState::VmOnly);
    assert!(!allocate.auto_entry_eligible());
    assert_eq!(allocate.attempts(), 0);
    assert_eq!(allocate.native_entries(), 0);
    assert_eq!(stats.compile_failures, 0);
}
