fn returning_product_chunk() -> crate::ValidatedChunk {
    returning_product_chunk_with_mode(crate::StructuralTypeMode::Immutable)
}

fn returning_product_chunk_with_mode(
    mode: crate::StructuralTypeMode,
) -> crate::ValidatedChunk {
    returning_product_chunk_from(product_chunk(), mode)
}

fn returning_product_chunk_from(
    mut chunk: Chunk,
    mode: crate::StructuralTypeMode,
) -> crate::ValidatedChunk {
    chunk.structural_types[0].mode = mode;
    install_authenticated_return_witness(&mut chunk, mode);
    chunk.main.locals = 1;
    chunk.main.return_structural = Some(crate::StructuralRepresentationId::new(0));
    let value = chunk.add_const(crate::Constant::I64(41));
    chunk
        .main
        .emit_op_u16(Op::StructuralDestinationCreate, 0);
    chunk.main.emit_op_u8(Op::StoreStructuralLocal, 0);
    chunk.main.emit_op_u8(Op::TakeStructuralLocal, 0);
    chunk.main.emit_op_u16(Op::LoadConst, value.0);
    chunk
        .main
        .emit_op_u16(Op::StructuralDestinationFieldInit, 0);
    chunk
        .main
        .emit_op_u16(Op::StructuralDestinationFinish, 0);
    chunk.main.emit(Op::Return);
    crate::validate_chunk(chunk, crate::ValidationPolicy::Unrestricted)
        .expect("validated structural return")
}

fn semantic_type(value_type: crate::StructuralType) -> Option<crate::SemanticDagType> {
    let kind = match value_type.kind {
        crate::StructuralKind::Unit => crate::SemanticDagKind::Unit,
        crate::StructuralKind::Bool => crate::SemanticDagKind::Bool,
        crate::StructuralKind::I64 => crate::SemanticDagKind::I64,
        crate::StructuralKind::F64 => crate::SemanticDagKind::F64,
        crate::StructuralKind::String => crate::SemanticDagKind::String,
        crate::StructuralKind::Path => crate::SemanticDagKind::Path,
        crate::StructuralKind::Bytes => crate::SemanticDagKind::Bytes,
        crate::StructuralKind::Product => crate::SemanticDagKind::Product,
        crate::StructuralKind::Enum => crate::SemanticDagKind::Enum,
        crate::StructuralKind::Static => crate::SemanticDagKind::Static,
        crate::StructuralKind::ByteVector => return None,
    };
    Some(crate::SemanticDagType::new(
        value_type.layout,
        value_type.semantic_type,
        kind,
    ))
}

fn returned_product_snapshot(
    chunk: &crate::ValidatedChunk,
    field_type: crate::StructuralType,
    field_count: usize,
) -> crate::SemanticDagSnapshot {
    let root_type = chunk.structural_types()[0].runtime_type;
    let mut nodes = vec![crate::SemanticDagNode::new(
        semantic_type(field_type).expect("supported field type"),
        crate::SemanticDagPayload::Inline(crate::InlineStructuralValue::I64(41)),
    )];
    nodes.push(crate::SemanticDagNode::new(
        semantic_type(root_type).expect("supported root type"),
        crate::SemanticDagPayload::Product(
            (0..field_count)
                .map(|_| crate::SemanticDagNodeId::new(0))
                .collect(),
        ),
    ));
    crate::SemanticDagSnapshot::new(
        nodes,
        crate::SemanticDagNodeId::new(1),
        crate::StructuralSnapshotLimits::DEFAULT,
    )
    .expect("semantic DAG")
}

#[test]
fn validated_structural_return_derives_exact_sealed_dag_shape() {
    let chunk = returning_product_chunk();
    let field_type = copy_field().runtime_type.expect("copy runtime type");
    let snapshot = returned_product_snapshot(&chunk, field_type, 1);
    let expected = snapshot.clone();
    let mut runtime = crate::SealedSemanticDagRuntime::new(crate::StructuralLimits::default())
        .expect("sealed runtime");
    let owner = runtime
        .rehydrate_authenticated_return(&chunk, snapshot)
        .expect("authenticated rehydration");
    let borrow = runtime.begin_borrow(&owner).expect("borrow");
    assert_eq!(runtime.export_snapshot(&borrow).expect("export"), expected);
    runtime.end_borrow(borrow).expect("end borrow");
    assert_eq!(runtime.release(owner).expect("release").regions_released, 1);
}

