use super::*;
use lkjscript_ir::{evaluate as evaluate_ssa, EvalConfig, EvalOutcome, EvalValue};

pub(super) fn program(return_type: &str, expression: &str) -> String {
    format!("main/\nsig/\ninputs/\n/inputs\noutput/\n{return_type}\n/output\n/sig\n{expression}\n/main\n")
}

pub(super) fn assert_scalar(source: &str, expected: Expected) {
    let program = compile_source(source, "conversion.lkjscript").expect("compile conversion");
    assert_eval(
        evaluate_ssa(program.ssa(), &EvalConfig::default()),
        expected,
    );
    assert_owned(
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
        ),
        expected,
    );
}

pub(super) fn assert_allocation_free_scalar(source: &str, expected: Expected) {
    let program = compile_source(source, "inline-scalar.lkjscript").expect("compile inline scalar");
    let eval_config = EvalConfig {
        max_allocations: 0,
        ..EvalConfig::default()
    };
    assert_eval(evaluate_ssa(program.ssa(), &eval_config), expected);
    let execution = ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy {
        max_allocations: 0,
        ..lkjscript_core::LimitedExecutionPolicy::conservative()
    });
    assert_owned(
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &execution,
        ),
        expected,
    );
}

#[derive(Clone, Copy)]
pub(super) enum Expected {
    Bool(bool),
    I64(i64),
    F64(u64),
}

fn assert_eval(outcome: EvalOutcome, expected: Expected) {
    match (outcome, expected) {
        (EvalOutcome::Returned(EvalValue::Bool(actual)), Expected::Bool(expected)) => {
            assert_eq!(actual, expected)
        }
        (EvalOutcome::Returned(EvalValue::I64(actual)), Expected::I64(expected)) => {
            assert_eq!(actual, expected)
        }
        (EvalOutcome::Returned(EvalValue::F64(actual)), Expected::F64(expected)) => {
            assert_eq!(actual.to_bits(), expected)
        }
        (actual, _) => panic!("unexpected evaluator result {actual:?}"),
    }
}

fn assert_owned(outcome: ExecutionOutcome, expected: Expected) {
    let ExecutionOutcome::Returned(actual) = outcome else {
        panic!("engine did not return")
    };
    match expected {
        Expected::Bool(expected) => assert_eq!(actual.as_bool(), Some(expected)),
        Expected::I64(expected) => assert_eq!(actual.as_i64(), Some(expected)),
        Expected::F64(expected) => assert_eq!(actual.as_f64_bits(), Some(expected)),
    }
    assert_eq!(actual.snapshot_object_count(), 0);
}
