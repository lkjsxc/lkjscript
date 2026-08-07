//! Dispatch for string and file opcodes.

use lkjscript_core::{Error, ErrorClass, Op, Result, Value};

use crate::run::Vm;

fn push_language_result(
    vm: &mut Vm<'_>,
    kind: lkjscript_core::SystemErrorKind,
    success: crate::run::structural_ops::HostValueType,
    result: Result<crate::run::structural_ops::HostValue>,
) {
    if result.is_err() && vm.resources.limit_exceeded() {
        return;
    }
    let result = crate::run::structural_ops::publish_system_result(vm, success, kind, result);
    match result {
        Ok(value) => vm.push(value),
        Err(error) => vm.allocation_error = Some(error),
    }
}

fn push_runtime_result(
    vm: &mut Vm<'_>,
    kind: lkjscript_core::SystemErrorKind,
    success: crate::run::structural_ops::HostValueType,
    result: Result<Value>,
) {
    let result = result
        .and_then(|value| crate::run::structural_ops::value_from_runtime(vm, &success, value));
    push_language_result(vm, kind, success, result);
}

fn execution_policy<T>(result: Result<T>) -> Result<Result<T>> {
    match result {
        Err(error) if !matches!(error.class(), ErrorClass::Ordinary) => Err(error),
        result => Ok(result),
    }
}

fn push_i64_result(vm: &mut Vm<'_>, kind: lkjscript_core::SystemErrorKind, result: Result<i64>) {
    push_language_result(
        vm,
        kind,
        crate::run::structural_ops::HostValueType::I64,
        result.map(crate::run::structural_ops::HostValue::I64),
    );
}

fn sleep_result(clock: &dyn lkjscript_host::Clock, milliseconds: u64) -> Result<Value> {
    clock
        .sleep(std::time::Duration::from_millis(milliseconds))
        .map(|()| Value::UNIT)
        .map_err(|error| lkjscript_core::Error::msg(format!("sys-wait-ms: {error}")))
}

fn wait_readable(
    vm: &Vm<'_>,
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

pub(super) fn clear_resource_aliases(vm: &mut Vm<'_>, handle: Value) {
    for value in &mut vm.stack {
        if *value == handle {
            *value = Value::INVALID;
        }
    }
}

mod byte_data;
mod files;
mod paths;
mod process;
mod sockets;
mod sqlite_read;
mod sqlite_write;
mod strings;
mod terminal;

pub fn dispatch_ext(vm: &mut Vm<'_>, op: u8) -> Result<bool> {
    if byte_data::dispatch(vm, op)? {
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
