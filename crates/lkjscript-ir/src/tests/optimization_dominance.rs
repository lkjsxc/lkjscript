use super::fixtures::*;
use crate::*;

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
        enums: Vec::new(),
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
        enums: Vec::new(),
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
