#![allow(clippy::expect_used, clippy::panic)]

use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Limits, Op};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_jit::{execute_forced, FailureCode, JitConfig, JitSession};
use lkjscript_vm::{run_chunk, run_chunk_auto, ExecutionInputs};

const WIDE_COUNT: usize = 300;
const STRESS_COUNT: usize = 1_024;

fn wide_scalar_source(count: usize) -> String {
    let mut lines = vec![
        "def".to_string() + "/",
        "name/".into(),
        "select-high".into(),
        "/name".into(),
        "fn/".into(),
        "sig/".into(),
        "inputs/".into(),
    ];
    lines.extend((0..count).map(|_| "i64".to_string()));
    lines.extend([
        "/inputs".into(),
        "output/".into(),
        "i64".into(),
        "/output".into(),
        "/sig".into(),
        "params/".into(),
    ]);
    for index in 0..count {
        lines.push(format!("p{index}"));
        lines.push("i64".into());
    }
    lines.extend([
        "/params".into(),
        format!("p{}", count - 1),
        "/fn".into(),
        "/def".into(),
        "main/".into(),
        "sig/".into(),
        "inputs/".into(),
        "/inputs".into(),
        "output/".into(),
        "i64".into(),
        "/output".into(),
        "/sig".into(),
        "let/".into(),
    ]);
    for index in 0..count {
        lines.extend([
            "bind/".into(),
            format!("x{index}"),
            index.to_string(),
            "/bind".into(),
        ]);
    }
    lines.push("select-high/".into());
    lines.extend((0..count).map(|index| format!("x{index}")));
    lines.extend([
        "/select-high".into(),
        "/let".into(),
        "/main".into(),
        String::new(),
    ]);
    lines.join("\n")
}

fn wide_owned_parameter_source(count: usize) -> String {
    let mut lines = vec![
        "def/".into(),
        "name/".into(),
        "owned-high".into(),
        "/name".into(),
        "fn/".into(),
        "sig/".into(),
        "inputs/".into(),
    ];
    lines.extend((0..count - 1).map(|_| "i64".to_string()));
    lines.push("byte-vector".into());
    lines.extend([
        "/inputs".into(),
        "output/".into(),
        "i64".into(),
        "/output".into(),
        "/sig".into(),
        "params/".into(),
    ]);
    for index in 0..count - 1 {
        lines.push(format!("p{index}"));
        lines.push("i64".into());
    }
    lines.extend([
        format!("p{}", count - 1),
        "byte-vector".into(),
        "/params".into(),
        "byte-slice-length/".into(),
        "borrow/".into(),
        format!("p{}", count - 1),
        "/borrow".into(),
        "/byte-slice-length".into(),
        "/fn".into(),
        "/def".into(),
        "main/".into(),
        "sig/".into(),
        "inputs/".into(),
        "/inputs".into(),
        "output/".into(),
        "i64".into(),
        "/output".into(),
        "/sig".into(),
        "let/".into(),
        "bind/".into(),
        "bytes".into(),
        "new-byte-vector/".into(),
        "7".into(),
        "/new-byte-vector".into(),
        "/bind".into(),
        "owned-high/".into(),
    ]);
    lines.extend((0..count - 1).map(|index| index.to_string()));
    lines.extend([
        "move/".into(),
        "bytes".into(),
        "/move".into(),
        "/owned-high".into(),
        "/let".into(),
        "/main".into(),
        String::new(),
    ]);
    lines.join("\n")
}

fn compile_wide() -> lkjscript_compiler::ExecutableProgram {
    compile_source(
        &wide_scalar_source(WIDE_COUNT),
        "generated-wide-executable.lkjscript",
        &Limits::default(),
    )
    .expect("compile generated wide executable through HIR, memory plan, SSA, and bytecode")
}

fn returned_i64(outcome: ExecutionOutcome) -> i64 {
    match outcome {
        ExecutionOutcome::Returned(value) => value.as_i64().expect("returned I64"),
        other => panic!("wide executable did not return: {other:?}"),
    }
}

