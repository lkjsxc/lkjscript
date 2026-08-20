use super::*;
use crate::core_ir::{
    CoreBlock, CoreField, CoreFunction, CoreType, CoreVariant, PRIMITIVE_TYPE_COUNT, SwitchArm,
    UNIT_TYPE,
};
use crate::ids::{Revision, SnapshotHash, WorkspaceId};
use crate::managed::{ExecutionMode, ManagedLimits};
use crate::schema::{MAXIMUM_BYTE_LITERAL_BYTES, Node};
use crate::type_layout::{FieldLayout, LayoutShape, ValueLayout, VariantLayout};
fn node(serial: u64) -> NodeId {
    NodeId::new(WorkspaceId::from_bytes([0x51; 16]), serial).expect("node")
}
fn primitives() -> Vec<CoreType> {
    crate::schema::SemanticType::PRIMITIVES
        .into_iter()
        .map(|semantic| CoreType {
            origin: None,
            kind: CoreTypeKind::from_semantic_primitive(semantic).expect("primitive kind"),
            layout: crate::type_layout::primitive_layout(semantic).expect("primitive layout"),
        })
        .collect()
}
const PRODUCT_TYPE: CoreTypeId = CoreTypeId(PRIMITIVE_TYPE_COUNT as u32);
const SUM_TYPE: CoreTypeId = CoreTypeId(PRIMITIVE_TYPE_COUNT as u32 + 1);
fn scalar_program() -> CoreProgram {
    CoreProgram {
        types: primitives(),
        entry: FunctionId(0),
        functions: vec![CoreFunction {
            origin: node(1),
            parameters: vec![],
            result: I64_TYPE,
            value_types: vec![I64_TYPE],
            frame_cells: 1,
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: node(2),
                parameters: vec![],
                instructions: vec![Instruction::ConstI64 {
                    origin: node(3),
                    result: ValueId(0),
                    value: 7,
                }],
                terminator: Terminator::Return {
                    origin: node(4),
                    value: ValueId(0),
                },
            }],
        }],
    }
}
fn product_type() -> CoreType {
    CoreType {
        origin: Some(node(10)),
        kind: CoreTypeKind::Product {
            fields: vec![
                CoreField {
                    origin: node(11),
                    ty: I64_TYPE,
                    cell_offset: 0,
                },
                CoreField {
                    origin: node(12),
                    ty: I64_TYPE,
                    cell_offset: 1,
                },
            ],
        },
        layout: ValueLayout {
            size: 16,
            align: 8,
            cells: 2,
            shape: LayoutShape::Product {
                fields: vec![
                    FieldLayout {
                        field: node(11),
                        offset: 0,
                        cells: 1,
                    },
                    FieldLayout {
                        field: node(12),
                        offset: 8,
                        cells: 1,
                    },
                ],
            },
        },
    }
}

fn two_cell_product_program(extra_i64_values: usize) -> CoreProgram {
    let mut types = primitives();
    types.push(product_type());
    let mut instructions = vec![
        Instruction::ConstI64 {
            origin: node(20),
            result: ValueId(0),
            value: 7,
        },
        Instruction::ConstI64 {
            origin: node(21),
            result: ValueId(1),
            value: 9,
        },
        Instruction::ConstructProduct {
            origin: node(22),
            result: ValueId(2),
            ty: PRODUCT_TYPE,
            fields: vec![ValueId(0), ValueId(1)],
        },
    ];
    for offset in 0..extra_i64_values {
        instructions.push(Instruction::ConstI64 {
            origin: node(23),
            result: ValueId(u32::try_from(3 + offset).expect("filler value")),
            value: 0,
        });
    }
    let mut value_types = vec![I64_TYPE, I64_TYPE, PRODUCT_TYPE];
    value_types.extend(std::iter::repeat_n(I64_TYPE, extra_i64_values));
    CoreProgram {
        types,
        entry: FunctionId(0),
        functions: vec![CoreFunction {
            origin: node(19),
            parameters: vec![],
            result: PRODUCT_TYPE,
            frame_cells: u64::try_from(4 + extra_i64_values).expect("frame cells"),
            value_types,
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: node(20),
                parameters: vec![],
                instructions,
                terminator: Terminator::Return {
                    origin: node(24),
                    value: ValueId(2),
                },
            }],
        }],
    }
}

fn policy(fuel: u64) -> RunPolicy {
    RunPolicy {
        fuel,
        maximum_frames: 10,
    }
}

fn one_function_program(
    result: CoreTypeId,
    value_types: Vec<CoreTypeId>,
    instructions: Vec<Instruction>,
    returned: ValueId,
) -> CoreProgram {
    let frame_cells = value_types
        .iter()
        .map(|ty| primitives()[ty.0 as usize].layout.cells)
        .sum();
    CoreProgram {
        types: primitives(),
        entry: FunctionId(0),
        functions: vec![CoreFunction {
            origin: node(100),
            parameters: vec![],
            result,
            value_types,
            frame_cells,
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: node(101),
                parameters: vec![],
                instructions,
                terminator: Terminator::Return {
                    origin: node(199),
                    value: returned,
                },
            }],
        }],
    }
}

fn byte_length_program(bytes: &[u8]) -> CoreProgram {
    one_function_program(
        I64_TYPE,
        vec![BYTES_TYPE, I64_TYPE],
        vec![
            Instruction::ConstBytes {
                origin: node(102),
                result: ValueId(0),
                value: ByteString::from_slice(bytes).unwrap(),
            },
            Instruction::BytesLen {
                origin: node(103),
                result: ValueId(1),
                value: ValueId(0),
            },
        ],
        ValueId(1),
    )
}

fn byte_index_program(bytes: &[u8], requested: i64) -> CoreProgram {
    one_function_program(
        I64_TYPE,
        vec![BYTES_TYPE, I64_TYPE, I64_TYPE],
        vec![
            Instruction::ConstBytes {
                origin: node(104),
                result: ValueId(0),
                value: ByteString::from_slice(bytes).unwrap(),
            },
            Instruction::ConstI64 {
                origin: node(105),
                result: ValueId(1),
                value: requested,
            },
            Instruction::BytesAt {
                origin: node(106),
                result: ValueId(2),
                value: ValueId(0),
                index: ValueId(1),
            },
        ],
        ValueId(2),
    )
}

fn byte_slice_program(bytes: &[u8], start: i64, length: i64) -> CoreProgram {
    one_function_program(
        BYTES_TYPE,
        vec![BYTES_TYPE, I64_TYPE, I64_TYPE, BYTES_TYPE],
        vec![
            Instruction::ConstBytes {
                origin: node(107),
                result: ValueId(0),
                value: ByteString::from_slice(bytes).unwrap(),
            },
            Instruction::ConstI64 {
                origin: node(108),
                result: ValueId(1),
                value: start,
            },
            Instruction::ConstI64 {
                origin: node(109),
                result: ValueId(2),
                value: length,
            },
            Instruction::BytesSlice {
                origin: node(110),
                result: ValueId(3),
                value: ValueId(0),
                start: ValueId(1),
                length: ValueId(2),
            },
        ],
        ValueId(3),
    )
}

