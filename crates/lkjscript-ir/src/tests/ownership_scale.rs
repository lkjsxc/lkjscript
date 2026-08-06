use super::fixtures::*;
use crate::*;

#[test]
fn ownership_verification_accepts_more_than_former_retained_state_and_work_boundaries() {
    const OWNED_PARAMETERS: u32 = 44_000;
    const STATE_CELLS_PER_BLOCK: usize = 3 * OWNED_PARAMETERS as usize;
    const FORMER_RETAINED_STATE_CELLS: usize = 2 * STATE_CELLS_PER_BLOCK;
    const FORMER_PRECHECK_WORK_AT_FIRST_PLAN: usize = 3 * OWNED_PARAMETERS as usize + 4;
    const _: () = assert!(STATE_CELLS_PER_BLOCK > 131_072);
    const _: () = assert!(FORMER_RETAINED_STATE_CELLS > 131_072);
    const _: () = assert!(FORMER_PRECHECK_WORK_AT_FIRST_PLAN > 131_072);

    let resource_kind = lkjscript_contracts::ResourceKind::FileReader;
    let resource_type = SsaType::Resource(resource_kind);
    let parameters = vec![resource_type.clone(); OWNED_PARAMETERS as usize];
    let places: Vec<_> = (0..OWNED_PARAMETERS)
        .map(|index| PlaceMetadata {
            id: PlaceId::new(index),
            binding: BindingId::new(index),
            ty: resource_type.clone(),
            drop_glue: Some(DropGlueIdentity::Resource(resource_kind)),
        })
        .collect();
    let entry_parameters: Vec<_> = (0..OWNED_PARAMETERS)
        .map(|index| BlockParameter {
            id: ValueId::new(index),
            ty: resource_type.clone(),
            owner_place: Some(PlaceId::new(index)),
            origin: Origin::SYNTHETIC,
        })
        .collect();
    let successor_parameters: Vec<_> = (0..OWNED_PARAMETERS)
        .map(|index| BlockParameter {
            id: ValueId::new(
                OWNED_PARAMETERS
                    .checked_add(index)
                    .expect("test ValueId geometry fits u32"),
            ),
            ty: resource_type.clone(),
            owner_place: Some(PlaceId::new(index)),
            origin: Origin::SYNTHETIC,
        })
        .collect();
    let cleanup = |id: u32, value_offset: u32| FailureCleanupPlan {
        id: FailureCleanupId::new(id),
        actions: (0..OWNED_PARAMETERS)
            .rev()
            .map(|index| FailureCleanupAction::DropOwner {
                place: Some(PlaceId::new(index)),
                value: ValueId::new(
                    value_offset
                        .checked_add(index)
                        .expect("test ValueId geometry fits u32"),
                ),
                glue: DropGlueIdentity::Resource(resource_kind),
            })
            .collect(),
    };
    let entry_cleanup = cleanup(0, 0_u32);
    let successor_cleanup = cleanup(1, OWNED_PARAMETERS);
    let call_id = ValueId::new(
        OWNED_PARAMETERS
            .checked_mul(2)
            .expect("test ValueId geometry fits u32"),
    );
    let mut call_metadata = metadata(EffectSet::PURE);
    call_metadata.failure_cleanup = Some(successor_cleanup.id);
    call_metadata.frame_state = Some(FrameState {
        bytecode_position: 0,
        locals: Vec::new(),
        operand_stack: Vec::new(),
    });
    let mut instructions = Vec::with_capacity(1 + OWNED_PARAMETERS as usize);
    instructions.push(Instruction {
        id: call_id,
        ty: SsaType::Unit,
        kind: InstructionKind::Call {
            target: CallTarget::Direct(FunctionId::new(1)),
            arguments: successor_parameters
                .iter()
                .map(|parameter| parameter.id)
                .collect(),
            consuming: vec![true; OWNED_PARAMETERS as usize],
            signature: Signature::monomorphic(parameters.clone(), SsaType::Unit),
            instantiation: None,
        },
        metadata: call_metadata,
    });
    instructions.extend((0..OWNED_PARAMETERS).map(|index| {
        place_end(
            call_id
                .raw()
                .checked_add(1)
                .and_then(|first| first.checked_add(index))
                .expect("test ValueId geometry fits u32"),
            index,
        )
    }));
    let result = instructions
        .last()
        .expect("wide ownership fixture has cleanup instructions")
        .id;
    let mut entry_metadata = block_metadata();
    entry_metadata.failure_cleanup = Some(entry_cleanup.id);
    let function = Function {
        id: FunctionId::new(1),
        name: "wide-ownership-state".into(),
        signature: Signature::monomorphic(parameters.clone(), SsaType::Unit),
        places,
        failure_cleanups: vec![entry_cleanup, successor_cleanup],
        effects: EffectSet::PURE,
        entry: BlockId::new(0),
        blocks: vec![
            Block {
                id: BlockId::new(0),
                parameters: entry_parameters.clone(),
                instructions: Vec::new(),
                terminator: Terminator::Branch {
                    target: BlockId::new(1),
                    arguments: entry_parameters
                        .iter()
                        .map(|parameter| parameter.id)
                        .collect(),
                },
                metadata: entry_metadata,
            },
            Block {
                id: BlockId::new(1),
                parameters: successor_parameters,
                instructions,
                terminator: Terminator::Return(result),
                metadata: block_metadata(),
            },
        ],
        origin: Origin::SYNTHETIC,
    };
    verify(ownership_program(function))
        .expect("264,000 cells under the former retained-state accounting must verify");
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
        .expect_err("list-nested function signatures cannot launder ownership types");
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
