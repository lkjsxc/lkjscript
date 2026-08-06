use super::*;

fn legacy_product() -> Chunk {
    let mut chunk = unit_chunk();
    chunk.products.push(ProductMetadata {
        id: ProductId::new(0),
        identity: crate::RuntimeLayoutId::new([1; 32]),
        region: false,
        name: "p".into(),
        fields: vec!["x".into()],
        region_fields: Vec::new(),
    });
    chunk.product_fields.push(ProductFieldRef {
        product: ProductId::new(0),
        field: 0,
    });
    chunk
}

#[test]
fn legacy_product_bytecode_operations_are_removed() {
    let mut construction = legacy_product();
    construction.main.code.clear();
    construction.main.emit(Op::Unit);
    construction.main.emit_op_u64(Op::MakeProduct, 0);
    construction.main.emit(Op::Return);
    assert!(error(construction)
        .contains("product construction requires structural or invocation-region metadata"));

    let mut projection = legacy_product();
    projection.main.code.clear();
    projection.main.emit(Op::Unit);
    projection.main.emit_op_u64(Op::LoadProductField, 0);
    projection.main.emit(Op::Return);
    assert!(error(projection)
        .contains("product projection requires structural or invocation-region metadata"));

    let mut update = legacy_product();
    update.main.code.clear();
    update.main.emit(Op::Unit);
    update.main.emit(Op::Unit);
    update.main.emit_op_u64(Op::WithProductField, 0);
    update.main.emit(Op::Return);
    assert!(
        error(update).contains("product update requires structural or invocation-region metadata")
    );
}
