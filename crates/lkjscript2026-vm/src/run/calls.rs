//! Call and pair opcodes.

use lkjscript2026_core::{Error, HeapObj, JitHook, Result, Value};

use crate::run::{Frame, Vm};

pub fn make_closure<J: JitHook>(vm: &mut Vm<'_, J>) -> Result<()> {
    let _caps = vm.read_u16()?;
    let proto_id = vm
        .pop()
        .as_int()
        .ok_or_else(|| Error::msg("MakeClosure expects proto index"))? as u32;
    let v = vm.arena.alloc(HeapObj::Closure {
        proto: proto_id,
        captures: Vec::new(),
    });
    vm.push(v);
    Ok(())
}

pub fn car<J: JitHook>(vm: &mut Vm<'_, J>) -> Result<()> {
    let p = vm.pop();
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
    let p = vm.pop();
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
    let callee = vm.pop();
    let obj = vm.arena.get(callee)?.clone();
    match obj {
        HeapObj::Closure { proto, .. } => {
            let _ = vm.jit.maybe_compile(vm.chunk, proto);
            let p = &vm.chunk.protos[proto as usize];
            if argc as usize != p.arity as usize {
                return Err(Error::msg(format!(
                    "arity mismatch for {}: got {argc}, want {}",
                    p.name, p.arity
                )));
            }
            let locals = p.locals;
            let args_start = vm.stack.len() - argc as usize;
            while vm.stack.len() < args_start + locals as usize {
                vm.stack.push(Value::NIL);
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
