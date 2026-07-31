#[test]
fn structural_borrow_blocks_drop() {
    let mut chunk = product_chunk();
    emit_finished_product(&mut chunk);
    chunk.main.emit_op_u8(Op::LoadStructuralOwnerLocal, 1);
    chunk.main.emit_op_u16(Op::StructuralBorrow, 1);
    chunk.main.emit_op_u8(Op::StoreStructuralLocal, 2);
    chunk.main.emit_op_u16(Op::StructuralDropPlace, 1);
    chunk.main.emit(Op::Return);
    assert!(error(chunk).contains("live loan"));
}

#[test]
fn structural_use_after_move_and_copy_attempt_fail_closed() {
    let mut moved = product_chunk();
    emit_finished_product(&mut moved);
    moved.main.emit_op_u16(Op::StructuralMove, 1);
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
    affine_copy.main.emit_op_u16(Op::StructuralCopy, 0);
    affine_copy.main.emit(Op::Return);
    assert!(error(affine_copy).contains("cannot duplicate an affine owner"));
}

#[test]
fn structural_owner_reference_blocks_owner_drop() {
    let mut chunk = product_chunk();
    emit_finished_product(&mut chunk);
    chunk.main.emit_op_u8(Op::LoadStructuralOwnerLocal, 1);
    chunk.main.emit_op_u16(Op::StructuralDropPlace, 1);
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
                physical_tag: 0,
                fields: vec![copy_field()],
            },
            crate::StructuralVariantLayout {
                variant: second,
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
    }];
    emit_finished_product(&mut chunk);
    chunk.main.emit_op_u16(Op::StructuralMove, 1);
    chunk
        .main
        .emit_op_u16(Op::StructuralAggregateConsumePayload, 0);
    chunk.main.emit(Op::Return);
    let message = error(chunk);
    assert!(message.contains("inactive variant"), "{message}");
}
