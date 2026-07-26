use super::*;

#[test]
fn selected_conditional_callee_trap_retains_exact_site_message() {
    let metadata = |effects, safepoint, failure| InstructionMetadata {
        origin: Origin::SYNTHETIC,
        effects,
        safepoint,
        failure,
        frame_state: (safepoint == IrSafepoint::Required).then_some(FrameState {
            bytecode_position: 0,
            locals: Vec::new(),
            operand_stack: Vec::new(),
        }),
    };
    let block_metadata = || BlockMetadata {
        loop_header: false,
        origin: Origin::SYNTHETIC,
        frame_state: None,
    };
    let callee_signature = Signature::monomorphic(vec![SsaType::Bool], SsaType::I64);
    let program = verify(Program {
        sources: vec![SourceMetadata {
            id: 0,
            path: "selected-trap.lkjscript".into(),
        }],
        products: Vec::new(),
        enums: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![
            Function {
                id: FunctionId::new(0),
                name: "choose".into(),
                signature: callee_signature.clone(),
                places: Vec::new(),
                effects: EffectSet::MAY_TRAP,
                entry: BlockId::new(0),
                blocks: vec![
                    Block {
                        id: BlockId::new(0),
                        parameters: vec![BlockParameter {
                            id: ValueId::new(0),
                            ty: SsaType::Bool,
                            owner_place: None,
                            origin: Origin::SYNTHETIC,
                        }],
                        instructions: Vec::new(),
                        terminator: Terminator::ConditionalBranch {
                            condition: ValueId::new(0),
                            true_target: BlockId::new(1),
                            true_arguments: Vec::new(),
                            false_target: BlockId::new(2),
                            false_arguments: Vec::new(),
                        },
                        metadata: block_metadata(),
                    },
                    Block {
                        id: BlockId::new(1),
                        parameters: Vec::new(),
                        instructions: vec![Instruction {
                            id: ValueId::new(1),
                            ty: SsaType::Str,
                            kind: InstructionKind::Constant(Constant::Str("first trap".into())),
                            metadata: metadata(
                                EffectSet::PURE,
                                IrSafepoint::None,
                                FailureBehavior::None,
                            ),
                        }],
                        terminator: Terminator::Trap {
                            value: ValueId::new(1),
                        },
                        metadata: block_metadata(),
                    },
                    Block {
                        id: BlockId::new(2),
                        parameters: Vec::new(),
                        instructions: vec![Instruction {
                            id: ValueId::new(2),
                            ty: SsaType::Str,
                            kind: InstructionKind::Constant(Constant::Str(
                                "selected callee trap".into(),
                            )),
                            metadata: metadata(
                                EffectSet::PURE,
                                IrSafepoint::None,
                                FailureBehavior::None,
                            ),
                        }],
                        terminator: Terminator::Trap {
                            value: ValueId::new(2),
                        },
                        metadata: block_metadata(),
                    },
                ],
                origin: Origin::SYNTHETIC,
            },
            Function {
                id: FunctionId::new(1),
                name: "main".into(),
                signature: Signature::monomorphic(Vec::new(), SsaType::I64),
                places: Vec::new(),
                effects: EffectSet::MAY_TRAP,
                entry: BlockId::new(0),
                blocks: vec![Block {
                    id: BlockId::new(0),
                    parameters: Vec::new(),
                    instructions: vec![
                        Instruction {
                            id: ValueId::new(0),
                            ty: SsaType::Bool,
                            kind: InstructionKind::Constant(Constant::Bool(false)),
                            metadata: metadata(
                                EffectSet::PURE,
                                IrSafepoint::None,
                                FailureBehavior::None,
                            ),
                        },
                        Instruction {
                            id: ValueId::new(1),
                            ty: SsaType::I64,
                            kind: InstructionKind::Call {
                                target: CallTarget::Direct(FunctionId::new(0)),
                                arguments: vec![ValueId::new(0)],
                                signature: callee_signature,
                                instantiation: None,
                            },
                            metadata: metadata(
                                EffectSet::MAY_TRAP,
                                IrSafepoint::Required,
                                FailureBehavior::Trap,
                            ),
                        },
                    ],
                    terminator: Terminator::Return(ValueId::new(1)),
                    metadata: block_metadata(),
                }],
                origin: Origin::SYNTHETIC,
            },
        ],
        main: FunctionId::new(1),
    })
    .expect("verify selected trap SSA");
    let executed = execute_forced(&program, &ExecutionConfig::default(), JitConfig::default())
        .expect("execute selected native trap");
    assert!(matches!(
        executed.outcome,
        ExecutionOutcome::Trapped(trap) if trap.as_str() == "selected callee trap"
    ));
}
