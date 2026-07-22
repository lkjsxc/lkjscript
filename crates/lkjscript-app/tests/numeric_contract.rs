#![allow(clippy::expect_used)]

use lkjscript_compiler::compile_source;
use lkjscript_core::{Limits, Op, Value};
use lkjscript_vm::run_chunk;

fn evaluate(expression: &str) -> lkjscript_core::Result<Value> {
    let source = format!("do/\n{expression}\n/do\n");
    let mut chunk = compile_source(&source, "numeric-contract.lkjscript", &Limits::default())?;
    let expression_end = chunk.main.code.len().saturating_sub(2);
    chunk.main.code.truncate(expression_end);
    chunk.main.emit(Op::Return);
    run_chunk(&chunk)
}

fn assert_true(expression: &str) {
    assert_eq!(
        evaluate(expression).expect("evaluate expression").as_bool(),
        Some(true)
    );
}

#[test]
fn compiled_i64_arithmetic_is_exact_across_f64_and_immediate_boundaries() {
    assert_true("eq/\n+/\n9007199254740993\n2\n/+\n9007199254740995\n/eq");
    assert_true("eq/\n+/\n1152921504606846975\n1\n/+\n1152921504606846976\n/eq");
    assert_true("eq/\ndiv/\n-7\n2\n/div\n-3\n/eq");
    assert_true("eq/\nbit-xor/\n-9223372036854775808\n-1\n/bit-xor\n9223372036854775807\n/eq");
}

#[test]
fn compiled_numeric_failures_and_ieee_equality_are_truthful() {
    assert!(evaluate("+/\n9223372036854775807\n1\n/+").is_err());
    assert!(evaluate("div/\n1\n0\n/div").is_err());
    assert_eq!(
        evaluate("eq/\n1.0\n1.0000000000005\n/eq")
            .expect("IEEE equality")
            .as_bool(),
        Some(false)
    );
    assert_true("eq/\n+/\n2.0\n1\n/+\n3.0\n/eq");
}
