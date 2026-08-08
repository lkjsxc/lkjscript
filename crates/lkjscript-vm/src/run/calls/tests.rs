use super::*;
use lkjscript_core::{validate_chunk, Chunk, ExecutionPolicy, FunctionProto, ValidationPolicy};

fn index_instruction(op: Op, index: u64) -> Vec<u8> {
    let mut code = vec![op as u8];
    code.extend_from_slice(&index.to_le_bytes());
    code
}

#[test]
fn tail_call_reuses_the_current_frame() {
    let mut chunk = Chunk::new();
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    chunk.protos.push(FunctionProto {
        name: "callee".into(),
        arity: 1,
        locals: 1,
        memory_plan: None,
        memory_witness_parameters: Vec::new(),
        call_witnesses: Vec::new(),
        parameter_structurals: Vec::new(),
        parameter_structural_places: Vec::new(),
        parameter_type_variables: Vec::new(),
        parameter_copy_kinds: vec![Some(lkjscript_core::StructuralKind::I64)],
        parameter_region_products: vec![None],
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
        code: {
            let mut code = index_instruction(Op::LoadLocal, 0);
            code.push(Op::Return as u8);
            code
        },
    });
    chunk.protos.push(FunctionProto {
        name: "caller".into(),
        arity: 0,
        locals: 0,
        memory_plan: None,
        memory_witness_parameters: Vec::new(),
        call_witnesses: Vec::new(),
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
        code: vec![Op::Unit as u8, Op::Return as u8],
    });
    let chunk =
        validate_chunk(chunk, ValidationPolicy::Unrestricted).expect("call test chunk validates");
    let mut vm = Vm::new(
        &chunk,
        crate::ExecutionInputs::default(),
        ExecutionPolicy::unrestricted(),
    );
    vm.frames.push(Frame {
        proto: Some(1),
        ip: 1,
        instruction_offset: 0,
        failure_cleanup_cursor: 0,
        stack_base: 0,
        locals_base: 0,
        unique_places: Vec::new(),
        borrowed_resources: Vec::new(),
        memory_witnesses: Vec::new(),
    });
    let argument = Value::from_i64(42);
    vm.push(argument);
    vm.push(vm.chunk.function_value(0).expect("function value"));

    call(&mut vm, 1, 0).expect("tail call");

    assert_eq!(vm.frames.len(), 1);
    assert_eq!(vm.frames[0].proto, Some(0));
    assert_eq!(vm.stack, vec![argument]);
}

#[test]
fn failed_call_entry_cleans_consuming_arguments_before_frame_unwind() {
    let mut chunk = Chunk::new();
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    let mut callee = Chunk::new().main;
    callee.name = "owned-argument".into();
    callee.arity = 1;
    callee.locals = 2;
    callee.parameter_structurals = vec![None];
    callee.parameter_structural_places = vec![None];
    callee.parameter_type_variables = vec![None];
    callee.parameter_copy_kinds = vec![None];
    callee.parameter_region_products = vec![None];
    callee.parameter_resources = vec![None];
    callee.parameter_resource_places = vec![None];
    callee.parameter_uniques = vec![Some(lkjscript_core::UniqueValueKind::ByteVector)];
    callee.parameter_unique_places = vec![Some(0)];
    callee.unique_places = 1;
    callee.emit_op_u64_pair(Op::ByteVectorDropPlace, 0, 0);
    callee.emit(Op::Pop);
    callee.emit_op_u64(Op::ByteVectorPlaceEnd, 0);
    callee.emit(Op::Pop);
    callee.emit(Op::Unit);
    callee.emit(Op::Return);
    chunk.protos.push(callee);
    let chunk = validate_chunk(chunk, ValidationPolicy::Unrestricted)
        .expect("owned-argument call validates");
    let policy = ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy {
        max_stack_values: 1,
        ..lkjscript_core::LimitedExecutionPolicy::conservative()
    });
    let mut vm = Vm::new(&chunk, crate::ExecutionInputs::default(), policy);
    vm.frames.push(Frame {
        proto: None,
        ip: 0,
        instruction_offset: 0,
        failure_cleanup_cursor: 0,
        stack_base: 0,
        locals_base: 0,
        unique_places: Vec::new(),
        borrowed_resources: Vec::new(),
        memory_witnesses: Vec::new(),
    });
    let owner = vm.unique.allocate(1).expect("allocate call argument");
    vm.push(owner);
    vm.push(vm.chunk.function_value(0).expect("function value"));

    let error = call(&mut vm, 1, 0).expect_err("callee frame exceeds stack policy");
    assert_eq!(
        error.class(),
        lkjscript_core::ErrorClass::Resource(lkjscript_core::ResourceLimitKind::StackValues)
    );
    vm.unique
        .verify_empty()
        .expect("failed call entry released its moved argument");
}

#[test]
fn borrowed_resource_parameters_remain_nonconsuming_in_callee_locals() {
    let mut chunk = Chunk::new();
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    let mut callee = Chunk::new().main;
    callee.name = "borrowed-resource".into();
    callee.arity = 1;
    callee.locals = 2;
    callee.parameter_resources = vec![Some(lkjscript_core::ResourceKind::TcpListener)];
    callee.parameter_resource_places = vec![None];
    callee.code = index_instruction(Op::LoadLocal, 0);
    callee
        .code
        .extend_from_slice(&index_instruction(Op::StoreLocal, 1));
    callee
        .code
        .extend_from_slice(&[Op::Pop as u8, Op::Unit as u8, Op::Return as u8]);
    chunk.protos.push(callee);
    let chunk = validate_chunk(chunk, ValidationPolicy::Unrestricted)
        .expect("borrowed-resource call validates");
    let mut vm = Vm::new(
        &chunk,
        crate::ExecutionInputs::default(),
        ExecutionPolicy::unrestricted(),
    );
    vm.frames.push(Frame {
        proto: None,
        ip: 0,
        instruction_offset: 0,
        failure_cleanup_cursor: 0,
        stack_base: 0,
        locals_base: 0,
        unique_places: Vec::new(),
        borrowed_resources: Vec::new(),
        memory_witnesses: Vec::new(),
    });
    let resource = Value::from_resource(17);
    vm.push(resource);
    vm.push(vm.chunk.function_value(0).expect("function value"));
    call(&mut vm, 1, 0).expect("borrowed-resource call");

    assert_eq!(vm.frames[1].borrowed_resources, vec![resource]);
    vm.push(resource);
    vm.frames[1].ip = 10;
    super::super::data::dispatch(&mut vm, Op::StoreLocal as u8)
        .expect("borrowed StoreLocal copies the view");
    assert_eq!(vm.peek().expect("copied view remains"), resource);
    assert_eq!(vm.stack[1], resource);
}
