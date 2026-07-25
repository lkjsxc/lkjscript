#![allow(clippy::expect_used, clippy::panic)]

use crate::{
    canonical_block_order, constant_fold_and_propagate, copy_propagate, direct_call_resolution,
    effect_aware_dce, empty_block_forwarding, evaluate, normalize_baseline, optimize,
    simplify_branches, unreachable_blocks, verify, verify_optimization, Block, BlockId,
    BlockMetadata, BlockParameter, BorrowKind, CallTarget, Constant, EffectSet, EvalConfig,
    EvalOutcome, EvalValue, FailureBehavior, FrameState, Function, FunctionId,
    GenericInstantiation, ImplId, ImplMetadata, Instruction, InstructionKind, InstructionMetadata,
    LoanId, OptimizationCertificate, OptimizationCertificateRecord, OptimizationEditKind,
    OptimizationFailureCode, OptimizationLimits, Origin, PlaceId, ProductMetadata, Program,
    RuntimeOp, Safepoint, Signature, SourceMetadata, SsaType, Terminator, TraitBound, TraitId,
    TraitMetadata, TraitRole, TraitWitness, TraitWitnessKind, TypeSubstitution, ValueId,
};

fn metadata(effects: EffectSet) -> InstructionMetadata {
    InstructionMetadata {
        origin: Origin::SYNTHETIC,
        effects,
        safepoint: Safepoint::None,
        failure: if effects.contains(EffectSet::MAY_TRAP) {
            FailureBehavior::Trap
        } else {
            FailureBehavior::None
        },
        frame_state: None,
    }
}

fn block_metadata() -> BlockMetadata {
    BlockMetadata {
        loop_header: false,
        origin: Origin::SYNTHETIC,
        frame_state: None,
    }
}

fn constant(id: u32, value: i64) -> Instruction {
    Instruction {
        id: ValueId::new(id),
        ty: SsaType::I64,
        kind: InstructionKind::Constant(Constant::I64(value)),
        metadata: metadata(EffectSet::PURE),
    }
}

fn one_block_program() -> Program {
    Program {
        sources: Vec::new(),
        products: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![Function {
            id: FunctionId::new(0),
            name: "main".into(),
            signature: Signature::monomorphic(Vec::new(), SsaType::I64),
            places: Vec::new(),
            effects: EffectSet::PURE,
            entry: BlockId::new(0),
            blocks: vec![Block {
                id: BlockId::new(0),
                parameters: Vec::new(),
                instructions: vec![constant(0, 42)],
                terminator: Terminator::Return(ValueId::new(0)),
                metadata: block_metadata(),
            }],
            origin: Origin::SYNTHETIC,
        }],
        main: FunctionId::new(0),
    }
}

fn core_traits() -> Vec<TraitMetadata> {
    [
        ("Copy", TraitRole::Copy),
        ("Clone", TraitRole::Clone),
        ("Drop", TraitRole::Drop),
        ("Send", TraitRole::Send),
        ("Sync", TraitRole::Sync),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (name, role))| TraitMetadata {
        id: TraitId::new(index as u32),
        name: name.into(),
        role,
        source: None,
    })
    .collect()
}

fn owned_buf_type() -> SsaType {
    SsaType::Owned(Box::new(SsaType::Buf))
}

fn owned_place(id: u32, binding: u32) -> crate::PlaceMetadata {
    crate::PlaceMetadata {
        id: PlaceId::new(id),
        binding: crate::BindingId::new(binding),
        ty: owned_buf_type(),
    }
}

fn ownership_program(function: Function) -> Program {
    let mut program = one_block_program();
    program.functions.push(function);
    program
}

