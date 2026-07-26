#![allow(clippy::expect_used, clippy::panic)]

use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Limits};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig};
use lkjscript_vm::run_chunk;

fn enum_match_source() -> String {
    concat!(
        "",
        "enum/\nname/\nMaybe\n/name\nforall/\nT\n/forall\nvariants/\n",
        "variant/\nname/\nNone\n/name\nfields/\n/fields\n/variant\n",
        "variant/\nname/\nSome\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\ntype/\nT\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\n->\nI64\n/sig\nmatch/\n",
        "variant-value/\ntype/\nMaybe/\nI64\n/Maybe\n/type\nvariant/\nSome\n/variant\nfields/\nvariant-field/\nname/\nvalue\n/name\n42\n/variant-field\n/fields\n/variant-value\n",
        "arms/\narm/\nvariant-pattern/\ntype/\nMaybe/\nI64\n/Maybe\n/type\nvariant/\nSome\n/variant\nfields/\nvariant-field-pattern/\nname/\nvalue\n/name\nbinding/\nname/\nx\n/name\n/binding\n/variant-field-pattern\n/fields\n/variant-pattern\nx\n/arm\n",
        "arm/\nvariant-pattern/\ntype/\nMaybe/\nI64\n/Maybe\n/type\nvariant/\nNone\n/variant\nfields/\n/fields\n/variant-pattern\n0\n/arm\n/arms\n/match\n/main\n",
    )
    .into()
}

fn single_evaluation_source() -> String {
    concat!(
        "main/\nsig/\n->\nI64\n/sig\n",
        "var/\nname/\ncounter\n/name\ntype/\nI64\n/type\n0\n",
        "match/\ndo/\nset/\ncounter\n+/\ncounter\n1\n/+\n/set\ncounter\n/do\n",
        "arms/\narm/\ni64-pattern/\n1\n/i64-pattern\ncounter\n/arm\n",
        "arm/\nbinding/\nname/\nother\n/name\n/binding\n99\n/arm\n",
        "/arms\n/match\n/var\n/main\n",
    )
    .into()
}

fn assert_all_engines(source: &str, expected: i64) {
    let program = compile_source(source, "match-engine.lkjscript", &Limits::default())
        .expect("compile match source");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(expected)),
    );
    let ExecutionOutcome::Returned(vm) = run_chunk(program.bytecode(), &ExecutionConfig::default())
    else {
        panic!("VM must return")
    };
    assert_eq!(vm.as_i64(), Some(expected));
    for execution in [
        execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("forced baseline match"),
        execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("forced proof match"),
    ] {
        let ExecutionOutcome::Returned(value) = execution.outcome else {
            panic!("generated match must return")
        };
        assert_eq!(value.as_i64(), Some(expected));
        assert!(execution.stats.native_entries > 0);
        assert_eq!(execution.stats.vm_fallbacks, 0);
    }
}

#[test]
fn enum_match_is_differential_through_all_forced_engines() {
    assert_all_engines(&enum_match_source(), 42);
}

#[test]
fn match_scrutinee_evaluates_exactly_once_through_all_engines() {
    assert_all_engines(&single_evaluation_source(), 1);
}
