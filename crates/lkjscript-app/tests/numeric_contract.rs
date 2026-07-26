#![allow(clippy::expect_used, clippy::panic)]

#[path = "numeric_contract/boundaries.rs"]
mod boundaries;
#[path = "numeric_contract/malformed.rs"]
mod malformed;
#[path = "numeric_contract/support.rs"]
mod support;

use lkjscript_compiler::compile_source;
use lkjscript_core::{Error, ExecutionConfig, ExecutionOutcome, Limits, OwnedValue};
use lkjscript_vm::run_chunk;

fn evaluate_typed(expression: &str, return_type: &str) -> lkjscript_core::Result<OwnedValue> {
    let source = format!("main/\nsig/\n->\n{return_type}\n/sig\n{expression}\n/main\n");
    let program = compile_source(&source, "numeric-contract.lkjscript", &Limits::default())?;
    match run_chunk(program.bytecode(), &ExecutionConfig::default()) {
        ExecutionOutcome::Returned(value) => Ok(value),
        ExecutionOutcome::Trapped(trap) => Err(Error::msg(trap.to_string())),
        other => Err(Error::msg(other.summary())),
    }
}

fn evaluate(expression: &str) -> lkjscript_core::Result<OwnedValue> {
    evaluate_typed(expression, "Bool")
}

fn assert_true(expression: &str) {
    assert_eq!(
        evaluate(expression).expect("evaluate expression").as_bool(),
        Some(true)
    );
}

#[test]
fn compiled_i64_arithmetic_is_exact_across_f64_and_immediate_boundaries() {
    assert_true("equal-value/\n+/\n9007199254740993\n2\n/+\n9007199254740995\n/equal-value");
    assert_true("equal-value/\n+/\n1152921504606846975\n1\n/+\n1152921504606846976\n/equal-value");
    assert_true("equal-value/\ndiv/\n-7\n2\n/div\n-3\n/equal-value");
    assert_true(
        "equal-value/\nbit-xor/\n-9223372036854775808\n-1\n/bit-xor\n9223372036854775807\n/equal-value",
    );
}

#[test]
fn compiled_numeric_failures_and_ieee_equality_are_truthful() {
    assert!(evaluate_typed("+/\n9223372036854775807\n1\n/+", "I64").is_err());
    assert!(evaluate_typed("div/\n1\n0\n/div", "I64").is_err());
    assert_eq!(
        evaluate("equal-value/\n1.0\n1.0000000000005\n/equal-value")
            .expect("IEEE equality")
            .as_bool(),
        Some(false)
    );
    assert_true(
        "equal-value/\n+/\n2.0\nf64-from-i64-rounded/\n1\n/f64-from-i64-rounded\n/+\n3.0\n/equal-value",
    );
}