fn owned_branch_function(equal_moves: bool) -> Function {
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

fn bounded_call_program() -> Program {
    let declared = Signature {
        type_parameters: vec!["T".into()],
        bounds: vec![TraitBound {
            parameter: "T".into(),
            trait_id: TraitId::new(0),
        }],
        parameters: vec![SsaType::TypeParameter("T".into())],
        result: Box::new(SsaType::TypeParameter("T".into())),
    };
    let resolved = Signature::monomorphic(vec![SsaType::I64], SsaType::I64);
    Program {
        sources: vec![SourceMetadata {
            id: 0,
            path: "traits.lkjscript".into(),
        }],
        products: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![
            Function {
                id: FunctionId::new(0),
                name: "copy-value".into(),
                signature: declared,
                places: Vec::new(),
                effects: EffectSet::PURE,
                entry: BlockId::new(0),
                blocks: vec![Block {
                    id: BlockId::new(0),
                    parameters: vec![BlockParameter {
                        id: ValueId::new(0),
                        ty: SsaType::TypeParameter("T".into()),
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    }],
                    instructions: Vec::new(),
                    terminator: Terminator::Return(ValueId::new(0)),
                    metadata: block_metadata(),
                }],
                origin: Origin::SYNTHETIC,
            },
            Function {
                id: FunctionId::new(1),
                name: "main".into(),
                signature: Signature::monomorphic(Vec::new(), SsaType::I64),
                places: Vec::new(),
                effects: EffectSet::PURE,
                entry: BlockId::new(0),
                blocks: vec![Block {
                    id: BlockId::new(0),
                    parameters: Vec::new(),
                    instructions: vec![
                        constant(0, 9),
                        Instruction {
                            id: ValueId::new(1),
                            ty: SsaType::I64,
                            kind: InstructionKind::Call {
                                target: CallTarget::Direct(FunctionId::new(0)),
                                arguments: vec![ValueId::new(0)],
                                signature: resolved,
                                instantiation: Some(GenericInstantiation {
                                    substitutions: vec![TypeSubstitution {
                                        parameter: "T".into(),
                                        ty: SsaType::I64,
                                    }],
                                    witnesses: vec![TraitWitness {
                                        trait_id: TraitId::new(0),
                                        ty: SsaType::I64,
                                        kind: TraitWitnessKind::AutoTrait,
                                    }],
                                }),
                            },
                            metadata: InstructionMetadata {
                                origin: Origin::SYNTHETIC,
                                effects: EffectSet::PURE,
                                safepoint: Safepoint::Required,
                                failure: FailureBehavior::None,
                                frame_state: Some(FrameState {
                                    bytecode_position: 1,
                                    locals: Vec::new(),
                                    operand_stack: Vec::new(),
                                }),
                            },
                        },
                    ],
                    terminator: Terminator::Return(ValueId::new(1)),
                    metadata: block_metadata(),
                }],
                origin: Origin::SYNTHETIC,
            },
        ],
        main: FunctionId::new(1),
    }
}

fn pass_program() -> Program {
    let add_signature = Signature::monomorphic(vec![SsaType::I64, SsaType::I64], SsaType::I64);
    Program {
        sources: Vec::new(),
        products: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![Function {
            id: FunctionId::new(0),
            name: "main".into(),
            signature: Signature::monomorphic(Vec::new(), SsaType::I64),
            places: Vec::new(),
            effects: EffectSet::PURE,
            entry: BlockId::new(0),
            blocks: vec![
                Block {
                    id: BlockId::new(0),
                    parameters: Vec::new(),
                    instructions: vec![
                        Instruction {
                            id: ValueId::new(0),
                            ty: SsaType::Bool,
                            kind: InstructionKind::Constant(Constant::Bool(true)),
                            metadata: metadata(EffectSet::PURE),
                        },
                        constant(1, 2),
                        constant(2, 3),
                        Instruction {
                            id: ValueId::new(3),
                            ty: SsaType::I64,
                            kind: InstructionKind::Runtime {
                                operation: RuntimeOp::Add,
                                arguments: vec![ValueId::new(1), ValueId::new(2)],
                                signature: add_signature,
                            },
                            metadata: metadata(EffectSet::MAY_TRAP),
                        },
                        Instruction {
                            id: ValueId::new(4),
                            ty: SsaType::I64,
                            kind: InstructionKind::Copy(ValueId::new(3)),
                            metadata: metadata(EffectSet::PURE),
                        },
                        constant(5, 99),
                    ],
                    terminator: Terminator::ConditionalBranch {
                        condition: ValueId::new(0),
                        true_target: BlockId::new(1),
                        true_arguments: vec![ValueId::new(4)],
                        false_target: BlockId::new(2),
                        false_arguments: vec![ValueId::new(4)],
                    },
                    metadata: block_metadata(),
                },
                Block {
                    id: BlockId::new(1),
                    parameters: vec![BlockParameter {
                        id: ValueId::new(6),
                        ty: SsaType::I64,
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    }],
                    instructions: Vec::new(),
                    terminator: Terminator::Branch {
                        target: BlockId::new(3),
                        arguments: vec![ValueId::new(6)],
                    },
                    metadata: block_metadata(),
                },
                Block {
                    id: BlockId::new(2),
                    parameters: vec![BlockParameter {
                        id: ValueId::new(7),
                        ty: SsaType::I64,
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    }],
                    instructions: Vec::new(),
                    terminator: Terminator::Branch {
                        target: BlockId::new(3),
                        arguments: vec![ValueId::new(7)],
                    },
                    metadata: block_metadata(),
                },
                Block {
                    id: BlockId::new(3),
                    parameters: vec![BlockParameter {
                        id: ValueId::new(8),
                        ty: SsaType::I64,
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    }],
                    instructions: Vec::new(),
                    terminator: Terminator::Return(ValueId::new(8)),
                    metadata: block_metadata(),
                },
                Block {
                    id: BlockId::new(4),
                    parameters: Vec::new(),
                    instructions: vec![constant(9, -1)],
                    terminator: Terminator::Return(ValueId::new(9)),
                    metadata: block_metadata(),
                },
            ],
            origin: Origin::SYNTHETIC,
        }],
        main: FunctionId::new(0),
    }
}

#[test]
fn verifier_rejects_malformed_move_borrow_and_loan_facts() {
    let mut valid = one_block_program();
    valid.functions.push(Function {
        id: FunctionId::new(1),
        name: "owned-id".into(),
        signature: Signature::monomorphic(
            vec![SsaType::Owned(Box::new(SsaType::Buf))],
            SsaType::Owned(Box::new(SsaType::Buf)),
        ),
        places: vec![crate::PlaceMetadata {
            id: PlaceId::new(0),
            binding: crate::BindingId::new(0),
            ty: SsaType::Owned(Box::new(SsaType::Buf)),
        }],
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: vec![BlockParameter {
                id: ValueId::new(0),
                ty: SsaType::Owned(Box::new(SsaType::Buf)),
                owner_place: Some(PlaceId::new(0)),
                origin: Origin::SYNTHETIC,
            }],
            instructions: vec![Instruction {
                id: ValueId::new(1),
                ty: SsaType::Owned(Box::new(SsaType::Buf)),
                kind: InstructionKind::Move {
                    place: PlaceId::new(0),
                    value: ValueId::new(0),
                },
                metadata: metadata(EffectSet::PURE),
            }],
            terminator: Terminator::Return(ValueId::new(1)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    });
    assert!(verify(valid.clone()).is_ok());

    let mut duplicate_place_end = valid.clone();
    duplicate_place_end.functions[1].blocks[0].instructions = vec![
        Instruction {
            id: ValueId::new(1),
            ty: SsaType::Unit,
            kind: InstructionKind::PlaceEnd {
                place: PlaceId::new(0),
            },
            metadata: metadata(EffectSet::PURE),
        },
        Instruction {
            id: ValueId::new(2),
            ty: SsaType::Unit,
            kind: InstructionKind::PlaceEnd {
                place: PlaceId::new(0),
            },
            metadata: metadata(EffectSet::PURE),
        },
        Instruction {
            id: ValueId::new(3),
            ty: owned_buf_type(),
            kind: InstructionKind::Move {
                place: PlaceId::new(0),
                value: ValueId::new(0),
            },
            metadata: metadata(EffectSet::PURE),
        },
    ];
    duplicate_place_end.functions[1].blocks[0].terminator = Terminator::Return(ValueId::new(3));
    let end_error = verify(duplicate_place_end).expect_err("duplicate PlaceEnd must fail");
    assert!(end_error.to_string().contains("not active"), "{end_error}");

    let mut copied = valid.clone();
    copied.functions[1].blocks[0].instructions[0].kind = InstructionKind::Copy(ValueId::new(0));
    assert!(verify(copied).is_err());

    let mut wrong_move = valid.clone();
    wrong_move.functions[1].blocks[0].instructions[0].ty = SsaType::Buf;
    assert!(verify(wrong_move).is_err());

    let mut unknown_place = valid.clone();
    let InstructionKind::Move { place, .. } =
        &mut unknown_place.functions[1].blocks[0].instructions[0].kind
    else {
        panic!("expected move fact");
    };
    *place = PlaceId::new(9);
    assert!(verify(unknown_place).is_err());

    let mut duplicate_place = valid.clone();
    let repeated_place = duplicate_place.functions[1].places[0].clone();
    duplicate_place.functions[1].places.push(repeated_place);
    assert!(verify(duplicate_place).is_err());

    let mut duplicate_loan = valid;
    duplicate_loan.functions[1].blocks[0].instructions = vec![
        Instruction {
            id: ValueId::new(1),
            ty: SsaType::Ref(Box::new(SsaType::Buf)),
            kind: InstructionKind::Borrow {
                place: PlaceId::new(0),
                loan: LoanId::new(0),
                kind: BorrowKind::Shared,
                value: ValueId::new(0),
            },
            metadata: metadata(EffectSet::PURE),
        },
        Instruction {
            id: ValueId::new(2),
            ty: SsaType::Ref(Box::new(SsaType::Buf)),
            kind: InstructionKind::Borrow {
                place: PlaceId::new(0),
                loan: LoanId::new(0),
                kind: BorrowKind::Shared,
                value: ValueId::new(0),
            },
            metadata: metadata(EffectSet::PURE),
        },
    ];
    duplicate_loan.functions[1].blocks[0].terminator = Terminator::Trap {
        message: "duplicate loan".into(),
    };
    assert!(verify(duplicate_loan).is_err());

    let borrowed_function = |id: u32, name: &str| Function {
        id: FunctionId::new(id),
        name: name.into(),
        signature: Signature::monomorphic(vec![owned_buf_type()], SsaType::I64),
        places: vec![owned_place(0, 0)],
        effects: EffectSet::READS_MEMORY,
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
                    ty: SsaType::Ref(Box::new(SsaType::Buf)),
                    kind: InstructionKind::Borrow {
                        place: PlaceId::new(0),
                        loan: LoanId::new(0),
                        kind: BorrowKind::Shared,
                        value: ValueId::new(0),
                    },
                    metadata: metadata(EffectSet::PURE),
                },
                Instruction {
                    id: ValueId::new(2),
                    ty: SsaType::I64,
                    kind: InstructionKind::Runtime {
                        operation: RuntimeOp::OwnedBufLen,
                        arguments: vec![ValueId::new(1)],
                        signature: Signature::monomorphic(
                            vec![SsaType::Ref(Box::new(SsaType::Buf))],
                            SsaType::I64,
                        ),
                    },
                    metadata: metadata(EffectSet::READS_MEMORY),
                },
            ],
            terminator: Terminator::Return(ValueId::new(2)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    };
    let mut cross_function_duplicate = one_block_program();
    cross_function_duplicate.functions.extend([
        borrowed_function(1, "borrow-one"),
        borrowed_function(2, "borrow-two"),
    ]);
    let error = verify(cross_function_duplicate)
        .expect_err("LoanIds must be unique across the whole public Program");
    assert!(
        error.to_string().contains("anywhere in the program"),
        "{error}"
    );
}

#[test]
fn ownership_cfg_dataflow_accepts_equal_moves_and_rejects_mismatched_paths() {
    verify(ownership_program(owned_branch_function(true)))
        .expect("equal explicit moves on both branch arms must join");

    let mut implicit_edge = owned_branch_function(true);
    implicit_edge.blocks[0].terminator = Terminator::ConditionalBranch {
        condition: ValueId::new(1),
        true_target: BlockId::new(1),
        true_arguments: Vec::new(),
        false_target: BlockId::new(2),
        false_arguments: Vec::new(),
    };
    let error = verify(ownership_program(implicit_edge))
        .expect_err("current affine owners require explicit edge arguments");
    assert!(
        error.to_string().contains("explicit block argument"),
        "{error}"
    );

    let mismatch = verify(ownership_program(owned_branch_function(false)))
        .expect_err("one moved and one initialized branch must not join");
    assert!(
        mismatch.to_string().contains("implicitly transfer")
            || mismatch.to_string().contains("join exactly"),
        "wrong branch mismatch diagnostic: {mismatch}"
    );

    let mut returned_original = ownership_program(owned_branch_function(true));
    returned_original.functions[1].blocks[1].terminator = Terminator::Return(ValueId::new(0));
    let error = verify(returned_original)
        .expect_err("Move followed by Return of the original owner must fail");
    assert!(error.to_string().contains("unavailable affine"), "{error}");

    let direct_return = Function {
        id: FunctionId::new(1),
        name: "implicit-return".into(),
        signature: Signature::monomorphic(vec![owned_buf_type()], owned_buf_type()),
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
            instructions: Vec::new(),
            terminator: Terminator::Return(ValueId::new(0)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    };
    let error = verify(ownership_program(direct_return))
        .expect_err("Owned entry parameter return requires explicit Move");
    assert!(
        error.to_string().contains("without explicit Move"),
        "{error}"
    );
}

#[test]
fn ownership_cfg_rejects_successor_reuse_duplicate_edges_and_loop_mismatch() {
    let successor_reuse = Function {
        id: FunctionId::new(1),
        name: "successor-reuse".into(),
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
                    ty: owned_buf_type(),
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
        signature: Signature::monomorphic(vec![owned_buf_type()], SsaType::Unit),
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
                    ty: owned_buf_type(),
                    kind: InstructionKind::Move {
                        place: PlaceId::new(0),
                        value: ValueId::new(0),
                    },
                    metadata: metadata(EffectSet::PURE),
                }],
                terminator: Terminator::Branch {
                    target: BlockId::new(1),
                    arguments: vec![ValueId::new(1), ValueId::new(1)],
                },
                metadata: block_metadata(),
            },
            Block {
                id: BlockId::new(1),
                parameters: vec![
                    BlockParameter {
                        id: ValueId::new(2),
                        ty: owned_buf_type(),
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    },
                    BlockParameter {
                        id: ValueId::new(3),
                        ty: owned_buf_type(),
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    },
                ],
                instructions: Vec::new(),
                terminator: Terminator::Trap {
                    message: "duplicate edge".into(),
                },
                metadata: block_metadata(),
            },
        ],
        origin: Origin::SYNTHETIC,
    };
    let error = verify(ownership_program(duplicate_edge))
        .expect_err("duplicate affine edge arguments must fail");
    assert!(
        error.to_string().contains("duplicates one affine"),
        "{error}"
    );

    let loop_mismatch = Function {
        id: FunctionId::new(1),
        name: "loop-mismatch".into(),
        signature: Signature::monomorphic(vec![owned_buf_type()], SsaType::Unit),
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
                    ty: owned_buf_type(),
                    owner_place: Some(PlaceId::new(0)),
                    origin: Origin::SYNTHETIC,
                }],
                instructions: vec![Instruction {
                    id: ValueId::new(2),
                    ty: owned_buf_type(),
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

#[test]
fn ownership_cfg_rejects_duplicate_calls_aliases_and_missing_provenance() {
    let callee = Function {
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
    };
    let caller = Function {
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
    };
    let mut duplicate_call = one_block_program();
    duplicate_call.functions.extend([callee.clone(), caller]);
    let error = verify(duplicate_call).expect_err("duplicate affine call arguments must fail");
    assert!(
        error.to_string().contains("duplicates one affine"),
        "{error}"
    );

    let implicit_call = Function {
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
    };
    let mut implicit_call_program = one_block_program();
    implicit_call_program
        .functions
        .extend([callee, implicit_call]);
    let error = verify(implicit_call_program)
        .expect_err("Owned entry call argument requires explicit Move");
    assert!(error.to_string().contains("explicit Move"), "{error}");

    let aliased_places = Function {
        id: FunctionId::new(1),
        name: "aliased-places".into(),
        signature: Signature::monomorphic(vec![owned_buf_type()], SsaType::Unit),
        places: vec![owned_place(0, 0), owned_place(1, 1)],
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
            instructions: vec![Instruction {
                id: ValueId::new(1),
                ty: SsaType::Unit,
                kind: InstructionKind::PlaceInit {
                    place: PlaceId::new(1),
                    value: ValueId::new(0),
                },
                metadata: metadata(EffectSet::PURE),
            }],
            terminator: Terminator::Return(ValueId::new(1)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    };
    let error = verify(ownership_program(aliased_places))
        .expect_err("one owner value cannot initialize two PlaceIds");
    assert!(error.to_string().contains("multiple PlaceIds"), "{error}");

    let mut missing_entry = owned_branch_function(true);
    missing_entry.blocks[0].parameters[0].owner_place = None;
    assert!(verify(ownership_program(missing_entry)).is_err());

    let missing_local = Function {
        id: FunctionId::new(1),
        name: "missing-local-provenance".into(),
        signature: Signature::monomorphic(Vec::new(), SsaType::Unit),
        places: vec![owned_place(0, 0)],
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: Vec::new(),
            instructions: vec![Instruction {
                id: ValueId::new(0),
                ty: SsaType::Unit,
                kind: InstructionKind::Constant(Constant::Unit),
                metadata: metadata(EffectSet::PURE),
            }],
            terminator: Terminator::Return(ValueId::new(0)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    };
    assert!(verify(ownership_program(missing_local)).is_err());
}

#[test]
fn ownership_verification_has_an_aggregate_state_work_bound() {
    let count = 22_000_u32;
    let parameters: Vec<_> = (0..count).map(|_| owned_buf_type()).collect();
    let places: Vec<_> = (0..count).map(|index| owned_place(index, index)).collect();
    let block_parameters: Vec<_> = (0..count)
        .map(|index| BlockParameter {
            id: ValueId::new(index),
            ty: owned_buf_type(),
            owner_place: Some(PlaceId::new(index)),
            origin: Origin::SYNTHETIC,
        })
        .collect();
    let function = Function {
        id: FunctionId::new(1),
        name: "wide-ownership-state".into(),
        signature: Signature::monomorphic(parameters, SsaType::Unit),
        places,
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: block_parameters,
            instructions: vec![Instruction {
                id: ValueId::new(count),
                ty: SsaType::Unit,
                kind: InstructionKind::Constant(Constant::Unit),
                metadata: metadata(EffectSet::PURE),
            }],
            terminator: Terminator::Return(ValueId::new(count)),
            metadata: block_metadata(),
        }],
        origin: Origin::SYNTHETIC,
    };
    let error = verify(ownership_program(function)).expect_err("wide state must be bounded");
    assert!(
        error.to_string().contains("ownership")
            && (error.to_string().contains("work") || error.to_string().contains("state")),
        "{error}"
    );
}

#[test]
fn ownership_verifier_bounds_cfg_shape_and_rejects_nested_function_laundering() {
    let function = Function {
        id: FunctionId::new(1),
        name: "too-many-blocks".into(),
        signature: Signature::monomorphic(Vec::new(), SsaType::Unit),
        places: Vec::new(),
        effects: EffectSet::MAY_TRAP,
        entry: BlockId::new(0),
        blocks: (0..=crate::SSA_VERIFY_MAX_BLOCKS_PER_FUNCTION)
            .map(|index| Block {
                id: BlockId::new(index as u32),
                parameters: Vec::new(),
                instructions: Vec::new(),
                terminator: Terminator::Trap {
                    message: "bounded".into(),
                },
                metadata: block_metadata(),
            })
            .collect(),
        origin: Origin::SYNTHETIC,
    };
    let error = verify(ownership_program(function)).expect_err("CFG block count must be bounded");
    assert!(error.to_string().contains("exceeds 4096 blocks"), "{error}");

    let mut nested_function = one_block_program();
    *nested_function.functions[0].signature.result =
        SsaType::Option(Box::new(SsaType::Function(Box::new(
            Signature::monomorphic(vec![owned_buf_type()], SsaType::Unit),
        ))));
    let error = verify(nested_function)
        .expect_err("collection-nested function signatures cannot launder ownership types");
    assert!(error.to_string().contains("storage position"), "{error}");

    let mut deeply_nested_function = one_block_program();
    let mut nested = SsaType::Unit;
    for _ in 0..70 {
        nested = SsaType::Function(Box::new(Signature::monomorphic(Vec::new(), nested)));
    }
    *deeply_nested_function.functions[0].signature.result = SsaType::Option(Box::new(nested));
    let error = verify(deeply_nested_function)
        .expect_err("nested function ownership scan must remain under type verifier bounds");
    assert!(
        error.to_string().contains("type nesting exceeds"),
        "{error}"
    );
}

#[test]
fn ownership_cfg_rejects_borrow_use_across_blocks() {
    let function = Function {
        id: FunctionId::new(1),
        name: "cross-block-loan".into(),
        signature: Signature::monomorphic(vec![owned_buf_type()], SsaType::I64),
        places: vec![owned_place(0, 0)],
        effects: EffectSet::READS_MEMORY,
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
                    ty: SsaType::Ref(Box::new(SsaType::Buf)),
                    kind: InstructionKind::Borrow {
                        place: PlaceId::new(0),
                        loan: LoanId::new(0),
                        kind: BorrowKind::Shared,
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
                instructions: vec![Instruction {
                    id: ValueId::new(2),
                    ty: SsaType::I64,
                    kind: InstructionKind::Runtime {
                        operation: RuntimeOp::OwnedBufLen,
                        arguments: vec![ValueId::new(1)],
                        signature: Signature::monomorphic(
                            vec![SsaType::Ref(Box::new(SsaType::Buf))],
                            SsaType::I64,
                        ),
                    },
                    metadata: metadata(EffectSet::READS_MEMORY),
                }],
                terminator: Terminator::Return(ValueId::new(2)),
                metadata: block_metadata(),
            },
        ],
        origin: Origin::SYNTHETIC,
    };
    let error = verify(ownership_program(function))
        .expect_err("Borrow result cannot cross blocks in the current slice");
    assert!(
        error.to_string().contains("outside its defining block"),
        "{error}"
    );
}

#[test]
fn verifier_accepts_dense_exact_program_and_evaluator_returns_exact_value() {
    let verified = verify(one_block_program()).expect("verify exact program");
    assert_eq!(
        evaluate(&verified, &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(42))
    );
}

#[test]
fn verifier_accepts_canonical_marker_witness_and_rejects_malformed_trait_facts() {
    let canonical = verify(bounded_call_program()).expect("verify canonical marker witness");
    assert_eq!(
        evaluate(&canonical, &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(9))
    );

    let mut unknown_trait = bounded_call_program();
    unknown_trait.functions[0].signature.bounds[0].trait_id = TraitId::new(99);
    assert!(verify(unknown_trait).is_err());

    let mut bounded_function_reference = bounded_call_program();
    let bounded_signature = bounded_function_reference.functions[0].signature.clone();
    bounded_function_reference.functions[1].signature = Signature::monomorphic(
        Vec::new(),
        SsaType::Function(Box::new(bounded_signature.clone())),
    );
    bounded_function_reference.functions[1].blocks[0].instructions = vec![Instruction {
        id: ValueId::new(0),
        ty: SsaType::Function(Box::new(bounded_signature)),
        kind: InstructionKind::FunctionRef(FunctionId::new(0)),
        metadata: metadata(EffectSet::PURE),
    }];
    bounded_function_reference.functions[1].blocks[0].terminator =
        Terminator::Return(ValueId::new(0));
    assert!(verify(bounded_function_reference).is_err());

    let mut deeply_nested_type = bounded_call_program();
    let mut nested = SsaType::I64;
    for _ in 0..70 {
        nested = SsaType::Option(Box::new(nested));
    }
    deeply_nested_type.products.push(ProductMetadata {
        id: crate::ProductId::new(0),
        name: "Deep".into(),
        fields: vec![crate::ProductField {
            name: "value".into(),
            ty: nested,
        }],
    });
    let deep_error = verify(deeply_nested_type).expect_err("deep type must be bounded");
    assert!(deep_error.to_string().contains("type nesting exceeds"));

    let mut unavailable_clone_bound = bounded_call_program();
    unavailable_clone_bound.functions[0].signature.bounds[0].trait_id = TraitId::new(1);
    assert!(verify(unavailable_clone_bound).is_err());

    let mut core_implementation = bounded_call_program();
    core_implementation.products.push(ProductMetadata {
        id: crate::ProductId::new(0),
        name: "Point".into(),
        fields: Vec::new(),
    });
    core_implementation.implementations.push(ImplMetadata {
        id: ImplId::new(0),
        trait_id: TraitId::new(1),
        product: crate::ProductId::new(0),
        source: 0,
    });
    assert!(verify(core_implementation).is_err());

    let mutate_instantiation =
        |program: &mut Program, mutate: &mut dyn FnMut(&mut GenericInstantiation)| {
            let InstructionKind::Call {
                instantiation: Some(instantiation),
                ..
            } = &mut program.functions[1].blocks[0].instructions[1].kind
            else {
                panic!("bounded call fixture lost instantiation");
            };
            mutate(instantiation);
        };

    let mut unknown_impl = bounded_call_program();
    mutate_instantiation(&mut unknown_impl, &mut |instantiation| {
        instantiation.witnesses[0].kind = TraitWitnessKind::Explicit(ImplId::new(99));
    });
    assert!(verify(unknown_impl).is_err());

    let mut wrong_witness_type = bounded_call_program();
    mutate_instantiation(&mut wrong_witness_type, &mut |instantiation| {
        instantiation.witnesses[0].ty = SsaType::Bool;
    });
    assert!(verify(wrong_witness_type).is_err());

    let mut duplicate_witness = bounded_call_program();
    mutate_instantiation(&mut duplicate_witness, &mut |instantiation| {
        instantiation
            .witnesses
            .push(instantiation.witnesses[0].clone());
    });
    assert!(verify(duplicate_witness).is_err());

    let mut wrong_substitution = bounded_call_program();
    mutate_instantiation(&mut wrong_substitution, &mut |instantiation| {
        instantiation.substitutions[0].parameter = "U".into();
    });
    assert!(verify(wrong_substitution).is_err());

    let mut missing_instantiation = bounded_call_program();
    if let InstructionKind::Call { instantiation, .. } =
        &mut missing_instantiation.functions[1].blocks[0].instructions[1].kind
    {
        *instantiation = None;
    }
    assert!(verify(missing_instantiation).is_err());
}

#[test]
fn verifier_rejects_generic_ownership_substitution() {
    let generic = Function {
        id: FunctionId::new(1),
        name: "generic-id".into(),
        signature: Signature {
            type_parameters: vec!["T".into()],
            bounds: Vec::new(),
            parameters: vec![SsaType::TypeParameter("T".into())],
            result: Box::new(SsaType::TypeParameter("T".into())),
        },
        places: Vec::new(),
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![Block {
            id: BlockId::new(0),
            parameters: vec![BlockParameter {
                id: ValueId::new(0),
                ty: SsaType::TypeParameter("T".into()),
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
        signature: Signature::monomorphic(vec![owned_buf_type()], owned_buf_type()),
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
                    ty: owned_buf_type(),
                    kind: InstructionKind::Call {
                        target: CallTarget::Direct(FunctionId::new(1)),
                        arguments: vec![ValueId::new(1)],
                        signature: Signature::monomorphic(vec![owned_buf_type()], owned_buf_type()),
                        instantiation: Some(GenericInstantiation {
                            substitutions: vec![TypeSubstitution {
                                parameter: "T".into(),
                                ty: owned_buf_type(),
                            }],
                            witnesses: Vec::new(),
                        }),
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

    let reference = SsaType::Ref(Box::new(SsaType::Buf));
    let reference_caller = Function {
        id: FunctionId::new(2),
        name: "generic-reference-caller".into(),
        signature: Signature::monomorphic(vec![reference.clone()], SsaType::I64),
        places: Vec::new(),
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
                        signature: Signature::monomorphic(
                            vec![reference.clone()],
                            reference.clone(),
                        ),
                        instantiation: Some(GenericInstantiation {
                            substitutions: vec![TypeSubstitution {
                                parameter: "T".into(),
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
                        operation: RuntimeOp::OwnedBufLen,
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
                signature: indirect_signature.clone(),
                instantiation: None,
            },
            metadata: InstructionMetadata {
                origin: Origin::SYNTHETIC,
                effects: EffectSet::CONSERVATIVE_CALL,
                safepoint: Safepoint::Required,
                failure: FailureBehavior::TrapOrOutcome,
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

#[test]
fn every_baseline_pass_is_isolated_verified_and_deterministic() {
    let verified = verify(pass_program()).expect("verify pass fixture");
    let original = verified.program().clone();

    let folded_a = constant_fold_and_propagate(&verified).expect("constant fold");
    let folded_b = constant_fold_and_propagate(&verified).expect("repeat constant fold");
    assert_eq!(folded_a.program(), folded_b.program());
    assert_eq!(verified.program(), &original);

    let copied = copy_propagate(&verified).expect("copy propagation");
    let reachable = unreachable_blocks(&verified).expect("unreachable blocks");
    let simplified = simplify_branches(&verified).expect("branch simplification");
    let forwarded = empty_block_forwarding(&verified).expect("empty forwarding");
    let dead = effect_aware_dce(&verified).expect("effect-aware DCE");
    let ordered = canonical_block_order(&verified).expect("canonical block order");
    for transformed in [copied, reachable, simplified, forwarded, dead, ordered] {
        assert!(!transformed.program().functions.is_empty());
    }

    let normalized_a = normalize_baseline(&verified).expect("normalize fixture");
    let normalized_b = normalize_baseline(&verified).expect("normalize fixture again");
    assert_eq!(normalized_a.program(), normalized_b.program());
    assert_eq!(
        evaluate(&normalized_a, &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(5))
    );
    assert_eq!(normalized_a.program().functions[0].blocks.len(), 2);
}

#[test]
fn direct_call_resolution_is_independent_verified_and_semantics_preserving() {
    let signature = Signature::monomorphic(Vec::new(), SsaType::I64);
    let call_effects = EffectSet::CONSERVATIVE_CALL;
    let program = Program {
        sources: Vec::new(),
        products: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![
            Function {
                id: FunctionId::new(0),
                name: "answer".into(),
                signature: signature.clone(),
                places: Vec::new(),
                effects: EffectSet::PURE,
                entry: BlockId::new(0),
                blocks: vec![Block {
                    id: BlockId::new(0),
                    parameters: Vec::new(),
                    instructions: vec![constant(0, 42)],
                    terminator: Terminator::Return(ValueId::new(0)),
                    metadata: block_metadata(),
                }],
                origin: Origin::SYNTHETIC,
            },
            Function {
                id: FunctionId::new(1),
                name: "main".into(),
                signature: signature.clone(),
                places: Vec::new(),
                effects: call_effects,
                entry: BlockId::new(0),
                blocks: vec![Block {
                    id: BlockId::new(0),
                    parameters: Vec::new(),
                    instructions: vec![
                        Instruction {
                            id: ValueId::new(0),
                            ty: SsaType::Function(Box::new(signature.clone())),
                            kind: InstructionKind::FunctionRef(FunctionId::new(0)),
                            metadata: metadata(EffectSet::PURE),
                        },
                        Instruction {
                            id: ValueId::new(1),
                            ty: SsaType::I64,
                            kind: InstructionKind::Call {
                                target: CallTarget::Indirect(ValueId::new(0)),
                                arguments: Vec::new(),
                                signature,
                                instantiation: None,
                            },
                            metadata: InstructionMetadata {
                                origin: Origin::SYNTHETIC,
                                effects: call_effects,
                                safepoint: Safepoint::Required,
                                failure: FailureBehavior::TrapOrOutcome,
                                frame_state: Some(FrameState {
                                    bytecode_position: 1,
                                    locals: Vec::new(),
                                    operand_stack: Vec::new(),
                                }),
                            },
                        },
                    ],
                    terminator: Terminator::Return(ValueId::new(1)),
                    metadata: block_metadata(),
                }],
                origin: Origin::SYNTHETIC,
            },
        ],
        main: FunctionId::new(1),
    };
    let verified = verify(program).expect("verify indirect call");
    let resolved = direct_call_resolution(&verified).expect("resolve direct call");
    let call = &resolved.program().functions[1].blocks[0].instructions[1];
    assert!(matches!(
        &call.kind,
        InstructionKind::Call {
            target: CallTarget::Direct(target),
            ..
        } if *target == FunctionId::new(0)
    ));
    assert_eq!(call.metadata.effects, EffectSet::PURE);
    assert_eq!(
        evaluate(&resolved, &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(42))
    );
    assert!(matches!(
        &verified.program().functions[1].blocks[0].instructions[1].kind,
        InstructionKind::Call {
            target: CallTarget::Indirect(_),
            ..
        }
    ));
}

#[test]
fn evaluator_executes_result_construction_and_projection_without_vm_helpers() {
    let result_type = SsaType::Result(Box::new(SsaType::I64), Box::new(SsaType::Str));
    let allocation_effects = EffectSet::ALLOCATES.union(EffectSet::MAY_TRAP);
    let program = Program {
        sources: Vec::new(),
        products: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![Function {
            id: FunctionId::new(0),
            name: "main".into(),
            signature: Signature::monomorphic(Vec::new(), SsaType::Bool),
            places: Vec::new(),
            effects: allocation_effects,
            entry: BlockId::new(0),
            blocks: vec![Block {
                id: BlockId::new(0),
                parameters: Vec::new(),
                instructions: vec![
                    constant(0, 7),
                    Instruction {
                        id: ValueId::new(1),
                        ty: result_type.clone(),
                        kind: InstructionKind::Runtime {
                            operation: RuntimeOp::Ok,
                            arguments: vec![ValueId::new(0)],
                            signature: Signature::monomorphic(
                                vec![SsaType::I64],
                                result_type.clone(),
                            ),
                        },
                        metadata: InstructionMetadata {
                            origin: Origin::SYNTHETIC,
                            effects: allocation_effects,
                            safepoint: Safepoint::Required,
                            failure: FailureBehavior::TrapOrOutcome,
                            frame_state: Some(FrameState {
                                bytecode_position: 1,
                                locals: Vec::new(),
                                operand_stack: Vec::new(),
                            }),
                        },
                    },
                    Instruction {
                        id: ValueId::new(2),
                        ty: SsaType::Bool,
                        kind: InstructionKind::Runtime {
                            operation: RuntimeOp::IsOk,
                            arguments: vec![ValueId::new(1)],
                            signature: Signature::monomorphic(vec![result_type], SsaType::Bool),
                        },
                        metadata: metadata(EffectSet::READS_MEMORY),
                    },
                ],
                terminator: Terminator::Return(ValueId::new(2)),
                metadata: block_metadata(),
            }],
            origin: Origin::SYNTHETIC,
        }],
        main: FunctionId::new(0),
    };
    let verified = verify(program).expect("verify Result evaluator program");
    assert_eq!(
        evaluate(&verified, &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::Bool(true))
    );
}

#[test]
fn evaluator_bounds_and_explicit_trap_exit_outcomes_are_deterministic() {
    let mut trapped = one_block_program();
    trapped.functions[0].blocks[0].instructions.clear();
    trapped.functions[0].blocks[0].terminator = Terminator::Trap {
        message: "explicit trap".into(),
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

fn optimizable_checked_program() -> Program {
    let checked = EffectSet::MAY_TRAP;
    let runtime = |id, operation, arguments, effects| Instruction {
        id: ValueId::new(id),
        ty: SsaType::I64,
        kind: InstructionKind::Runtime {
            operation,
            arguments,
            signature: Signature::monomorphic(vec![SsaType::I64, SsaType::I64], SsaType::I64),
        },
        metadata: metadata(effects),
    };
    Program {
        sources: Vec::new(),
        products: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![
            Function {
                id: FunctionId::new(0),
                name: "step".into(),
                signature: Signature::monomorphic(vec![SsaType::I64], SsaType::I64),
                places: Vec::new(),
                effects: checked,
                entry: BlockId::new(0),
                blocks: vec![Block {
                    id: BlockId::new(0),
                    parameters: vec![BlockParameter {
                        id: ValueId::new(0),
                        ty: SsaType::I64,
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    }],
                    instructions: vec![
                        constant(1, 0),
                        runtime(
                            2,
                            RuntimeOp::BitXor,
                            vec![ValueId::new(0), ValueId::new(1)],
                            EffectSet::PURE,
                        ),
                        constant(3, -1),
                        runtime(
                            4,
                            RuntimeOp::BitAnd,
                            vec![ValueId::new(2), ValueId::new(3)],
                            EffectSet::PURE,
                        ),
                        runtime(
                            5,
                            RuntimeOp::Add,
                            vec![ValueId::new(4), ValueId::new(0)],
                            checked,
                        ),
                        runtime(
                            6,
                            RuntimeOp::Add,
                            vec![ValueId::new(4), ValueId::new(0)],
                            checked,
                        ),
                        runtime(
                            7,
                            RuntimeOp::BitXor,
                            vec![ValueId::new(5), ValueId::new(6)],
                            EffectSet::PURE,
                        ),
                    ],
                    terminator: Terminator::Return(ValueId::new(7)),
                    metadata: block_metadata(),
                }],
                origin: Origin::SYNTHETIC,
            },
            Function {
                id: FunctionId::new(1),
                name: "main".into(),
                signature: Signature::monomorphic(Vec::new(), SsaType::I64),
                places: Vec::new(),
                effects: checked,
                entry: BlockId::new(0),
                blocks: vec![Block {
                    id: BlockId::new(0),
                    parameters: Vec::new(),
                    instructions: vec![
                        constant(0, 9),
                        Instruction {
                            id: ValueId::new(1),
                            ty: SsaType::I64,
                            kind: InstructionKind::Call {
                                target: CallTarget::Direct(FunctionId::new(0)),
                                arguments: vec![ValueId::new(0)],
                                signature: Signature::monomorphic(vec![SsaType::I64], SsaType::I64),
                                instantiation: None,
                            },
                            metadata: InstructionMetadata {
                                origin: Origin::SYNTHETIC,
                                effects: checked,
                                safepoint: Safepoint::Required,
                                failure: FailureBehavior::Trap,
                                frame_state: Some(FrameState {
                                    bytecode_position: 0,
                                    locals: Vec::new(),
                                    operand_stack: Vec::new(),
                                }),
                            },
                        },
                    ],
                    terminator: Terminator::Return(ValueId::new(1)),
                    metadata: block_metadata(),
                }],
                origin: Origin::SYNTHETIC,
            },
        ],
        main: FunctionId::new(1),
    }
}

#[test]
fn proof_optimization_is_deterministic_and_evaluator_equivalent() {
    let input = verify(optimizable_checked_program()).expect("verify optimization input");
    let first = optimize(&input, OptimizationLimits::default()).expect("optimize first");
    let second = optimize(&input, OptimizationLimits::default()).expect("optimize second");
    assert_eq!(first.program(), second.program());
    assert_eq!(first.certificate(), second.certificate());
    assert_eq!(first.stats(), second.stats());
    assert_eq!(first.stats().algebraic_rewrites, 2);
    assert_eq!(first.stats().checked_i64_rewrites, 1);
    assert!(first.stats().output_instructions < first.stats().input_instructions);
    assert_eq!(
        evaluate(&input, &EvalConfig::default()),
        evaluate(first.verified_program(), &EvalConfig::default())
    );

    let mut division = optimizable_checked_program();
    for instruction in &mut division.functions[0].blocks[0].instructions {
        if matches!(instruction.id.raw(), 5 | 6) {
            let InstructionKind::Runtime { operation, .. } = &mut instruction.kind else {
                panic!("checked division fixture lost runtime instruction");
            };
            *operation = RuntimeOp::Divide;
        }
    }
    let division = verify(division).expect("verify duplicate checked division");
    let optimized_division = optimize(&division, OptimizationLimits::default())
        .expect("optimize duplicate checked division");
    assert_eq!(optimized_division.stats().checked_i64_rewrites, 1);
    assert_eq!(
        evaluate(&division, &EvalConfig::default()),
        evaluate(
            optimized_division.verified_program(),
            &EvalConfig::default()
        )
    );

    let mut trapping_division = division.into_program();
    trapping_division.functions[1].blocks[0].instructions[0].kind =
        InstructionKind::Constant(Constant::I64(0));
    let trapping_division = verify(trapping_division).expect("verify trapping division");
    let optimized_trap = optimize(&trapping_division, OptimizationLimits::default())
        .expect("optimize trapping checked division");
    assert!(matches!(
        evaluate(&trapping_division, &EvalConfig::default()),
        EvalOutcome::Trapped(_)
    ));
    assert_eq!(
        evaluate(&trapping_division, &EvalConfig::default()),
        evaluate(optimized_trap.verified_program(), &EvalConfig::default())
    );
}

#[test]
fn forged_stale_wrong_operation_operand_and_nondominating_certificates_fail_closed() {
    let input = verify(optimizable_checked_program()).expect("verify optimization input");
    let optimized = optimize(&input, OptimizationLimits::default()).expect("optimize");
    let candidate = optimized.program().clone();

    let reject = |certificate: OptimizationCertificate| {
        let error = verify_optimization(
            &input,
            candidate.clone(),
            certificate,
            OptimizationLimits::default(),
        )
        .expect_err("forged certificate must fail");
        assert!(matches!(
            error.code(),
            OptimizationFailureCode::CertificateMismatch
                | OptimizationFailureCode::IllegalEdit
                | OptimizationFailureCode::CandidateMismatch
        ));
    };

    let mut stale = optimized.certificate().clone();
    stale.records[0].value = ValueId::new(u32::MAX);
    reject(stale);

    let mut wrong_operation = optimized.certificate().clone();
    wrong_operation.records[0].expected_operation = RuntimeOp::BitOr;
    reject(wrong_operation);

    let mut wrong_operand = optimized.certificate().clone();
    wrong_operand.records[0].expected_operands[0] = ValueId::new(7);
    reject(wrong_operand);

    let mut nondominating = optimized.certificate().clone();
    nondominating.records[0].replacement = ValueId::new(7);
    reject(nondominating);

    let mut wrong_candidate = candidate;
    wrong_candidate.functions[0].name = "stale-step".into();
    let error = verify_optimization(
        &input,
        wrong_candidate,
        optimized.certificate().clone(),
        OptimizationLimits::default(),
    )
    .expect_err("stale candidate must fail");
    assert_eq!(error.code(), OptimizationFailureCode::CandidateMismatch);
}

#[test]
fn forged_effectful_edit_and_all_optimization_budgets_fail_closed() {
    let allocation_effects = EffectSet::ALLOCATES.union(EffectSet::MAY_TRAP);
    let mut program = one_block_program();
    program.functions[0].signature = Signature::monomorphic(Vec::new(), SsaType::Str);
    program.functions[0].effects = allocation_effects;
    program.functions[0].blocks[0].instructions = vec![Instruction {
        id: ValueId::new(0),
        ty: SsaType::Str,
        kind: InstructionKind::Runtime {
            operation: RuntimeOp::EmptyStr,
            arguments: Vec::new(),
            signature: Signature::monomorphic(Vec::new(), SsaType::Str),
        },
        metadata: InstructionMetadata {
            origin: Origin::SYNTHETIC,
            effects: allocation_effects,
            safepoint: Safepoint::Required,
            failure: FailureBehavior::TrapOrOutcome,
            frame_state: Some(FrameState {
                bytecode_position: 0,
                locals: Vec::new(),
                operand_stack: Vec::new(),
            }),
        },
    }];
    let input = verify(program).expect("verify effectful input");
    let unmodified = optimize(&input, OptimizationLimits::default()).expect("no effectful edits");
    assert!(unmodified.certificate().records.is_empty());
    let forged = OptimizationCertificate {
        records: vec![OptimizationCertificateRecord {
            sequence: 0,
            function: FunctionId::new(0),
            block: BlockId::new(0),
            value: ValueId::new(0),
            kind: OptimizationEditKind::GlobalValueNumbering,
            expected_operation: RuntimeOp::EmptyStr,
            expected_operands: Vec::new(),
            replacement: ValueId::new(0),
        }],
    };
    let error = verify_optimization(
        &input,
        unmodified.program().clone(),
        forged,
        OptimizationLimits::default(),
    )
    .expect_err("allocation certificate must fail");
    assert_eq!(error.code(), OptimizationFailureCode::CertificateMismatch);

    let optimizable = verify(optimizable_checked_program()).expect("verify budget input");
    for limits in [
        OptimizationLimits {
            max_work_units: 0,
            ..OptimizationLimits::default()
        },
        OptimizationLimits {
            max_certificate_records: 0,
            ..OptimizationLimits::default()
        },
        OptimizationLimits {
            max_certificate_bytes: 0,
            ..OptimizationLimits::default()
        },
        OptimizationLimits {
            max_iterations: 0,
            ..OptimizationLimits::default()
        },
    ] {
        let error = optimize(&optimizable, limits).expect_err("budget must fail closed");
        assert_eq!(error.code(), OptimizationFailureCode::BudgetExceeded);
    }
}

#[test]
fn dominator_ordered_checked_gvn_accepts_dominance_and_rejects_siblings() {
    let checked = EffectSet::MAY_TRAP;
    let add = |id| Instruction {
        id: ValueId::new(id),
        ty: SsaType::I64,
        kind: InstructionKind::Runtime {
            operation: RuntimeOp::Add,
            arguments: vec![ValueId::new(0), ValueId::new(0)],
            signature: Signature::monomorphic(vec![SsaType::I64, SsaType::I64], SsaType::I64),
        },
        metadata: metadata(checked),
    };
    let dominating = Program {
        sources: Vec::new(),
        products: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![Function {
            id: FunctionId::new(0),
            name: "main".into(),
            signature: Signature::monomorphic(Vec::new(), SsaType::I64),
            places: Vec::new(),
            effects: checked,
            entry: BlockId::new(0),
            blocks: vec![
                Block {
                    id: BlockId::new(0),
                    parameters: Vec::new(),
                    instructions: vec![constant(0, 9), add(1)],
                    terminator: Terminator::Branch {
                        target: BlockId::new(1),
                        arguments: Vec::new(),
                    },
                    metadata: block_metadata(),
                },
                Block {
                    id: BlockId::new(1),
                    parameters: Vec::new(),
                    instructions: vec![add(2)],
                    terminator: Terminator::Return(ValueId::new(2)),
                    metadata: block_metadata(),
                },
            ],
            origin: Origin::SYNTHETIC,
        }],
        main: FunctionId::new(0),
    };
    let dominating = verify(dominating).expect("verify dominating GVN input");
    let optimized = optimize(&dominating, OptimizationLimits::default()).expect("dominating GVN");
    assert_eq!(optimized.stats().checked_i64_rewrites, 1);
    assert_eq!(
        evaluate(optimized.verified_program(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(18))
    );

    let siblings = Program {
        sources: Vec::new(),
        products: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![Function {
            id: FunctionId::new(0),
            name: "main".into(),
            signature: Signature::monomorphic(Vec::new(), SsaType::I64),
            places: Vec::new(),
            effects: checked,
            entry: BlockId::new(0),
            blocks: vec![
                Block {
                    id: BlockId::new(0),
                    parameters: Vec::new(),
                    instructions: vec![
                        constant(0, 9),
                        Instruction {
                            id: ValueId::new(1),
                            ty: SsaType::Bool,
                            kind: InstructionKind::Constant(Constant::Bool(true)),
                            metadata: metadata(EffectSet::PURE),
                        },
                    ],
                    terminator: Terminator::ConditionalBranch {
                        condition: ValueId::new(1),
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
                    instructions: vec![add(2)],
                    terminator: Terminator::Branch {
                        target: BlockId::new(3),
                        arguments: vec![ValueId::new(2)],
                    },
                    metadata: block_metadata(),
                },
                Block {
                    id: BlockId::new(2),
                    parameters: Vec::new(),
                    instructions: vec![add(3)],
                    terminator: Terminator::Branch {
                        target: BlockId::new(3),
                        arguments: vec![ValueId::new(3)],
                    },
                    metadata: block_metadata(),
                },
                Block {
                    id: BlockId::new(3),
                    parameters: vec![BlockParameter {
                        id: ValueId::new(4),
                        ty: SsaType::I64,
                        owner_place: None,
                        origin: Origin::SYNTHETIC,
                    }],
                    instructions: Vec::new(),
                    terminator: Terminator::Return(ValueId::new(4)),
                    metadata: block_metadata(),
                },
            ],
            origin: Origin::SYNTHETIC,
        }],
        main: FunctionId::new(0),
    };
    let siblings = verify(siblings).expect("verify sibling GVN input");
    let sibling_output =
        optimize(&siblings, OptimizationLimits::default()).expect("optimize siblings");
    assert_eq!(sibling_output.stats().gvn_rewrites, 0);
    let forged = OptimizationCertificate {
        records: vec![OptimizationCertificateRecord {
            sequence: 0,
            function: FunctionId::new(0),
            block: BlockId::new(2),
            value: ValueId::new(3),
            kind: OptimizationEditKind::CheckedI64GlobalValueNumbering,
            expected_operation: RuntimeOp::Add,
            expected_operands: vec![ValueId::new(0), ValueId::new(0)],
            replacement: ValueId::new(2),
        }],
    };
    let error = verify_optimization(
        &siblings,
        sibling_output.program().clone(),
        forged,
        OptimizationLimits::default(),
    )
    .expect_err("sibling expression does not dominate");
    assert_eq!(error.code(), OptimizationFailureCode::CertificateMismatch);
}

#[test]
fn optimization_candidate_equality_is_exact_for_f64_bits() {
    let mut program = one_block_program();
    program.functions[0].signature = Signature::monomorphic(Vec::new(), SsaType::F64);
    program.functions[0].blocks[0].instructions[0] = Instruction {
        id: ValueId::new(0),
        ty: SsaType::F64,
        kind: InstructionKind::Constant(Constant::F64(-0.0)),
        metadata: metadata(EffectSet::PURE),
    };
    let input = verify(program).expect("verify signed-zero proof input");
    let optimized = optimize(&input, OptimizationLimits::default()).expect("optimize signed zero");
    let mut forged = optimized.program().clone();
    let InstructionKind::Constant(Constant::F64(value)) =
        &mut forged.functions[0].blocks[0].instructions[0].kind
    else {
        panic!("expected retained F64 constant");
    };
    *value = 0.0;
    let error = verify_optimization(
        &input,
        forged,
        optimized.certificate().clone(),
        OptimizationLimits::default(),
    )
    .expect_err("signed-zero candidate forgery must fail");
    assert_eq!(error.code(), OptimizationFailureCode::CandidateMismatch);
}
