#![allow(clippy::expect_used, clippy::field_reassign_with_default)]

use std::time::Duration;

use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Limits, OwnedValue, ResourceLimitKind};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_jit::{execute_forced, FailureCode, JitConfig, JitSession, TierState};
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
    assert!(native.stats.poll_v1_calls > 0);
    assert!(native.stats.direct_native_calls > 0);
    assert_eq!(native.stats.vm_fallbacks, 0);
    assert_eq!(native.stats.functions.len(), 2);
    assert!(native
        .stats
        .functions
        .iter()
        .all(|function| function.native_entries() > 0));
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
    }

    let exit = "main/\nsig/\n->\nUnit\n/sig\nexit/\n17\n/exit\n/main\n";
    assert_eq!(
        execution(forced(exit, "exit.lkjscript").outcome),
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
fn forced_mode_rejects_references_allocation_products_host_io_and_recursion() {
    let unsupported = [
        (
            "allocation.lkjscript",
            "main/\nsig/\n->\nStr\n/sig\nempty-str/\n/empty-str\n/main\n",
        ),
        (
            "host.lkjscript",
            "main/\nsig/\n->\nUnit\n/sig\nflush/\n/flush\n/main\n",
        ),
        (
            "product.lkjscript",
            "product/\nname/\nBoxed\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\nI64\n/type\n/field\n/fields\n/product\nmain/\nsig/\n->\nProduct\nBoxed\n/sig\nproduct-value/\nBoxed\nfield/\nvalue\n1\n/field\n/product-value\n/main\n",
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
    }

    let recursion = "def/\nname/\nrecur\n/name\nfn/\nsig/\nI64\n->\nI64\n/sig\nparams/\nn\nI64\n/params\nif/\nlte/\nn\n0\n/lte\n0\nrecur/\n-/\nn\n1\n/-\n/recur\n/if\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nrecur/\n3\n/recur\n/main\n";
    let program = compile(recursion, "recursion.lkjscript");
    let error = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect_err("recursion is an explicit MVP boundary");
    assert_eq!(error.code(), FailureCode::RecursionUnsupported);
}

#[test]
fn native_poll_deadline_fuel_and_code_work_limits_are_bounded() {
    let program = compile(F64_LOOP, "limits.lkjscript");
    let mut deadline = ExecutionConfig::default();
    deadline.wall_time = Some(Duration::ZERO);
    let outcome = execute_forced(program.ssa(), &deadline, JitConfig::default())
        .expect("deadline is a language outcome");
    assert_eq!(execution(outcome.outcome), Scalar::Deadline);

    let mut fuel = ExecutionConfig::default();
    fuel.instruction_fuel = 0;
    let outcome = execute_forced(program.ssa(), &fuel, JitConfig::default())
        .expect("fuel is a language outcome");
    assert_eq!(execution(outcome.outcome), Scalar::Fuel);

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
    assert_eq!(allocate.state(), TierState::Disabled);
    assert_eq!(allocate.attempts(), 1);
    assert_eq!(allocate.native_entries(), 0);
    assert_eq!(stats.compile_failures, 1);
}
