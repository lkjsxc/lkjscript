use super::enum_source_variants::{nested_source, nullary_source};
use super::*;
fn projected_program() -> lkjscript_ir::VerifiedProgram {
    let source = concat!(
        "enum/\nname/\nmaybe\n/name\nforall/\nt\n/forall\nvariants/\n",
        "variant/\nname/\nnone\n/name\nfields/\n/fields\n/variant\n",
        "variant/\nname/\nsome\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\ntype/\nt\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nmatch/\n",
        "variant-value/\ntype/\nmaybe/\ni64\n/maybe\n/type\nvariant/\nsome\n/variant\nfields/\nvariant-field/\nname/\nvalue\n/name\n42\n/variant-field\n/fields\n/variant-value\n",
        "arms/\narm/\nvariant-pattern/\ntype/\nmaybe/\ni64\n/maybe\n/type\nvariant/\nsome\n/variant\nfields/\nvariant-field-pattern/\nname/\nvalue\n/name\nbinding/\nname/\nx\n/name\n/binding\n/variant-field-pattern\n/fields\n/variant-pattern\nx\n/arm\n",
        "arm/\nvariant-pattern/\ntype/\nmaybe/\ni64\n/maybe\n/type\nvariant/\nnone\n/variant\nfields/\n/fields\n/variant-pattern\n0\n/arm\n/arms\n/match\n/main\n",
    );
    compile_source(source, "enum-project.lkjscript", &Limits::default())
        .expect("compile structural enum projection")
        .ssa()
        .clone()
}

#[test]
fn variant_test_and_active_projection_execute_in_both_generated_tiers() {
    let program = projected_program();
    assert_eq!(
        evaluate(&program, &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(42))
    );
    for execution in [
        execute_forced(&program, &ExecutionConfig::default(), JitConfig::default())
            .expect("baseline projects enum"),
        execute_optimizing(&program, &ExecutionConfig::default(), JitConfig::default())
            .expect("proof tier projects enum"),
    ] {
        let ExecutionOutcome::Returned(value) = execution.outcome else {
            panic!("generated tier must return projection")
        };
        assert_eq!(value.as_i64(), Some(42));
        assert!(execution.stats.native_entries > 0);
        assert_eq!(execution.stats.runtime_heap_successes, 0);
        assert_eq!(execution.stats.collector_runtime_invocations, 0);
        assert!(execution.stats.structural_runtime_calls > 0);
        assert_eq!(execution.stats.native_structural.live_roots, 0);
        assert_eq!(execution.stats.native_structural.live_loans, 0);
        assert_eq!(execution.stats.native_structural.live_destinations, 0);
        assert_eq!(execution.stats.vm_fallbacks, 0);
    }
}

#[test]
fn nullary_enum_is_differential_and_enters_generated_tiers() {
    let compiled = compile_source(
        &nullary_source(),
        "enum-nullary.lkjscript",
        &Limits::default(),
    )
    .expect("compile nullary enum");
    let physical_tag = evaluator_owned(&compiled)
        .enum_physical_tag()
        .expect("evaluator returns structural nullary enum");
    let ExecutionOutcome::Returned(vm) = run_chunk(
        compiled.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ) else {
        panic!("VM returns nullary enum")
    };
    assert_eq!(vm.enum_physical_tag(), Some(physical_tag));
    for execution in [
        execute_forced(
            compiled.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("baseline returns nullary enum"),
        execute_optimizing(
            compiled.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("proof returns nullary enum"),
    ] {
        let ExecutionOutcome::Returned(value) = execution.outcome else {
            panic!("generated tier returns nullary enum")
        };
        assert_eq!(value.enum_physical_tag(), Some(physical_tag));
        assert!(execution.stats.native_entries > 0);
        assert_eq!(execution.stats.vm_fallbacks, 0);
    }
}

#[test]
fn nested_generic_enum_survives_forced_collection_in_generated_tiers() {
    let compiled = compile_source(
        &nested_source(),
        "enum-nested.lkjscript",
        &Limits::default(),
    )
    .expect("compile nested generic enum");
    let physical_tag = evaluator_owned(&compiled)
        .enum_physical_tag()
        .expect("evaluator returns nested structural enum");
    let config = JitConfig {
        force_gc_before_allocation: true,
        ..JitConfig::default()
    };
    for execution in [
        execute_forced(compiled.ssa(), &ExecutionConfig::default(), config)
            .expect("baseline returns nested enum"),
        execute_optimizing(compiled.ssa(), &ExecutionConfig::default(), config)
            .expect("proof returns nested enum"),
    ] {
        let ExecutionOutcome::Returned(value) = execution.outcome else {
            panic!("generated tier must return nested enum")
        };
        assert_eq!(value.enum_physical_tag(), Some(physical_tag));
        assert!(value.snapshot_object_count() >= 2);
        assert_eq!(execution.stats.collections, 0);
        assert_eq!(execution.stats.maximum_roots, 0);
        assert_eq!(execution.stats.runtime_heap_successes, 0);
        assert_eq!(execution.stats.collector_runtime_invocations, 0);
        assert!(execution.stats.structural_runtime_calls > 0);
        assert!(execution.stats.native_structural.calls > 0);
        assert_eq!(execution.stats.native_structural.live_roots, 0);
        assert_eq!(execution.stats.native_structural.live_loans, 0);
        assert_eq!(execution.stats.native_structural.live_destinations, 0);
        assert_eq!(execution.stats.native_structural.teardown_failures, 0);
        assert!(execution.stats.code_objects.iter().all(|object| {
            !object
                .runtime_calls
                .contains(&lkjscript_native::RuntimeCallSlot::HeapDispatch)
        }));
        assert_eq!(execution.stats.vm_fallbacks, 0);
    }
}
