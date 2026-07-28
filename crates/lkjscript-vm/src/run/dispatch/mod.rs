//! Opcode dispatch.

use lkjscript_core::{Error, Result};

use super::{RuntimeTier, Vm};

pub fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    if super::unique_ops::handles(op) {
        return super::unique_ops::dispatch(vm, op);
    }
    if super::numeric::conversion::handles(op) {
        return super::numeric::conversion::dispatch(vm, op);
    }
    if super::numeric::handles(op) {
        return super::numeric::dispatch(vm, op);
    }
    if super::control::handles(op) {
        return super::control::dispatch(vm, op);
    }
    if super::enum_value::handles(op) {
        return super::enum_value::dispatch(vm, op);
    }
    if super::product::handles(op) {
        return super::product::dispatch(vm, op);
    }
    if super::host_ops::handles(op) {
        return super::host_ops::dispatch(vm, op);
    }
    if super::data::handles(op) {
        return super::data::dispatch(vm, op);
    }
    if crate::run::ext_ops::dispatch_ext(vm, op)? {
        return Ok(());
    }
    Err(Error::msg(format!("unknown opcode {op}")))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
