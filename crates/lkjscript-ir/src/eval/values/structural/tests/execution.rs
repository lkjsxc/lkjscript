use super::*;

#[test]
fn static_and_dynamic_strings_use_artifacts_and_roots() -> crate::Result<()> {
    let mut static_program = one_block_program();
    install_structural_type(
        &mut static_program,
        SsaType::Str,
        StructuralLayoutKind::String,
    );
    let function = &mut static_program.functions[0];
    *function.signature.result = SsaType::Str;
    function.blocks[0].instructions = vec![Instruction {
        id: ValueId::new(0),
        ty: SsaType::Str,
        kind: InstructionKind::Constant(Constant::Str("static-value".into())),
        metadata: metadata(EffectSet::PURE),
    }];
    let static_program = verify(static_program)?;
    let (outcome, observation) = evaluate_observed(&static_program, &EvalConfig::default());
    assert!(matches!(
        outcome,
        EvalOutcome::Returned(EvalValue::ReturnedOwned(ref value))
            if matches!(value.as_structural().map(|value| &value.payload),
                Some(SemanticPayload::String(bytes)) if bytes == b"static-value")
    ));
    assert_eq!(observation.static_string_artifacts, 1);
    assert_eq!(observation.metrics.allocations, 0);
    assert!(observation.assert_empty().is_ok());

    let mut dynamic_program = one_block_program();
    install_structural_type(
        &mut dynamic_program,
        SsaType::Str,
        StructuralLayoutKind::String,
    );
    let function = &mut dynamic_program.functions[0];
    *function.signature.result = SsaType::Str;
    function.effects = RuntimeOp::EmptyStr.effects();
    function.blocks[0].instructions = vec![Instruction {
        id: ValueId::new(0),
        ty: SsaType::Str,
        kind: InstructionKind::Runtime {
            operation: RuntimeOp::EmptyStr,
            arguments: Vec::new(),
            signature: Signature::monomorphic(Vec::new(), SsaType::Str),
        },
        metadata: allocating_metadata(0, RuntimeOp::EmptyStr.effects()),
    }];
    function.failure_cleanups.push(FailureCleanupPlan {
        id: FailureCleanupId::new(0),
        actions: vec![FailureCleanupAction::DropOwner {
            place: None,
            value: ValueId::new(0),
            glue: DropGlueIdentity::Structural(StructuralDropGlueIdentity::String {
                type_id: StructuralTypeId::new(0),
                layout: StructuralLayoutId::new(0),
            }),
        }],
    });
    function.blocks[0].metadata = block_metadata_cleanup(0);
    let dynamic_program = verify(dynamic_program)?;
    let (outcome, observation) = evaluate_observed(&dynamic_program, &EvalConfig::default());
    assert!(matches!(
        outcome,
        EvalOutcome::Returned(EvalValue::ReturnedOwned(ref value))
            if matches!(value.as_structural().map(|value| &value.payload),
                Some(SemanticPayload::String(bytes)) if bytes.is_empty())
    ));
    assert_eq!(observation.metrics.allocations, 1);
    assert!(observation
        .events
        .iter()
        .any(|event| event.kind == StructuralEventKind::Publish));
    assert!(observation.assert_empty().is_ok());
    Ok(())
}

#[test]
fn legacy_product_constructor_cannot_bypass_structural_destination() -> crate::Result<()> {
    let mut program = one_block_program();
    let product = ProductId::new(0);
    let fields = vec![SsaType::Str, SsaType::I64];
    program.products.push(ProductMetadata {
        id: product,
        name: "text-and-count".into(),
        fields: vec![
            ProductField {
                name: "text".into(),
                ty: SsaType::Str,
            },
            ProductField {
                name: "count".into(),
                ty: SsaType::I64,
            },
        ],
    });
    install_structural_type(&mut program, SsaType::Str, StructuralLayoutKind::String);
    install_structural_type(
        &mut program,
        SsaType::Product(product),
        StructuralLayoutKind::Product { product, fields },
    );
    let function = &mut program.functions[0];
    *function.signature.result = SsaType::Product(product);
    function.effects = EffectSet::ALLOCATES;
    function.blocks[0].instructions = vec![
        Instruction {
            id: ValueId::new(0),
            ty: SsaType::Str,
            kind: InstructionKind::Constant(Constant::Str("nested".into())),
            metadata: metadata(EffectSet::PURE),
        },
        Instruction {
            id: ValueId::new(1),
            ty: SsaType::I64,
            kind: InstructionKind::Constant(Constant::I64(7)),
            metadata: metadata(EffectSet::PURE),
        },
        Instruction {
            id: ValueId::new(2),
            ty: SsaType::Product(product),
            kind: InstructionKind::ProductValue {
                product,
                fields: vec![ValueId::new(0), ValueId::new(1)],
            },
            metadata: allocating_metadata(2, EffectSet::ALLOCATES),
        },
    ];
    function.blocks[0].terminator = Terminator::Return(ValueId::new(2));
    function.failure_cleanups.push(FailureCleanupPlan {
        id: FailureCleanupId::new(0),
        actions: vec![FailureCleanupAction::DropOwner {
            place: None,
            value: ValueId::new(2),
            glue: DropGlueIdentity::Structural(StructuralDropGlueIdentity::Product {
                type_id: StructuralTypeId::new(1),
                product,
                layout: StructuralLayoutId::new(1),
            }),
        }],
    });
    function.blocks[0].metadata = block_metadata_cleanup(0);
    let error = verify(program)
        .err()
        .ok_or_else(|| IrError::new("legacy constructor selected structural memory"))?;
    assert!(
        error
            .to_string()
            .contains("ProductValue cannot construct a type with structural metadata"),
        "{error}"
    );
    Ok(())
}
