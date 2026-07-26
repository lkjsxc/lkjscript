use super::*;
use lkjscript_ir::{evaluate as evaluate_ssa, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig};

pub(super) fn edition2(return_type: &str, expression: &str) -> String {
    format!("edition/\n2\n/edition\nmain/\nsig/\n->\n{return_type}\n/sig\n{expression}\n/main\n")
}

pub(super) fn assert_scalar(source: &str, expected: Expected) {
    let program = compile_source(source, "conversion.lkjscript", &Limits::default())
        .expect("compile conversion");
    assert_eval(
        evaluate_ssa(program.ssa(), &EvalConfig::default()),
        expected,
    );
    assert_owned(
        run_chunk(program.bytecode(), &ExecutionConfig::default()),
        expected,
    );
    for execution in [
        execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("forced baseline conversion"),
        execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("forced proof conversion"),
    ] {
        assert_owned(execution.outcome, expected);
        assert!(execution.stats.native_entries > 0);
        assert_eq!(execution.stats.vm_fallbacks, 0);
        assert!(execution.stats.runtime_heap_attempts > 0);
        assert!(execution.stats.code_objects.iter().any(|object| {
            let sites = object.numeric_conversion_sites;
            let exact_sites = sites.f64_from_i64_exact
                + sites.f64_from_i64_rounded
                + sites.i64_from_f64_exact
                + sites.i64_from_f64_trunc;
            exact_sites == 1
                && object
                    .runtime_calls
                    .contains(&lkjscript_native::RuntimeCallSlot::HeapDispatchV1)
        }));
    }
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
        Expected::F64(expected) => assert_eq!(actual.as_f64().map(f64::to_bits), Some(expected)),
    }
}
