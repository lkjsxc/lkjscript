//! Call and list opcodes.

use lkjscript_core::{Error, Op, Result, Value};
#[cfg(feature = "jit")]
use lkjscript_jit::{EntryDecision, NativeValue, ScalarInvocationOutcome, ValueType};

use crate::run::{Frame, RuntimeTier, Vm};

pub fn make_closure<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let captures = vm.read_index()?;
    if captures != 0 {
        return Err(Error::msg("captured closures are unsupported"));
    }
    let value = vm.pop()?;
    let prototype = value
        .as_function_prototype()
        .ok_or_else(|| Error::msg("MakeClosure expects proto index"))?;
    vm.push(vm.chunk.function_value(prototype)?);
    Ok(())
}

pub fn car<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    representation: Option<lkjscript_core::StructuralRepresentationId>,
) -> Result<()> {
    let list = vm.pop()?;
    let value = vm.list_first(list, representation)?;
    vm.push(value);
    Ok(())
}

pub fn cdr<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let list = vm.pop()?;
    let value = vm.list_rest(list)?;
    vm.push(value);
    Ok(())
}

mod execution;
pub use execution::call;
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests;