fn byte_equality_program(left: &[u8], right: &[u8]) -> CoreProgram {
    one_function_program(
        BOOL_TYPE,
        vec![BYTES_TYPE, BYTES_TYPE, BOOL_TYPE],
        vec![
            Instruction::ConstBytes {
                origin: node(111),
                result: ValueId(0),
                value: ByteString::from_slice(left).unwrap(),
            },
            Instruction::ConstBytes {
                origin: node(112),
                result: ValueId(1),
                value: ByteString::from_slice(right).unwrap(),
            },
            Instruction::BytesEqual {
                origin: node(113),
                result: ValueId(2),
                lhs: ValueId(0),
                rhs: ValueId(1),
            },
        ],
        ValueId(2),
    )
}

fn byte_concat_program(left: &[u8], right: &[u8]) -> CoreProgram {
    one_function_program(
        BYTES_TYPE,
        vec![BYTES_TYPE, BYTES_TYPE, BYTES_TYPE],
        vec![
            Instruction::ConstBytes {
                origin: node(114),
                result: ValueId(0),
                value: ByteString::from_slice(left).unwrap(),
            },
            Instruction::ConstBytes {
                origin: node(115),
                result: ValueId(1),
                value: ByteString::from_slice(right).unwrap(),
            },
            Instruction::BytesConcat {
                origin: node(116),
                result: ValueId(2),
                lhs: ValueId(0),
                rhs: ValueId(1),
            },
        ],
        ValueId(2),
    )
}

fn byte_concat_argument_program() -> CoreProgram {
    CoreProgram {
        types: primitives(),
        entry: FunctionId(0),
        functions: vec![CoreFunction {
            origin: node(117),
            parameters: vec![ValueId(0), ValueId(1)],
            result: BYTES_TYPE,
            value_types: vec![BYTES_TYPE, BYTES_TYPE, BYTES_TYPE],
            frame_cells: 3,
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: node(118),
                parameters: vec![ValueId(0), ValueId(1)],
                instructions: vec![Instruction::BytesConcat {
                    origin: node(119),
                    result: ValueId(2),
                    lhs: ValueId(0),
                    rhs: ValueId(1),
                }],
                terminator: Terminator::Return {
                    origin: node(120),
                    value: ValueId(2),
                },
            }],
        }],
    }
}

fn run_core_value(program: &CoreProgram, policy: RunPolicy) -> Result<RuntimeValue> {
    let mut managed = InvocationStore::default();
    let flat = interpret_with_store(program, vec![], policy, &mut managed)?;
    preflight_flat_output(program, &managed, &flat, program.functions[0].origin)?;
    from_flat(program, &managed, &flat, 1, program.functions[0].origin)
}

fn run_core_value_with_mode(
    program: &CoreProgram,
    policy: RunPolicy,
    mode: ExecutionMode,
) -> Result<(RuntimeValue, crate::managed::ManagedMetrics)> {
    let mut managed = InvocationStore::new(
        ManagedLimits {
            cumulative_visible_bytes: MAX_RUN_MANAGED_VISIBLE_BYTES,
            live_backing_bytes: MAX_RUN_RETAINED_BACKING_BYTES,
            live_objects: MAX_RUN_MANAGED_OBJECTS,
        },
        mode,
    );
    let flat = interpret_with_store(program, vec![], policy, &mut managed)?;
    preflight_flat_output(program, &managed, &flat, program.functions[0].origin)?;
    let value = from_flat(program, &managed, &flat, 1, program.functions[0].origin)?;
    Ok((value, managed.metrics()))
}

#[test]
fn managed_sequence_objects_match_the_canonical_allocate_new_oracle() {
    let origin = node(400);
    let elements = vec![
        RuntimeValue::I64(7),
        RuntimeValue::Text(TextString::try_from_str("ok").unwrap()),
        RuntimeValue::Sequence {
            ty: node(401),
            elements: vec![RuntimeValue::Bool(true)],
        },
    ];
    let mut managed = InvocationStore::default();
    let handle = managed
        .allocate_sequence(elements.clone(), origin)
        .expect("allocate managed sequence");
    assert_eq!(
        managed.materialize_sequence(handle, origin).unwrap(),
        elements
    );
    let canonical = encode_sequence(&elements, origin).unwrap();
    assert_eq!(decode_sequence(&canonical, origin).unwrap(), elements);
    assert_eq!(
        managed
            .sequence_object(handle, origin)
            .unwrap()
            .retained_bytes,
        canonical.len()
    );
    assert_eq!(managed.metrics().cumulative_visible_bytes, 2);

    managed.share(handle, origin).unwrap();
    managed.drop_claim(handle, origin).unwrap();
    assert_eq!(
        managed.materialize_sequence(handle, origin).unwrap(),
        elements
    );
    managed.drop_claim(handle, origin).unwrap();
    assert_eq!(
        managed
            .materialize_sequence(handle, origin)
            .unwrap_err()
            .code,
        ErrorCode::InvalidManagedHandle
    );
}

#[test]
fn sequence_slice_and_concat_match_flat_allocate_new_results() {
    let origin = node(402);
    let mut managed = InvocationStore::default();
    let source = managed
        .allocate_sequence(
            vec![
                RuntimeValue::I64(1),
                RuntimeValue::I64(2),
                RuntimeValue::I64(3),
            ],
            origin,
        )
        .expect("source sequence");
    let (slice, slice_length) = managed
        .slice_sequence(source, 1, 3, origin)
        .expect("bounded slice");
    assert_eq!(slice_length, 2);
    assert_eq!(
        managed.materialize_sequence(slice, origin).unwrap(),
        vec![RuntimeValue::I64(2), RuntimeValue::I64(3)]
    );

    let suffix = managed
        .allocate_sequence(vec![RuntimeValue::I64(4)], origin)
        .expect("suffix sequence");
    let (combined, combined_length) = managed
        .concat_sequence(slice, suffix, origin)
        .expect("bounded concatenation");
    assert_eq!(combined_length, 3);
    assert_eq!(
        managed.materialize_sequence(combined, origin).unwrap(),
        vec![
            RuntimeValue::I64(2),
            RuntimeValue::I64(3),
            RuntimeValue::I64(4)
        ]
    );
    assert_eq!(
        managed
            .slice_sequence(source, 2, 4, origin)
            .expect_err("one-over slice")
            .code,
        ErrorCode::RuntimeTrap
    );
}

#[test]
fn sequence_concat_rejects_one_over_the_element_limit_before_allocation() {
    let origin = node(403);
    let mut managed = InvocationStore::default();
    let maximum = managed
        .allocate_sequence(vec![RuntimeValue::Unit; MAXIMUM_SEQUENCE_ELEMENTS], origin)
        .expect("maximum sequence");
    let one = managed
        .allocate_sequence(vec![RuntimeValue::Unit], origin)
        .expect("one-element sequence");
    assert_eq!(
        managed
            .concat_sequence(maximum, one, origin)
            .expect_err("one-over concatenation")
            .code,
        ErrorCode::PolicyExceeded
    );
}

