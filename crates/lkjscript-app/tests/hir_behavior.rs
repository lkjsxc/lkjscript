#![allow(clippy::expect_used)]

use lkjscript_compiler::compile_source;
use lkjscript_core::{Limits, Op, Value};
use lkjscript_vm::{run_chunk, run_chunk_with_args};

fn expression_chunk(source: &str) -> lkjscript_core::Result<lkjscript_core::Chunk> {
    let mut chunk = compile_source(source, "hir-behavior.lkjscript", &Limits::default())?;
    assert!(
        chunk
            .main
            .code
            .ends_with(&[Op::Unit as u8, Op::Return as u8]),
        "compiler epilogue changed"
    );
    let expression_end = chunk.main.code.len() - 2;
    chunk.main.code.truncate(expression_end);
    chunk.main.emit(Op::Return);
    Ok(chunk)
}

fn evaluate_program(source: &str) -> lkjscript_core::Result<Value> {
    run_chunk(&expression_chunk(source)?)
}

fn evaluate_program_with_args(source: &str, args: &[String]) -> lkjscript_core::Result<Value> {
    run_chunk_with_args(&expression_chunk(source)?, args)
}

#[test]
fn resolved_locals_and_short_circuit_operations_preserve_runtime_behavior() {
    let shadowing = "def/\nname/\nshadow\n/name\nfn/\nsig/\nI64\n->\nI64\n/sig\nparams/\nx\nI64\n/params\nlet/\nbind/\nx\n2\n/bind\nlet/\nbind/\nx\n3\n/bind\nx\n/let\n/let\n/fn\n/def\ndo/\nshadow/\n9\n/shadow\n/do\n";
    assert_eq!(
        evaluate_program(shadowing)
            .expect("evaluate shadowing")
            .as_small_i64(),
        Some(3)
    );

    let short_circuit = "def/\nname/\ntouched\n/name\ntype/\nI64\n/type\n0\n/def\ndef/\nname/\ntouch\n/name\nfn/\nsig/\n->\nBool\n/sig\nparams/\n/params\ndo/\nset/\ntouched\n1\n/set\ntrue\n/do\n/fn\n/def\ndo/\nand/\nfalse\ntouch/\n/touch\n/and\ntouched\n/do\n";
    assert_eq!(
        evaluate_program(short_circuit)
            .expect("evaluate short circuit")
            .as_small_i64(),
        Some(0)
    );

    let set_result =
        "def/\nname/\nvalue\n/name\ntype/\nI64\n/type\n0\n/def\ndo/\nset/\nvalue\n2\n/set\n/do\n";
    let set_result = evaluate_program(set_result).expect("evaluate set result");
    assert!(set_result.is_unit());
    assert!(!set_result.is_invalid());
}

#[test]
fn explicit_option_and_argument_absence_survive_source_to_vm_lowering() {
    let none = "do/\nnone/\nStr\n/none\n/do\n";
    let value = evaluate_program(none).expect("evaluate none");
    assert!(value.is_none());
    assert!(!value.is_invalid());

    let some = "do/\nunwrap-some/\nsome/\n42\n/some\n/unwrap-some\n/do\n";
    assert_eq!(
        evaluate_program(some)
            .expect("evaluate some")
            .as_small_i64(),
        Some(42)
    );

    let missing = "do/\narg/\n0\n/arg\n/do\n";
    assert!(evaluate_program_with_args(missing, &[])
        .expect("evaluate missing argument")
        .is_none());
    let negative = "do/\narg/\n-1\n/arg\n/do\n";
    assert!(evaluate_program_with_args(negative, &["ignored".into()])
        .expect("evaluate negative argument")
        .is_none());

    let present = "do/\nstr-len/\nunwrap-some/\narg/\n0\n/arg\n/unwrap-some\n/str-len\n/do\n";
    assert_eq!(
        evaluate_program_with_args(present, &["hello".into()])
            .expect("evaluate present argument")
            .as_small_i64(),
        Some(5)
    );

    let unwrap_none = "do/\nunwrap-some/\nnone/\nI64\n/none\n/unwrap-some\n/do\n";
    let error = evaluate_program(unwrap_none)
        .expect_err("unwrap-some on none must trap")
        .to_string();
    assert!(error.contains("unwrap-some on none"));
}

#[test]
fn typed_empty_lists_survive_source_to_vm_lowering() {
    let empty = "do/\nempty-list/\nI64\n/empty-list\n/do\n";
    let value = evaluate_program(empty).expect("evaluate empty list");
    assert!(value.is_empty_list());
    assert!(!value.is_invalid());
    assert!(!value.is_unit());

    let predicate = "do/\nempty-list?/\nempty-list/\nI64\n/empty-list\n/empty-list?\n/do\n";
    assert_eq!(
        evaluate_program(predicate)
            .expect("evaluate empty-list predicate")
            .as_bool(),
        Some(true)
    );

    let tail = "do/\ncdr/\ncons/\n7\nempty-list/\nI64\n/empty-list\n/cons\n/cdr\n/do\n";
    assert!(evaluate_program(tail)
        .expect("evaluate final list tail")
        .is_empty_list());

    let invalid = "do/\ncar/\nempty-list/\nI64\n/empty-list\n/car\n/do\n";
    assert!(evaluate_program(invalid).is_err());
}

#[test]
fn unit_and_exact_three_arm_if_survive_source_to_vm_lowering() {
    let conditional = "do/\nif/\ntrue\nunit\nunit\n/if\n/do\n";
    let value = evaluate_program(conditional).expect("evaluate Unit conditional");
    assert!(value.is_unit());
    assert!(!value.is_invalid());

    let loop_result = "do/\nwhile/\nfalse\n/while\n/do\n";
    assert!(evaluate_program(loop_result)
        .expect("evaluate Unit while")
        .is_unit());
}
