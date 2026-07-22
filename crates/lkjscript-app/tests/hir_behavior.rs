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

fn assert_program_bool(source: &str, expected: bool) {
    assert_eq!(
        evaluate_program(source)
            .expect("evaluate Bool program")
            .as_bool(),
        Some(expected)
    );
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
fn explicit_equality_families_survive_source_to_vm_lowering() {
    assert_program_bool(
        "do/\nequal-value/\n9223372036854775807\n9223372036854775807\n/equal-value\n/do\n",
        true,
    );
    assert_program_bool(
        "do/\nequal-value/\nstr/\nsame\n/str\nstr/\nsame\n/str\n/equal-value\n/do\n",
        true,
    );
    assert_program_bool(
        "do/\nequal-value/\nstr/\nsym:same\n/str\nstr-append/\nstr/\nsym:\n/str\nstr/\nsame\n/str\n/str-append\n/equal-value\n/do\n",
        true,
    );
    assert_program_bool(
        "do/\nequal-value/\nquote/\nsame\n/quote\nquote/\nsame\n/quote\n/equal-value\n/do\n",
        true,
    );
    assert_program_bool(
        "do/\nequal-value/\nsome/\n42\n/some\nsome/\n42\n/some\n/equal-value\n/do\n",
        true,
    );
    assert_program_bool(
        "do/\nequal-value/\nnone/\nI64\n/none\nsome/\n42\n/some\n/equal-value\n/do\n",
        false,
    );
    assert_program_bool(
        "do/\nlet/\nbind/\nbuffer\nbuf-new/\n1\n/buf-new\n/bind\nsame-object/\nbuffer\nbuffer\n/same-object\n/let\n/do\n",
        true,
    );
    assert_program_bool(
        "do/\nlist-equal/\ncons/\n1\nempty-list/\nI64\n/empty-list\n/cons\ncons/\n1\nempty-list/\nI64\n/empty-list\n/cons\n/list-equal\n/do\n",
        true,
    );
    assert_program_bool(
        "do/\nf64-bits-equal/\n0.0\n-0.0\n/f64-bits-equal\n/do\n",
        false,
    );
}

#[test]
fn immutable_nominal_products_preserve_original_values_end_to_end() {
    let source = "product/\nname/\nPoint\n/name\nfields/\nfield/\nname/\nx\n/name\ntype/\nI64\n/type\n/field\nfield/\nname/\ny\n/name\ntype/\nI64\n/type\n/field\n/fields\n/product\ndo/\nlet/\nbind/\noriginal\nproduct-value/\nPoint\nfield/\nx\n1\n/field\nfield/\ny\n2\n/field\n/product-value\n/bind\nbind/\nupdated\nwith-field/\noriginal\nx\n7\n/with-field\n/bind\nand/\nequal-value/\nfield/\noriginal\nx\n/field\n1\n/equal-value\nequal-value/\nfield/\nupdated\nx\n/field\n7\n/equal-value\n/and\n/let\n/do\n";
    assert_program_bool(source, true);

    let zero_field = "product/\nname/\nMarker\n/name\nfields/\n/fields\n/product\ndo/\nproduct-value/\nMarker\n/product-value\n/do\n";
    let value = evaluate_program(zero_field).expect("evaluate zero-field product");
    assert!(value.as_heap().is_some());

    let mut wide = String::from("product/\nname/\nWide\n/name\nfields/\n");
    for index in 0..15 {
        wide.push_str(&format!(
            "field/\nname/\nf{index}\n/name\ntype/\nI64\n/type\n/field\n"
        ));
    }
    wide.push_str("/fields\n/product\ndo/\nequal-value/\nfield/\nproduct-value/\nWide\n");
    for index in 0..15 {
        wide.push_str(&format!("field/\nf{index}\n{index}\n/field\n"));
    }
    wide.push_str("/product-value\nf14\n/field\n14\n/equal-value\n/do\n");
    assert_program_bool(&wide, true);

    let constructor_order = "def/\nname/\ncounter\n/name\ntype/\nI64\n/type\n0\n/def\ndef/\nname/\nnext\n/name\nfn/\nsig/\n->\nI64\n/sig\nparams/\n/params\ndo/\nset/\ncounter\n+/\ncounter\n1\n/+\n/set\ncounter\n/do\n/fn\n/def\nproduct/\nname/\nPair\n/name\nfields/\nfield/\nname/\nfirst\n/name\ntype/\nI64\n/type\n/field\nfield/\nname/\nsecond\n/name\ntype/\nI64\n/type\n/field\n/fields\n/product\ndo/\nlet/\nbind/\npair\nproduct-value/\nPair\nfield/\nfirst\nnext/\n/next\n/field\nfield/\nsecond\nnext/\n/next\n/field\n/product-value\n/bind\nand/\nequal-value/\nfield/\npair\nfirst\n/field\n1\n/equal-value\nequal-value/\nfield/\npair\nsecond\n/field\n2\n/equal-value\n/and\n/let\n/do\n";
    assert_program_bool(constructor_order, true);

    let replacement_order = "def/\nname/\ncounter\n/name\ntype/\nI64\n/type\n0\n/def\nproduct/\nname/\nBoxed\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\nI64\n/type\n/field\n/fields\n/product\ndef/\nname/\nmake\n/name\nfn/\nsig/\n->\nProduct\nBoxed\n/sig\nparams/\n/params\ndo/\nset/\ncounter\n+/\ncounter\n1\n/+\n/set\nproduct-value/\nBoxed\nfield/\nvalue\ncounter\n/field\n/product-value\n/do\n/fn\n/def\ndef/\nname/\nreplacement\n/name\nfn/\nsig/\n->\nI64\n/sig\nparams/\n/params\ndo/\nset/\ncounter\n+/\ncounter\n1\n/+\n/set\ncounter\n/do\n/fn\n/def\ndo/\nequal-value/\nfield/\nwith-field/\nmake/\n/make\nvalue\nreplacement/\n/replacement\n/with-field\nvalue\n/field\n2\n/equal-value\n/do\n";
    assert_program_bool(replacement_order, true);
}

#[test]
fn invalid_or_removed_equality_categories_fail_during_compilation() {
    for expression in [
        "eq/\n1\n1\n/eq",
        "ne/\n1\n1\n/ne",
        "equal-value/\n1\n1.0\n/equal-value",
        "equal-value/\nbuf-new/\n1\n/buf-new\nbuf-new/\n1\n/buf-new\n/equal-value",
        "same-object/\n1\n1\n/same-object",
        "f64-bits-equal/\n1\n1\n/f64-bits-equal",
        "list-equal/\nempty-list/\nList\nI64\n/empty-list\nempty-list/\nList\nI64\n/empty-list\n/list-equal",
    ] {
        let source = format!("do/\n{expression}\n/do\n");
        assert!(
            compile_source(&source, "invalid-equality.lkjscript", &Limits::default()).is_err(),
            "accepted {expression}"
        );
    }

    let function_identity = "def/\nname/\nf\n/name\nfn/\nsig/\n->\nUnit\n/sig\nparams/\n/params\nunit\n/fn\n/def\ndo/\nequal-value/\nf\nf\n/equal-value\n/do\n";
    assert!(compile_source(
        function_identity,
        "closure-equality.lkjscript",
        &Limits::default()
    )
    .is_err());

    let unconstrained = "def/\nname/\nsame\n/name\nforall/\nT\n/forall\nfn/\nsig/\nT\nT\n->\nBool\n/sig\nparams/\nleft\nT\nright\nT\n/params\nequal-value/\nleft\nright\n/equal-value\n/fn\n/def\n";
    assert!(compile_source(
        unconstrained,
        "generic-equality.lkjscript",
        &Limits::default()
    )
    .is_err());
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
