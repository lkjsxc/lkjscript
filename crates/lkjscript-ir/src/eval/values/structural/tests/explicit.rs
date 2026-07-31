use super::*;

#[test]
fn explicit_publish_destination_and_finish_execute_in_structural_runtime() -> crate::Result<()> {
    let mut program = one_block_program();
    let product = ProductId::new(0);
    program.products.push(ProductMetadata {
        id: product,
        name: "published-text".into(),
        fields: vec![ProductField {
            name: "text".into(),
            ty: SsaType::Str,
        }],
    });
    install_structural_type(&mut program, SsaType::Str, StructuralLayoutKind::String);
    install_structural_type(
        &mut program,
        SsaType::Product(product),
        StructuralLayoutKind::Product {
            product,
            fields: vec![SsaType::Str],
        },
    );
    let destination_type = SsaType::StructuralDestination(StructuralTypeId::new(1));
    let function = &mut program.functions[0];
    *function.signature.result = SsaType::Product(product);
    function.effects = EffectSet::ALLOCATES.union(EffectSet::WRITES_MEMORY);
    let string_glue = DropGlueIdentity::Structural(StructuralDropGlueIdentity::String {
        type_id: StructuralTypeId::new(0),
        layout: StructuralLayoutId::new(0),
    });
    let destination_glue = DropGlueIdentity::Structural(StructuralDropGlueIdentity::Destination {
        type_id: StructuralTypeId::new(1),
        layout: StructuralLayoutId::new(1),
    });
    let product_glue = DropGlueIdentity::Structural(StructuralDropGlueIdentity::Product {
        type_id: StructuralTypeId::new(1),
        product,
        layout: StructuralLayoutId::new(1),
    });
    function.failure_cleanups = vec![
        cleanup_plan(0, vec![drop_action(1, string_glue)]),
        cleanup_plan(
            1,
            vec![
                drop_action(2, destination_glue),
                drop_action(1, string_glue),
            ],
        ),
        cleanup_plan(2, vec![drop_action(3, destination_glue)]),
        cleanup_plan(3, vec![drop_action(4, product_glue)]),
    ];
    function.blocks[0].instructions = vec![
        Instruction {
            id: ValueId::new(0),
            ty: SsaType::Str,
            kind: InstructionKind::Constant(Constant::Str("published".into())),
            metadata: metadata(EffectSet::PURE),
        },
        Instruction {
            id: ValueId::new(1),
            ty: SsaType::Str,
            kind: InstructionKind::StructuralPublish {
                representation: StructuralRepresentationId::new(0),
                value: ValueId::new(0),
            },
            metadata: allocating_metadata(1, EffectSet::ALLOCATES),
        },
        Instruction {
            id: ValueId::new(2),
            ty: destination_type.clone(),
            kind: InstructionKind::DestinationCreate {
                representation: StructuralRepresentationId::new(5),
                active_variant: None,
            },
            metadata: allocating_cleanup_metadata(2, EffectSet::ALLOCATES, 0),
        },
        Instruction {
            id: ValueId::new(3),
            ty: destination_type,
            kind: InstructionKind::DestinationFieldInit {
                destination: ValueId::new(2),
                field: 0,
                value: ValueId::new(1),
            },
            metadata: allocating_cleanup_metadata(
                3,
                EffectSet::WRITES_MEMORY.union(EffectSet::ALLOCATES),
                1,
            ),
        },
        Instruction {
            id: ValueId::new(4),
            ty: SsaType::Product(product),
            kind: InstructionKind::DestinationFinish {
                destination: ValueId::new(3),
            },
            metadata: allocating_cleanup_metadata(4, EffectSet::ALLOCATES, 2),
        },
    ];
    function.blocks[0].terminator = Terminator::Return(ValueId::new(4));
    function.blocks[0].metadata.failure_cleanup = Some(FailureCleanupId::new(3));

    let program = verify(program)?;
    let (outcome, observation) = evaluate_observed(&program, &EvalConfig::default());
    assert!(matches!(
        outcome,
        EvalOutcome::Returned(EvalValue::ReturnedOwned(ref value))
            if matches!(value.as_structural().map(|value| &value.payload),
                Some(SemanticPayload::Product(fields))
                    if matches!(&fields[0].payload,
                        SemanticPayload::String(bytes) if bytes == b"published"))
    ));
    assert!(observation
        .events
        .iter()
        .any(|event| event.kind == StructuralEventKind::Publish));
    assert_eq!(observation.metrics.destinations_created, 1);
    assert_eq!(observation.metrics.destinations_completed, 1);
    assert!(observation.assert_empty().is_ok());
    Ok(())
}

#[test]
fn affine_structural_copy_is_rejected_before_evaluation() {
    let mut program = one_block_program();
    install_structural_type(&mut program, SsaType::Str, StructuralLayoutKind::String);
    program.memory.types[0].mode = StructuralTypeMode::Affine;
    let function = &mut program.functions[0];
    *function.signature.result = SsaType::Str;
    function.effects = EffectSet::ALLOCATES;
    let glue = DropGlueIdentity::Structural(StructuralDropGlueIdentity::String {
        type_id: StructuralTypeId::new(0),
        layout: StructuralLayoutId::new(0),
    });
    function.failure_cleanups = vec![
        cleanup_plan(0, vec![drop_action(1, glue)]),
        cleanup_plan(1, vec![drop_action(2, glue), drop_action(1, glue)]),
    ];
    function.blocks[0].instructions = vec![
        Instruction {
            id: ValueId::new(0),
            ty: SsaType::Str,
            kind: InstructionKind::Constant(Constant::Str("affine".into())),
            metadata: metadata(EffectSet::PURE),
        },
        Instruction {
            id: ValueId::new(1),
            ty: SsaType::Str,
            kind: InstructionKind::StructuralPublish {
                representation: StructuralRepresentationId::new(0),
                value: ValueId::new(0),
            },
            metadata: allocating_metadata(1, EffectSet::ALLOCATES),
        },
        Instruction {
            id: ValueId::new(2),
            ty: SsaType::Str,
            kind: InstructionKind::StructuralCopy {
                representation: StructuralRepresentationId::new(0),
                value: ValueId::new(1),
            },
            metadata: allocating_cleanup_metadata(2, EffectSet::ALLOCATES, 0),
        },
    ];
    function.blocks[0].terminator = Terminator::Return(ValueId::new(2));
    function.blocks[0].metadata.failure_cleanup = Some(FailureCleanupId::new(1));
    assert!(matches!(
        verify(program),
        Err(error) if error.to_string().contains("cannot duplicate an affine owner")
    ));
}

fn cleanup_plan(id: u32, actions: Vec<FailureCleanupAction>) -> FailureCleanupPlan {
    FailureCleanupPlan {
        id: FailureCleanupId::new(id),
        actions,
    }
}

fn drop_action(value: u32, glue: DropGlueIdentity) -> FailureCleanupAction {
    FailureCleanupAction::DropOwner {
        place: None,
        value: ValueId::new(value),
        glue,
    }
}
