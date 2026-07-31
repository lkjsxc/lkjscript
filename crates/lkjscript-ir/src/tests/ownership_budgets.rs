use super::fixtures::*;
use crate::*;

#[test]
fn ownership_verification_has_an_aggregate_state_work_bound() {
    let count = 22_000_u32;
    let parameters: Vec<_> = (0..count).map(|_| byte_vector_type()).collect();
    let places: Vec<_> = (0..count).map(|index| owned_place(index, index)).collect();
    let block_parameters: Vec<_> = (0..count)
        .map(|index| BlockParameter {
            id: ValueId::new(index),
            ty: byte_vector_type(),
            owner_place: Some(PlaceId::new(index)),
            origin: Origin::SYNTHETIC,
        })
        .collect();
    let function = Function {
        id: FunctionId::new(1),
        name: "wide-ownership-state".into(),
        signature: Signature::monomorphic(parameters, SsaType::Unit),
        places,
        failure_cleanups: Vec::new(),
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
        failure_cleanups: Vec::new(),
        effects: EffectSet::MAY_TRAP,
        entry: BlockId::new(0),
        blocks: (0..=crate::SSA_VERIFY_MAX_BLOCKS_PER_FUNCTION)
            .map(|index| Block {
                id: BlockId::new(index as u32),
                parameters: Vec::new(),
                instructions: Vec::new(),
                terminator: Terminator::Trap {
                    value: ValueId::new(0),
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
        SsaType::List(Box::new(SsaType::Function(Box::new(
            Signature::monomorphic(vec![byte_vector_type()], SsaType::Unit),
        ))));
    let error = verify(nested_function)
        .expect_err("collection-nested function signatures cannot launder ownership types");
    assert!(error.to_string().contains("storage position"), "{error}");

    let mut deeply_nested_function = one_block_program();
    let mut nested = SsaType::Unit;
    for _ in 0..70 {
        nested = SsaType::Function(Box::new(Signature::monomorphic(Vec::new(), nested)));
    }
    *deeply_nested_function.functions[0].signature.result = SsaType::List(Box::new(nested));
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
        signature: Signature::monomorphic(vec![byte_vector_type()], SsaType::I64),
        places: vec![owned_place(0, 0)],
        failure_cleanups: Vec::new(),
        effects: EffectSet::READS_MEMORY,
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
                    ty: SsaType::ByteSlice,
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
                        operation: RuntimeOp::ByteSliceLength,
                        arguments: vec![ValueId::new(1)],
                        signature: Signature::monomorphic(vec![SsaType::ByteSlice], SsaType::I64),
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
