//! Byte-buffer and bounded terminal/poll host helpers.

use lkjscript_core::{Error, HeapObj, Result, Value};

use crate::arena::Arena;
use crate::host_ext::ResourceTable;

fn as_buf_mut(arena: &mut Arena, value: Value) -> Result<&mut Vec<u8>> {
    match arena.get_mut(value)? {
        HeapObj::Buf(buffer) => Ok(buffer),
        _ => Err(Error::msg("expected buf")),
    }
}

fn as_buf(arena: &Arena, value: Value) -> Result<&[u8]> {
    match arena.get(value)? {
        HeapObj::Buf(buffer) => Ok(buffer.as_slice()),
        _ => Err(Error::msg("expected buf")),
    }
}

pub fn buf_new(arena: &mut Arena, size: i64) -> Result<Value> {
    if !(0..=1_000_000).contains(&size) {
        return Err(Error::msg("buf-new size out of range"));
    }
    let size = usize::try_from(size).map_err(|_| Error::msg("buf-new size out of range"))?;
    Ok(arena.alloc(HeapObj::Buf(vec![0_u8; size])))
}

pub fn buf_len(arena: &Arena, value: Value) -> Result<i64> {
    i64::try_from(as_buf(arena, value)?.len()).map_err(|_| Error::msg("buf-len out of range"))
}

pub fn buf_ref(arena: &Arena, value: Value, index: i64) -> Result<i64> {
    let index = buffer_index(index, "buf-ref")?;
    let byte = *as_buf(arena, value)?
        .get(index)
        .ok_or_else(|| Error::msg("buf-ref out of bounds"))?;
    Ok(i64::from(byte))
}

pub fn buf_set(arena: &mut Arena, value: Value, index: i64, byte: i64) -> Result<Value> {
    let index = buffer_index(index, "buf-set")?;
    let byte = u8::try_from(byte).map_err(|_| Error::msg("buf-set byte out of range"))?;
    let buffer = as_buf_mut(arena, value)?;
    let slot = buffer
        .get_mut(index)
        .ok_or_else(|| Error::msg("buf-set out of bounds"))?;
    *slot = byte;
    Ok(Value::UNIT)
}

pub fn buf_get_u32(arena: &Arena, value: Value, index: i64) -> Result<i64> {
    let index = buffer_index(index, "buf-get-u32")?;
    let end = index
        .checked_add(4)
        .ok_or_else(|| Error::msg("buf-get-u32 index overflow"))?;
    let bytes = as_buf(arena, value)?
        .get(index..end)
        .ok_or_else(|| Error::msg("buf-get-u32 out of bounds"))?;
    let mut word = [0_u8; 4];
    word.copy_from_slice(bytes);
    Ok(i64::from(u32::from_le_bytes(word)))
}

pub fn buf_set_u32(arena: &mut Arena, value: Value, index: i64, number: i64) -> Result<Value> {
    let index = buffer_index(index, "buf-set-u32")?;
    let end = index
        .checked_add(4)
        .ok_or_else(|| Error::msg("buf-set-u32 index overflow"))?;
    let number = u32::try_from(number).map_err(|_| Error::msg("buf-set-u32 value out of range"))?;
    let destination = as_buf_mut(arena, value)?
        .get_mut(index..end)
        .ok_or_else(|| Error::msg("buf-set-u32 out of bounds"))?;
    destination.copy_from_slice(&number.to_le_bytes());
    Ok(Value::UNIT)
}

pub fn buf_clone(arena: &mut Arena, value: Value) -> Result<Value> {
    let bytes = as_buf(arena, value)?.to_vec();
    Ok(arena.alloc(HeapObj::Buf(bytes)))
}

pub fn sys_tty_get(
    arena: &mut Arena,
    handles: &ResourceTable,
    handle: Value,
    buffer: Value,
) -> Result<Value> {
    let raw = handles.raw_fd(handle, "sys-tty-get")?;
    let state = as_buf_mut(arena, buffer)?;
    lkjscript_sys::tty_get(raw, state)
        .map_err(|error| Error::msg(format!("sys-tty-get: {error}")))?;
    Ok(Value::UNIT)
}

pub fn sys_tty_set(
    arena: &Arena,
    handles: &ResourceTable,
    handle: Value,
    buffer: Value,
) -> Result<Value> {
    let raw = handles.raw_fd(handle, "sys-tty-set")?;
    let state = as_buf(arena, buffer)?;
    lkjscript_sys::tty_set(raw, state)
        .map_err(|error| Error::msg(format!("sys-tty-set: {error}")))?;
    Ok(Value::UNIT)
}

pub fn sys_poll(handles: &ResourceTable, handle: Value, timeout: i64) -> Result<i64> {
    let raw = handles.raw_fd(handle, "sys-poll")?;
    let timeout =
        i32::try_from(timeout).map_err(|_| Error::msg("sys-poll timeout out of range"))?;
    if timeout < 0 {
        return Err(Error::msg("sys-poll timeout out of range"));
    }
    let ready = lkjscript_sys::poll_fd(raw, timeout)
        .map_err(|error| Error::msg(format!("sys-poll: {error}")))?;
    Ok(i64::from(ready))
}

pub fn stdin_handle() -> Value {
    ResourceTable::stdin_handle()
}

pub fn sys_isatty(handles: &ResourceTable, handle: Value) -> Result<Value> {
    let raw = handles.raw_fd(handle, "sys-isatty")?;
    Ok(Value::from_bool(lkjscript_sys::is_tty(raw)))
}

pub fn sys_tty_guard_save(arena: &Arena, buffer: Value) -> Result<Value> {
    let state = as_buf(arena, buffer)?;
    lkjscript_sys::tty_guard_save(state)
        .map_err(|error| Error::msg(format!("sys-tty-guard-save: {error}")))?;
    Ok(Value::UNIT)
}

pub fn sys_tty_guard_clear() -> Result<Value> {
    lkjscript_sys::tty_guard_clear()
        .map_err(|error| Error::msg(format!("sys-tty-guard-clear: {error}")))?;
    Ok(Value::UNIT)
}

fn buffer_index(index: i64, operation: &str) -> Result<usize> {
    usize::try_from(index).map_err(|_| Error::msg(format!("{operation} index out of range")))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use lkjscript_core::Value;

    use crate::arena::Arena;
    use crate::host_ext::ResourceTable;

    use super::{buf_new, buf_set, buf_set_u32, sys_poll};

    #[test]
    fn polling_rejects_invalid_handles_and_timeouts() {
        let handles = ResourceTable::default();
        let integer = Value::from_small_i64(1).expect("small integer");
        assert!(sys_poll(&handles, integer, 0).is_err());
        assert!(sys_poll(&handles, ResourceTable::stdin_handle(), -1).is_err());
    }

    #[test]
    fn buffer_narrowing_rejects_truncation_and_wrapping() {
        let mut arena = Arena::default();
        let buffer = buf_new(&mut arena, 4).expect("buffer");
        assert!(buf_set(&mut arena, buffer, 0, -1).is_err());
        assert!(buf_set(&mut arena, buffer, 0, 256).is_err());
        assert!(buf_set_u32(&mut arena, buffer, 0, -1).is_err());
        assert!(buf_set_u32(&mut arena, buffer, 0, i64::from(u32::MAX) + 1).is_err());
        assert!(buf_set(&mut arena, buffer, 0, 255).is_ok());
        assert!(buf_set_u32(&mut arena, buffer, 0, i64::from(u32::MAX)).is_ok());
    }
}
