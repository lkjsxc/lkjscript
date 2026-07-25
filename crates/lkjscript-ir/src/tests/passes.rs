use super::fixtures::*;
use crate::*;

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
