use super::*;

#[test]
fn indexes_metadata_categories_and_capture_metadata_are_checked() {
    let mut constant = unit_chunk();
    constant.main.code = vec![Op::LoadConst as u8, 0, 0, Op::Return as u8];
    assert!(error(constant).contains("constant index"));

    let mut captures = unit_chunk();
    captures.constants.push(Constant::Proto(0));
    captures.protos.push(FunctionProto {
        name: "f".into(),
        arity: 0,
        locals: 0,
        memory_plan: None,
        memory_witness_parameters: Vec::new(),
        call_witnesses: Vec::new(),
        parameter_structurals: Vec::new(),
        parameter_structural_places: Vec::new(),
        parameter_type_variables: Vec::new(),
        parameter_copy_kinds: Vec::new(),
        parameter_region_products: Vec::new(),
        return_copy_kind: None,
        return_region_product: None,
        return_structural: None,
        return_type_variable: None,
        parameter_resources: Vec::new(),
        parameter_resource_places: Vec::new(),
        return_resource: None,
        parameter_uniques: Vec::new(),
        parameter_unique_places: Vec::new(),
        return_unique: None,
        unique_places: 0,
        failure_cleanups: Vec::new(),
        failure_cleanup_ranges: Vec::new(),
        code: vec![Op::Unit as u8, Op::Return as u8],
    });
    captures.main.code = vec![
        Op::LoadConst as u8,
        0,
        0,
        Op::MakeClosure as u8,
        1,
        0,
        Op::Return as u8,
    ];
    assert!(error(captures).contains("capture metadata"));
}

#[test]
fn global_closures_must_match_declared_prototypes() {
    let mut chunk = unit_chunk();
    chunk.global_names.push("function".into());
    chunk.global_prototypes.push(Some(0));
    for name in ["declared", "stored"] {
        chunk.protos.push(FunctionProto {
            name: name.into(),
            arity: 0,
            locals: 0,
            memory_plan: None,
            memory_witness_parameters: Vec::new(),
            call_witnesses: Vec::new(),
            parameter_structurals: Vec::new(),
            parameter_structural_places: Vec::new(),
            parameter_type_variables: Vec::new(),
            parameter_copy_kinds: Vec::new(),
            parameter_region_products: Vec::new(),
            return_copy_kind: None,
            return_region_product: None,
            return_structural: None,
            return_type_variable: None,
            parameter_resources: Vec::new(),
            parameter_resource_places: Vec::new(),
            return_resource: None,
            parameter_uniques: Vec::new(),
            parameter_unique_places: Vec::new(),
            return_unique: None,
            unique_places: 0,
            failure_cleanups: Vec::new(),
            failure_cleanup_ranges: Vec::new(),
            code: vec![Op::Unit as u8, Op::Return as u8],
        });
    }
    let stored = chunk.add_const(Constant::Proto(1));
    chunk.main.code.clear();
    chunk.main.emit_op_u16(Op::LoadConst, stored.0);
    chunk.main.emit_op_u16(Op::MakeClosure, 0);
    chunk.main.emit_op_u16(Op::StoreGlobal, 0);
    chunk.main.emit(Op::Pop);
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    let message = error(chunk);
    assert!(message.contains("declared prototype metadata"), "{message}");
}

#[test]
fn explicit_trap_requires_a_string_value_and_terminates_control_flow() {
    let mut valid = Chunk::new();
    valid.constants.push(Constant::Str("explicit trap".into()));
    valid.main.emit_op_u16(Op::LoadConst, 0);
    valid.main.emit(Op::Trap);
    let validated =
        validate_chunk(valid, ValidationPolicy::Unrestricted).expect("explicit trap validates");
    assert_eq!(validated.main_instructions()[1].op(), Op::Trap);

    let mut wrong = Chunk::new();
    wrong.constants.push(Constant::I64(7));
    wrong.main.emit_op_u16(Op::LoadConst, 0);
    wrong.main.emit(Op::Trap);
    assert!(error(wrong).contains("expected string"));
}

#[test]
fn unrestricted_validation_crosses_former_chunk_limit_and_low_byte_policy_is_typed() {
    let mut chunk = unit_chunk();
    chunk.constants.push(Constant::StaticBytes(
        vec![0; 16 * 1024 * 1024 + 1].into_boxed_slice(),
    ));

    let policy_error = validate_chunk(
        chunk.clone(),
        ValidationPolicy::Limited {
            max_total_bytes: 1024,
        },
    )
    .expect_err("low untrusted artifact byte policy must fail");
    assert_eq!(policy_error.class(), crate::ErrorClass::BytecodePolicy);
    assert!(policy_error.to_string().contains("total encoded bytes"));

    validate_chunk(chunk, ValidationPolicy::Unrestricted)
        .expect("trusted unrestricted validation must cross the former 16 MiB limit");
}

#[test]
fn malformed_bytecode_failure_is_identical_under_low_and_unrestricted_policy() {
    let mut malformed = unit_chunk();
    malformed.global_names.push("same".into());
    malformed.global_names.push("same".into());

    let unrestricted = validate_chunk(malformed.clone(), ValidationPolicy::Unrestricted)
        .expect_err("duplicate global metadata must fail");
    let limited = validate_chunk(malformed, ValidationPolicy::Limited { max_total_bytes: 0 })
        .expect_err("malformed bytecode must fail before artifact policy");
    assert_eq!(limited, unrestricted);
    assert_eq!(limited.class(), crate::ErrorClass::Ordinary);
}

#[test]
fn unreachable_operands_and_duplicate_metadata_still_fail() {
    let mut unreachable = unit_chunk();
    unreachable.main.code.extend_from_slice(&[
        Op::LoadGlobal as u8,
        0,
        0,
        Op::Unit as u8,
        Op::Return as u8,
    ]);
    assert!(error(unreachable).contains("global index"));

    let mut duplicate = unit_chunk();
    duplicate.global_names.push("same".into());
    duplicate.global_names.push("same".into());
    assert!(error(duplicate).contains("duplicate bytecode global"));
}
