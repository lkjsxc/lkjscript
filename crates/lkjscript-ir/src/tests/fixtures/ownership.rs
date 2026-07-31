use super::*;
use crate::*;

pub(crate) fn owned_branch_function(equal_moves: bool) -> Function {
    let second_instruction = if equal_moves {
        Instruction {
            id: ValueId::new(5),
            ty: byte_vector_type(),
            kind: InstructionKind::Move {
                place: PlaceId::new(0),
                value: ValueId::new(3),
            },
            metadata: metadata_cleanup(EffectSet::PURE, 3),
        }
    } else {
        Instruction {
            id: ValueId::new(5),
            ty: SsaType::Unit,
            kind: InstructionKind::Constant(Constant::Unit),
            metadata: metadata_cleanup(EffectSet::PURE, 3),
        }
    };
    Function {
        id: FunctionId::new(1),
        name: "owned-branch".into(),
        signature: Signature::monomorphic(vec![byte_vector_type()], byte_vector_type()),
        places: vec![owned_place(0, 0)],
        failure_cleanups: if equal_moves {
            vec![
                drop_plan(0, Some(0), 0),
                drop_plan(1, Some(0), 2),
                drop_plan(2, None, 4),
                drop_plan(3, Some(0), 3),
                drop_plan(4, None, 5),
                drop_plan(5, None, 6),
            ]
        } else {
            vec![
                drop_plan(0, Some(0), 0),
                drop_plan(1, Some(0), 2),
                drop_plan(2, None, 4),
                drop_plan(3, Some(0), 3),
                drop_plan(4, None, 6),
            ]
        },
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![
            Block {
                id: BlockId::new(0),
                parameters: vec![BlockParameter {
                    id: ValueId::new(0),
                    ty: byte_vector_type(),
                    owner_place: Some(PlaceId::new(0)),
                    origin: Origin::SYNTHETIC,
                }],
                instructions: vec![Instruction {
                    id: ValueId::new(1),
                    ty: SsaType::Bool,
                    kind: InstructionKind::Constant(Constant::Bool(true)),
                    metadata: metadata_cleanup(EffectSet::PURE, 0),
                }],
                terminator: Terminator::ConditionalBranch {
                    condition: ValueId::new(1),
                    true_target: BlockId::new(1),
                    true_arguments: vec![ValueId::new(0)],
                    false_target: BlockId::new(2),
                    false_arguments: vec![ValueId::new(0)],
                },
                metadata: block_metadata_cleanup(0),
            },
            Block {
                id: BlockId::new(1),
                parameters: vec![BlockParameter {
                    id: ValueId::new(2),
                    ty: byte_vector_type(),
                    owner_place: Some(PlaceId::new(0)),
                    origin: Origin::SYNTHETIC,
                }],
                instructions: vec![Instruction {
                    id: ValueId::new(4),
                    ty: byte_vector_type(),
                    kind: InstructionKind::Move {
                        place: PlaceId::new(0),
                        value: ValueId::new(2),
                    },
                    metadata: metadata_cleanup(EffectSet::PURE, 1),
                }],
                terminator: Terminator::Branch {
                    target: BlockId::new(3),
                    arguments: vec![ValueId::new(4)],
                },
                metadata: block_metadata_cleanup(2),
            },
            Block {
                id: BlockId::new(2),
                parameters: vec![BlockParameter {
                    id: ValueId::new(3),
                    ty: byte_vector_type(),
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
                metadata: block_metadata_cleanup(if equal_moves { 4 } else { 3 }),
            },
            Block {
                id: BlockId::new(3),
                parameters: vec![BlockParameter {
                    id: ValueId::new(6),
                    ty: byte_vector_type(),
                    owner_place: None,
                    origin: Origin::SYNTHETIC,
                }],
                instructions: Vec::new(),
                terminator: Terminator::Return(ValueId::new(6)),
                metadata: block_metadata_cleanup(if equal_moves { 5 } else { 4 }),
            },
        ],
        origin: Origin::SYNTHETIC,
    }
}

fn drop_plan(id: u32, place: Option<u32>, value: u32) -> FailureCleanupPlan {
    FailureCleanupPlan {
        id: FailureCleanupId::new(id),
        actions: vec![FailureCleanupAction::DropOwner {
            place: place.map(PlaceId::new),
            value: ValueId::new(value),
            glue: DropGlueIdentity::ByteVector,
        }],
    }
}
