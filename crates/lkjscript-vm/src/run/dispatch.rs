//! Opcode dispatch.

use lkjscript_core::{Error, HeapObj, JitHook, Op, Result, Value};

use super::calls::{call, car, cdr, make_closure};
use super::numeric::{as_f64, bin_cmp, bin_num};
use super::Vm;
use crate::host::{flush_out, print_value, read_byte, write_byte, write_str};

fn eq_values<J: JitHook>(vm: &mut Vm<'_, J>) -> Result<()> {
    let b = vm.pop();
    let a = vm.pop();
    if let (Some(x), Some(y)) = (a.as_int(), b.as_int()) {
        vm.push(Value::from_bool(x == y));
        return Ok(());
    }
    if let (Ok(sa), Ok(sb)) = (
        crate::host_ext::as_str(&vm.arena, a),
        crate::host_ext::as_str(&vm.arena, b),
    ) {
        vm.push(Value::from_bool(sa == sb));
        return Ok(());
    }
    if let (Ok(fa), Ok(fb)) = (as_f64(vm, a), as_f64(vm, b)) {
        vm.push(Value::from_bool((fa - fb).abs() < 1e-12));
        return Ok(());
    }
    vm.push(Value::from_bool(a.raw() == b.raw()));
    Ok(())
}

fn bin_bits<J: JitHook>(vm: &mut Vm<'_, J>, f: fn(i64, i64) -> i64) -> Result<()> {
    let b = vm
        .pop()
        .as_int()
        .ok_or_else(|| Error::msg("bit op expects int"))?;
    let a = vm
        .pop()
        .as_int()
        .ok_or_else(|| Error::msg("bit op expects int"))?;
    vm.push(Value::from_int(f(a, b)));
    Ok(())
}