#[test]
fn byte_operations_have_exact_content_bounds_and_logical_fuel() {
    assert_eq!(
        run_core_value(&byte_length_program(b""), policy(4)).unwrap(),
        RuntimeValue::I64(0)
    );
    assert_eq!(
        run_core_value(
            &byte_length_program(&vec![0; MAXIMUM_BYTE_LITERAL_BYTES]),
            policy(4)
        )
        .unwrap(),
        RuntimeValue::I64(MAXIMUM_BYTE_LITERAL_BYTES as i64)
    );
    assert_eq!(
        run_core_value(&byte_length_program(b"x"), policy(3))
            .expect_err("one fuel below exact length cost")
            .code,
        ErrorCode::ExecutionFuelExhausted
    );

    for (index, expected) in [(0, 0), (2, 255)] {
        assert_eq!(
            run_core_value(&byte_index_program(&[0, 7, 255], index), policy(5)).unwrap(),
            RuntimeValue::I64(expected)
        );
    }
    for (bytes, index) in [
        (&b"abc"[..], -1),
        (&b"abc"[..], 3),
        (&b"abc"[..], i64::MAX),
        (&b""[..], 0),
    ] {
        assert_eq!(
            run_core_value(&byte_index_program(bytes, index), policy(5))
                .expect_err("byte index bounds")
                .code,
            ErrorCode::ByteIndexOutOfBounds
        );
    }

    for (start, length, expected) in [
        (0, 0, &b""[..]),
        (4, 0, &b""[..]),
        (0, 4, &b"abcd"[..]),
        (1, 2, &b"bc"[..]),
    ] {
        assert_eq!(
            run_core_value(&byte_slice_program(b"abcd", start, length), policy(7)).unwrap(),
            RuntimeValue::Bytes(ByteString::from_slice(expected).unwrap())
        );
        assert_eq!(
            run_core_value(&byte_slice_program(b"abcd", start, length), policy(6))
                .expect_err("one fuel below exact slice cost")
                .code,
            ErrorCode::ExecutionFuelExhausted
        );
    }
    for (start, length) in [(5, 0), (-1, 0), (0, -1), (i64::MAX, 1), (3, 2)] {
        assert_eq!(
            run_core_value(&byte_slice_program(b"abcd", start, length), policy(6))
                .expect_err("byte slice bounds")
                .code,
            ErrorCode::ByteSliceOutOfBounds
        );
    }

    for (left, right, expected, compared) in [
        (&b""[..], &b""[..], true, 0),
        (&b"abc"[..], &b"abc"[..], true, 3),
        (&b"abc"[..], &b"xbc"[..], false, 1),
        (&b"abc"[..], &b"abx"[..], false, 3),
        (&b"abc"[..], &b"ab"[..], false, 0),
    ] {
        let fuel = 5 + compared;
        assert_eq!(
            run_core_value(&byte_equality_program(left, right), policy(fuel)).unwrap(),
            RuntimeValue::Bool(expected)
        );
        assert_eq!(
            run_core_value(&byte_equality_program(left, right), policy(fuel - 1))
                .expect_err("one fuel below equality work")
                .code,
            ErrorCode::ExecutionFuelExhausted
        );
    }

    for (left, right, expected) in [
        (&b""[..], &b""[..], &b""[..]),
        (&b""[..], &b"abc"[..], &b"abc"[..]),
        (&b"abc"[..], &b""[..], &b"abc"[..]),
        (&b"abc"[..], &b"def"[..], &b"abcdef"[..]),
    ] {
        let fuel = 5 + expected.len();
        assert_eq!(
            run_core_value(&byte_concat_program(left, right), policy(fuel as u64)).unwrap(),
            RuntimeValue::Bytes(ByteString::from_slice(expected).unwrap())
        );
        assert_eq!(
            run_core_value(&byte_concat_program(left, right), policy((fuel - 1) as u64))
                .expect_err("one fuel below exact concat work")
                .code,
            ErrorCode::ExecutionFuelExhausted
        );
    }

    let program = byte_concat_argument_program();
    for (right_length, expected) in [
        (MAXIMUM_BYTE_STRING_BYTES / 2, None),
        (
            MAXIMUM_BYTE_STRING_BYTES / 2 + 1,
            Some(ErrorCode::ByteValueTooLarge),
        ),
    ] {
        let mut managed = InvocationStore::default();
        let left = to_flat(
            &program,
            &mut managed,
            &RuntimeValue::Bytes(
                ByteString::new(vec![0x61; MAXIMUM_BYTE_STRING_BYTES / 2]).unwrap(),
            ),
            BYTES_TYPE,
            1,
            node(117),
        )
        .unwrap();
        let right = to_flat(
            &program,
            &mut managed,
            &RuntimeValue::Bytes(ByteString::new(vec![0x62; right_length]).unwrap()),
            BYTES_TYPE,
            1,
            node(117),
        )
        .unwrap();
        let outcome = interpret_with_store(
            &program,
            vec![left, right],
            policy(MAX_RUN_FUEL),
            &mut managed,
        );
        match expected {
            None => {
                let value = outcome.unwrap();
                assert_eq!(
                    managed
                        .bytes(
                            match value.cells[0] {
                                Cell::Bytes(handle) => handle,
                                Cell::Scalar(_) => panic!("concat result must be managed"),
                            },
                            node(120),
                        )
                        .unwrap()
                        .len(),
                    MAXIMUM_BYTE_STRING_BYTES
                );
            }
            Some(code) => assert_eq!(outcome.unwrap_err().code, code),
        }
    }
}

#[test]
fn allocate_new_oracle_and_ownership_reuse_are_observably_equivalent() {
    let program = byte_concat_program(b"ownership", b"-oracle");
    let fuel = policy(5 + 16);
    let (oracle, oracle_metrics) =
        run_core_value_with_mode(&program, fuel, ExecutionMode::Oracle).unwrap();
    let (optimized, optimized_metrics) =
        run_core_value_with_mode(&program, fuel, ExecutionMode::Ownership).unwrap();
    assert_eq!(oracle, optimized);
    assert_eq!(oracle_metrics.reuse_hits, 0);
    assert_eq!(optimized_metrics.reuse_attempts, 1);
    assert_eq!(optimized_metrics.reuse_hits, 1);
    assert!(optimized_metrics.copied_bytes < oracle_metrics.copied_bytes);
    assert!(optimized_metrics.peak_live_backing_bytes < oracle_metrics.peak_live_backing_bytes);
    eprintln!(
        "managed_bytes oracle_copied={} optimized_copied={} oracle_peak_backing={} optimized_peak_backing={} reuse_hits={}",
        oracle_metrics.copied_bytes,
        optimized_metrics.copied_bytes,
        oracle_metrics.peak_live_backing_bytes,
        optimized_metrics.peak_live_backing_bytes,
        optimized_metrics.reuse_hits
    );

    let exhausted = policy(5 + 16 - 1);
    for mode in [ExecutionMode::Oracle, ExecutionMode::Ownership] {
        assert_eq!(
            run_core_value_with_mode(&program, exhausted, mode)
                .unwrap_err()
                .code,
            ErrorCode::ExecutionFuelExhausted
        );
    }
}

