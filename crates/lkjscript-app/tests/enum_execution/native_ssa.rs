use super::enum_source_variants::{nested_source, nullary_source};
use super::*;
use lkjscript_ir::{
    verify, EffectSet, FailureBehavior, Instruction, InstructionKind, Safepoint, SsaType,
    Terminator, ValueId,
};

fn projected_program() -> lkjscript_ir::VerifiedProgram {
    let compiled = compile_source(&source(), "enum-project.lkjscript", &Limits::default())
        .expect("compile enum construction");
    let mut program = compiled.ssa().program().clone();
    let function = &mut program.functions[program.main.index().expect("main indexes")];
    let block = &mut function.blocks[function.entry.index().expect("entry indexes")];
    let constructor = block
        .instructions
        .iter()
        .find(|instruction| matches!(instruction.kind, InstructionKind::EnumValue { .. }))
        .cloned()
        .expect("enum constructor exists");
    let InstructionKind::EnumValue {
        enum_id,
        variant,
        layout,
        ..
    } = constructor.kind
    else {
        unreachable!()
    };
    let definition = program
        .enums
        .iter()
        .find(|definition| definition.id == enum_id)
        .expect("enum metadata exists");
    let selected = definition
        .variants
        .iter()
        .find(|candidate| candidate.id == variant)
        .expect("variant metadata exists");
    let metadata = lkjscript_ir::InstructionMetadata {
        origin: constructor.metadata.origin,
        effects: EffectSet::READS_MEMORY,
        safepoint: Safepoint::None,
        failure: FailureBehavior::None,
        frame_state: None,
    };
    let test_id = ValueId::new(constructor.id.raw() + 1);
    block.instructions.push(Instruction {
        id: test_id,
        ty: SsaType::Bool,
        kind: InstructionKind::EnumIsVariant {
            enum_id,
            variant,
            layout,
            value: constructor.id,
        },
        metadata: metadata.clone(),
    });
    let projection_id = ValueId::new(test_id.raw() + 1);
    block.instructions.push(Instruction {
        id: projection_id,
        ty: SsaType::I64,
        kind: InstructionKind::EnumField {
            enum_id,
            variant,
            field: selected.fields[0].id,
            layout,
            value: constructor.id,
        },
        metadata,
    });
    *function.signature.result = SsaType::I64;
    block.terminator = Terminator::Return(projection_id);
    verify(program).expect("hand-built projection SSA verifies")
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
        assert!(execution.stats.runtime_heap_successes >= 3);
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
    let EvalOutcome::Returned(EvalValue::Enum { physical_tag, .. }) =
        evaluate(compiled.ssa(), &EvalConfig::default())
    else {
        panic!("evaluator returns nullary enum")
    };
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
    let EvalOutcome::Returned(EvalValue::Enum { physical_tag, .. }) =
        evaluate(compiled.ssa(), &EvalConfig::default())
    else {
        panic!("evaluator returns nested enum")
    };
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
        assert!(execution.stats.collections >= 2);
        assert!(execution.stats.maximum_roots > 0);
        assert!(execution.stats.code_objects.iter().any(|object| {
            !object.exact_scalar_stack_maps
                && object
                    .runtime_calls
                    .contains(&lkjscript_native::RuntimeCallSlot::HeapDispatch)
        }));
        assert_eq!(execution.stats.vm_fallbacks, 0);
    }
}
