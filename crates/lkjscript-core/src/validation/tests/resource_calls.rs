use super::*;

#[test]
fn resource_call_and_return_metadata_is_enforced() {
    let mut call = unit_chunk();
    let prototype = call.add_const(Constant::Proto(0));
    call.protos.push(FunctionProto {
        name: "resource-parameter".into(),
        arity: 1,
        locals: 1,
        memory_plan: None,
        memory_witness_parameters: Vec::new(),
        call_witnesses: Vec::new(),
        parameter_structurals: Vec::new(),
        parameter_structural_places: Vec::new(),
        parameter_type_variables: Vec::new(),
        parameter_copy_kinds: vec![None],
        parameter_region_products: vec![None],
        return_copy_kind: None,
        return_region_product: None,
        return_structural: None,
        return_type_variable: None,
        parameter_resources: vec![Some(crate::ResourceKind::FileReader)],
        parameter_resource_places: vec![Some(0)],
        return_resource: None,
        parameter_uniques: Vec::new(),
        parameter_unique_places: Vec::new(),
        return_unique: None,
        unique_places: 1,
        failure_cleanups: Vec::new(),
        failure_cleanup_ranges: Vec::new(),
        code: {
            let mut code = index_instruction(Op::LoadLocal, 0);
            code.push(Op::Return as u8);
            code
        },
    });
    call.main.code.clear();
    call.main.emit(Op::Unit);
    call.main.emit_op_u16(Op::LoadConst, prototype.0);
    call.main.emit(Op::MakeClosure);
    call.main.emit_u16(0);
    call.main.emit_op_u8(Op::Call, 1);
    call.main.emit(Op::Return);
    let message = error(call);
    assert!(
        message.contains("call argument does not match"),
        "{message}"
    );

    let mut returned = unit_chunk();
    returned.protos.push(FunctionProto {
        name: "borrowed-resource-return".into(),
        arity: 1,
        locals: 1,
        memory_plan: None,
        memory_witness_parameters: Vec::new(),
        call_witnesses: Vec::new(),
        parameter_structurals: Vec::new(),
        parameter_structural_places: Vec::new(),
        parameter_type_variables: Vec::new(),
        parameter_copy_kinds: vec![None],
        parameter_region_products: vec![None],
        return_copy_kind: None,
        return_region_product: None,
        return_structural: None,
        return_type_variable: None,
        parameter_resources: vec![Some(crate::ResourceKind::InputStream)],
        parameter_resource_places: vec![None],
        return_resource: Some(crate::ResourceReturnKind::Resource(
            crate::ResourceKind::InputStream,
        )),
        parameter_uniques: Vec::new(),
        parameter_unique_places: Vec::new(),
        return_unique: None,
        unique_places: 0,
        failure_cleanups: Vec::new(),
        failure_cleanup_ranges: Vec::new(),
        code: {
            let mut code = index_instruction(Op::LoadLocal, 0);
            code.push(Op::Return as u8);
            code
        },
    });
    let message = error(returned);
    assert!(message.contains("return does not match"), "{message}");
}