#[test]
fn deterministic_generated_concat_corpus_matches_oracle() {
    const SEED: u64 = 0x6c6b_6a73_6372_6970;
    const CASES: usize = 256;
    let mut state = SEED;
    for case in 0..CASES {
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let left_len = usize::try_from(next() % 65).unwrap();
        let right_len = usize::try_from(next() % 65).unwrap();
        let left = (0..left_len).map(|_| next() as u8).collect::<Vec<_>>();
        let right = (0..right_len).map(|_| next() as u8).collect::<Vec<_>>();
        let program = byte_concat_program(&left, &right);
        let exact_fuel = u64::try_from(5 + left_len + right_len).unwrap();
        let oracle =
            run_core_value_with_mode(&program, policy(exact_fuel), ExecutionMode::Oracle).unwrap();
        let optimized =
            run_core_value_with_mode(&program, policy(exact_fuel), ExecutionMode::Ownership)
                .unwrap();
        assert_eq!(optimized.0, oracle.0, "generated case {case}");
        if exact_fuel > 0 && case % 16 == 0 {
            let oracle_error =
                run_core_value_with_mode(&program, policy(exact_fuel - 1), ExecutionMode::Oracle)
                    .unwrap_err();
            let optimized_error = run_core_value_with_mode(
                &program,
                policy(exact_fuel - 1),
                ExecutionMode::Ownership,
            )
            .unwrap_err();
            assert_eq!(optimized_error.code, oracle_error.code);
            assert_eq!(optimized_error.target, oracle_error.target);
        }
    }
    eprintln!("concat-differential seed={SEED:#018x} cases={CASES}");
}

#[test]
fn recursive_shared_managed_argument_unwinds_every_claim_iteratively() {
    let program = CoreProgram {
        types: primitives(),
        entry: FunctionId(0),
        functions: vec![CoreFunction {
            origin: node(121),
            parameters: vec![ValueId(0)],
            result: BYTES_TYPE,
            value_types: vec![BYTES_TYPE, BYTES_TYPE, I64_TYPE],
            frame_cells: 3,
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: node(122),
                parameters: vec![ValueId(0)],
                instructions: vec![
                    Instruction::Call {
                        origin: node(123),
                        result: ValueId(1),
                        function: FunctionId(0),
                        arguments: vec![ValueId(0)],
                    },
                    Instruction::BytesLen {
                        origin: node(124),
                        result: ValueId(2),
                        value: ValueId(0),
                    },
                ],
                terminator: Terminator::Return {
                    origin: node(125),
                    value: ValueId(1),
                },
            }],
        }],
    };
    let mut managed = InvocationStore::default();
    let argument = to_flat(
        &program,
        &mut managed,
        &RuntimeValue::Bytes(ByteString::from_slice(b"shared-recursion").unwrap()),
        BYTES_TYPE,
        1,
        node(121),
    )
    .unwrap();
    assert_eq!(
        interpret_with_store(
            &program,
            vec![argument],
            RunPolicy {
                fuel: 1_000,
                maximum_frames: 8,
            },
            &mut managed,
        )
        .unwrap_err()
        .code,
        ErrorCode::ExecutionFrameExhausted
    );
    let metrics = managed.metrics();
    assert_eq!(metrics.reference_count_increments, 7);
    assert_eq!(metrics.live_objects, 0);
    assert_eq!(metrics.live_backing_bytes, 0);
}

#[test]
fn managed_store_handles_views_and_physical_accounting_are_bounded_and_canonical() {
    let origin = node(200);
    let mut store = InvocationStore::default();
    let root = store
        .allocate_backing(&vec![0xa5; MAX_RUN_RETAINED_BACKING_BYTES], origin)
        .expect("exact retained backing maximum");
    assert_eq!(
        store.metrics().cumulative_visible_bytes,
        MAX_RUN_RETAINED_BACKING_BYTES
    );
    assert_eq!(
        store.metrics().live_backing_bytes,
        MAX_RUN_RETAINED_BACKING_BYTES
    );
    assert_eq!(store.metrics().live_objects, 2);
    assert_eq!(
        store.allocate_backing(&[0], origin).unwrap_err().code,
        ErrorCode::RetainedBytePolicyExceeded
    );

    let one = store.slice(root, 17, 1, origin).unwrap();
    assert_eq!(store.bytes(one, origin).unwrap(), &[0xa5]);
    assert_eq!(store.metrics().retained_by_views, 0);
    store.share(root, origin).unwrap();
    store.drop_claim(root, origin).unwrap();
    assert_eq!(
        store.bytes(root, origin).unwrap().len(),
        MAX_RUN_RETAINED_BACKING_BYTES
    );
    let nested = store.slice(one, 1, 0, origin).unwrap();
    assert_eq!(store.bytes(nested, origin).unwrap(), b"");
    let mut deeply_nested = nested;
    for _ in 0..128 {
        deeply_nested = store.slice(deeply_nested, 0, 0, origin).unwrap();
        assert_eq!(store.bytes(deeply_nested, origin).unwrap(), b"");
    }
    store.drop_claim(root, origin).unwrap();
    assert_eq!(
        store.metrics().retained_by_views,
        MAX_RUN_RETAINED_BACKING_BYTES
    );
    assert_eq!(
        store.metrics().live_backing_bytes,
        MAX_RUN_RETAINED_BACKING_BYTES
    );

    let wrong_kind_program = CoreProgram {
        types: primitives(),
        entry: FunctionId(0),
        functions: vec![CoreFunction {
            origin,
            parameters: vec![ValueId(0)],
            result: BYTES_TYPE,
            value_types: vec![BYTES_TYPE],
            frame_cells: 1,
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin,
                parameters: vec![ValueId(0)],
                instructions: vec![],
                terminator: Terminator::Return {
                    origin,
                    value: ValueId(0),
                },
            }],
        }],
    };
    let wrong_kind = new_frame(
        &wrong_kind_program,
        FunctionId(0),
        &[FlatValue {
            ty: BYTES_TYPE,
            cells: vec![Cell::Scalar(0)],
        }],
        None,
    )
    .unwrap();
    assert_eq!(
        require_bytes_handle(
            &wrong_kind_program,
            &wrong_kind_program.functions[0],
            &wrong_kind,
            ValueId(0),
            origin
        )
        .unwrap_err()
        .code,
        ErrorCode::InvalidManagedHandle
    );

    let mut visible = InvocationStore::default();
    let root = visible
        .allocate_backing(&vec![0; MAX_RUN_RETAINED_BACKING_BYTES], origin)
        .unwrap();
    for _ in 0..3 {
        visible
            .slice(root, 0, MAX_RUN_RETAINED_BACKING_BYTES as i64, origin)
            .unwrap();
    }
    assert_eq!(
        visible.metrics().cumulative_visible_bytes,
        MAX_RUN_MANAGED_VISIBLE_BYTES
    );
    assert_eq!(
        visible.slice(root, 0, 1, origin).unwrap_err().code,
        ErrorCode::ManagedVisibleBytePolicyExceeded
    );
    visible.drop_claim(root, origin).unwrap();
    assert_eq!(
        visible.metrics().retained_by_views,
        MAX_RUN_RETAINED_BACKING_BYTES
    );

    let mut objects = InvocationStore::default();
    for _ in 0..(MAX_RUN_MANAGED_OBJECTS / 2) {
        objects.allocate_backing(b"", origin).unwrap();
    }
    assert_eq!(objects.metrics().live_objects, MAX_RUN_MANAGED_OBJECTS);
    assert_eq!(
        objects.allocate_backing(b"", origin).unwrap_err().code,
        ErrorCode::ManagedObjectPolicyExceeded
    );

    let mut distinct = InvocationStore::default();
    let left = distinct.allocate_backing(b"same", origin).unwrap();
    let right = distinct.allocate_backing(b"same", origin).unwrap();
    assert_eq!(
        distinct.bytes(left, origin).unwrap(),
        distinct.bytes(right, origin).unwrap()
    );
    assert_ne!(left, right);
    assert_eq!(distinct.metrics().live_backing_bytes, 8);
}