pub fn dispatch<J: JitHook>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    match op {
        x if x == Op::Nop as u8 => Ok(()),
        x if x == Op::LoadConst as u8 => {
            let id = vm.read_u16()? as usize;
            let v = vm.load_const(id)?;
            vm.push(v);
            Ok(())
        }
        x if x == Op::LoadLocal as u8 => {
            let slot = vm.read_u8()? as usize;
            let base = vm.frames.last().map(|f| f.locals_base).unwrap_or(0);
            vm.push(vm.stack.get(base + slot).copied().unwrap_or(Value::NIL));
            Ok(())
        }
        x if x == Op::StoreLocal as u8 => {
            let slot = vm.read_u8()? as usize;
            let base = vm.frames.last().map(|f| f.locals_base).unwrap_or(0);
            let v = vm.peek();
            let idx = base + slot;
            while vm.stack.len() <= idx {
                vm.stack.push(Value::NIL);
            }
            vm.stack[idx] = v;
            Ok(())
        }
        x if x == Op::LoadGlobal as u8 => {
            let id = vm.read_u16()? as usize;
            vm.push(vm.globals.get(id).copied().unwrap_or(Value::NIL));
            Ok(())
        }
        x if x == Op::StoreGlobal as u8 => {
            let id = vm.read_u16()? as usize;
            let v = vm.peek();
            if id < vm.globals.len() {
                vm.globals[id] = v;
            }
            Ok(())
        }
        x if x == Op::Add as u8 => bin_num(vm, |a, b| a + b),
        x if x == Op::Sub as u8 => bin_num(vm, |a, b| a - b),
        x if x == Op::Mul as u8 => bin_num(vm, |a, b| a * b),
        x if x == Op::Div as u8 => bin_num(vm, |a, b| a / b),
        x if x == Op::Eq as u8 => eq_values(vm),
        x if x == Op::Ne as u8 => {
            eq_values(vm)?;
            let v = vm.pop();
            vm.push(Value::from_bool(!v.is_truthy()));
            Ok(())
        }
        x if x == Op::Lt as u8 => bin_cmp(vm, |a, b| a < b),
        x if x == Op::Le as u8 => bin_cmp(vm, |a, b| a <= b),
        x if x == Op::Gt as u8 => bin_cmp(vm, |a, b| a > b),
        x if x == Op::Ge as u8 => bin_cmp(vm, |a, b| a >= b),
        x if x == Op::Not as u8 => {
            let v = vm.pop();
            vm.push(Value::from_bool(!v.is_truthy()));
            Ok(())
        }
        x if x == Op::BitAnd as u8 => bin_bits(vm, |a, b| a & b),
        x if x == Op::BitOr as u8 => bin_bits(vm, |a, b| a | b),
        x if x == Op::BitXor as u8 => bin_bits(vm, |a, b| a ^ b),
        x if x == Op::Jump as u8 => {
            let at = vm.read_u16()? as usize;
            if let Some(fr) = vm.frames.last_mut() {
                fr.ip = at;
            }
            Ok(())
        }
        x if x == Op::JumpIfFalse as u8 => {
            let at = vm.read_u16()? as usize;
            if !vm.pop().is_truthy() {
                if let Some(fr) = vm.frames.last_mut() {
                    fr.ip = at;
                }
            }
            Ok(())
        }
        x if x == Op::MakeClosure as u8 => make_closure(vm),
        x if x == Op::Call as u8 => {
            let argc = vm.read_u8()?;
            call(vm, argc)
        }
        x if x == Op::Return as u8 => {
            let ret = vm.pop();
            let frame = vm.frames.pop().ok_or_else(|| Error::msg("return"))?;
            vm.stack.truncate(frame.stack_base);
            vm.push(ret);
            Ok(())
        }
        x if x == Op::Cons as u8 => {
            let cdr_v = vm.pop();
            let car_v = vm.pop();
            let v = vm.arena.alloc(HeapObj::Pair {
                car: car_v,
                cdr: cdr_v,
            });
            vm.push(v);
            Ok(())
        }
        x if x == Op::Car as u8 => car(vm),
        x if x == Op::Cdr as u8 => cdr(vm),
        x if x == Op::IsNil as u8 => {
            let v = vm.pop();
            vm.push(Value::from_bool(v.is_nil()));
            Ok(())
        }
        x if x == Op::IsNull as u8 => {
            let v = vm.pop();
            vm.push(Value::from_bool(v.is_nil()));
            Ok(())
        }
        x if x == Op::Print as u8 => {
            let v = vm.pop();
            print_value(&vm.arena, v)?;
            vm.push(Value::NIL);
            Ok(())
        }
        x if x == Op::Flush as u8 => {
            flush_out()?;
            vm.push(Value::NIL);
            Ok(())
        }
        x if x == Op::ReadByte as u8 => {
            vm.push(read_byte()?);
            Ok(())
        }
        x if x == Op::WriteByte as u8 => {
            let v = vm.pop();
            vm.push(write_byte(v)?);
            Ok(())
        }
        x if x == Op::WriteStr as u8 => {
            let v = vm.pop();
            vm.push(write_str(&vm.arena, v)?);
            Ok(())
        }
        x if x == Op::Exit as u8 => {
            vm.exit_code = Some(vm.pop().as_int().unwrap_or(0) as i32);
            Ok(())
        }
        x if crate::run::ext_ops::dispatch_ext(vm, x)? => Ok(()),
        x if x == Op::Pop as u8 => {
            let _ = vm.pop();
            Ok(())
        }
        x if x == Op::Dup as u8 => {
            let v = vm.peek();
            vm.push(v);
            Ok(())
        }
        x if x == Op::False as u8 => {
            vm.push(Value::FALSE);
            Ok(())
        }
        x if x == Op::True as u8 => {
            vm.push(Value::TRUE);
            Ok(())
        }
        x if x == Op::Nil as u8 => {
            vm.push(Value::NIL);
            Ok(())
        }
        other => Err(Error::msg(format!("unknown opcode {other}"))),
    }
}
