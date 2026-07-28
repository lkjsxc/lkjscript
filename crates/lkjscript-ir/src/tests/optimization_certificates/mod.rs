use super::fixtures::*;
use crate::*;

mod scheduled;

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
            failure_cleanup: None,
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
            max_certificate_bytes_estimate: 0,
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
