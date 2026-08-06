use super::fixtures::*;
use crate::*;

#[test]
fn ownership_cfg_rejects_successor_reuse_duplicate_edges_and_loop_mismatch() {
    let successor_reuse = Function {
        id: FunctionId::new(1),
        name: "successor-reuse".into(),
        signature: Signature::monomorphic(vec![byte_vector_type()], byte_vector_type()),
        places: vec![owned_place(0, 0)],
        failure_cleanups: Vec::new(),
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
                    ty: byte_vector_type(),
                    kind: InstructionKind::Move {
                        place: PlaceId::new(0),
                        value: ValueId::new(0),
                    },
                    metadata: metadata(EffectSet::PURE),
                }],
                terminator: Terminator::Branch {
                    target: BlockId::new(1),
                    arguments: Vec::new(),
                },
                metadata: block_metadata(),
            },
            Block {
                id: BlockId::new(1),
                parameters: Vec::new(),
                instructions: Vec::new(),
                terminator: Terminator::Return(ValueId::new(0)),
                metadata: block_metadata(),
            },
        ],
        origin: Origin::SYNTHETIC,
    };
    assert!(verify(ownership_program(successor_reuse)).is_err());

    let duplicate_edge = Function {
        id: FunctionId::new(1),
        name: "duplicate-edge".into(),
        signature: Signature::monomorphic(vec![byte_vector_type()], SsaType::Unit),
        places: vec![owned_place(0, 0)],
        failure_cleanups: vec![
            FailureCleanupNode {
                action: FailureCleanupAction::DropOwner {
                    place: Some(PlaceId::new(0)),
                    value: ValueId::new(0),
                    glue: DropGlueIdentity::ByteVector,
                },
                next: None,
            },
            FailureCleanupNode {
                action: FailureCleanupAction::DropOwner {
                    place: None,
                    value: ValueId::new(1),
                    glue: DropGlueIdentity::ByteVector,
                },
                next: None,
            },
        ],
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
                    ty: byte_vector_type(),
                    kind: InstructionKind::Move {
                        place: PlaceId::new(0),
                        value: ValueId::new(0),
                    },
                    metadata: metadata_cleanup(EffectSet::PURE, 0),
                }],
                terminator: Terminator::Branch {
                    target: BlockId::new(1),
                    arguments: vec![ValueId::new(1), ValueId::new(1)],
                },
                metadata: block_metadata_cleanup(1),
            },
            Block {
                id: BlockId::new(1),
                parameters: vec![
                    BlockParameter {
                        id: ValueId::new(2),
                        ty: byte_vector_type(),
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    },
                    BlockParameter {
                        id: ValueId::new(3),
                        ty: byte_vector_type(),
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    },
                ],
                instructions: Vec::new(),
                terminator: Terminator::Trap {
                    value: ValueId::new(2),
                },
                metadata: block_metadata(),
            },
        ],
        origin: Origin::SYNTHETIC,
    };
    let error = verify(ownership_program(duplicate_edge))
        .expect_err("duplicate affine edge arguments must fail");
    assert!(
        error.to_string().contains("duplicates affine argument"),
        "{error}"
    );

    let loop_mismatch = Function {
        id: FunctionId::new(1),
        name: "loop-mismatch".into(),
        signature: Signature::monomorphic(vec![byte_vector_type()], SsaType::Unit),
        places: vec![owned_place(0, 0)],
        failure_cleanups: Vec::new(),
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
                instructions: Vec::new(),
                terminator: Terminator::Branch {
                    target: BlockId::new(1),
                    arguments: vec![ValueId::new(0)],
                },
                metadata: block_metadata(),
            },
            Block {
                id: BlockId::new(1),
                parameters: vec![BlockParameter {
                    id: ValueId::new(1),
                    ty: byte_vector_type(),
                    owner_place: Some(PlaceId::new(0)),
                    origin: Origin::SYNTHETIC,
                }],
                instructions: vec![Instruction {
                    id: ValueId::new(2),
                    ty: byte_vector_type(),
                    kind: InstructionKind::Move {
                        place: PlaceId::new(0),
                        value: ValueId::new(1),
                    },
                    metadata: metadata(EffectSet::PURE),
                }],
                terminator: Terminator::Branch {
                    target: BlockId::new(1),
                    arguments: vec![ValueId::new(2)],
                },
                metadata: BlockMetadata {
                    loop_header: true,
                    origin: Origin::SYNTHETIC,
                    failure_cleanup: None,
                    frame_state: Some(FrameState {
                        bytecode_position: 0,
                        locals: Vec::new(),
                        operand_stack: Vec::new(),
                    }),
                },
            },
        ],
        origin: Origin::SYNTHETIC,
    };
    assert!(verify(ownership_program(loop_mismatch)).is_err());
}
