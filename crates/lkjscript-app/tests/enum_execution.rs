#![allow(clippy::expect_used, clippy::panic)]

use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionOutcome, ExecutionPolicy, OwnedValue};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig};
use lkjscript_vm::run_chunk;

#[path = "enum_execution/adversarial.rs"]
mod adversarial;
#[path = "enum_execution/authenticated_option_result.rs"]
mod authenticated_option_result;
#[path = "enum_execution/authenticated_rehydration.rs"]
mod authenticated_rehydration;
#[path = "enum_execution/enum_source_variants.rs"]
mod enum_source_variants;
#[path = "enum_execution/native_ssa.rs"]
mod native_ssa;
#[path = "enum_execution/recursive.rs"]
mod recursive;
#[path = "enum_execution/scale.rs"]
mod scale;

fn evaluator_owned(program: &lkjscript_compiler::ExecutableProgram) -> OwnedValue {
    match evaluate(program.ssa(), &EvalConfig::default()) {
        EvalOutcome::Returned(EvalValue::ReturnedOwned(value)) => value,
        outcome => panic!("evaluator must return a key-free structural value: {outcome:?}"),
    }
}

fn source() -> String {
    concat!(
        "",
        "enum/\nname/\nmaybe\n/name\nforall/\nt\n/forall\nvariants/\n",
        "variant/\nname/\nnone\n/name\nfields/\n/fields\n/variant\n",
        "variant/\nname/\nsome\n/name\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\ntype/\nt\n/type\n/variant-field\n",
        "/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\nmaybe/\ni64\n/maybe\n/output\n/sig\n",
        "variant-value/\ntype/\nmaybe/\ni64\n/maybe\n/type\n",
        "variant/\nsome\n/variant\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\n42\n/variant-field\n",
        "/fields\n/variant-value\n/main\n",
    )
    .into()
}

#[test]
fn source_construction_is_differential_on_evaluator_and_vm() {
    let program = compile_source(&source(), "enum.lkjscript").expect("compile enum construction");
    let evaluated = evaluator_owned(&program);
    let physical_tag = evaluated
        .enum_physical_tag()
        .expect("evaluator returns structural enum tag");
    assert_eq!(evaluated.enum_field_i64(0), Some(42));
    let ExecutionOutcome::Returned(value) = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    ) else {
        panic!("VM must return enum")
    };
    assert_eq!(value.enum_physical_tag(), Some(physical_tag));
    assert_eq!(value.enum_field_i64(0), Some(42));
}

fn ordered_source() -> String {
    concat!(
        "",
        "enum/\nname/\npair\n/name\nvariants/\n",
        "variant/\nname/\nboth\n/name\nfields/\n",
        "variant-field/\nname/\nfirst\n/name\ntype/\ni64\n/type\n/variant-field\n",
        "variant-field/\nname/\nsecond\n/name\ntype/\ni64\n/type\n/variant-field\n",
        "/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\npair/\n/pair\n/output\n/sig\n",
        "var/\nname/\nx\n/name\ntype/\ni64\n/type\n0\n",
        "variant-value/\ntype/\npair/\n/pair\n/type\nvariant/\nboth\n/variant\nfields/\n",
        "variant-field/\nname/\nfirst\n/name\n",
        "do/\nset/\nx\nadd/\nx\n1\n/add\n/set\nx\n/do\n/variant-field\n",
        "variant-field/\nname/\nsecond\n/name\n",
        "do/\nset/\nx\nadd/\nx\n1\n/add\n/set\nx\n/do\n/variant-field\n",
        "/fields\n/variant-value\n/var\n/main\n",
    )
    .into()
}

#[test]
fn fields_evaluate_once_in_declaration_order() {
    let program = compile_source(&ordered_source(), "enum-order.lkjscript")
        .expect("compile ordered enum fields");
    let evaluated = evaluator_owned(&program);
    assert_eq!(evaluated.enum_field_i64(0), Some(1));
    assert_eq!(evaluated.enum_field_i64(1), Some(2));
    let ExecutionOutcome::Returned(value) = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    ) else {
        panic!("VM must return enum")
    };
    assert_eq!(value.enum_field_i64(0), Some(1));
    assert_eq!(value.enum_field_i64(1), Some(2));
}

#[test]
fn forced_native_tiers_execute_enum_in_generated_code_without_fallback() {
    let program =
        compile_source(&source(), "enum-jit.lkjscript").expect("compile enum construction");
    let physical_tag = evaluator_owned(&program)
        .enum_physical_tag()
        .expect("evaluator returns structural enum tag");
    for execution in [
        execute_forced(
            program.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        )
        .expect("baseline executes enum"),
        execute_optimizing(
            program.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        )
        .expect("proof tier executes enum"),
    ] {
        let ExecutionOutcome::Returned(value) = execution.outcome else {
            panic!("native tier must return enum")
        };
        assert_eq!(value.enum_physical_tag(), Some(physical_tag));
        assert_eq!(value.enum_field_i64(0), Some(42));
        assert!(execution.stats.native_entries > 0);
        assert_eq!(execution.stats.runtime_heap_successes, 0);
        assert!(execution.stats.structural_runtime_calls > 0);
        assert_eq!(execution.stats.native_structural.live_roots, 0);
        assert_eq!(execution.stats.native_structural.live_destinations, 0);
        assert_eq!(execution.stats.vm_fallbacks, 0);
    }
}