fn store_with_witness(witness: std::sync::Arc<std::sync::atomic::AtomicUsize>) -> InvocationStore {
    InvocationStore::with_drop_witness(
        ManagedLimits {
            cumulative_visible_bytes: MAX_RUN_MANAGED_VISIBLE_BYTES,
            live_backing_bytes: MAX_RUN_RETAINED_BACKING_BYTES,
            live_objects: MAX_RUN_MANAGED_OBJECTS,
        },
        ExecutionMode::Ownership,
        witness,
    )
}

fn byte_pair_output_program() -> CoreProgram {
    let pair = CoreType {
        origin: Some(node(220)),
        kind: CoreTypeKind::Product {
            fields: vec![
                CoreField {
                    origin: node(221),
                    ty: BYTES_TYPE,
                    cell_offset: 0,
                },
                CoreField {
                    origin: node(222),
                    ty: BYTES_TYPE,
                    cell_offset: 1,
                },
            ],
        },
        layout: ValueLayout {
            size: 16,
            align: 8,
            cells: 2,
            shape: LayoutShape::Product {
                fields: vec![
                    FieldLayout {
                        field: node(221),
                        offset: 0,
                        cells: 1,
                    },
                    FieldLayout {
                        field: node(222),
                        offset: 8,
                        cells: 1,
                    },
                ],
            },
        },
    };
    let mut types = primitives();
    types.push(pair);
    CoreProgram {
        types,
        entry: FunctionId(0),
        functions: vec![CoreFunction {
            origin: node(223),
            parameters: vec![ValueId(0)],
            result: PRODUCT_TYPE,
            value_types: vec![BYTES_TYPE, PRODUCT_TYPE],
            frame_cells: 3,
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: node(224),
                parameters: vec![ValueId(0)],
                instructions: vec![Instruction::ConstructProduct {
                    origin: node(225),
                    result: ValueId(1),
                    ty: PRODUCT_TYPE,
                    fields: vec![ValueId(0), ValueId(0)],
                }],
                terminator: Terminator::Return {
                    origin: node(226),
                    value: ValueId(1),
                },
            }],
        }],
    }
}

#[test]
fn managed_store_drops_on_success_trap_fuel_frame_and_output_failures() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let success_witness = Arc::new(AtomicUsize::new(0));
    let owned_output;
    {
        let program = byte_slice_program(b"abcd", 1, 2);
        let mut store = store_with_witness(Arc::clone(&success_witness));
        let flat = interpret_with_store(&program, vec![], policy(7), &mut store).unwrap();
        preflight_flat_output(&program, &store, &flat, program.functions[0].origin).unwrap();
        owned_output = from_flat(&program, &store, &flat, 1, program.functions[0].origin)
            .expect("owned public result before store drop");
    }
    assert_eq!(success_witness.load(Ordering::SeqCst), 1);
    assert_eq!(
        owned_output,
        RuntimeValue::Bytes(ByteString::from_slice(b"bc").unwrap())
    );

    let trap_witness = Arc::new(AtomicUsize::new(0));
    {
        let program = byte_index_program(b"abc", -1);
        let mut store = store_with_witness(Arc::clone(&trap_witness));
        assert_eq!(
            interpret_with_store(&program, vec![], policy(5), &mut store)
                .unwrap_err()
                .code,
            ErrorCode::ByteIndexOutOfBounds
        );
        assert_eq!(store.metrics().live_objects, 0);
    }
    assert_eq!(trap_witness.load(Ordering::SeqCst), 1);

    let fuel_witness = Arc::new(AtomicUsize::new(0));
    {
        let program = byte_equality_program(b"abc", b"abc");
        let mut store = store_with_witness(Arc::clone(&fuel_witness));
        assert_eq!(
            interpret_with_store(&program, vec![], policy(7), &mut store)
                .unwrap_err()
                .code,
            ErrorCode::ExecutionFuelExhausted
        );
        assert_eq!(store.metrics().live_objects, 0);
    }
    assert_eq!(fuel_witness.load(Ordering::SeqCst), 1);

    let frame_witness = Arc::new(AtomicUsize::new(0));
    {
        let recursive = one_function_program(
            BYTES_TYPE,
            vec![BYTES_TYPE, BYTES_TYPE],
            vec![
                Instruction::ConstBytes {
                    origin: node(227),
                    result: ValueId(0),
                    value: ByteString::from_slice(b"allocated").unwrap(),
                },
                Instruction::Call {
                    origin: node(228),
                    result: ValueId(1),
                    function: FunctionId(0),
                    arguments: vec![],
                },
            ],
            ValueId(1),
        );
        let mut store = store_with_witness(Arc::clone(&frame_witness));
        assert_eq!(
            interpret_with_store(
                &recursive,
                vec![],
                RunPolicy {
                    fuel: 100,
                    maximum_frames: 1,
                },
                &mut store,
            )
            .unwrap_err()
            .code,
            ErrorCode::ExecutionFrameExhausted
        );
        assert_eq!(store.metrics().live_objects, 0);
    }
    assert_eq!(frame_witness.load(Ordering::SeqCst), 1);

    let output_witness = Arc::new(AtomicUsize::new(0));
    {
        let program = byte_pair_output_program();
        core_ir::verify(&program).unwrap();
        let mut store = store_with_witness(Arc::clone(&output_witness));
        let argument = to_flat(
            &program,
            &mut store,
            &RuntimeValue::Bytes(ByteString::new(vec![0; 40 * 1024]).unwrap()),
            BYTES_TYPE,
            1,
            program.functions[0].origin,
        )
        .unwrap();
        let flat = interpret_with_store(&program, vec![argument], policy(10), &mut store)
            .expect("pure product construction");
        assert_eq!(store.metrics().live_backing_bytes, 40 * 1024);
        assert_eq!(
            preflight_flat_output_with_limit(
                &program,
                &store,
                &flat,
                program.functions[0].origin,
                MAXIMUM_BYTE_STRING_BYTES,
            )
            .unwrap_err()
            .code,
            ErrorCode::ResultBytePolicyExceeded
        );
    }
    assert_eq!(output_witness.load(Ordering::SeqCst), 1);
}

