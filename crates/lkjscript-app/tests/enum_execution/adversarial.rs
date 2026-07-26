use super::*;
use lkjscript_ir::{
    verify, EffectSet, FailureBehavior, Instruction, InstructionKind, Safepoint, SsaType,
    Terminator, ValueId,
};

fn two_payload_variants() -> String {
    concat!(
        "",
        "enum/\nname/\nChoice\n/name\nvariants/\n",
        "variant/\nname/\nLeft\n/name\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\ntype/\nI64\n/type\n/variant-field\n",
        "/fields\n/variant\n",
        "variant/\nname/\nRight\n/name\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\ntype/\nI64\n/type\n/variant-field\n",
        "/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\n->\nChoice/\n/Choice\n/sig\n",
        "variant-value/\ntype/\nChoice/\n/Choice\n/type\n",
        "variant/\nLeft\n/variant\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\n9\n/variant-field\n",
        "/fields\n/variant-value\n/main\n",
    )
    .into()
}

#[test]
fn enum_heap_preflight_rejects_before_partial_scalar_box_publication() {
    let float_source = source().replace("I64", "F64").replace("42", "1.5");
    let compiled = compile_source(
        &float_source,
        "enum-float-limit.lkjscript",
        &Limits::default(),
    )
    .expect("compile F64 enum");
    let execution = ExecutionConfig {
        max_allocations: 1,
        ..ExecutionConfig::default()
    };
    for result in [
        execute_forced(compiled.ssa(), &execution, JitConfig::default())
            .expect("baseline returns enum allocation resource outcome"),
        execute_optimizing(compiled.ssa(), &execution, JitConfig::default())
            .expect("proof returns enum allocation resource outcome"),
    ] {
        assert!(matches!(
            result.outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations)
        ));
        assert_eq!(result.stats.allocations, 0);
        assert!(result.stats.native_entries > 0);
        assert_eq!(result.stats.vm_fallbacks, 0);
    }
}

#[test]
fn inactive_projection_is_rejected_before_any_generated_access() {
    let compiled = compile_source(
        &two_payload_variants(),
        "enum-inactive.lkjscript",
        &Limits::default(),
    )
    .expect("compile two-variant enum");
    let mut program = compiled.ssa().program().clone();
    let function = &mut program.functions[program.main.index().expect("main indexes")];
    let block = &mut function.blocks[0];
    let constructor = block
        .instructions
        .iter()
        .find(|instruction| matches!(instruction.kind, InstructionKind::EnumValue { .. }))
        .cloned()
        .expect("enum constructor exists");
    let InstructionKind::EnumValue {
        enum_id, layout, ..
    } = constructor.kind
    else {
        unreachable!()
    };
    let inactive = &program
        .enums
        .iter()
        .find(|definition| definition.id == enum_id)
        .expect("enum exists")
        .variants[1];
    let projection = ValueId::new(constructor.id.raw() + 1);
    block.instructions.push(Instruction {
        id: projection,
        ty: SsaType::I64,
        kind: InstructionKind::EnumField {
            enum_id,
            variant: inactive.id,
            field: inactive.fields[0].id,
            layout,
            value: constructor.id,
        },
        metadata: lkjscript_ir::InstructionMetadata {
            origin: constructor.metadata.origin,
            effects: EffectSet::READS_MEMORY,
            safepoint: Safepoint::None,
            failure: FailureBehavior::None,
            frame_state: None,
        },
    });
    *function.signature.result = SsaType::I64;
    block.terminator = Terminator::Return(projection);
    let error = verify(program).expect_err("inactive projection must not gain verified authority");
    assert!(error.to_string().contains("inactive enum projection"));
}
