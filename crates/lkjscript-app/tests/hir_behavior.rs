#![allow(clippy::expect_used)]

use lkjscript_compiler::compile_source;
use lkjscript_core::{Limits, Op, Value};
use lkjscript_vm::run_chunk;

fn evaluate_program(source: &str) -> lkjscript_core::Result<Value> {
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
    run_chunk(&chunk)
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
    assert!(!set_result.is_nil());
}

#[test]
fn typed_empty_lists_survive_source_to_vm_lowering() {
    let empty = "do/\nempty-list/\nI64\n/empty-list\n/do\n";
    let value = evaluate_program(empty).expect("evaluate empty list");
    assert!(value.is_empty_list());
    assert!(!value.is_nil());
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
    assert!(!value.is_nil());

    let loop_result = "do/\nwhile/\nfalse\n/while\n/do\n";
    assert!(evaluate_program(loop_result)
        .expect("evaluate Unit while")
        .is_unit());
}