#[test]
fn mandatory_deep_nominal_result_rejects_before_compile_or_execution() {
    let workspace = WorkspaceId::from_bytes([0x52; 16]);
    let id = |serial| NodeId::new(workspace, serial).expect("node");
    let depth = MAX_RUNTIME_VALUE_DEPTH as u64 + 1;
    let declarations = (0..depth)
        .map(|index| id(4 + index * 2))
        .collect::<Vec<_>>();
    let function = id(4 + depth * 2);
    let mut nodes = BTreeMap::from([
        (
            id(1),
            Node::WorkspaceRoot {
                packages: vec![id(2)],
                targets: Vec::new(),
            },
        ),
        (
            id(2),
            Node::Package {
                owner: id(1),
                name: "p".into(),
                modules: vec![id(3)],
                entry: Some(function),
            },
        ),
        (
            id(3),
            Node::Module {
                owner: id(2),
                name: "m".into(),
                types: declarations.clone(),
                functions: vec![function],
            },
        ),
        (
            function,
            Node::Function {
                owner: id(3),
                name: "main".into(),
                parameters: vec![],
                result: SemanticType::Nominal(declarations[0]),
                body: None,
            },
        ),
    ]);
    for index in 0..depth {
        let declaration = id(4 + index * 2);
        let field = id(5 + index * 2);
        nodes.insert(
            declaration,
            Node::ProductType {
                owner: id(3),
                name: format!("T{index}"),
                fields: vec![field],
            },
        );
        let ty = if index + 1 == depth {
            SemanticType::I64
        } else {
            SemanticType::Nominal(id(4 + (index + 1) * 2))
        };
        nodes.insert(
            field,
            Node::ProductField {
                owner: declaration,
                ordinal: 0,
                name: "next".into(),
                ty,
            },
        );
    }
    let snapshot = Snapshot {
        workspace,
        revision: Revision::INITIAL,
        root: id(1),
        next_serial: function.serial() + 1,
        tombstones: BTreeSet::new(),
        nodes,
        hash: SnapshotHash::from_bytes([0; 32]),
    };
    let error =
        compile_and_run(&snapshot, function, &[], policy(100)).expect_err("result preflight");
    assert_eq!(error.code, ErrorCode::PolicyExceeded);
    assert!(error.message.contains("mandatory result"));
}

#[test]
fn sum_result_preflight_uses_componentwise_variant_maxima() {
    let workspace = WorkspaceId::from_bytes([0x53; 16]);
    let id = |serial| NodeId::new(workspace, serial).expect("node");
    let root = id(1);
    let wide = id(2);
    let deep_start = id(100);
    let wide_fields = (0..64_u64).map(|offset| id(3 + offset)).collect::<Vec<_>>();
    let mut nodes = BTreeMap::new();
    nodes.insert(
        root,
        Node::SumType {
            owner: id(999),
            name: "Mixed".into(),
            variants: vec![id(70), id(71)],
        },
    );
    nodes.insert(
        id(70),
        Node::SumVariant {
            owner: root,
            ordinal: 0,
            name: "wide".into(),
            payload: Some(SemanticType::Nominal(wide)),
        },
    );
    nodes.insert(
        id(71),
        Node::SumVariant {
            owner: root,
            ordinal: 1,
            name: "deep".into(),
            payload: Some(SemanticType::Nominal(deep_start)),
        },
    );
    nodes.insert(
        wide,
        Node::ProductType {
            owner: id(999),
            name: "Wide".into(),
            fields: wide_fields.clone(),
        },
    );
    for (ordinal, field) in wide_fields.iter().enumerate() {
        nodes.insert(
            *field,
            Node::ProductField {
                owner: wide,
                ordinal: u32::try_from(ordinal).expect("ordinal"),
                name: format!("f{ordinal}"),
                ty: SemanticType::Unit,
            },
        );
    }
    for offset in 0..MAX_RUNTIME_VALUE_DEPTH {
        let declaration = id(100 + u64::try_from(offset * 2).expect("serial"));
        let field = id(101 + u64::try_from(offset * 2).expect("serial"));
        nodes.insert(
            declaration,
            Node::ProductType {
                owner: id(999),
                name: format!("Deep{offset}"),
                fields: vec![field],
            },
        );
        nodes.insert(
            field,
            Node::ProductField {
                owner: declaration,
                ordinal: 0,
                name: "next".into(),
                ty: if offset + 1 == MAX_RUNTIME_VALUE_DEPTH {
                    SemanticType::Unit
                } else {
                    SemanticType::Nominal(
                        id(100 + u64::try_from((offset + 1) * 2).expect("serial")),
                    )
                },
            },
        );
    }
    let snapshot = Snapshot {
        workspace,
        revision: Revision::INITIAL,
        root: id(999),
        next_serial: 1_000,
        tombstones: BTreeSet::new(),
        nodes,
        hash: SnapshotHash::from_bytes([0; 32]),
    };
    let error = preflight_result(&snapshot, SemanticType::Nominal(root), root)
        .expect_err("deep variant must dominate depth while wide variant dominates items");
    assert_eq!(error.code, ErrorCode::PolicyExceeded);
    assert!(error.message.contains("depth"));
}

#[test]
fn deterministic_fuel_charges_base_and_returned_cells_and_traps_leave_runtime_usable() {
    let program = scalar_program();
    assert_eq!(
        interpret(&program, vec![], policy(3))
            .expect("exact fuel")
            .cells,
        vec![7]
    );
    assert_eq!(
        interpret(&program, vec![], policy(2))
            .expect_err("copy fuel")
            .code,
        ErrorCode::ExecutionFuelExhausted
    );
    assert_eq!(
        interpret(&program, vec![], policy(3))
            .expect("later run")
            .cells,
        vec![7]
    );
}

#[test]
fn aggregate_copy_fuel_is_exact_and_peak_overflow_precedes_return_copy() {
    let program = two_cell_product_program(0);
    assert_eq!(
        interpret(&program, vec![], policy(8))
            .expect("exact product fuel")
            .cells,
        vec![7, 9]
    );
    assert_eq!(
        interpret(&program, vec![], policy(7))
            .expect_err("product copy fuel")
            .code,
        ErrorCode::ExecutionFuelExhausted
    );

    let peak = two_cell_product_program(MAX_RUN_LIVE_CELLS - 4);
    let error = interpret(&peak, vec![], policy(MAX_RUN_FUEL))
        .expect_err("return scratch exceeds live-cell peak");
    assert_eq!(error.code, ErrorCode::ExecutionFrameExhausted);
    assert_eq!(error.target, Some(node(24)));
    assert_eq!(
        interpret(&program, vec![], policy(8))
            .expect("runtime remains usable after peak trap")
            .cells,
        vec![7, 9]
    );
}

