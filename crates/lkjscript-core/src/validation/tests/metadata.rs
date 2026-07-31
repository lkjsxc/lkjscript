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
        validate_chunk(valid, &ValidationLimits::default()).expect("explicit trap validates");
    assert_eq!(validated.main_instructions()[1].op(), Op::Trap);

    let mut wrong = Chunk::new();
    wrong.constants.push(Constant::I64(7));
    wrong.main.emit_op_u16(Op::LoadConst, 0);
    wrong.main.emit(Op::Trap);
    assert!(error(wrong).contains("expected string"));
}

#[test]
fn configured_code_table_metadata_and_constant_limits_are_enforced() {
    let code_limits = ValidationLimits {
        max_function_code_bytes: 1,
        ..ValidationLimits::default()
    };
    assert!(validate_chunk(unit_chunk(), &code_limits)
        .expect_err("code limit")
        .to_string()
        .contains("code bytes"));

    let encoded_limits = ValidationLimits {
        max_encoded_bytes: 1,
        ..ValidationLimits::default()
    };
    assert!(validate_chunk(unit_chunk(), &encoded_limits)
        .expect_err("encoded limit")
        .to_string()
        .contains("encoded bytecode"));

    let mut table = unit_chunk();
    table.constants.push(Constant::I64(1));
    let table_limits = ValidationLimits {
        max_table_entries: 0,
        ..ValidationLimits::default()
    };
    assert!(validate_chunk(table, &table_limits)
        .expect_err("table limit")
        .to_string()
        .contains("table"));

    let metadata_limits = ValidationLimits {
        max_metadata_bytes: 0,
        ..ValidationLimits::default()
    };
    assert!(validate_chunk(unit_chunk(), &metadata_limits)
        .expect_err("metadata limit")
        .to_string()
        .contains("metadata"));

    let mut data = unit_chunk();
    data.constants.push(Constant::Str("x".into()));
    let data_limits = ValidationLimits {
        max_constant_data_bytes: 0,
        ..ValidationLimits::default()
    };
    assert!(validate_chunk(data, &data_limits)
        .expect_err("constant data limit")
        .to_string()
        .contains("constant 0"));
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
