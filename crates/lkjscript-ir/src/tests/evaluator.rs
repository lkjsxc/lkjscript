use super::fixtures::*;
use crate::*;

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
