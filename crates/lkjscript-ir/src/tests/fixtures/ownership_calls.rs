mod implicit;
pub(crate) use implicit::implicit_call_caller;

use super::*;
use crate::*;

pub(super) fn drop_action(place: u32, value: u32) -> FailureCleanupAction {
    FailureCleanupAction::DropOwner {
        place: Some(PlaceId::new(place)),
        value: ValueId::new(value),
        glue: DropGlueIdentity::ByteVector,
    }
}

pub(crate) fn ownership_callee() -> Function {
    Function {
        id: FunctionId::new(1),
        name: "take-two".into(),
        signature: Signature::monomorphic(
            vec![byte_vector_type(), byte_vector_type()],
            SsaType::Unit,
        ),
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
                    ty: byte_vector_type(),
                    owner_place: Some(PlaceId::new(0)),
                    origin: Origin::SYNTHETIC,
                },
                BlockParameter {
                    id: ValueId::new(1),
                    ty: byte_vector_type(),
                    owner_place: Some(PlaceId::new(1)),
                    origin: Origin::SYNTHETIC,
                },
            ],
            instructions: vec![
                drop_byte_cleanup(2, 1, 1, 0),
                place_end_cleanup(3, 1, 1),
                drop_byte_cleanup(4, 0, 0, 1),
                place_end(5, 0),
                Instruction {
                    id: ValueId::new(6),
                    ty: SsaType::Unit,
                    kind: InstructionKind::Constant(Constant::Unit),
                    metadata: metadata(EffectSet::PURE),
                },
            ],
            terminator: Terminator::Return(ValueId::new(6)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    }
}

pub(crate) fn duplicate_call_caller() -> Function {
    Function {
        id: FunctionId::new(2),
        name: "duplicate-call".into(),
        signature: Signature::monomorphic(vec![byte_vector_type()], SsaType::Unit),
        places: vec![owned_place(0, 0)],
        failure_cleanups: vec![FailureCleanupPlan {
            id: FailureCleanupId::new(0),
            actions: vec![drop_action(0, 0)],
        }],
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: vec![BlockParameter {
                id: ValueId::new(0),
                ty: byte_vector_type(),
                owner_place: Some(PlaceId::new(0)),
                origin: Origin::SYNTHETIC,
            }],
            instructions: vec![
                Instruction {
                    id: ValueId::new(1),
                    ty: byte_vector_type(),
                    kind: InstructionKind::Move {
                        place: PlaceId::new(0),
                        value: ValueId::new(0),
                    },
                    metadata: metadata_cleanup(EffectSet::PURE, 0),
                },
                Instruction {
                    id: ValueId::new(2),
                    ty: SsaType::Unit,
                    kind: InstructionKind::Call {
                        target: CallTarget::Direct(FunctionId::new(1)),
                        arguments: vec![ValueId::new(1), ValueId::new(1)],
                        consuming: vec![true, true],
                        signature: Signature::monomorphic(
                            vec![byte_vector_type(), byte_vector_type()],
                            SsaType::Unit,
                        ),
                        instantiation: None,
                    },
                    metadata: InstructionMetadata {
                        origin: Origin::SYNTHETIC,
                        effects: EffectSet::PURE,
                        failure: FailureBehavior::None,
                        failure_cleanup: None,
                        frame_state: Some(FrameState {
                            bytecode_position: 0,
                            locals: Vec::new(),
                            operand_stack: Vec::new(),
                        }),
                    },
                },
            ],
            terminator: Terminator::Return(ValueId::new(2)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    }
}