#[test]
fn fuel_contract_meters_projection_variant_match_call_edge_and_zero_cell_values() {
    let mut types = primitives();
    types.push(product_type());
    types.push(CoreType {
        origin: Some(node(13)),
        kind: CoreTypeKind::Sum {
            variants: vec![
                CoreVariant {
                    origin: node(14),
                    payload: None,
                    discriminant: 0,
                },
                CoreVariant {
                    origin: node(15),
                    payload: Some(PRODUCT_TYPE),
                    discriminant: 1,
                },
                CoreVariant {
                    origin: node(16),
                    payload: Some(UNIT_TYPE),
                    discriminant: 2,
                },
            ],
        },
        layout: ValueLayout {
            size: 24,
            align: 8,
            cells: 3,
            shape: LayoutShape::Sum {
                discriminant_bytes: 1,
                payload_offset: 8,
                variants: vec![
                    VariantLayout {
                        variant: node(14),
                        discriminant: 0,
                        payload_size: 0,
                        payload_align: 1,
                        payload_cells: 0,
                    },
                    VariantLayout {
                        variant: node(15),
                        discriminant: 1,
                        payload_size: 16,
                        payload_align: 8,
                        payload_cells: 2,
                    },
                    VariantLayout {
                        variant: node(16),
                        discriminant: 2,
                        payload_size: 0,
                        payload_align: 1,
                        payload_cells: 0,
                    },
                ],
            },
        },
    });
    let program = CoreProgram {
        types,
        entry: FunctionId(0),
        functions: vec![CoreFunction {
            origin: node(30),
            parameters: vec![ValueId(0), ValueId(1)],
            result: UNIT_TYPE,
            value_types: vec![UNIT_TYPE, PRODUCT_TYPE, SUM_TYPE],
            frame_cells: 5,
            entry: BlockId(0),
            blocks: vec![],
        }],
    };
    let function = &program.functions[0];
    assert_eq!(
        instruction_copy_cells(
            &program,
            function,
            &Instruction::ProjectField {
                origin: node(31),
                result: ValueId(1),
                value: ValueId(1),
                field: 0,
            },
        )
        .expect("projection fuel"),
        2
    );
    assert_eq!(
        instruction_copy_cells(
            &program,
            function,
            &Instruction::ConstructVariant {
                origin: node(32),
                result: ValueId(2),
                sum: SUM_TYPE,
                variant: 0,
                payload: None,
            },
        )
        .expect("nullary variant full canonicalization fuel"),
        3
    );
    assert_eq!(
        instruction_copy_cells(
            &program,
            function,
            &Instruction::ConstructVariant {
                origin: node(33),
                result: ValueId(2),
                sum: SUM_TYPE,
                variant: 1,
                payload: Some(ValueId(1)),
            },
        )
        .expect("payload variant canonicalization and copy fuel"),
        5
    );
    assert_eq!(
        instruction_copy_cells(
            &program,
            function,
            &Instruction::ConstructVariant {
                origin: node(34),
                result: ValueId(2),
                sum: SUM_TYPE,
                variant: 2,
                payload: Some(ValueId(0)),
            },
        )
        .expect("zero-cell payload logical copy fuel"),
        4
    );
    assert_eq!(
        edge_copy_cost(&program, function, &[ValueId(0), ValueId(1)]).expect("call and edge fuel"),
        3
    );
    let payload_arm = SwitchArm {
        variant: 1,
        target: BlockId(0),
        arguments: vec![SwitchArgument::Payload, SwitchArgument::Value(ValueId(0))],
    };
    assert_eq!(
        switch_edge_cost_and_cells(&program, function, &payload_arm, Some(PRODUCT_TYPE),)
            .expect("selected match payload fuel"),
        (3, 2)
    );
}

#[test]
fn selected_large_switch_arm_exhausts_fuel_before_edge_materialization() {
    const ARM_ARGUMENTS: usize = 4_096;
    let mut types = primitives();
    types.push(CoreType {
        origin: Some(node(40)),
        kind: CoreTypeKind::Sum {
            variants: vec![CoreVariant {
                origin: node(41),
                payload: None,
                discriminant: 0,
            }],
        },
        layout: ValueLayout {
            size: 1,
            align: 1,
            cells: 1,
            shape: LayoutShape::Sum {
                discriminant_bytes: 1,
                payload_offset: 1,
                variants: vec![VariantLayout {
                    variant: node(41),
                    discriminant: 0,
                    payload_size: 0,
                    payload_align: 1,
                    payload_cells: 0,
                }],
            },
        },
    });
    let source_values = (1..=ARM_ARGUMENTS)
        .map(|index| ValueId(u32::try_from(index).expect("source value")))
        .collect::<Vec<_>>();
    let target_parameters = (ARM_ARGUMENTS + 1..=ARM_ARGUMENTS * 2)
        .map(|index| ValueId(u32::try_from(index).expect("target parameter")))
        .collect::<Vec<_>>();
    let mut instructions = vec![Instruction::ConstructVariant {
        origin: node(42),
        result: ValueId(0),
        sum: PRODUCT_TYPE,
        variant: 0,
        payload: None,
    }];
    instructions.extend(source_values.iter().map(|result| Instruction::ConstUnit {
        origin: node(43),
        result: *result,
    }));
    let program = CoreProgram {
        types,
        entry: FunctionId(0),
        functions: vec![CoreFunction {
            origin: node(39),
            parameters: vec![],
            result: UNIT_TYPE,
            value_types: std::iter::once(PRODUCT_TYPE)
                .chain(std::iter::repeat_n(UNIT_TYPE, ARM_ARGUMENTS * 2))
                .collect(),
            frame_cells: 1,
            entry: BlockId(0),
            blocks: vec![
                CoreBlock {
                    origin: node(42),
                    parameters: vec![],
                    instructions,
                    terminator: Terminator::SwitchVariant {
                        origin: node(44),
                        scrutinee: ValueId(0),
                        arms: vec![SwitchArm {
                            variant: 0,
                            target: BlockId(1),
                            arguments: source_values
                                .iter()
                                .copied()
                                .map(SwitchArgument::Value)
                                .collect(),
                        }],
                    },
                },
                CoreBlock {
                    origin: node(45),
                    parameters: target_parameters.clone(),
                    instructions: vec![],
                    terminator: Terminator::Return {
                        origin: node(46),
                        value: target_parameters[0],
                    },
                },
            ],
        }],
    };
    let error = interpret(
        &program,
        vec![],
        policy(u64::try_from(ARM_ARGUMENTS).expect("fuel") + 3),
    )
    .expect_err("selected edge copy must require fuel before values are materialized");
    assert_eq!(error.code, ErrorCode::ExecutionFuelExhausted);
    assert_eq!(error.target, Some(node(44)));
}

