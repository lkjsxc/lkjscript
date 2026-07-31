use super::fixtures::*;
use crate::*;

#[test]
fn verifier_rejects_direct_malformed_id_use_dominance_edge_loop_and_metadata() {
    let mut duplicate = one_block_program();
    duplicate.functions[0].blocks[0]
        .instructions
        .push(constant(0, 7));
    assert!(verify(duplicate).is_err());

    let mut use_before_definition = one_block_program();
    use_before_definition.functions[0].blocks[0].instructions = vec![
        Instruction {
            id: ValueId::new(0),
            ty: SsaType::I64,
            kind: InstructionKind::Copy(ValueId::new(1)),
            metadata: metadata(EffectSet::PURE),
        },
        constant(1, 7),
    ];
    assert!(verify(use_before_definition).is_err());

    let mut indirect_target_after_call = one_block_program();
    let indirect_signature = Signature::monomorphic(Vec::new(), SsaType::I64);
    indirect_target_after_call.functions[0].effects = EffectSet::CONSERVATIVE_CALL;
    indirect_target_after_call.functions[0].blocks[0].instructions = vec![
        Instruction {
            id: ValueId::new(0),
            ty: SsaType::I64,
            kind: InstructionKind::Call {
                target: CallTarget::Indirect(ValueId::new(1)),
                arguments: Vec::new(),
                consuming: Vec::new(),
                signature: indirect_signature.clone(),
                instantiation: None,
            },
            metadata: InstructionMetadata {
                origin: Origin::SYNTHETIC,
                effects: EffectSet::CONSERVATIVE_CALL,
                failure: FailureBehavior::TrapOrOutcome,
                failure_cleanup: None,
                frame_state: Some(FrameState {
                    bytecode_position: 0,
                    locals: Vec::new(),
                    operand_stack: Vec::new(),
                }),
            },
        },
        Instruction {
            id: ValueId::new(1),
            ty: SsaType::Function(Box::new(indirect_signature)),
            kind: InstructionKind::FunctionRef(FunctionId::new(0)),
            metadata: metadata(EffectSet::PURE),
        },
    ];
    let indirect_operands = indirect_target_after_call.functions[0].blocks[0].instructions[0]
        .kind
        .operands();
    assert_eq!(indirect_operands, vec![ValueId::new(1)]);
    assert!(verify(indirect_target_after_call).is_err());

    let mut missing_frame_state = bounded_call_program();
    missing_frame_state.functions[1].blocks[0].instructions[1]
        .metadata
        .frame_state = None;
    let error = verify(missing_frame_state).expect_err("call without frame state must fail");
    assert!(error.to_string().contains("requires frame-state metadata"));

    let mut bad_return = one_block_program();
    bad_return.functions[0].blocks[0].terminator = Terminator::Return(ValueId::new(8));
    assert!(verify(bad_return).is_err());

    let mut bad_effect = one_block_program();
    bad_effect.functions[0].blocks[0].instructions[0]
        .metadata
        .effects = EffectSet::MAY_TRAP;
    assert!(verify(bad_effect).is_err());

    let mut bad_edge = pass_program();
    if let Terminator::ConditionalBranch { true_arguments, .. } =
        &mut bad_edge.functions[0].blocks[0].terminator
    {
        true_arguments.clear();
    }
    assert!(verify(bad_edge).is_err());

    let mut unmarked_loop = one_block_program();
    unmarked_loop.functions[0].blocks[0].terminator = Terminator::Branch {
        target: BlockId::new(0),
        arguments: Vec::new(),
    };
    assert!(verify(unmarked_loop).is_err());
}
