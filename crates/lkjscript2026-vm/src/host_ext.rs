//! String, filesystem, and thin socket host ops.

use std::os::fd::RawFd;

use lkjscript2026_core::{Error, HeapObj, Result, Value};
use lkjscript2026_sys::OwnedFd;

use crate::arena::Arena;

pub fn as_str<'a>(arena: &'a Arena, v: Value) -> Result<&'a str> {
    match arena.get(v)? {
        HeapObj::Str(s) => Ok(s.as_str()),
        HeapObj::Symbol(s) => Ok(s.as_str()),
        _ => Err(Error::msg("expected string")),
    }
}

pub fn str_len(arena: &Arena, v: Value) -> Result<Value> {
    Ok(Value::from_int(as_str(arena, v)?.len() as i64))
}

pub fn str_ref(arena: &Arena, s: Value, i: Value) -> Result<Value> {
    let idx = i.as_int().ok_or_else(|| Error::msg("str-ref index"))? as usize;
    let bytes = as_str(arena, s)?.as_bytes();
    let b = *bytes.get(idx).ok_or_else(|| Error::msg("str-ref OOB"))?;
    Ok(Value::from_int(b as i64))
}

pub fn str_append(arena: &mut Arena, a: Value, b: Value) -> Result<Value> {
    let mut out = as_str(arena, a)?.to_string();
    out.push_str(as_str(arena, b)?);
    Ok(arena.alloc(HeapObj::Str(out)))
}

pub fn str_slice(arena: &mut Arena, s: Value, start: Value, end: Value) -> Result<Value> {
    let st = start.as_int().ok_or_else(|| Error::msg("slice start"))? as usize;
    let en = end.as_int().ok_or_else(|| Error::msg("slice end"))? as usize;
    let bytes = as_str(arena, s)?.as_bytes();
    if st > en || en > bytes.len() {
        return Err(Error::msg("str-slice OOB"));
    }
    let text = std::str::from_utf8(&bytes[st..en]).map_err(|_| Error::msg("utf8"))?;
    Ok(arena.alloc(HeapObj::Str(text.to_string())))
}

pub fn str_from_byte(arena: &mut Arena, b: Value) -> Result<Value> {
    let n = b.as_int().ok_or_else(|| Error::msg("str-from-byte"))? as u8;
    Ok(arena.alloc(HeapObj::Str(String::from(char::from(n)))))
}

enum IoHandle {
    File(OwnedFd),
    Sock(OwnedFd),
}

pub struct FdTable {
    slots: Vec<Option<IoHandle>>,
}

impl Default for FdTable {
    fn default() -> Self {
        Self { slots: Vec::new() }
    }
}

impl FdTable {
    pub fn sys_open_read(&mut self, path: &str) -> Result<Value> {
        let f = lkjscript2026_sys::open_read(path)
            .map_err(|e| Error::msg(format!("sys-open-read: {e}")))?;
        Ok(Value::from_int(self.push(IoHandle::File(f)) as i64))
    }

    pub fn sys_open_write(&mut self, path: &str) -> Result<Value> {
        let f = lkjscript2026_sys::open_write(path)
            .map_err(|e| Error::msg(format!("sys-open-write: {e}")))?;
        Ok(Value::from_int(self.push(IoHandle::File(f)) as i64))
    }

    pub fn sys_path_exists(path: &str) -> Result<Value> {
        let ok = lkjscript2026_sys::path_exists(path)
            .map_err(|e| Error::msg(format!("sys-path-exists: {e}")))?;
        Ok(Value::from_int(if ok { 1 } else { 0 }))
    }

    pub fn sys_socket(&mut self) -> Result<Value> {
        let sock = lkjscript2026_sys::tcp_socket()
            .map_err(|e| Error::msg(format!("sys-socket: {e}")))?;
        Ok(Value::from_int(self.push(IoHandle::Sock(sock)) as i64))
    }

    pub fn sys_bind(&mut self, fd: Value, port: Value) -> Result<Value> {
        let raw = self.sock_raw(fd)?;
        let p = port
            .as_int()
            .ok_or_else(|| Error::msg("sys-bind port"))? as u16;
        lkjscript2026_sys::set_reuseaddr(raw)
            .map_err(|e| Error::msg(format!("sys-bind reuse: {e}")))?;
        lkjscript2026_sys::bind_ipv4_any(raw, p)
            .map_err(|e| Error::msg(format!("sys-bind: {e}")))?;
        Ok(Value::NIL)
    }

