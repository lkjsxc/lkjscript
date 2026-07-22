//! Numeric helpers for the VM.

use lkjscript_core::{Error, HeapObj, JitHook, Result, Value};

use crate::run::Vm;

pub fn as_f64<J: JitHook>(vm: &Vm<'_, J>, v: Value) -> Result<f64> {
    if let Some(n) = v.as_int() {
        return Ok(n as f64);
    }
    match vm.arena.get(v)? {
        HeapObj::Float(f) => Ok(*f),
        _ => Err(Error::msg("expected number")),
    }
}

pub fn push_num<J: JitHook>(vm: &mut Vm<'_, J>, n: f64) {
    if n.fract() == 0.0 && n >= (i32::MIN as f64) && n <= (i32::MAX as f64) {
        vm.push(Value::from_int(n as i64));
    } else {
        let v = vm.arena.alloc(HeapObj::Float(n));
        vm.push(v);
    }
}

pub fn bin_num<J: JitHook>(vm: &mut Vm<'_, J>, f: impl Fn(f64, f64) -> f64) -> Result<()> {
    let b = vm.pop();
    let a = vm.pop();
    let r = f(as_f64(vm, a)?, as_f64(vm, b)?);
    push_num(vm, r);
    Ok(())
}

pub fn bin_cmp<J: JitHook>(vm: &mut Vm<'_, J>, f: impl Fn(f64, f64) -> bool) -> Result<()> {
    let b = vm.pop();
    let a = vm.pop();
    let r = f(as_f64(vm, a)?, as_f64(vm, b)?);
    vm.push(Value::from_bool(r));
    Ok(())
}
