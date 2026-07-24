//! Call and pair opcodes.

use lkjscript_core::{Error, HeapObj, JitHook, Op, Result, Value};

use crate::run::{Frame, Vm};

pub fn make_closure<J: JitHook>(vm: &mut Vm<'_, J>) -> Result<()> {
    let _caps = vm.read_u16()?;
    let value = vm.pop()?;
    let proto_id = vm
        .as_i64(value)
        .map_err(|_| Error::msg("MakeClosure expects proto index"))?;
    let proto_id =
        u32::try_from(proto_id).map_err(|_| Error::msg("MakeClosure proto index out of range"))?;
    let v = vm.arena.alloc(HeapObj::Closure {
        proto: proto_id,
        captures: Vec::new(),
    });
    vm.push(v);
    Ok(())
}

pub fn car<J: JitHook>(vm: &mut Vm<'_, J>) -> Result<()> {
    let p = vm.pop()?;
    match vm.arena.get(p)? {
        HeapObj::Pair { car, .. } => {
            let c = *car;
            vm.push(c);
            Ok(())
        }
        _ => Err(Error::msg("car expects pair")),
    }
}

pub fn cdr<J: JitHook>(vm: &mut Vm<'_, J>) -> Result<()> {
    let p = vm.pop()?;
    match vm.arena.get(p)? {
        HeapObj::Pair { cdr, .. } => {
            let c = *cdr;
            vm.push(c);
            Ok(())
        }
        _ => Err(Error::msg("cdr expects pair")),
    }
}

pub fn call<J: JitHook>(vm: &mut Vm<'_, J>, argc: u8) -> Result<()> {
    let callee = vm.pop()?;
    let obj = vm.arena.get(callee)?.clone();
    match obj {
        HeapObj::Closure { proto, .. } => {
            vm.jit.observe_call(vm.chunk, proto);
            let p = vm
                .chunk
                .protos()
                .get(proto as usize)
                .ok_or_else(|| Error::msg("call proto index out of range"))?;
            if argc as usize != p.arity as usize {
                return Err(Error::msg(format!(
                    "arity mismatch for {}: got {argc}, want {}",
                    p.name, p.arity
                )));
            }
            let locals = p.locals;
            let argument_count = usize::from(argc);
            let args_start = vm
                .stack
                .len()
                .checked_sub(argument_count)
                .ok_or_else(|| Error::msg("call argument stack underflow"))?;
            if is_tail_position(vm) {
                let stack_base = vm.frames.last().map(|frame| frame.stack_base).unwrap_or(0);
                let args = vm.stack[args_start..].to_vec();
                vm.stack.truncate(stack_base);
                vm.stack.extend_from_slice(&args);
                while vm.stack.len() < stack_base + locals as usize {
                    vm.stack.push(Value::INVALID);
                }
                if let Some(frame) = vm.frames.last_mut() {
                    *frame = Frame {
                        proto,
                        ip: 0,
                        stack_base,
                        locals_base: stack_base,
                    };
                }
                return Ok(());
            }
            while vm.stack.len() < args_start + locals as usize {
                vm.stack.push(Value::INVALID);
            }
            vm.frames.push(Frame {
                proto,
                ip: 0,
                stack_base: args_start,
                locals_base: args_start,
            });
            Ok(())
        }
        _ => Err(Error::msg("call expects closure")),
    }
}

fn is_tail_position<J: JitHook>(vm: &Vm<'_, J>) -> bool {
    let Some(frame) = vm.frames.last() else {
        return false;
    };
    if frame.proto == u32::MAX {
        return false;
    }
    vm.chunk
        .protos()
        .get(frame.proto as usize)
        .and_then(|proto| proto.code.get(frame.ip))
        .copied()
        == Some(Op::Return as u8)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;
    use lkjscript_core::{
        validate_chunk, Chunk, ExecutionConfig, FunctionProto, NullJit, ValidationLimits,
    };

    #[test]
    fn tail_call_reuses_the_current_frame() {
        let mut chunk = Chunk::new();
        chunk.main.emit(Op::Unit);
        chunk.main.emit(Op::Return);
        chunk.protos.push(FunctionProto {
            name: "callee".into(),
            arity: 1,
            locals: 1,
            code: vec![Op::LoadLocal as u8, 0, Op::Return as u8],
        });
        chunk.protos.push(FunctionProto {
            name: "caller".into(),
            arity: 0,
            locals: 0,
            code: vec![Op::Unit as u8, Op::Return as u8],
        });
        let chunk =
            validate_chunk(chunk, &ValidationLimits::default()).expect("call test chunk validates");
        let mut vm = Vm::new(&chunk, NullJit, Vec::new(), ExecutionConfig::default());
        vm.frames.push(Frame {
            proto: 1,
            ip: 1,
            stack_base: 0,
            locals_base: 0,
        });
        let argument = vm.make_i64(42);
        vm.push(argument);
        let callee = vm.arena.alloc(HeapObj::Closure {
            proto: 0,
            captures: Vec::new(),
        });
        vm.push(callee);

        call(&mut vm, 1).expect("tail call");

        assert_eq!(vm.frames.len(), 1);
        assert_eq!(vm.frames[0].proto, 0);
        assert_eq!(vm.stack, vec![argument]);
    }
}
