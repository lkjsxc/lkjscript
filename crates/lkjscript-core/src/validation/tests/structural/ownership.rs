#[test]
fn structural_borrow_blocks_drop() {
    let mut chunk = product_chunk();
    emit_finished_product(&mut chunk);
    chunk.main.emit_op_u8(Op::LoadStructuralOwnerLocal, 1);
    chunk.main.emit_op_u64(Op::StructuralBorrow, 1);
    chunk.main.emit_op_u8(Op::StoreStructuralLocal, 2);
    chunk
        .main
        .emit_op_u64_pair(Op::StructuralDropPlace, 0, 1);
    chunk.main.emit(Op::Return);
    assert!(error(chunk).contains("live loan"));
}

#[test]
fn structural_use_after_move_and_copy_attempt_fail_closed() {
    let mut moved = product_chunk();
    emit_finished_product(&mut moved);
    moved.main.emit_op_u64_pair(Op::StructuralMove, 0, 1);
    moved.main.emit_op_u8(Op::StoreStructuralLocal, 2);
    moved.main.emit_op_u8(Op::LoadStructuralOwnerLocal, 1);
    moved.main.emit(Op::Return);
    assert!(error(moved).contains("owner local is empty"));

    let mut copied = product_chunk();
    emit_finished_product(&mut copied);
    copied.main.emit_op_u8(Op::LoadLocal, 1);
    copied.main.emit(Op::Return);
    assert!(error(copied).contains("typed local opcodes"));

    let mut affine_copy = product_chunk();
    emit_finished_product(&mut affine_copy);
    affine_copy
        .main
        .emit_op_u8(Op::LoadStructuralOwnerLocal, 1);
    affine_copy.main.emit_op_u64(Op::StructuralCopy, 0);
    affine_copy.main.emit(Op::Return);
    assert!(error(affine_copy).contains("cannot duplicate an affine owner"));
}

#[test]
fn aggregate_field_copy_rejects_noncopy_structural_targets() {
    let mut chunk = product_chunk();
    let target_type = runtime_type(4, crate::StructuralKind::String);
    let target_id = crate::StructuralTypeId::new(1);
    let field = crate::StructuralFieldMetadata {
        identity: identity(7),
        runtime_type: Some(target_type),
        route: crate::StructuralFieldRoute::Structural(target_id),
        resource: None,
    };
    assert!(matches!(
        chunk.structural_layouts[0].kind,
        crate::StructuralLayoutKind::Product { .. }
    ));
    if let crate::StructuralLayoutKind::Product { fields, .. } =
        &mut chunk.structural_layouts[0].kind
    {
        fields[0] = field;
    }
    chunk.structural_destinations[0].fields[0] = field;
    chunk.structural_layouts.push(crate::StructuralLayoutMetadata {
        id: crate::StructuralLayoutId::new(1),
        identity: identity(6),
        kind: crate::StructuralLayoutKind::String,
    });
    chunk.structural_types.push(crate::StructuralTypeMetadata {
        id: target_id,
        witness: witness(2),
        identity: identity(7),
        runtime_type: target_type,
        kind: crate::StructuralTypeKind::String,
        layout: crate::StructuralLayoutId::new(1),
        mode: crate::StructuralTypeMode::Immutable,
    });
    chunk.structural_representations.push(
        crate::StructuralRepresentationMetadata {
            id: crate::StructuralRepresentationId::new(3),
            type_id: target_id,
            witness: witness(2),
            witness_group: crate::MemoryWitnessGroupId::new([0; 32]),
            witness_member: 0,
            layout: crate::StructuralLayoutId::new(1),
            category: crate::StructuralValueCategory::Owner,
            storage: crate::StructuralStorage::UniqueStructural,
            route: [4; 32],
        },
    );
    chunk.structural_aggregate_fields = vec![crate::StructuralAggregateFieldRef {
        representation: crate::StructuralRepresentationId::new(1),
        active_variant: None,
        field: 0,
        result: field,
        result_representation: Some(crate::StructuralRepresentationId::new(3)),
    }];
    let mut proto = Chunk::new().main;
    proto.name = "malicious-field-copy".into();
    proto.arity = 1;
    proto.locals = 1;
    proto.memory_plan = chunk.memory_plan;
    proto.parameter_structurals = vec![Some(crate::StructuralRepresentationId::new(0))];
    proto.parameter_structural_places = vec![None];
    proto.parameter_type_variables = vec![None];
    proto.return_structural = Some(crate::StructuralRepresentationId::new(3));
    proto.emit_op_u64(Op::LoadStructuralOwnerLocal, 0);
    proto.emit_op_u64(Op::StructuralAggregateFieldCopy, 0);
    proto.emit(Op::Return);
    chunk.protos.push(proto);
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    let mut missing = chunk.clone();
    missing.structural_aggregate_fields[0].result_representation = None;
    assert!(error(missing).contains("result representation is missing"));
    let message = error(chunk);
    assert!(
        message.contains("copy field target is not copy-mode"),
        "{message}"
    );
}

#[test]
fn malformed_wide_aggregate_field_reference_is_rejected_before_indexing() {
    let mut chunk = product_chunk();
    chunk.structural_aggregate_fields = vec![crate::StructuralAggregateFieldRef {
        representation: crate::StructuralRepresentationId::new(1),
        active_variant: None,
        field: 300,
        result: copy_field(),
        result_representation: None,
    }];
    let message = error(chunk);
    assert!(message.contains("aggregate-field result metadata is stale"), "{message}");
}

#[test]
fn structural_owner_reference_blocks_owner_drop() {
    let mut chunk = product_chunk();
    emit_finished_product(&mut chunk);
    chunk.main.emit_op_u8(Op::LoadStructuralOwnerLocal, 1);
    chunk
        .main
        .emit_op_u64_pair(Op::StructuralDropPlace, 0, 1);
    chunk.main.emit(Op::Return);
    assert!(error(chunk).contains("live loan"));
}

#[test]
fn inactive_enum_payload_is_rejected() {
    let mut chunk = product_chunk();
    let enum_id = crate::EnumId::new([9; 32]);
    let first = crate::VariantId::new([1; 32]);
    let second = crate::VariantId::new([2; 32]);
    chunk.structural_layouts[0].kind = crate::StructuralLayoutKind::Enum {
        enum_id,
        runtime_layout: identity(4),
        variants: vec![
            crate::StructuralVariantLayout {
                variant: first,
                source_order: 0,
                physical_tag: 0,
                fields: vec![copy_field()],
            },
            crate::StructuralVariantLayout {
                variant: second,
                source_order: 1,
                physical_tag: 1,
                fields: vec![copy_field()],
            },
        ],
    };
    chunk.structural_types[0].kind = crate::StructuralTypeKind::Enum(enum_id);
    chunk.structural_types[0].runtime_type.kind = crate::StructuralKind::Enum;
    chunk.structural_destinations[0].active_variant = Some(first);
    chunk.structural_payloads = vec![crate::StructuralPayloadRef {
        representation: crate::StructuralRepresentationId::new(0),
        variant: second,
        result: copy_field(),
        result_representation: None,
    }];
    emit_finished_product(&mut chunk);
    chunk.main.emit_op_u64_pair(Op::StructuralMove, 0, 1);
    chunk
        .main
        .emit_op_u64(Op::StructuralAggregateConsumePayload, 0);
    chunk.main.emit(Op::Return);
    let message = error(chunk);
    assert!(message.contains("inactive variant"), "{message}");
}
