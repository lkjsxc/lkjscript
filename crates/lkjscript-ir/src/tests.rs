#![allow(clippy::expect_used, clippy::panic)]

use crate::{
    canonical_block_order, constant_fold_and_propagate, copy_propagate, direct_call_resolution,
    effect_aware_dce, empty_block_forwarding, evaluate, normalize_baseline, simplify_branches,
    unreachable_blocks, verify, Block, BlockId, BlockMetadata, BlockParameter, CallTarget,
    Constant, EffectSet, EvalConfig, EvalOutcome, EvalValue, FailureBehavior, FrameState, Function,
    FunctionId, GenericInstantiation, ImplId, ImplMetadata, Instruction, InstructionKind,
    InstructionMetadata, Origin, ProductMetadata, Program, RuntimeOp, Safepoint, Signature,
    SourceMetadata, SsaType, Terminator, TraitBound, TraitId, TraitMetadata, TraitRole,
    TraitWitness, TraitWitnessKind, TypeSubstitution, ValueId,
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
                effects: EffectSet::PURE,
                entry: BlockId::new(0),
                blocks: vec![Block {
                    id: BlockId::new(0),
                    parameters: vec![BlockParameter {
                        id: ValueId::new(0),
                        ty: SsaType::TypeParameter("T".into()),
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
