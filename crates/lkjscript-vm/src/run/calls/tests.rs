use super::*;
use lkjscript_core::{validate_chunk, Chunk, ExecutionConfig, FunctionProto, ValidationLimits};

use crate::run::NoTier as NullJit;

#[test]
fn tail_call_reuses_the_current_frame() {
    let mut chunk = Chunk::new();
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    chunk.protos.push(FunctionProto {
        name: "callee".into(),
        arity: 1,
        locals: 1,
        parameter_resources: Vec::new(),
        return_resource: None,
        code: vec![Op::LoadLocal as u8, 0, Op::Return as u8],
    });
    chunk.protos.push(FunctionProto {
        name: "caller".into(),
        arity: 0,
        locals: 0,
        parameter_resources: Vec::new(),
        return_resource: None,
        code: vec![Op::Unit as u8, Op::Return as u8],
    });
    let chunk =
        validate_chunk(chunk, &ValidationLimits::default()).expect("call test chunk validates");
    let mut vm = Vm::new(
        &chunk,
        NullJit,
        crate::ExecutionInputs::default(),
        ExecutionConfig::default(),
    );
    vm.frames.push(Frame {
        proto: 1,
        ip: 1,
        stack_base: 0,
        locals_base: 0,
    });
    let argument = vm.make_i64(42).expect("test argument");
    vm.push(argument);
    let callee = vm
        .arena
        .alloc(HeapObj::Closure {
            proto: 0,
            captures: Vec::new(),
        })
        .expect("test closure allocation");
    vm.push(callee);

    call(&mut vm, 1).expect("tail call");

    assert_eq!(vm.frames.len(), 1);
    assert_eq!(vm.frames[0].proto, 0);
    assert_eq!(vm.stack, vec![argument]);
}