    pub fn sys_listen(&mut self, fd: Value, backlog: Value) -> Result<Value> {
        let raw = self.sock_raw(fd)?;
        let n = backlog
            .as_int()
            .ok_or_else(|| Error::msg("sys-listen backlog"))? as i32;
        lkjscript2026_sys::listen_sock(raw, n)
            .map_err(|e| Error::msg(format!("sys-listen: {e}")))?;
        Ok(Value::NIL)
    }

    pub fn sys_accept(&mut self, fd: Value) -> Result<Value> {
        let raw = self.sock_raw(fd)?;
        let client = lkjscript2026_sys::accept_sock(raw)
            .map_err(|e| Error::msg(format!("sys-accept: {e}")))?;
        Ok(Value::from_int(self.push(IoHandle::Sock(client)) as i64))
    }

    pub fn sys_recv(&mut self, arena: &mut Arena, fd: Value) -> Result<Value> {
        let raw = self.sock_raw(fd)?;
        let mut buf = vec![0u8; 4096];
        let n = lkjscript2026_sys::recv_sock(raw, &mut buf)
            .map_err(|e| Error::msg(format!("sys-recv: {e}")))?;
        buf.truncate(n);
        let text = String::from_utf8_lossy(&buf).into_owned();
        Ok(arena.alloc(HeapObj::Str(text)))
    }

    pub fn sys_send(&mut self, arena: &Arena, fd: Value, data: Value) -> Result<Value> {
        let raw = self.sock_raw(fd)?;
        let bytes = as_str(arena, data)?.as_bytes();
        let mut sent = 0;
        while sent < bytes.len() {
            let n = lkjscript2026_sys::send_sock(raw, &bytes[sent..])
                .map_err(|e| Error::msg(format!("sys-send: {e}")))?;
            if n == 0 {
                break;
            }
            sent += n;
        }
        Ok(Value::NIL)
    }

    pub fn close(&mut self, fd: Value) -> Result<Value> {
        let i = fd.as_int().ok_or_else(|| Error::msg("close fd"))? as usize;
        if let Some(slot) = self.slots.get_mut(i) {
            *slot = None;
        }
        Ok(Value::NIL)
    }

    pub fn read_byte(&mut self, fd: Value) -> Result<Value> {
        let i = fd.as_int().ok_or_else(|| Error::msg("read-byte-fd"))? as usize;
        let mut buf = [0u8; 1];
        let n = match self.slots.get_mut(i).and_then(|s| s.as_mut()) {
            Some(IoHandle::File(f)) => lkjscript2026_sys::read_fd(f.as_raw(), &mut buf)
                .map_err(|e| Error::msg(format!("read-byte-fd: {e}")))?,
            Some(IoHandle::Sock(s)) => lkjscript2026_sys::recv_sock(s.as_raw(), &mut buf)
                .map_err(|e| Error::msg(format!("read-byte-fd: {e}")))?,
            _ => return Err(Error::msg("bad fd")),
        };
        if n == 0 {
            Ok(Value::from_int(-1))
        } else {
            Ok(Value::from_int(buf[0] as i64))
        }
    }

    pub fn write_byte(&mut self, fd: Value, b: Value) -> Result<Value> {
        let i = fd.as_int().ok_or_else(|| Error::msg("write-byte-fd"))? as usize;
        let n = b.as_int().ok_or_else(|| Error::msg("byte"))? as u8;
        match self.slots.get_mut(i).and_then(|s| s.as_mut()) {
            Some(IoHandle::File(f)) => {
                lkjscript2026_sys::write_fd(f.as_raw(), &[n])
                    .map_err(|e| Error::msg(format!("write-byte-fd: {e}")))?;
            }
            Some(IoHandle::Sock(s)) => {
                lkjscript2026_sys::send_sock(s.as_raw(), &[n])
                    .map_err(|e| Error::msg(format!("write-byte-fd: {e}")))?;
            }
            _ => return Err(Error::msg("bad fd")),
        }
        Ok(Value::NIL)
    }

    fn sock_raw(&self, fd: Value) -> Result<RawFd> {
        let i = fd.as_int().ok_or_else(|| Error::msg("sock fd"))? as usize;
        match self.slots.get(i).and_then(|s| s.as_ref()) {
            Some(IoHandle::Sock(s)) => Ok(s.as_raw()),
            _ => Err(Error::msg("not a socket fd")),
        }
    }

    fn push(&mut self, h: IoHandle) -> usize {
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(h);
                return i;
            }
        }
        self.slots.push(Some(h));
        self.slots.len() - 1
    }
}
