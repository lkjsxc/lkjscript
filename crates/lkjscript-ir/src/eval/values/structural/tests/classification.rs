use super::*;

#[test]
fn exact_memory_metadata_selects_structural_execution() {
    let mut program = Program {
        prepared_identity: lkjscript_contracts::PreparedProgramIdentity::UNBOUND,
        memory: StructuralMemoryMetadata::default(),
        region_products: Vec::new(),
        sources: Vec::new(),
        products: Vec::new(),
        enums: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: Vec::new(),
        main: FunctionId::new(0),
    };
    let legacy = ProductId::new(0);
    program.products.push(ProductMetadata {
        id: legacy,
        name: "legacy-list".into(),
        fields: vec![ProductField {
            name: "items".into(),
            ty: SsaType::List(Box::new(SsaType::I64)),
        }],
    });
    assert_eq!(
        mode(&program, SsaType::Product(legacy)),
        Ok(AggregateMode::Legacy)
    );

    let planned = ProductId::new(1);
    let planned_type = SsaType::Product(planned);
    program.products.push(ProductMetadata {
        id: planned,
        name: "planned".into(),
        fields: vec![ProductField {
            name: "text".into(),
            ty: SsaType::Str,
        }],
    });
    assert_eq!(
        mode(&program, planned_type.clone()),
        Ok(AggregateMode::Legacy)
    );
    install_structural_type(
        &mut program,
        planned_type.clone(),
        StructuralLayoutKind::Product {
            product: planned,
            fields: vec![SsaType::Str],
        },
    );
    assert_eq!(mode(&program, planned_type), Ok(AggregateMode::Structural));
}

#[test]
fn structural_type_metadata_rejects_duplicate_semantic_types() {
    let mut program = one_block_program();
    install_structural_type(&mut program, SsaType::Str, StructuralLayoutKind::String);
    let mut duplicate = program.memory.types[0].clone();
    duplicate.id = StructuralTypeId::new(1);
    program.memory.types.push(duplicate);
    let result = crate::verify::verify_program(&program);
    assert!(
        result.is_err(),
        "duplicate structural semantic type verified"
    );
    let message = result
        .err()
        .map_or_else(String::new, |error| error.to_string());
    assert!(message.contains("unique by semantic type"), "{message}");
}

#[test]
fn structural_type_metadata_rejects_zero_and_duplicate_memory_witnesses() {
    let mut zero = one_block_program();
    install_structural_type(&mut zero, SsaType::Str, StructuralLayoutKind::String);
    zero.memory.types[0].witness = MemoryWitnessId::new([0; 32]);
    let result = crate::verify::verify_program(&zero);
    assert!(result.is_err(), "zero structural memory witness verified");
    let message = result
        .err()
        .map_or_else(String::new, |error| error.to_string());
    assert!(message.contains("resolved memory witness"), "{message}");

    let mut duplicate = one_block_program();
    install_structural_type(&mut duplicate, SsaType::Str, StructuralLayoutKind::String);
    install_structural_type(&mut duplicate, SsaType::Path, StructuralLayoutKind::Path);
    duplicate.memory.types[1].witness = duplicate.memory.types[0].witness;
    let result = crate::verify::verify_program(&duplicate);
    assert!(
        result.is_err(),
        "duplicate structural memory witness verified"
    );
    let message = result
        .err()
        .map_or_else(String::new, |error| error.to_string());
    assert!(
        message.contains("witness identities must be unique"),
        "{message}"
    );
}

#[test]
fn closure_reconstruction_rejects_mixed_and_recursive_metadata() {
    let mut program = Program {
        prepared_identity: lkjscript_contracts::PreparedProgramIdentity::UNBOUND,
        memory: StructuralMemoryMetadata::default(),
        region_products: Vec::new(),
        sources: Vec::new(),
        products: Vec::new(),
        enums: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: Vec::new(),
        main: FunctionId::new(0),
    };
    program.products.push(ProductMetadata {
        id: ProductId::new(0),
        name: "mixed".into(),
        fields: vec![
            ProductField {
                name: "items".into(),
                ty: SsaType::List(Box::new(SsaType::I64)),
            },
            ProductField {
                name: "text".into(),
                ty: SsaType::Str,
            },
        ],
    });
    let mixed = SsaType::Product(ProductId::new(0));
    assert_eq!(mode(&program, mixed.clone()), Ok(AggregateMode::Legacy));
    install_structural_type(
        &mut program,
        mixed.clone(),
        StructuralLayoutKind::Product {
            product: ProductId::new(0),
            fields: vec![SsaType::List(Box::new(SsaType::I64)), SsaType::Str],
        },
    );
    assert!(mode(&program, mixed).is_err());
    program.products.push(ProductMetadata {
        id: ProductId::new(1),
        name: "recursive".into(),
        fields: vec![ProductField {
            name: "again".into(),
            ty: SsaType::Product(ProductId::new(1)),
        }],
    });
    let recursive = SsaType::Product(ProductId::new(1));
    install_structural_type(
        &mut program,
        recursive.clone(),
        StructuralLayoutKind::Product {
            product: ProductId::new(1),
            fields: vec![recursive.clone()],
        },
    );
    assert_eq!(mode(&program, recursive), Ok(AggregateMode::Structural));
}

#[test]
fn ordinary_unplanned_string_execution_remains_legacy() -> crate::Result<()> {
    let mut program = one_block_program();
    let function = &mut program.functions[0];
    *function.signature.result = SsaType::Str;
    function.blocks[0].instructions = vec![Instruction {
        id: ValueId::new(0),
        ty: SsaType::Str,
        kind: InstructionKind::Constant(Constant::Str("legacy".into())),
        metadata: metadata(EffectSet::PURE),
    }];
    let program = verify(program)?;
    let (outcome, observation) = evaluate_observed(&program, &EvalConfig::default());
    assert_eq!(
        outcome,
        EvalOutcome::Returned(EvalValue::Str("legacy".into()))
    );
    assert_eq!(observation.static_string_artifacts, 1);
    assert!(observation.assert_empty().is_ok());
    Ok(())
}

fn mode(program: &Program, ty: SsaType) -> std::result::Result<AggregateMode, String> {
    aggregate_mode(program, &ty)
}
