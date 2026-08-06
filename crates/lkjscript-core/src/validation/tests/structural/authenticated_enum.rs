#[test]
fn authenticated_enum_validates_active_variant_and_round_trips() {
    let (chunk, enum_type, field_type) = returning_enum_chunk();
    let snapshot = enum_snapshot(enum_type, field_type, 0);
    let expected = snapshot.clone();
    let mut runtime = crate::SealedSemanticDagRuntime::new(crate::StructuralLimits::default())
        .expect("sealed runtime");
    let owner = runtime
        .rehydrate_authenticated_return(&chunk, snapshot)
        .expect("authenticated enum rehydration");
    let borrow = runtime.begin_borrow(&owner).expect("enum borrow");
    assert_eq!(runtime.export_snapshot(&borrow).expect("export"), expected);
    runtime.end_borrow(borrow).expect("end borrow");
    runtime.release(owner).expect("release enum");
    assert_eq!(runtime.metrics().runtime.live_domains, 0);

    let wrong_tag = enum_snapshot(enum_type, field_type, 9);
    let failure = runtime
        .rehydrate_authenticated_return(&chunk, wrong_tag)
        .expect_err("unknown physical tag rejected");
    assert_eq!(
        failure.error,
        crate::SealedSemanticDagError::ValidatedShapeMismatch
    );
    assert_eq!(runtime.metrics().runtime.live_domains, 0);
}

fn returning_enum_chunk() -> (
    crate::ValidatedChunk,
    crate::StructuralType,
    crate::StructuralType,
) {
    let mut chunk = product_chunk();
    let enum_id = crate::EnumId::new([19; 32]);
    let first = crate::VariantId::new([20; 32]);
    let second = crate::VariantId::new([21; 32]);
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
                fields: Vec::new(),
            },
        ],
    };
    chunk.structural_types[0].kind = crate::StructuralTypeKind::Enum(enum_id);
    chunk.structural_types[0].runtime_type = runtime_type(22, crate::StructuralKind::Enum);
    chunk.structural_types[0].mode = crate::StructuralTypeMode::Immutable;
    chunk.structural_destinations[0].active_variant = Some(first);
    install_authenticated_return_witness(&mut chunk, crate::StructuralTypeMode::Immutable);
    chunk.main.locals = 1;
    chunk.main.return_structural = Some(crate::StructuralRepresentationId::new(0));
    let value = chunk
        .add_const(crate::Constant::I64(41))
        .expect("add value constant");
    chunk
        .main
        .emit_op_u16(Op::StructuralDestinationCreate, 0);
    chunk.main.emit_op_u8(Op::StoreStructuralLocal, 0);
    chunk.main.emit_op_u8(Op::TakeStructuralLocal, 0);
    chunk.main.emit_op_u64(Op::LoadConst, value.0);
    chunk
        .main
        .emit_op_u16(Op::StructuralDestinationFieldInit, 0);
    chunk
        .main
        .emit_op_u16(Op::StructuralDestinationFinish, 0);
    chunk.main.emit(Op::Return);
    let chunk = crate::validate_chunk(chunk, crate::ValidationPolicy::Unrestricted)
        .expect("validated enum return");
    let enum_type = chunk.structural_types()[0].runtime_type;
    let field_type = copy_field().runtime_type.expect("field type");
    (chunk, enum_type, field_type)
}

fn enum_snapshot(
    enum_type: crate::StructuralType,
    field_type: crate::StructuralType,
    tag: u16,
) -> crate::SemanticDagSnapshot {
    crate::SemanticDagSnapshot::new(
        vec![
            crate::SemanticDagNode::new(
                semantic_type(field_type).expect("field semantic type"),
                crate::SemanticDagPayload::Inline(crate::InlineStructuralValue::I64(41)),
            ),
            crate::SemanticDagNode::new(
                semantic_type(enum_type).expect("enum semantic type"),
                crate::SemanticDagPayload::Enum {
                    tag,
                    fields: vec![crate::SemanticDagNodeId::new(0)],
                },
            ),
        ],
        crate::SemanticDagNodeId::new(1),
        crate::StructuralSnapshotLimits::DEFAULT,
    )
    .expect("enum snapshot")
}