#[test]
fn three_hundred_parameters_arguments_and_live_lexical_locals_execute_in_vm() {
    let program = compile_wide();
    let function = program
        .bytecode()
        .protos()
        .iter()
        .find(|proto| proto.arity == WIDE_COUNT)
        .expect("wide function prototype");
    assert_eq!(function.arity, WIDE_COUNT);
    assert!(function.locals >= WIDE_COUNT);
    assert!(program.bytecode().main().locals > usize::from(u8::MAX));

    let call = program
        .bytecode()
        .main_instructions()
        .iter()
        .find(|instruction| instruction.op() == Op::Call)
        .expect("wide call instruction");
    assert_eq!(call.operand().index(), Some(WIDE_COUNT));
    assert!(program
        .bytecode()
        .main_instructions()
        .iter()
        .any(|instruction| {
            matches!(instruction.op(), Op::LoadLocal | Op::TakeUniqueLocal)
                && instruction
                    .operand()
                    .index()
                    .is_some_and(|slot| slot > usize::from(u8::MAX))
        }));

    let expected = i64::try_from(WIDE_COUNT - 1).expect("test width fits i64");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(expected))
    );
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    assert_eq!(returned_i64(outcome), expected);
}

#[test]
fn one_thousand_parameters_arguments_and_live_locals_execute_in_vm() {
    let source = wide_scalar_source(STRESS_COUNT);
    let program = compile_source(
        &source,
        "wide-executable-stress.lkjscript",
        &Limits::default(),
    )
    .expect("compile stress-width scalar source");
    let function = &program.bytecode().protos()[0];
    assert_eq!(function.arity, STRESS_COUNT);
    assert!(function.locals > usize::from(u8::MAX));
    assert!(program
        .bytecode()
        .proto_instructions(0)
        .expect("decoded stress function")
        .iter()
        .any(|instruction| {
            instruction.op() == Op::LoadLocal
                && instruction.operand().index() == Some(STRESS_COUNT - 1)
        }));
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    assert_eq!(
        returned_i64(outcome),
        i64::try_from(STRESS_COUNT - 1).expect("test width fits i64")
    );
}

#[test]
fn owned_parameter_above_byte_width_executes_and_cleans_up() {
    let program = compile_source(
        &wide_owned_parameter_source(WIDE_COUNT),
        "generated-wide-owned-parameter.lkjscript",
        &Limits::default(),
    )
    .expect("compile wide owned parameter through the production pipeline");
    let function = program
        .bytecode()
        .protos()
        .iter()
        .find(|proto| proto.arity == WIDE_COUNT)
        .expect("wide owned function prototype");
    assert_eq!(function.parameter_unique_places[WIDE_COUNT - 1], Some(0));
    assert_eq!(function.unique_places, 1);
    assert!(function.failure_cleanups.iter().any(|plan| {
        plan.actions.iter().any(|action| match action {
            lkjscript_core::FailureCleanupAction::EndBorrow { local, .. }
            | lkjscript_core::FailureCleanupAction::DropUnique { local, .. } => {
                *local > usize::from(u8::MAX)
            }
            _ => false,
        })
    }));
    let outcome = run_chunk(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    assert_eq!(returned_i64(outcome), 7);
}

#[test]
fn automatic_engine_keeps_high_signature_on_the_generic_vm_path() {
    let program = compile_wide();
    let config = JitConfig {
        auto_threshold: 1,
        ..JitConfig::default()
    };
    let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), config);
    let (outcome, stats) = run_chunk_auto(
        program.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionConfig::default(),
        session,
    );
    assert_eq!(
        returned_i64(outcome),
        i64::try_from(WIDE_COUNT - 1).expect("test width fits i64")
    );
    let function = stats
        .functions
        .iter()
        .find(|function| function.name() == "select-high")
        .expect("wide function tier record");
    assert!(!function.auto_entry_eligible());
    assert_eq!(function.native_entries(), 0);

    let error = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect_err("forced native mode reports the unsupported high signature");
    assert_eq!(error.code(), FailureCode::UnsupportedSignature);
}
