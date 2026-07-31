use super::fixtures::*;
use crate::*;

#[test]
fn verifier_rejects_generic_ownership_substitution() {
    let generic = Function {
        id: FunctionId::new(1),
        name: "generic-id".into(),
        signature: Signature {
            type_parameters: vec!["t".into()],
            bounds: Vec::new(),
            parameters: vec![SsaType::TypeParameter("t".into())],
            result: Box::new(SsaType::TypeParameter("t".into())),
        },
        places: Vec::new(),
        failure_cleanups: Vec::new(),
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: vec![BlockParameter {
                id: ValueId::new(0),
                ty: SsaType::TypeParameter("t".into()),
                owner_place: None,
                origin: Origin::SYNTHETIC,
            }],
            instructions: Vec::new(),
            terminator: Terminator::Return(ValueId::new(0)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    };
    let caller = Function {
        id: FunctionId::new(2),
        name: "generic-owned-caller".into(),
        signature: Signature::monomorphic(vec![byte_vector_type()], byte_vector_type()),
        places: vec![owned_place(0, 0)],
        failure_cleanups: vec![
            FailureCleanupPlan {
                id: FailureCleanupId::new(0),
                actions: vec![FailureCleanupAction::DropOwner {
                    place: Some(PlaceId::new(0)),
                    value: ValueId::new(0),
                    glue: DropGlueIdentity::ByteVector,
                }],
            },
            FailureCleanupPlan {
                id: FailureCleanupId::new(1),
                actions: vec![FailureCleanupAction::DropOwner {
                    place: None,
                    value: ValueId::new(2),
                    glue: DropGlueIdentity::ByteVector,
                }],
            },
        ],
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
                    ty: byte_vector_type(),
                    kind: InstructionKind::Call {
                        target: CallTarget::Direct(FunctionId::new(1)),
                        arguments: vec![ValueId::new(1)],
                        consuming: vec![true],
                        signature: Signature::monomorphic(
                            vec![byte_vector_type()],
                            byte_vector_type(),
                        ),
                        instantiation: Some(GenericInstantiation {
                            substitutions: vec![TypeSubstitution {
                                parameter: "t".into(),
                                ty: byte_vector_type(),
                            }],
                            witnesses: Vec::new(),
                        }),
                    },
                    metadata: InstructionMetadata {
                        origin: Origin::SYNTHETIC,
                        effects: EffectSet::PURE,
                        safepoint: Safepoint::Required,
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
            metadata: block_metadata_cleanup(1),
        }],
        origin: Origin::SYNTHETIC,
    };
    let mut program = one_block_program();
    program.functions.extend([generic.clone(), caller]);
    let error = verify(program).expect_err("generic ownership substitution must fail");
    assert!(
        error
            .to_string()
            .contains("ownership/reference generic instantiation is unavailable"),
        "wrong generic ownership diagnostic: {error}"
    );

    let reference = SsaType::ByteSlice;
    let reference_caller = Function {
        id: FunctionId::new(2),
        name: "generic-reference-caller".into(),
        signature: Signature::monomorphic(vec![reference.clone()], SsaType::I64),
        places: Vec::new(),
        failure_cleanups: Vec::new(),
        effects: EffectSet::READS_MEMORY,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: vec![BlockParameter {
                id: ValueId::new(0),
                ty: reference.clone(),
                owner_place: None,
                origin: Origin::SYNTHETIC,
            }],
            instructions: vec![
                Instruction {
                    id: ValueId::new(1),
                    ty: reference.clone(),
                    kind: InstructionKind::Call {
                        target: CallTarget::Direct(FunctionId::new(1)),
                        arguments: vec![ValueId::new(0)],
                        consuming: vec![false],
                        signature: Signature::monomorphic(
                            vec![reference.clone()],
                            reference.clone(),
                        ),
                        instantiation: Some(GenericInstantiation {
                            substitutions: vec![TypeSubstitution {
                                parameter: "t".into(),
                                ty: reference.clone(),
                            }],
                            witnesses: Vec::new(),
                        }),
                    },
                    metadata: InstructionMetadata {
                        origin: Origin::SYNTHETIC,
                        effects: EffectSet::PURE,
                        safepoint: Safepoint::Required,
                        failure: FailureBehavior::None,
                        failure_cleanup: None,
                        frame_state: Some(FrameState {
                            bytecode_position: 0,
                            locals: Vec::new(),
                            operand_stack: Vec::new(),
                        }),
                    },
                },
                Instruction {
                    id: ValueId::new(2),
                    ty: SsaType::I64,
                    kind: InstructionKind::Runtime {
                        operation: RuntimeOp::ByteSliceLength,
                        arguments: vec![ValueId::new(1)],
                        signature: Signature::monomorphic(vec![reference], SsaType::I64),
                    },
                    metadata: metadata(EffectSet::READS_MEMORY),
                },
            ],
            terminator: Terminator::Return(ValueId::new(2)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    };
    let mut reference_program = one_block_program();
    reference_program
        .functions
        .extend([generic, reference_caller]);
    let error =
        verify(reference_program).expect_err("reference-valued generic user-call result must fail");
    assert!(error.to_string().contains("user-call result"), "{error}");
}