#[test]
fn validated_structural_return_requires_exact_return_metadata() {
    let chunk = crate::validate_chunk(unit_chunk(), crate::ValidationPolicy::Unrestricted)
        .expect("validated unit chunk");
    let snapshot = crate::SemanticDagSnapshot::new(
        vec![crate::SemanticDagNode::new(
            semantic_type(runtime_type(9, crate::StructuralKind::Unit))
                .expect("supported unit type"),
            crate::SemanticDagPayload::Inline(crate::InlineStructuralValue::Unit),
        )],
        crate::SemanticDagNodeId::new(0),
        crate::StructuralSnapshotLimits::DEFAULT,
    )
    .expect("unit snapshot");
    let mut runtime = crate::SealedSemanticDagRuntime::new(crate::StructuralLimits::default())
        .expect("sealed runtime");
    let failure = runtime
        .rehydrate_authenticated_return(&chunk, snapshot)
        .expect_err("missing structural return rejected");
    assert_eq!(failure.error, crate::SealedSemanticDagError::MissingValidatedReturn);
    assert_eq!(runtime.metrics().runtime.live_domains, 0);
}

#[test]
fn validated_structural_return_rejects_affine_and_resource_metadata() {
    let affine = returning_product_chunk_with_mode(crate::StructuralTypeMode::Affine);
    let field_type = copy_field().runtime_type.expect("copy runtime type");
    let snapshot = returned_product_snapshot(&affine, field_type, 1);
    let mut runtime = crate::SealedSemanticDagRuntime::new(crate::StructuralLimits::default())
        .expect("sealed runtime");
    let failure = runtime
        .rehydrate_authenticated_return(&affine, snapshot)
        .expect_err("affine root rejected");
    assert_eq!(
        failure.error,
        crate::SealedSemanticDagError::UnauthenticatedValidatedType
    );

    let mut marked = product_chunk();
    if let crate::StructuralLayoutKind::Product { fields, .. } =
        &mut marked.structural_layouts[0].kind
    {
        fields[0].resource = Some(crate::ResourceKind::FileReader);
    }
    marked.structural_destinations[0].fields[0].resource =
        Some(crate::ResourceKind::FileReader);
    let marked = returning_product_chunk_from(marked, crate::StructuralTypeMode::Immutable);
    let snapshot = returned_product_snapshot(&marked, field_type, 1);
    let failure = runtime
        .rehydrate_authenticated_return(&marked, snapshot)
        .expect_err("resource-marked field rejected");
    assert_eq!(failure.error, crate::SealedSemanticDagError::UnsupportedValidatedType);
    assert_eq!(runtime.metrics().runtime.live_domains, 0);
}

#[test]
fn validated_structural_return_rejects_forged_type_and_shape_before_allocation() {
    let chunk = returning_product_chunk();
    let forged = runtime_type(44, crate::StructuralKind::I64);
    let snapshot = returned_product_snapshot(&chunk, forged, 1);
    let mut runtime = crate::SealedSemanticDagRuntime::new(crate::StructuralLimits::default())
        .expect("sealed runtime");
    let failure = runtime
        .rehydrate_authenticated_return(&chunk, snapshot)
        .expect_err("forged type rejected");
    assert_eq!(failure.error, crate::SealedSemanticDagError::ValidatedShapeMismatch);
    assert_eq!(runtime.metrics().runtime.live_domains, 0);

    let root_type = chunk.structural_types()[0].runtime_type;
    let snapshot = crate::SemanticDagSnapshot::new(
        vec![crate::SemanticDagNode::new(
            semantic_type(root_type).expect("supported root type"),
            crate::SemanticDagPayload::Product(Vec::new()),
        )],
        crate::SemanticDagNodeId::new(0),
        crate::StructuralSnapshotLimits::DEFAULT,
    )
    .expect("empty product snapshot");
    let failure = runtime
        .rehydrate_authenticated_return(&chunk, snapshot)
        .expect_err("field shape rejected");
    assert_eq!(failure.error, crate::SealedSemanticDagError::ValidatedShapeMismatch);
    assert_eq!(runtime.metrics().runtime.live_domains, 0);
}
