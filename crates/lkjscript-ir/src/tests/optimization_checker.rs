use super::fixtures::*;
use crate::*;

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

#[test]
fn enum_operations_and_metadata_are_proof_preserved_exactly() {
    let mut program = one_block_program();
    program.enums.push(enum_metadata());
    program.functions[0].signature = Signature::monomorphic(Vec::new(), enum_type());
    program.functions[0].effects = EffectSet::ALLOCATES;
    program.functions[0].blocks[0].instructions = vec![
        constant(0, 42),
        Instruction {
            id: ValueId::new(1),
            ty: enum_type(),
            kind: InstructionKind::EnumValue {
                enum_id: EnumId::new([1; 32]),
                variant: VariantId::new([2; 32]),
                layout: RuntimeLayoutId::new([6; 32]),
                fields: vec![ValueId::new(0)],
            },
            metadata: InstructionMetadata {
                origin: Origin::SYNTHETIC,
                effects: EffectSet::ALLOCATES,
                safepoint: Safepoint::Required,
                failure: FailureBehavior::StructuredOutcome,
                failure_cleanup: None,
                frame_state: Some(FrameState {
                    bytecode_position: 0,
                    locals: Vec::new(),
                    operand_stack: Vec::new(),
                }),
            },
        },
    ];
    program.functions[0].blocks[0].terminator = Terminator::Return(ValueId::new(1));
    let input = verify(program).expect("verify enum proof input");
    let optimized = optimize(&input, OptimizationLimits::default()).expect("optimize enum");
    assert!(optimized.certificate().records.is_empty());
    assert!(matches!(
        optimized.program().functions[0].blocks[0].instructions[1].kind,
        InstructionKind::EnumValue { .. }
    ));

    let mut forged = optimized.program().clone();
    forged.enums[0].name = "Forged".into();
    let error = verify_optimization(
        &input,
        forged,
        optimized.certificate().clone(),
        OptimizationLimits::default(),
    )
    .expect_err("enum metadata forgery must fail exact reconstruction");
    assert_eq!(error.code(), OptimizationFailureCode::CandidateMismatch);
}

#[test]
fn independent_checker_rejects_plausible_xor_and_commutative_gvn_discovery_bugs() {
    let mut xor = one_block_program();
    xor.functions[0].blocks[0].instructions = vec![
        constant(0, 7),
        runtime(
            1,
            RuntimeOp::BitXor,
            vec![ValueId::new(0), ValueId::new(0)],
            EffectSet::PURE,
        ),
    ];
    xor.functions[0].blocks[0].terminator = Terminator::Return(ValueId::new(1));
    let xor = verify(xor).expect("verify same-operand xor");
    let unchanged = optimize(&xor, OptimizationLimits::default()).expect("optimize xor");
    assert!(unchanged.certificate().records.is_empty());
    let plausible_xor_identity = OptimizationCertificate {
        records: vec![OptimizationCertificateRecord {
            sequence: 0,
            function: FunctionId::new(0),
            block: BlockId::new(0),
            value: ValueId::new(1),
            kind: OptimizationEditKind::AlgebraicIdentity,
            expected_operation: RuntimeOp::BitXor,
            expected_operands: vec![ValueId::new(0), ValueId::new(0)],
            replacement: ValueId::new(0),
        }],
    };
    let error = verify_optimization(
        &xor,
        unchanged.program().clone(),
        plausible_xor_identity,
        OptimizationLimits::default(),
    )
    .expect_err("xor x x is not the identity x");
    assert_eq!(error.code(), OptimizationFailureCode::CertificateMismatch);

    let checked = EffectSet::MAY_TRAP;
    let mut commutative = one_block_program();
    commutative.functions[0].effects = checked;
    commutative.functions[0].blocks[0].instructions = vec![
        constant(0, 2),
        constant(1, 3),
        runtime(
            2,
            RuntimeOp::Add,
            vec![ValueId::new(0), ValueId::new(1)],
            checked,
        ),
        runtime(
            3,
            RuntimeOp::Add,
            vec![ValueId::new(1), ValueId::new(0)],
            checked,
        ),
    ];
    commutative.functions[0].blocks[0].terminator = Terminator::Return(ValueId::new(3));
    let commutative = verify(commutative).expect("verify commuted checked additions");
    let unchanged = optimize(&commutative, OptimizationLimits::default())
        .expect("optimizer requires exact operand order");
    assert!(unchanged.certificate().records.is_empty());
    let plausible_commutative_gvn = OptimizationCertificate {
        records: vec![OptimizationCertificateRecord {
            sequence: 0,
            function: FunctionId::new(0),
            block: BlockId::new(0),
            value: ValueId::new(3),
            kind: OptimizationEditKind::CheckedI64GlobalValueNumbering,
            expected_operation: RuntimeOp::Add,
            expected_operands: vec![ValueId::new(1), ValueId::new(0)],
            replacement: ValueId::new(2),
        }],
    };
    let error = verify_optimization(
        &commutative,
        unchanged.program().clone(),
        plausible_commutative_gvn,
        OptimizationLimits::default(),
    )
    .expect_err("checker does not invent commutativity");
    assert_eq!(error.code(), OptimizationFailureCode::CertificateMismatch);
}
