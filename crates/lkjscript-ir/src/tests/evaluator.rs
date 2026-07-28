use super::fixtures::*;
use crate::*;

#[test]
fn evaluator_executes_generic_enum_construction_and_tag_test() {
    let mut program = one_block_program();
    program.enums.push(enum_metadata());
    let function = &mut program.functions[0];
    *function.signature.result = SsaType::Bool;
    function.effects = EffectSet::ALLOCATES.union(EffectSet::READS_MEMORY);
    let block = &mut function.blocks[0];
    block.instructions.push(Instruction {
        id: ValueId::new(1),
        ty: enum_type(),
        kind: InstructionKind::EnumValue {
            enum_id: EnumId::new([1; 32]),
            variant: VariantId::new([2; 32]),
            layout: RuntimeLayoutId::new([6; 32]),
            fields: vec![ValueId::new(0)],
        },
        metadata: InstructionMetadata {
            origin: Origin::SYNTHETIC,
            effects: EffectSet::ALLOCATES,
            safepoint: Safepoint::Required,
            failure: FailureBehavior::StructuredOutcome,
            failure_cleanup: None,
            frame_state: Some(FrameState {
                bytecode_position: 1,
                locals: Vec::new(),
                operand_stack: Vec::new(),
            }),
        },
    });
    block.instructions.push(Instruction {
        id: ValueId::new(2),
        ty: SsaType::Bool,
        kind: InstructionKind::EnumIsVariant {
            enum_id: EnumId::new([1; 32]),
            variant: VariantId::new([2; 32]),
            layout: RuntimeLayoutId::new([6; 32]),
            value: ValueId::new(1),
        },
        metadata: metadata(EffectSet::READS_MEMORY),
    });
    block.terminator = Terminator::Return(ValueId::new(2));
    let verified = verify(program).expect("verify generic enum evaluator program");
    assert_eq!(
        evaluate(&verified, &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::Bool(true))
    );
}

#[test]
fn evaluator_bounds_and_explicit_trap_exit_outcomes_are_deterministic() {
    let mut trapped = one_block_program();
    trapped.functions[0].blocks[0].instructions = vec![Instruction {
        id: ValueId::new(0),
        ty: SsaType::Str,
        kind: InstructionKind::Constant(Constant::Str("explicit trap".into())),
        metadata: metadata(EffectSet::PURE),
    }];
    trapped.functions[0].blocks[0].terminator = Terminator::Trap {
        value: ValueId::new(0),
    };
    let trapped = verify(trapped).expect("verify trap");
    assert_eq!(
        evaluate(&trapped, &EvalConfig::default()),
        EvalOutcome::Trapped("explicit trap".into())
    );

    let mut exited = one_block_program();
    exited.functions[0].blocks[0].terminator = Terminator::Exit {
        code: ValueId::new(0),
    };
    let exited = verify(exited).expect("verify exit");
    assert_eq!(
        evaluate(&exited, &EvalConfig::default()),
        EvalOutcome::Exited(42)
    );

    let bounded = verify(one_block_program()).expect("verify bounded program");
    let config = EvalConfig {
        fuel: 0,
        ..EvalConfig::default()
    };
    assert_eq!(
        evaluate(&bounded, &config),
        EvalOutcome::ResourceLimitExceeded("fuel".into())
    );
}

fn enum_projection_program(project_variant: VariantId, field: VariantFieldId) -> Program {
    let mut program = one_block_program();
    program.enums.push(enum_metadata());
    let function = &mut program.functions[0];
    *function.signature.result = SsaType::I64;
    function.effects = EffectSet::ALLOCATES.union(EffectSet::READS_MEMORY);
    let block = &mut function.blocks[0];
    block.instructions.push(Instruction {
        id: ValueId::new(1),
        ty: enum_type(),
        kind: InstructionKind::EnumValue {
            enum_id: EnumId::new([1; 32]),
            variant: VariantId::new([2; 32]),
            layout: RuntimeLayoutId::new([6; 32]),
            fields: vec![ValueId::new(0)],
        },
        metadata: InstructionMetadata {
            origin: Origin::SYNTHETIC,
            effects: EffectSet::ALLOCATES,
            safepoint: Safepoint::Required,
            failure: FailureBehavior::StructuredOutcome,
            failure_cleanup: None,
            frame_state: Some(FrameState {
                bytecode_position: 1,
                locals: Vec::new(),
                operand_stack: Vec::new(),
            }),
        },
    });
    block.instructions.push(Instruction {
        id: ValueId::new(2),
        ty: SsaType::I64,
        kind: InstructionKind::EnumField {
            enum_id: EnumId::new([1; 32]),
            variant: project_variant,
            field,
            layout: RuntimeLayoutId::new([6; 32]),
            value: ValueId::new(1),
        },
        metadata: metadata(EffectSet::READS_MEMORY),
    });
    block.terminator = Terminator::Return(ValueId::new(2));
    program
}

#[test]
fn enum_projection_requires_active_verified_provenance() {
    let active = enum_projection_program(VariantId::new([2; 32]), VariantFieldId::new([3; 32]));
    let verified = verify(active).expect("active projection verifies");
    assert_eq!(
        evaluate(&verified, &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(42))
    );
    let inactive = enum_projection_program(VariantId::new([4; 32]), VariantFieldId::new([5; 32]));
    let error = verify(inactive).expect_err("inactive projection rejected");
    assert!(error.to_string().contains("inactive enum projection"));
}

#[test]
fn enum_verifier_rejects_layout_and_substitution_mismatches() {
    let mut layout = enum_projection_program(VariantId::new([2; 32]), VariantFieldId::new([3; 32]));
    if let InstructionKind::EnumValue { layout, .. } =
        &mut layout.functions[0].blocks[0].instructions[1].kind
    {
        *layout = RuntimeLayoutId::new([9; 32]);
    }
    assert!(verify(layout)
        .expect_err("layout mismatch")
        .to_string()
        .contains("layout identity mismatch"));
    let mut substitution =
        enum_projection_program(VariantId::new([2; 32]), VariantFieldId::new([3; 32]));
    substitution.functions[0].blocks[0].instructions[1].ty = SsaType::Enum {
        id: EnumId::new([1; 32]),
        arguments: Vec::new(),
    };
    assert!(verify(substitution)
        .expect_err("substitution mismatch")
        .to_string()
        .contains("substitution"));
}
