use super::*;

#[allow(clippy::expect_used)]
pub(crate) fn test_chunk() -> ValidatedChunk {
    let mut chunk = lkjscript_core::Chunk::new();
    chunk.main.emit(lkjscript_core::Op::Unit);
    chunk.main.emit(lkjscript_core::Op::Return);
    chunk.protos.push(lkjscript_core::FunctionProto {
        name: "test-function".into(),
        arity: 0,
        locals: 0,
        memory_plan: None,
        parameter_structurals: Vec::new(),
        parameter_structural_places: Vec::new(),
        parameter_type_variables: Vec::new(),
        parameter_copy_kinds: Vec::new(),
        parameter_region_products: Vec::new(),
        return_copy_kind: None,
        return_region_product: None,
        return_structural: None,
        return_type_variable: None,
        parameter_resources: Vec::new(),
        parameter_resource_places: Vec::new(),
        return_resource: None,
        parameter_uniques: Vec::new(),
        parameter_unique_places: Vec::new(),
        return_unique: None,
        unique_places: 0,
        failure_cleanups: Vec::new(),
        failure_cleanup_ranges: Vec::new(),
        code: vec![
            lkjscript_core::Op::Unit as u8,
            lkjscript_core::Op::Return as u8,
        ],
    });
    lkjscript_core::validate_chunk(chunk, &lkjscript_core::ValidationLimits::default())
        .expect("VM unit-test chunk validates")
}
