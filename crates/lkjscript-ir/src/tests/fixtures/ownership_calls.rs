use super::*;
use crate::*;

pub(crate) fn ownership_callee() -> Function {
    Function {
        id: FunctionId::new(1),
        name: "take-two".into(),
        signature: Signature::monomorphic(vec![owned_buf_type(), owned_buf_type()], SsaType::Unit),
        places: vec![owned_place(0, 0), owned_place(1, 1)],
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
            instructions: vec![Instruction {
                id: ValueId::new(2),
                ty: SsaType::Unit,
                kind: InstructionKind::Constant(Constant::Unit),
                metadata: metadata(EffectSet::PURE),
            }],
            terminator: Terminator::Return(ValueId::new(2)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    }
}

pub(crate) fn duplicate_call_caller() -> Function {
    Function {
        id: FunctionId::new(2),
        name: "duplicate-call".into(),
        signature: Signature::monomorphic(vec![owned_buf_type()], SsaType::Unit),
        places: vec![owned_place(0, 0)],
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: vec![BlockParameter {
                id: ValueId::new(0),
                ty: owned_buf_type(),
                owner_place: Some(PlaceId::new(0)),
                origin: Origin::SYNTHETIC,
            }],
            instructions: vec![
                Instruction {
                    id: ValueId::new(1),
                    ty: owned_buf_type(),
                    kind: InstructionKind::Move {
                        place: PlaceId::new(0),
                        value: ValueId::new(0),
                    },
                    metadata: metadata(EffectSet::PURE),
                },
                Instruction {
                    id: ValueId::new(2),
                    ty: SsaType::Unit,
                    kind: InstructionKind::Call {
                        target: CallTarget::Direct(FunctionId::new(1)),
                        arguments: vec![ValueId::new(1), ValueId::new(1)],
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

pub(crate) fn implicit_call_caller() -> Function {
    Function {
        id: FunctionId::new(2),
        name: "implicit-call".into(),
        signature: Signature::monomorphic(vec![owned_buf_type(), owned_buf_type()], SsaType::Unit),
        places: vec![owned_place(0, 0), owned_place(1, 1)],
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
                    metadata: metadata(EffectSet::PURE),
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