#[test]
fn scalar_operations_run_with_a_frame_exactly_at_the_live_cell_cap() {
    let filler_count = MAX_RUN_LIVE_CELLS - 4;
    let filler_parameters = (5..5 + filler_count)
        .map(|index| ValueId(u32::try_from(index).expect("filler parameter")))
        .collect::<Vec<_>>();
    let filler_unit = ValueId(u32::try_from(5 + filler_count).expect("unit parameter"));
    let mut value_types = vec![I64_TYPE, I64_TYPE, I64_TYPE, BOOL_TYPE, UNIT_TYPE];
    value_types.extend(std::iter::repeat_n(I64_TYPE, filler_count));
    value_types.push(UNIT_TYPE);
    let program = CoreProgram {
        types: primitives(),
        entry: FunctionId(0),
        functions: vec![CoreFunction {
            origin: node(50),
            parameters: vec![],
            result: UNIT_TYPE,
            value_types,
            frame_cells: u64::try_from(MAX_RUN_LIVE_CELLS).expect("frame cells"),
            entry: BlockId(0),
            blocks: vec![
                CoreBlock {
                    origin: node(51),
                    parameters: vec![],
                    instructions: vec![
                        Instruction::ConstI64 {
                            origin: node(52),
                            result: ValueId(0),
                            value: 20,
                        },
                        Instruction::ConstI64 {
                            origin: node(53),
                            result: ValueId(1),
                            value: 22,
                        },
                        Instruction::AddI64 {
                            origin: node(54),
                            result: ValueId(2),
                            lhs: ValueId(0),
                            rhs: ValueId(1),
                        },
                        Instruction::LtI64 {
                            origin: node(55),
                            result: ValueId(3),
                            lhs: ValueId(0),
                            rhs: ValueId(2),
                        },
                        Instruction::ConstUnit {
                            origin: node(56),
                            result: ValueId(4),
                        },
                    ],
                    terminator: Terminator::Return {
                        origin: node(57),
                        value: ValueId(4),
                    },
                },
                CoreBlock {
                    origin: node(58),
                    parameters: filler_parameters
                        .iter()
                        .copied()
                        .chain(std::iter::once(filler_unit))
                        .collect(),
                    instructions: vec![],
                    terminator: Terminator::Return {
                        origin: node(59),
                        value: filler_unit,
                    },
                },
            ],
        }],
    };
    assert_eq!(
        interpret(&program, vec![], policy(7))
            .expect("scalar direct writes at exact live-cell cap")
            .cells,
        Vec::<u64>::new()
    );
}

#[test]
fn returned_frames_release_live_cells_and_recursive_callee_exhausts_before_allocation() {
    const CALLEE_VALUES: usize = 1_024;
    const CALLS: usize = 70;
    let callee_instructions = (0..CALLEE_VALUES)
        .map(|value| Instruction::ConstI64 {
            origin: node(20),
            result: ValueId(u32::try_from(value).expect("value")),
            value: i64::try_from(value).expect("literal"),
        })
        .collect::<Vec<_>>();
    let callee = CoreFunction {
        origin: node(19),
        parameters: vec![],
        result: I64_TYPE,
        value_types: vec![I64_TYPE; CALLEE_VALUES],
        frame_cells: CALLEE_VALUES as u64,
        entry: BlockId(0),
        blocks: vec![CoreBlock {
            origin: node(20),
            parameters: vec![],
            instructions: callee_instructions,
            terminator: Terminator::Return {
                origin: node(21),
                value: ValueId(u32::try_from(CALLEE_VALUES - 1).expect("return")),
            },
        }],
    };
    let caller_instructions = (0..CALLS)
        .map(|value| Instruction::Call {
            origin: node(4),
            result: ValueId(u32::try_from(value).expect("call value")),
            function: FunctionId(1),
            arguments: vec![],
        })
        .collect::<Vec<_>>();
    let caller = CoreFunction {
        origin: node(1),
        parameters: vec![],
        result: I64_TYPE,
        value_types: vec![I64_TYPE; CALLS],
        frame_cells: CALLS as u64,
        entry: BlockId(0),
        blocks: vec![CoreBlock {
            origin: node(2),
            parameters: vec![],
            instructions: caller_instructions,
            terminator: Terminator::Return {
                origin: node(5),
                value: ValueId(u32::try_from(CALLS - 1).expect("return")),
            },
        }],
    };
    let program = CoreProgram {
        types: primitives(),
        functions: vec![caller, callee],
        entry: FunctionId(0),
    };
    assert_eq!(
        interpret(
            &program,
            vec![],
            RunPolicy {
                fuel: 1_000_000,
                maximum_frames: 1_000
            }
        )
        .expect("released callee cells")
        .cells,
        vec![(CALLEE_VALUES - 1) as u64]
    );

    let mut recursive_instructions = (0..CALLEE_VALUES)
        .map(|value| Instruction::ConstI64 {
            origin: node(30),
            result: ValueId(u32::try_from(value).expect("value")),
            value: 0,
        })
        .collect::<Vec<_>>();
    recursive_instructions.push(Instruction::Call {
        origin: node(31),
        result: ValueId(u32::try_from(CALLEE_VALUES).expect("call result")),
        function: FunctionId(0),
        arguments: vec![],
    });
    let recursive = CoreProgram {
        types: primitives(),
        functions: vec![CoreFunction {
            origin: node(29),
            parameters: vec![],
            result: I64_TYPE,
            value_types: vec![I64_TYPE; CALLEE_VALUES + 1],
            frame_cells: (CALLEE_VALUES + 1) as u64,
            entry: BlockId(0),
            blocks: vec![CoreBlock {
                origin: node(30),
                parameters: vec![],
                instructions: recursive_instructions,
                terminator: Terminator::Return {
                    origin: node(32),
                    value: ValueId(u32::try_from(CALLEE_VALUES).expect("return")),
                },
            }],
        }],
        entry: FunctionId(0),
    };
    let exhausted = interpret(
        &recursive,
        vec![],
        RunPolicy {
            fuel: 1_000_000,
            maximum_frames: 1_000,
        },
    )
    .expect_err("recursive live cells");
    assert_eq!(exhausted.code, ErrorCode::ExecutionFrameExhausted);
    assert_eq!(exhausted.target, Some(node(31)));
}

#[test]
fn entry_live_cell_exhaustion_precedes_allocation() {
    let mut program = scalar_program();
    program.functions[0].value_types = vec![I64_TYPE; MAX_RUN_LIVE_CELLS + 1];
    program.functions[0].frame_cells = u64::try_from(MAX_RUN_LIVE_CELLS + 1).expect("cells");
    program.functions[0].parameters = (0..MAX_RUN_LIVE_CELLS + 1)
        .map(|value| ValueId(u32::try_from(value).expect("value")))
        .collect();
    let parameters = program.functions[0].parameters.clone();
    program.functions[0].blocks[0].parameters = parameters;
    program.functions[0].blocks[0].instructions.clear();
    program.functions[0].blocks[0].terminator = Terminator::Return {
        origin: node(4),
        value: ValueId(0),
    };
    let args = vec![
        FlatValue {
            ty: I64_TYPE,
            cells: vec![Cell::Scalar(0)]
        };
        MAX_RUN_LIVE_CELLS + 1
    ];
    let error = interpret(
        &program,
        args,
        RunPolicy {
            fuel: MAX_RUN_FUEL,
            maximum_frames: MAX_RUN_FRAMES,
        },
    )
    .expect_err("live cells");
    assert_eq!(error.code, ErrorCode::ExecutionFrameExhausted);
    assert!(error.message.contains("live-cell"));
}
