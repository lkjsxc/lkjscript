#[test]
fn structural_table_accepts_configured_max_and_rejects_plus_one() {
    let mut chunk = product_chunk();
    chunk.structural_representations.clear();
    let product_type = chunk.structural_types[0].runtime_type;
    chunk.structural_destinations.clear();
    chunk.structural_destination_fields.clear();
    chunk
        .structural_types
        .extend((1..3).map(|raw| crate::StructuralTypeMetadata {
            id: crate::StructuralTypeId::new(raw),
            witness: witness(u8::try_from(raw).unwrap_or(u8::MAX).saturating_add(1)),
            identity: identity(u8::try_from(raw).unwrap_or(u8::MAX)),
            runtime_type: product_type,
            kind: crate::StructuralTypeKind::Product(crate::ProductId::new(0)),
            layout: crate::StructuralLayoutId::new(0),
            mode: crate::StructuralTypeMode::Affine,
        }));
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    let limits = ValidationLimits {
        max_table_entries: 3,
        ..ValidationLimits::default()
    };
    validate_chunk(chunk.clone(), &limits).expect("configured structural maximum validates");
    chunk.structural_types.push(crate::StructuralTypeMetadata {
        id: crate::StructuralTypeId::new(3),
        witness: witness(4),
        identity: identity(6),
        runtime_type: product_type,
        kind: crate::StructuralTypeKind::Product(crate::ProductId::new(0)),
        layout: crate::StructuralLayoutId::new(0),
        mode: crate::StructuralTypeMode::Affine,
    });
    assert!(validate_chunk(chunk, &limits)
        .expect_err("configured maximum plus one must fail")
        .to_string()
        .contains("limit 3"));
}
