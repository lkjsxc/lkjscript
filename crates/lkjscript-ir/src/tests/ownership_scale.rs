use super::fixtures::*;
use crate::*;

#[test]
fn ownership_verification_accepts_more_than_former_retained_state_and_work_boundaries() {
    const OWNED_PARAMETERS: u64 = 44_000;
    const OWNED_PARAMETER_COUNT: usize = 44_000;
    const STATE_CELLS_PER_BLOCK: usize = 3 * OWNED_PARAMETER_COUNT;
    const FORMER_RETAINED_STATE_CELLS: usize = 2 * STATE_CELLS_PER_BLOCK;
    const FORMER_PRECHECK_WORK_AT_FIRST_PLAN: usize = 3 * OWNED_PARAMETER_COUNT + 4;
    const _: () = assert!(STATE_CELLS_PER_BLOCK > 131_072);
    const _: () = assert!(FORMER_RETAINED_STATE_CELLS > 131_072);
    const _: () = assert!(FORMER_PRECHECK_WORK_AT_FIRST_PLAN > 131_072);

    let resource_kind = lkjscript_contracts::ResourceKind::FileReader;
    let resource_type = SsaType::Resource(resource_kind);
    let parameters = vec![resource_type.clone(); OWNED_PARAMETER_COUNT];
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
                    .expect("test ValueId geometry fits u64"),
            ),
            ty: resource_type.clone(),
            owner_place: Some(PlaceId::new(index)),
            origin: Origin::SYNTHETIC,
        })
        .collect();
    let cleanup = |start: u64, value_offset: u64| {
        let mut nodes = Vec::with_capacity(OWNED_PARAMETER_COUNT);
        for index in 0..OWNED_PARAMETERS {
            nodes.push(FailureCleanupNode {
                action: FailureCleanupAction::DropOwner {
                    place: Some(PlaceId::new(index)),
                    value: ValueId::new(
                        value_offset
                            .checked_add(index)
                            .expect("test ValueId geometry fits u64"),
                    ),
                    glue: DropGlueIdentity::Resource(resource_kind),
                },
                next: (index > 0).then(|| FailureCleanupId::new(start + index - 1)),
            });
        }
        let root = FailureCleanupId::new(start + OWNED_PARAMETERS - 1);
        (nodes, root)
    };
    let (mut failure_cleanups, entry_cleanup) = cleanup(0, 0_u64);
    let successor_start =
        u64::try_from(failure_cleanups.len()).expect("test cleanup geometry fits u64");
    let (successor_nodes, successor_cleanup) = cleanup(successor_start, OWNED_PARAMETERS);
    failure_cleanups.extend(successor_nodes);
    let call_id = ValueId::new(
        OWNED_PARAMETERS
            .checked_mul(2)
            .expect("test ValueId geometry fits u64"),
    );
    let mut call_metadata = metadata(EffectSet::PURE);
    call_metadata.failure_cleanup = Some(FailureCleanupRoots::single(successor_cleanup));
    call_metadata.frame_state = Some(FrameState {
        bytecode_position: 0,
        locals: Vec::new(),
        operand_stack: Vec::new(),
    });
    let mut instructions = Vec::with_capacity(1 + OWNED_PARAMETER_COUNT);
    instructions.push(Instruction {
        id: call_id,
        ty: SsaType::Unit,
        kind: InstructionKind::Call {
            target: CallTarget::Direct(FunctionId::new(1)),
            arguments: successor_parameters
                .iter()
                .map(|parameter| parameter.id)
                .collect(),
            consuming: vec![true; OWNED_PARAMETER_COUNT],
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
                .expect("test ValueId geometry fits u64"),
            index,
        )
    }));
    let result = instructions
        .last()
        .expect("wide ownership fixture has cleanup instructions")
        .id;
    let mut entry_metadata = block_metadata();
    entry_metadata.failure_cleanup = Some(FailureCleanupRoots::single(entry_cleanup));
    let function = Function {
        id: FunctionId::new(1),
        name: "wide-ownership-state".into(),
        signature: Signature::monomorphic(parameters.clone(), SsaType::Unit),
        places,
        failure_cleanups,
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
fn ownership_verifier_rejects_nested_function_laundering() {
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
    for _ in 0..300 {
        nested = SsaType::Function(Box::new(Signature::monomorphic(Vec::new(), nested)));
    }
    *deeply_nested_function.functions[0].signature.result = SsaType::List(Box::new(nested));
    let error = verify(deeply_nested_function)
        .expect_err("fixture still has a deliberately mismatched return value");
    assert!(
        error.to_string().contains("returns the wrong type"),
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
