//! Call and pair opcodes.

use lkjscript_core::{Error, HeapObj, Op, Result, Value};
use lkjscript_jit::{EntryDecision, NativeValue, ScalarInvocationOutcome, ValueType};

use crate::run::{Frame, RuntimeTier, Vm};

pub fn make_closure<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let captures = vm.read_u16()?;
    if captures != 0 {
        return Err(Error::msg("captured closures are unsupported"));
    }
    let value = vm.pop()?;
    let proto_id = vm
        .as_i64(value)
        .map_err(|_| Error::msg("MakeClosure expects proto index"))?;
    let proto_id =
        u32::try_from(proto_id).map_err(|_| Error::msg("MakeClosure proto index out of range"))?;
    vm.push(vm.chunk.function_value(proto_id)?);
    Ok(())
}

pub fn car<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
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

pub fn cdr<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
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

mod execution;
pub use execution::call;
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests;
