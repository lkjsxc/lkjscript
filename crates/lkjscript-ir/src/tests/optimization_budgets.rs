use super::fixtures::*;
use crate::*;

#[test]
fn optimization_preflight_growth_and_quadratic_probe_budgets_fail_closed() {
    let input = verify(one_block_program()).expect("verify small preflight input");
    let optimized = optimize(&input, OptimizationLimits::default()).expect("optimize small input");
    let mut oversized = optimized.program().clone();
    oversized.functions[0].name = "x".repeat(1_024);
    let error = verify_optimization(
        &input,
        oversized,
        optimized.certificate().clone(),
        OptimizationLimits {
            max_string_and_metadata_bytes: 128,
            ..OptimizationLimits::default()
        },
    )
    .expect_err("candidate string bytes are preflight bounded");
    assert_eq!(error.code(), OptimizationFailureCode::BudgetExceeded);

    let mut excessive_type_parameters = optimized.program().clone();
    excessive_type_parameters.functions[0]
        .signature
        .type_parameters = vec![String::new(); 1_024];
    let error = verify_optimization(
        &input,
        excessive_type_parameters,
        optimized.certificate().clone(),
        OptimizationLimits {
            max_metadata_items: 128,
            ..OptimizationLimits::default()
        },
    )
    .expect_err("candidate type-parameter entries are preflight bounded");
    assert_eq!(error.code(), OptimizationFailureCode::BudgetExceeded);

    let mut grown = input.program().clone();
    grown.functions[0].blocks[0].instructions.push(Instruction {
        id: ValueId::new(1),
        ty: SsaType::I64,
        kind: InstructionKind::Copy(ValueId::new(0)),
        metadata: metadata(EffectSet::PURE),
    });
    let error = verify_optimization(
        &input,
        grown,
        OptimizationCertificate::default(),
        OptimizationLimits::default(),
    )
    .expect_err("candidate instruction growth is bounded before verification");
    assert_eq!(error.code(), OptimizationFailureCode::BudgetExceeded);

    let mut wide = one_block_program();
    let mut instructions = vec![constant(0, 2), constant(1, 3)];
    for raw in 2..66 {
        instructions.push(runtime(
            raw,
            RuntimeOp::BitXor,
            vec![ValueId::new(0), ValueId::new(1)],
            EffectSet::PURE,
        ));
    }
    wide.functions[0].blocks[0].instructions = instructions;
    wide.functions[0].blocks[0].terminator = Terminator::Return(ValueId::new(65));
    let wide = verify(wide).expect("verify wide duplicate-expression input");
    let optimized = optimize(&wide, OptimizationLimits::default()).expect("bounded wide optimize");
    assert_eq!(optimized.certificate().records.len(), 63);
    assert_eq!(optimized.stats().discovery_passes, 1);
    assert_eq!(optimized.stats().checker_passes, 1);
    assert_eq!(optimized.stats().reconstruction_passes, 2);
    assert_eq!(optimized.stats().cleanup_passes, 14);
    assert_eq!(optimized.stats().iterations, 14);
    assert_eq!(
        optimized.stats().optimizing_passes,
        optimized.stats().discovery_passes
            + optimized.stats().checker_passes
            + optimized.stats().reconstruction_passes
            + optimized.stats().cleanup_passes
            + optimized.stats().validation_passes
    );
    let error = optimize(
        &wide,
        OptimizationLimits {
            max_work_units: 2_000,
            ..OptimizationLimits::default()
        },
    )
    .expect_err("wide optimization exceeds a small aggregate cap");
    assert_eq!(error.code(), OptimizationFailureCode::BudgetExceeded);

    let mut quadratic = one_block_program();
    let mut instructions = vec![constant(0, 2), constant(1, 3)];
    for raw in 2..1_026 {
        instructions.push(runtime(
            raw,
            RuntimeOp::BitXor,
            vec![ValueId::new(0), ValueId::new(1)],
            EffectSet::PURE,
        ));
    }
    quadratic.functions[0].blocks[0].instructions = instructions;
    quadratic.functions[0].blocks[0].terminator = Terminator::Return(ValueId::new(1_025));
    let quadratic = verify(quadratic).expect("verify quadratic expression-probe input");
    let error = optimize(
        &quadratic,
        OptimizationLimits {
            max_work_units: 200_000,
            ..OptimizationLimits::default()
        },
    )
    .expect_err("quadratic candidate probes stop at the aggregate work cap");
    assert_eq!(error.code(), OptimizationFailureCode::BudgetExceeded);
}

#[test]
fn unreachable_diamond_and_loop_are_cleaned_deterministically() {
    let branch = |id, target| Block {
        id: BlockId::new(id),
        parameters: Vec::new(),
        instructions: Vec::new(),
        terminator: Terminator::Branch {
            target: BlockId::new(target),
            arguments: Vec::new(),
        },
        metadata: block_metadata(),
    };
    let unreachable_loop_entry = branch(5, 6);
    let program = Program {
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
            effects: EffectSet::PURE,
            entry: BlockId::new(0),
            blocks: vec![
                Block {
                    id: BlockId::new(0),
                    parameters: Vec::new(),
                    instructions: vec![constant(0, 9)],
                    terminator: Terminator::Return(ValueId::new(0)),
                    metadata: block_metadata(),
                },
                Block {
                    id: BlockId::new(1),
                    parameters: Vec::new(),
                    instructions: vec![Instruction {
                        id: ValueId::new(1),
                        ty: SsaType::Bool,
                        kind: InstructionKind::Constant(Constant::Bool(true)),
                        metadata: metadata(EffectSet::PURE),
                    }],
                    terminator: Terminator::ConditionalBranch {
                        condition: ValueId::new(1),
                        true_target: BlockId::new(2),
                        true_arguments: Vec::new(),
                        false_target: BlockId::new(3),
                        false_arguments: Vec::new(),
                    },
                    metadata: block_metadata(),
                },
                branch(2, 4),
                branch(3, 4),
                Block {
                    id: BlockId::new(4),
                    parameters: Vec::new(),
                    instructions: vec![constant(2, 17)],
                    terminator: Terminator::Return(ValueId::new(2)),
                    metadata: block_metadata(),
                },
                unreachable_loop_entry,
                branch(6, 5),
            ],
            origin: Origin::SYNTHETIC,
        }],
        main: FunctionId::new(0),
    };
    let input = verify(program).expect("ordinary verifier accepts unreachable components");
    let first = optimize(&input, OptimizationLimits::default()).expect("clean unreachable CFG");
    let second = optimize(&input, OptimizationLimits::default()).expect("repeat unreachable CFG");
    assert_eq!(first.program(), second.program());
    assert_eq!(first.program().functions[0].blocks.len(), 1);
    assert!(first.certificate().records.is_empty());
    assert_eq!(
        evaluate(first.verified_program(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(9))
    );
}
