//! Checked byte-view host boundaries for bulk I/O, entropy, hashing, and terminal state.

use crate::host_ext::ResourceTable;
use crate::run::unique::UniqueRuntime;
use lkjscript_core::{Error, Result, Value, MAX_BULK_IO_BYTES};

fn bounded(bytes: &[u8], operation: &str) -> Result<()> {
    if bytes.len() > MAX_BULK_IO_BYTES {
        Err(Error::msg(format!(
            "{operation} byte-slice exceeds bulk I/O limit"
        )))
    } else {
        Ok(())
    }
}

pub(crate) fn read_into(
    unique: &mut UniqueRuntime,
    resources: &ResourceTable,
    resource: Value,
    view: Value,
) -> Result<i64> {
    let destination = unique.exclusive_bytes(view)?;
    bounded(destination, "read-into")?;
    let count = resources.read_into(resource, destination)?;
    i64::try_from(count).map_err(|_| Error::msg("read-into count out of range"))
}

pub(crate) fn write_from(
    unique: &mut UniqueRuntime,
    resources: &ResourceTable,
    resource: Value,
    view: Value,
) -> Result<i64> {
    let source = unique.shared_bytes(view)?;
    bounded(source, "write-from")?;
    let count = resources.write_from(resource, source)?;
    i64::try_from(count).map_err(|_| Error::msg("write-from count out of range"))
}

pub(crate) fn fill_random(unique: &mut UniqueRuntime, view: Value) -> Result<Value> {
    let destination = unique.exclusive_bytes(view)?;
    bounded(destination, "fill-random")?;
    lkjscript_sys::random_fill(destination)
        .map_err(|error| Error::msg(format!("fill-random: {error}")))?;
    Ok(Value::UNIT)
}

pub(crate) fn sha256(unique: &mut UniqueRuntime, view: Value) -> Result<Value> {
    let source = unique.shared_bytes(view)?;
    bounded(source, "sha256")?;
    let digest = lkjscript_core::sha256(source);
    unique.allocate_bytes(digest.to_vec())
}

pub(crate) fn tty_get(
    unique: &mut UniqueRuntime,
    resources: &ResourceTable,
    resource: Value,
    view: Value,
) -> Result<Value> {
    let raw = resources.raw_fd(resource, "get-terminal-state")?;
    let state = unique.exclusive_bytes(view)?;
    lkjscript_sys::tty_get(raw, state)
        .map_err(|error| Error::msg(format!("get-terminal-state: {error}")))?;
    Ok(Value::UNIT)
}

pub(crate) fn tty_set(
    unique: &mut UniqueRuntime,
    resources: &ResourceTable,
    resource: Value,
    view: Value,
) -> Result<Value> {
    let raw = resources.raw_fd(resource, "set-terminal-state")?;
    let state = unique.shared_bytes(view)?;
    lkjscript_sys::tty_set(raw, state)
        .map_err(|error| Error::msg(format!("set-terminal-state: {error}")))?;
    Ok(Value::UNIT)
}

pub(crate) fn tty_guard_save(unique: &mut UniqueRuntime, view: Value) -> Result<Value> {
    let state = unique.shared_bytes(view)?;
    lkjscript_sys::tty_guard_save(state)
        .map_err(|error| Error::msg(format!("save-terminal-guard: {error}")))?;
    Ok(Value::UNIT)
}

pub(crate) fn tty_guard_clear() -> Result<Value> {
    lkjscript_sys::tty_guard_clear()
        .map_err(|error| Error::msg(format!("clear-terminal-guard: {error}")))?;
    Ok(Value::UNIT)
}

pub(crate) fn poll(resources: &ResourceTable, resource: Value, timeout: i64) -> Result<i64> {
    let raw = resources.raw_fd(resource, "poll-streams")?;
    let timeout = i32::try_from(timeout).map_err(|_| Error::msg("poll timeout out of range"))?;
    if timeout < 0 {
        return Err(Error::msg("poll timeout out of range"));
    }
    lkjscript_sys::poll_fd(raw, timeout)
        .map(i64::from)
        .map_err(|error| Error::msg(format!("poll-streams: {error}")))
}

pub(crate) fn standard_input() -> Value {
    ResourceTable::stdin_handle()
}

pub(crate) fn is_terminal(resources: &ResourceTable, resource: Value) -> Result<Value> {
    let raw = resources.raw_fd(resource, "is-terminal")?;
    Ok(Value::from_bool(lkjscript_sys::is_tty(raw)))
}
