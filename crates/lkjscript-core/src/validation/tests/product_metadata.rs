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
    construction.main.code = vec![
        Op::Unit as u8,
        Op::MakeProduct as u8,
        0,
        0,
        Op::Return as u8,
    ];
    assert!(error(construction)
        .contains("product construction requires structural or invocation-region metadata"));

    let mut projection = legacy_product();
    projection.main.code = vec![
        Op::Unit as u8,
        Op::LoadProductField as u8,
        0,
        0,
        Op::Return as u8,
    ];
    assert!(error(projection)
        .contains("product projection requires structural or invocation-region metadata"));

    let mut update = legacy_product();
    update.main.code = vec![
        Op::Unit as u8,
        Op::Unit as u8,
        Op::WithProductField as u8,
        0,
        0,
        Op::Return as u8,
    ];
    assert!(
        error(update).contains("product update requires structural or invocation-region metadata")
    );
}
