use super::*;
use crate::*;

pub(crate) fn owned_branch_function(equal_moves: bool) -> Function {
    let second_instruction = if equal_moves {
        Instruction {
            id: ValueId::new(5),
            ty: owned_buf_type(),
            kind: InstructionKind::Move {
                place: PlaceId::new(0),
                value: ValueId::new(3),
            },
            metadata: metadata(EffectSet::PURE),
        }
    } else {
        Instruction {
            id: ValueId::new(5),
            ty: SsaType::Unit,
            kind: InstructionKind::Constant(Constant::Unit),
            metadata: metadata(EffectSet::PURE),
        }
    };
    Function {
        id: FunctionId::new(1),
        name: "owned-branch".into(),
        signature: Signature::monomorphic(vec![owned_buf_type()], owned_buf_type()),
        places: vec![owned_place(0, 0)],
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![
            Block {
                id: BlockId::new(0),
                parameters: vec![BlockParameter {
                    id: ValueId::new(0),
                    ty: owned_buf_type(),
                    owner_place: Some(PlaceId::new(0)),
                    origin: Origin::SYNTHETIC,
                }],
                instructions: vec![Instruction {
                    id: ValueId::new(1),
                    ty: SsaType::Bool,
                    kind: InstructionKind::Constant(Constant::Bool(true)),
                    metadata: metadata(EffectSet::PURE),
                }],
                terminator: Terminator::ConditionalBranch {
                    condition: ValueId::new(1),
                    true_target: BlockId::new(1),
                    true_arguments: vec![ValueId::new(0)],
                    false_target: BlockId::new(2),
                    false_arguments: vec![ValueId::new(0)],
                },
                metadata: block_metadata(),
            },
            Block {
                id: BlockId::new(1),
                parameters: vec![BlockParameter {
                    id: ValueId::new(2),
                    ty: owned_buf_type(),
                    owner_place: Some(PlaceId::new(0)),
                    origin: Origin::SYNTHETIC,
                }],
                instructions: vec![Instruction {
                    id: ValueId::new(4),
                    ty: owned_buf_type(),
                    kind: InstructionKind::Move {
                        place: PlaceId::new(0),
                        value: ValueId::new(2),
                    },
                    metadata: metadata(EffectSet::PURE),
                }],
                terminator: Terminator::Branch {
                    target: BlockId::new(3),
                    arguments: vec![ValueId::new(4)],
                },
                metadata: block_metadata(),
            },
            Block {
                id: BlockId::new(2),
                parameters: vec![BlockParameter {
                    id: ValueId::new(3),
                    ty: owned_buf_type(),
                    owner_place: Some(PlaceId::new(0)),
                    origin: Origin::SYNTHETIC,
                }],
                instructions: vec![second_instruction],
                terminator: Terminator::Branch {
                    target: BlockId::new(3),
                    arguments: vec![if equal_moves {
                        ValueId::new(5)
                    } else {
                        ValueId::new(3)
                    }],
                },
                metadata: block_metadata(),
            },
            Block {
                id: BlockId::new(3),
                parameters: vec![BlockParameter {
                    id: ValueId::new(6),
                    ty: owned_buf_type(),
                    owner_place: None,
                    origin: Origin::SYNTHETIC,
                }],
                instructions: Vec::new(),
                terminator: Terminator::Return(ValueId::new(6)),
                metadata: block_metadata(),
            },
        ],
        origin: Origin::SYNTHETIC,
    }
}
