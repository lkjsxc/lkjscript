#![allow(clippy::expect_used, clippy::panic)]

use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Limits, ResourceLimitKind};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_jit::{execute_forced, execute_optimizing, FailureCode, JitConfig};
use lkjscript_vm::run_chunk;

fn source() -> String {
    concat!(
        "edition/\n2\n/edition\n",
        "enum/\nname/\nMaybe\n/name\nforall/\nT\n/forall\nvariants/\n",
        "variant/\nname/\nNone\n/name\nfields/\n/fields\n/variant\n",
        "variant/\nname/\nSome\n/name\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\ntype/\nT\n/type\n/variant-field\n",
        "/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\n->\nMaybe/\nI64\n/Maybe\n/sig\n",
        "variant-value/\ntype/\nMaybe/\nI64\n/Maybe\n/type\n",
        "variant/\nSome\n/variant\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\n42\n/variant-field\n",
        "/fields\n/variant-value\n/main\n",
    )
    .into()
}

#[test]
fn source_construction_is_differential_on_evaluator_and_vm() {
    let program = compile_source(&source(), "enum.lkjscript", &Limits::default())
        .expect("compile enum construction");
    let EvalOutcome::Returned(EvalValue::Enum {
        physical_tag,
        payload,
        ..
    }) = evaluate(program.ssa(), &EvalConfig::default())
    else {
        panic!("evaluator must return enum")
    };
    assert_eq!(payload, [EvalValue::I64(42)]);
    let ExecutionOutcome::Returned(value) =
        run_chunk(program.bytecode(), &ExecutionConfig::default())
    else {
        panic!("VM must return enum")
    };
    assert_eq!(value.enum_physical_tag(), Some(physical_tag));
    assert_eq!(value.enum_field_i64(0), Some(42));
}

fn ordered_source() -> String {
    concat!(
        "edition/\n2\n/edition\n",
        "enum/\nname/\nPair\n/name\nvariants/\n",
        "variant/\nname/\nBoth\n/name\nfields/\n",
        "variant-field/\nname/\nfirst\n/name\ntype/\nI64\n/type\n/variant-field\n",
        "variant-field/\nname/\nsecond\n/name\ntype/\nI64\n/type\n/variant-field\n",
        "/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\n->\nPair/\n/Pair\n/sig\n",
        "var/\nname/\nx\n/name\ntype/\nI64\n/type\n0\n",
        "variant-value/\ntype/\nPair/\n/Pair\n/type\nvariant/\nBoth\n/variant\nfields/\n",
        "variant-field/\nname/\nfirst\n/name\n",
        "do/\nset/\nx\n+/\nx\n1\n/+\n/set\nx\n/do\n/variant-field\n",
        "variant-field/\nname/\nsecond\n/name\n",
        "do/\nset/\nx\n+/\nx\n1\n/+\n/set\nx\n/do\n/variant-field\n",
        "/fields\n/variant-value\n/var\n/main\n",
    )
    .into()
}

#[test]
fn fields_evaluate_once_in_declaration_order() {
    let program = compile_source(
        &ordered_source(),
        "enum-order.lkjscript",
        &Limits::default(),
    )
    .expect("compile ordered enum fields");
    let EvalOutcome::Returned(EvalValue::Enum { payload, .. }) =
        evaluate(program.ssa(), &EvalConfig::default())
    else {
        panic!("evaluator must return enum")
    };
    assert_eq!(payload, [EvalValue::I64(1), EvalValue::I64(2)]);
    let ExecutionOutcome::Returned(value) =
        run_chunk(program.bytecode(), &ExecutionConfig::default())
    else {
        panic!("VM must return enum")
    };
    assert_eq!(value.enum_field_i64(0), Some(1));
    assert_eq!(value.enum_field_i64(1), Some(2));
}

#[test]
fn logical_construction_exhausts_before_allocation_on_both_engines() {
    let program = compile_source(&source(), "enum-limit.lkjscript", &Limits::default())
        .expect("compile enum construction");
    let evaluator = EvalConfig {
        max_logical_aggregate_constructions: 0,
        ..EvalConfig::default()
    };
    assert_eq!(
        evaluate(program.ssa(), &evaluator),
        EvalOutcome::ResourceLimitExceeded("logical_aggregate_constructions".into())
    );
    let execution = ExecutionConfig {
        max_logical_aggregate_constructions: 0,
        ..ExecutionConfig::default()
    };
    assert!(matches!(
        run_chunk(program.bytecode(), &execution),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::LogicalAggregateConstructions)
    ));
}

#[test]
fn forced_native_tiers_reject_enum_program_without_fallback_claim() {
    let program = compile_source(&source(), "enum-jit.lkjscript", &Limits::default())
        .expect("compile enum construction");
    for error in [
        execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect_err("baseline rejects enum"),
        execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect_err("proof tier rejects enum"),
    ] {
        assert_eq!(error.code(), FailureCode::UnsupportedType);
        assert!(error.detail().contains("no fallback was claimed"));
    }
}
