//! Byte-buffer and thin sys opcode helpers.

use lkjscript2026_core::{Error, HeapObj, Result, Value};

use crate::arena::Arena;

fn as_buf_mut<'a>(arena: &'a mut Arena, v: Value) -> Result<&'a mut Vec<u8>> {
    match arena.get_mut(v)? {
        HeapObj::Buf(b) => Ok(b),
        _ => Err(Error::msg("expected buf")),
    }
}

fn as_buf<'a>(arena: &'a Arena, v: Value) -> Result<&'a [u8]> {
    match arena.get(v)? {
        HeapObj::Buf(b) => Ok(b.as_slice()),
        _ => Err(Error::msg("expected buf")),
    }
}

pub fn buf_new(arena: &mut Arena, n: Value) -> Result<Value> {
    let len = n
        .as_int()
        .ok_or_else(|| Error::msg("buf-new expects int"))?;
    if len < 0 || len > 1_000_000 {
        return Err(Error::msg("buf-new size out of range"));
    }
    Ok(arena.alloc(HeapObj::Buf(vec![0u8; len as usize])))
}

pub fn buf_len(arena: &Arena, v: Value) -> Result<Value> {
    Ok(Value::from_int(as_buf(arena, v)?.len() as i64))
}

pub fn buf_ref(arena: &Arena, v: Value, i: Value) -> Result<Value> {
    let buf = as_buf(arena, v)?;
    let idx = i
        .as_int()
        .ok_or_else(|| Error::msg("buf-ref index"))? as usize;
    let b = *buf
        .get(idx)
        .ok_or_else(|| Error::msg("buf-ref OOB"))?;
    Ok(Value::from_int(b as i64))
}

pub fn buf_set(arena: &mut Arena, v: Value, i: Value, byte: Value) -> Result<Value> {
    let idx = i
        .as_int()
        .ok_or_else(|| Error::msg("buf-set index"))? as usize;
    let b = byte
        .as_int()
        .ok_or_else(|| Error::msg("buf-set byte"))?;
    let buf = as_buf_mut(arena, v)?;
    let slot = buf
        .get_mut(idx)
        .ok_or_else(|| Error::msg("buf-set OOB"))?;
    *slot = (b & 0xff) as u8;
    Ok(Value::NIL)
}

pub fn buf_get_u32(arena: &Arena, v: Value, i: Value) -> Result<Value> {
    let buf = as_buf(arena, v)?;
    let idx = i
        .as_int()
        .ok_or_else(|| Error::msg("buf-get-u32 index"))? as usize;
    if idx + 4 > buf.len() {
        return Err(Error::msg("buf-get-u32 OOB"));
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&buf[idx..idx + 4]);
    Ok(Value::from_int(u32::from_le_bytes(bytes) as i64))
}

pub fn buf_set_u32(arena: &mut Arena, v: Value, i: Value, n: Value) -> Result<Value> {
    let idx = i
        .as_int()
        .ok_or_else(|| Error::msg("buf-set-u32 index"))? as usize;
    let val = n
        .as_int()
        .ok_or_else(|| Error::msg("buf-set-u32 value"))? as u32;
    let buf = as_buf_mut(arena, v)?;
    if idx + 4 > buf.len() {
        return Err(Error::msg("buf-set-u32 OOB"));
    }
    buf[idx..idx + 4].copy_from_slice(&val.to_le_bytes());
    Ok(Value::NIL)
}

pub fn buf_clone(arena: &mut Arena, v: Value) -> Result<Value> {
    let bytes = as_buf(arena, v)?.to_vec();
    Ok(arena.alloc(HeapObj::Buf(bytes)))
}

pub fn sys_ioctl(arena: &mut Arena, fd: Value, req: Value, buf: Value) -> Result<Value> {
    let fd = fd
        .as_int()
        .ok_or_else(|| Error::msg("sys-ioctl fd"))? as i32;
    let req = req
        .as_int()
        .ok_or_else(|| Error::msg("sys-ioctl req"))? as u64;
    let bytes = as_buf_mut(arena, buf)?;
    lkjscript2026_sys::ioctl_buf(fd, req, bytes)
        .map_err(|e| Error::msg(format!("sys-ioctl: {e}")))?;
    Ok(Value::NIL)
}

pub fn sys_poll(fd: Value, timeout: Value) -> Result<Value> {
    let fd = fd.as_int().ok_or_else(|| Error::msg("sys-poll fd"))? as i32;
    let ms = timeout
        .as_int()
        .ok_or_else(|| Error::msg("sys-poll timeout"))? as i32;
    let ready = lkjscript2026_sys::poll_fd(fd, ms)
        .map_err(|e| Error::msg(format!("sys-poll: {e}")))?;
    Ok(Value::from_int(if ready { 1 } else { 0 }))
}

pub fn stdin_fd() -> Value {
    Value::from_int(lkjscript2026_sys::STDIN_FD as i64)
}

pub fn isatty(fd: Value) -> Result<Value> {
    let fd = fd.as_int().ok_or_else(|| Error::msg("isatty fd"))? as i32;
    Ok(Value::from_int(if lkjscript2026_sys::is_tty(fd) {
        1
    } else {
        0
    }))
}

pub fn tty_guard_save(arena: &Arena, buf: Value) -> Result<Value> {
    let bytes = as_buf(arena, buf)?;
    lkjscript2026_sys::tty_guard_save(bytes);
    Ok(Value::NIL)
}

pub fn tty_guard_clear() -> Value {
    lkjscript2026_sys::tty_guard_clear();
    Value::NIL
}
