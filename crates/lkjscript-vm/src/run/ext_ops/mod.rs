//! Dispatch for string and file opcodes.

use lkjscript_core::{HeapObj, Op, Result, Value};

use crate::run::{RuntimeTier, Vm};

fn push_language_result<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    kind: lkjscript_core::SystemErrorKind,
    result: Result<Value>,
) {
    match crate::host_ext::language_result(&mut vm.arena, kind, result) {
        Ok(value) => vm.push(value),
        Err(error) => vm.allocation_error = Some(error),
    }
}

fn push_i64_result<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    kind: lkjscript_core::SystemErrorKind,
    result: Result<i64>,
) {
    let result = result.map(Value::from_i64);
    push_language_result(vm, kind, result);
}

fn sleep_result(milliseconds: u64) -> Result<Value> {
    lkjscript_sys::sleep_ms(milliseconds)
        .map(|()| Value::UNIT)
        .map_err(|error| lkjscript_core::Error::msg(format!("sys-wait-ms: {error}")))
}

fn wait_readable<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    handle: Value,
    operation: &str,
) -> Result<Option<lkjscript_core::Error>> {
    let Some(timeout) = vm.deadline_timeout_ms()? else {
        return Ok(None);
    };
    match vm.resources.poll_readable(handle, timeout, operation) {
        Ok(true) => Ok(None),
        Ok(false) => Err(lkjscript_core::Error::deadline(format!(
            "execution wall deadline exceeded during {operation}"
        ))),
        Err(error) => Ok(Some(error)),
    }
}

mod buffers;
mod files;
mod paths;
mod process;
mod sockets;
mod sqlite_read;
mod sqlite_write;
mod strings;
mod terminal;

pub fn dispatch_ext<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    if buffers::dispatch(vm, op)? {
        return Ok(true);
    }
    if files::dispatch(vm, op)? {
        return Ok(true);
    }
    if paths::dispatch(vm, op)? {
        return Ok(true);
    }
    if process::dispatch(vm, op)? {
        return Ok(true);
    }
    if sockets::dispatch(vm, op)? {
        return Ok(true);
    }
    if sqlite_read::dispatch(vm, op)? {
        return Ok(true);
    }
    if sqlite_write::dispatch(vm, op)? {
        return Ok(true);
    }
    if strings::dispatch(vm, op)? {
        return Ok(true);
    }
    if terminal::dispatch(vm, op)? {
        return Ok(true);
    }
    Ok(false)
}
