use super::*;

pub(crate) fn implicit_call_caller() -> Function {
    Function {
        id: FunctionId::new(2),
        name: "implicit-call".into(),
        signature: Signature::monomorphic(vec![owned_buf_type(), owned_buf_type()], SsaType::Unit),
        places: vec![owned_place(0, 0), owned_place(1, 1)],
        failure_cleanups: vec![
            FailureCleanupPlan {
                id: FailureCleanupId::new(0),
                actions: vec![drop_action(1, 1), drop_action(0, 0)],
            },
            FailureCleanupPlan {
                id: FailureCleanupId::new(1),
                actions: vec![drop_action(0, 0)],
            },
        ],
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: vec![
                BlockParameter {
                    id: ValueId::new(0),
                    ty: owned_buf_type(),
                    owner_place: Some(PlaceId::new(0)),
                    origin: Origin::SYNTHETIC,
                },
                BlockParameter {
                    id: ValueId::new(1),
                    ty: owned_buf_type(),
                    owner_place: Some(PlaceId::new(1)),
                    origin: Origin::SYNTHETIC,
                },
            ],
            instructions: vec![
                Instruction {
                    id: ValueId::new(2),
                    ty: owned_buf_type(),
                    kind: InstructionKind::Move {
                        place: PlaceId::new(1),
                        value: ValueId::new(1),
                    },
                    metadata: metadata_cleanup(EffectSet::PURE, 0),
                },
                Instruction {
                    id: ValueId::new(3),
                    ty: SsaType::Unit,
                    kind: InstructionKind::Call {
                        target: CallTarget::Direct(FunctionId::new(1)),
                        arguments: vec![ValueId::new(0), ValueId::new(2)],
                        signature: Signature::monomorphic(
                            vec![owned_buf_type(), owned_buf_type()],
                            SsaType::Unit,
                        ),
                        instantiation: None,
                    },
                    metadata: InstructionMetadata {
                        origin: Origin::SYNTHETIC,
                        effects: EffectSet::PURE,
                        safepoint: Safepoint::Required,
                        failure: FailureBehavior::None,
                        failure_cleanup: Some(FailureCleanupId::new(1)),
                        frame_state: Some(FrameState {
                            bytecode_position: 0,
                            locals: Vec::new(),
                            operand_stack: Vec::new(),
                        }),
                    },
                },
            ],
            terminator: Terminator::Return(ValueId::new(3)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    }
}
