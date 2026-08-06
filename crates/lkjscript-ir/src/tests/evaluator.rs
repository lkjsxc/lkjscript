use super::fixtures::*;
use crate::*;

fn unsupported_enum_program(kind: InstructionKind, ty: SsaType, effects: EffectSet) -> Program {
    let mut program = one_block_program();
    program.enums.push(enum_metadata());
    let function = &mut program.functions[0];
    *function.signature.result = ty.clone();
    function.effects = effects;
    let frame_state = effects
        .contains(EffectSet::ALLOCATES)
        .then_some(FrameState {
            bytecode_position: 1,
            locals: Vec::new(),
            operand_stack: Vec::new(),
        });
    function.blocks[0].instructions.push(Instruction {
        id: ValueId::new(1),
        ty,
        kind,
        metadata: InstructionMetadata {
            origin: Origin::SYNTHETIC,
            effects,
            failure: if effects.contains(EffectSet::ALLOCATES) {
                FailureBehavior::StructuredOutcome
            } else {
                FailureBehavior::None
            },
            failure_cleanup: None,
            frame_state,
        },
    });
    function.blocks[0].terminator = Terminator::Return(ValueId::new(1));
    program
}

#[test]
fn enum_instructions_are_unsupported_without_structural_operations() {
    let cases = [
        (
            InstructionKind::EnumValue {
                enum_id: EnumId::new([1; 32]),
                variant: VariantId::new([2; 32]),
                layout: RuntimeLayoutId::new([6; 32]),
                fields: vec![ValueId::new(0)],
            },
            enum_type(),
            EffectSet::ALLOCATES,
            "unsupported without structural metadata and operations",
        ),
        (
            InstructionKind::EnumIsVariant {
                enum_id: EnumId::new([1; 32]),
                variant: VariantId::new([2; 32]),
                layout: RuntimeLayoutId::new([6; 32]),
                value: ValueId::new(0),
            },
            SsaType::Bool,
            EffectSet::READS_MEMORY,
            "resource-result variant test identity/type mismatch",
        ),
        (
            InstructionKind::EnumField {
                enum_id: EnumId::new([1; 32]),
                variant: VariantId::new([2; 32]),
                field: VariantFieldId::new([3; 32]),
                field_index: 0,
                layout: RuntimeLayoutId::new([6; 32]),
                value: ValueId::new(0),
            },
            SsaType::I64,
            EffectSet::READS_MEMORY,
            "lacks verified active-variant provenance",
        ),
    ];
    for (kind, ty, effects, expected) in cases {
        let error = verify(unsupported_enum_program(kind, ty, effects))
            .expect_err("enum instruction without structural operations must fail");
        let message = error.to_string();
        assert!(
            message.contains(expected),
            "wrong enum instruction rejection: {message}"
        );
    }
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
