use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Limits, OwnedValue};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_vm::run_chunk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarOutcome {
    Unit,
    Bool(bool),
    I64(i64),
    F64(u64),
    Str(String),
    Path(Vec<u8>),
    Exited(i64),
    Trapped,
    Other(String),
}

pub fn evaluator_outcome(outcome: EvalOutcome) -> ScalarOutcome {
    match outcome {
        EvalOutcome::Returned(EvalValue::Unit) => ScalarOutcome::Unit,
        EvalOutcome::Returned(EvalValue::Bool(value)) => ScalarOutcome::Bool(value),
        EvalOutcome::Returned(EvalValue::I64(value)) => ScalarOutcome::I64(value),
        EvalOutcome::Returned(EvalValue::F64(value)) => ScalarOutcome::F64(value.to_bits()),
        EvalOutcome::Returned(EvalValue::Str(value)) => ScalarOutcome::Str(value),
        EvalOutcome::Returned(EvalValue::ReturnedOwned(value)) => vm_value(&value),
        EvalOutcome::Exited(code) => ScalarOutcome::Exited(code),
        EvalOutcome::Trapped(_) => ScalarOutcome::Trapped,
        other => ScalarOutcome::Other(format!("{other:?}")),
    }
}

fn vm_value(value: &OwnedValue) -> ScalarOutcome {
    if value.is_unit() {
        ScalarOutcome::Unit
    } else if let Some(value) = value.as_bool() {
        ScalarOutcome::Bool(value)
    } else if let Some(value) = value.as_i64() {
        ScalarOutcome::I64(value)
    } else if let Some(value) = value.as_f64() {
        ScalarOutcome::F64(value.to_bits())
    } else if let Some(value) = value.as_str() {
        ScalarOutcome::Str(value.to_owned())
    } else if let Some(value) = value.as_path_bytes() {
        ScalarOutcome::Path(value.to_vec())
    } else {
        ScalarOutcome::Other(format!("{value:?}"))
    }
}

pub fn vm_outcome(outcome: ExecutionOutcome) -> ScalarOutcome {
    match outcome {
        ExecutionOutcome::Returned(value) => vm_value(&value),
        ExecutionOutcome::Exited(code) => ScalarOutcome::Exited(i64::from(code)),
        ExecutionOutcome::Trapped(_) => ScalarOutcome::Trapped,
        other => ScalarOutcome::Other(other.summary()),
    }
}

pub fn compare_source(source: &str, name: &str) -> ScalarOutcome {
    let marked;
    let source = if source.starts_with("") {
        source
    } else {
        marked = source.to_string();
        &marked
    };
    let program = compile_source(source, name, &Limits::default()).expect("compile SSA fixture");
    let evaluated = evaluator_outcome(evaluate(program.ssa(), &EvalConfig::default()));
    let executed = vm_outcome(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ));
    assert_eq!(evaluated, executed, "SSA/VM mismatch for {name}");
    assert_eq!(
        program.bytecode_links().functions.len(),
        program.ssa().program().functions.len()
    );
    evaluated
}

pub fn main_source(return_type: &str, expression: &str) -> String {
    format!("main/\nsig/\ninputs/\n/inputs\noutput/\n{return_type}\n/output\n/sig\n{expression}\n/main\n")
}
