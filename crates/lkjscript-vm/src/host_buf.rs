//! Byte-buffer and bounded terminal/poll host helpers.

use lkjscript_core::{Error, HeapObj, Result, Value};

use crate::arena::Arena;
use crate::host_ext::FdTable;

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

pub fn buf_new(arena: &mut Arena, size: Value) -> Result<Value> {
    let size = size
        .as_int()
        .ok_or_else(|| Error::msg("buf-new expects int"))?;
    if !(0..=1_000_000).contains(&size) {
        return Err(Error::msg("buf-new size out of range"));
    }
    let size = usize::try_from(size).map_err(|_| Error::msg("buf-new size out of range"))?;
    Ok(arena.alloc(HeapObj::Buf(vec![0_u8; size])))
}

pub fn buf_len(arena: &Arena, value: Value) -> Result<Value> {
    let length = i64::try_from(as_buf(arena, value)?.len())
        .map_err(|_| Error::msg("buf-len out of range"))?;
    Ok(Value::from_int(length))
}

pub fn buf_ref(arena: &Arena, value: Value, index: Value) -> Result<Value> {
    let index = buffer_index(index, "buf-ref")?;
    let byte = *as_buf(arena, value)?
        .get(index)
        .ok_or_else(|| Error::msg("buf-ref out of bounds"))?;
    Ok(Value::from_int(i64::from(byte)))
}

pub fn buf_set(arena: &mut Arena, value: Value, index: Value, byte: Value) -> Result<Value> {
    let index = buffer_index(index, "buf-set")?;
    let byte = byte
        .as_int()
        .ok_or_else(|| Error::msg("buf-set byte"))?;
    let buffer = as_buf_mut(arena, value)?;
    let slot = buffer
        .get_mut(index)
        .ok_or_else(|| Error::msg("buf-set out of bounds"))?;
    *slot = (byte & 0xff) as u8;
    Ok(Value::NIL)
}

pub fn buf_get_u32(arena: &Arena, value: Value, index: Value) -> Result<Value> {
    let index = buffer_index(index, "buf-get-u32")?;
    let end = index
        .checked_add(4)
        .ok_or_else(|| Error::msg("buf-get-u32 index overflow"))?;
    let bytes = as_buf(arena, value)?
        .get(index..end)
        .ok_or_else(|| Error::msg("buf-get-u32 out of bounds"))?;
    let mut word = [0_u8; 4];
    word.copy_from_slice(bytes);
    Ok(Value::from_int(i64::from(u32::from_le_bytes(word))))
}

pub fn buf_set_u32(arena: &mut Arena, value: Value, index: Value, number: Value) -> Result<Value> {
    let index = buffer_index(index, "buf-set-u32")?;
    let end = index
        .checked_add(4)
        .ok_or_else(|| Error::msg("buf-set-u32 index overflow"))?;
    let number = number
        .as_int()
        .ok_or_else(|| Error::msg("buf-set-u32 value"))? as u32;
    let destination = as_buf_mut(arena, value)?
        .get_mut(index..end)
        .ok_or_else(|| Error::msg("buf-set-u32 out of bounds"))?;
    destination.copy_from_slice(&number.to_le_bytes());
    Ok(Value::NIL)
}

pub fn buf_clone(arena: &mut Arena, value: Value) -> Result<Value> {
    let bytes = as_buf(arena, value)?.to_vec();
    Ok(arena.alloc(HeapObj::Buf(bytes)))
}

pub fn sys_tty_get(
    arena: &mut Arena,
    handles: &FdTable,
    handle: Value,
    buffer: Value,
) -> Result<Value> {
    let raw = handles.raw_fd(handle, "sys-tty-get")?;
    let state = as_buf_mut(arena, buffer)?;
    lkjscript_sys::tty_get(raw, state)
        .map_err(|error| Error::msg(format!("sys-tty-get: {error}")))?;
    Ok(Value::NIL)
}

pub fn sys_tty_set(
    arena: &Arena,
    handles: &FdTable,
    handle: Value,
    buffer: Value,
) -> Result<Value> {
    let raw = handles.raw_fd(handle, "sys-tty-set")?;
    let state = as_buf(arena, buffer)?;
    lkjscript_sys::tty_set(raw, state)
        .map_err(|error| Error::msg(format!("sys-tty-set: {error}")))?;
    Ok(Value::NIL)
}

pub fn sys_poll(handles: &FdTable, handle: Value, timeout: Value) -> Result<Value> {
    let raw = handles.raw_fd(handle, "sys-poll")?;
    let timeout = timeout
        .as_int()
        .ok_or_else(|| Error::msg("sys-poll timeout"))?;
    let timeout = i32::try_from(timeout)
        .map_err(|_| Error::msg("sys-poll timeout out of range"))?;
    if timeout < 0 {
        return Err(Error::msg("sys-poll timeout out of range"));
    }
    let ready = lkjscript_sys::poll_fd(raw, timeout)
        .map_err(|error| Error::msg(format!("sys-poll: {error}")))?;
    Ok(Value::from_int(i64::from(ready)))
}

pub fn stdin_handle() -> Value {
    FdTable::stdin_handle()
}

pub fn isatty(handles: &FdTable, handle: Value) -> Result<Value> {
    let raw = handles.raw_fd(handle, "isatty")?;
    Ok(Value::from_bool(lkjscript_sys::is_tty(raw)))
}

pub fn tty_guard_save(arena: &Arena, buffer: Value) -> Result<Value> {
    let state = as_buf(arena, buffer)?;
    lkjscript_sys::tty_guard_save(state)
        .map_err(|error| Error::msg(format!("tty-guard-save: {error}")))?;
    Ok(Value::NIL)
}

pub fn tty_guard_clear() -> Result<Value> {
    lkjscript_sys::tty_guard_clear()
        .map_err(|error| Error::msg(format!("tty-guard-clear: {error}")))?;
    Ok(Value::NIL)
}

fn buffer_index(value: Value, operation: &str) -> Result<usize> {
    let index = value
        .as_int()
        .ok_or_else(|| Error::msg(format!("{operation} index")))?;
    usize::try_from(index).map_err(|_| Error::msg(format!("{operation} index out of range")))
}
