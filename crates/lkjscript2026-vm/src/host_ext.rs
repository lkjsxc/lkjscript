//! String, filesystem, and TCP host ops.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

use lkjscript2026_core::{Error, HeapObj, Result, Value};

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

pub fn path_exists(path: &str) -> Result<Value> {
    Ok(Value::from_int(if Path::new(path).exists() { 1 } else { 0 }))
}

pub fn str_from_byte(arena: &mut Arena, b: Value) -> Result<Value> {
    let n = b.as_int().ok_or_else(|| Error::msg("str-from-byte"))? as u8;
    Ok(arena.alloc(HeapObj::Str(String::from(char::from(n)))))
}

enum IoHandle {
    File(File),
    Listener(TcpListener),
    Stream(TcpStream),
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
    pub fn open_read(&mut self, path: &str) -> Result<Value> {
        let f = File::open(path).map_err(|e| Error::msg(format!("open-read: {e}")))?;
        Ok(Value::from_int(self.push(IoHandle::File(f)) as i64))
    }

    pub fn open_write(&mut self, path: &str) -> Result<Value> {
        let f = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .map_err(|e| Error::msg(format!("open-write: {e}")))?;
        Ok(Value::from_int(self.push(IoHandle::File(f)) as i64))
    }

    pub fn tcp_listen(&mut self, port: Value) -> Result<Value> {
        let p = port.as_int().ok_or_else(|| Error::msg("tcp-listen port"))? as u16;
        let addr = format!("0.0.0.0:{p}");
        let lis = TcpListener::bind(&addr).map_err(|e| Error::msg(format!("tcp-listen: {e}")))?;
        Ok(Value::from_int(self.push(IoHandle::Listener(lis)) as i64))
    }

    pub fn tcp_accept(&mut self, fd: Value) -> Result<Value> {
        let i = fd.as_int().ok_or_else(|| Error::msg("tcp-accept fd"))? as usize;
        let stream = match self.slots.get(i).and_then(|s| s.as_ref()) {
            Some(IoHandle::Listener(lis)) => {
                let (s, _) = lis
                    .accept()
                    .map_err(|e| Error::msg(format!("tcp-accept: {e}")))?;
                s
            }
            _ => return Err(Error::msg("tcp-accept: not a listener")),
        };
        Ok(Value::from_int(self.push(IoHandle::Stream(stream)) as i64))
    }

    pub fn tcp_recv(&mut self, arena: &mut Arena, fd: Value) -> Result<Value> {
        let i = fd.as_int().ok_or_else(|| Error::msg("tcp-recv fd"))? as usize;
        let mut buf = vec![0u8; 4096];
        let n = match self.slots.get_mut(i).and_then(|s| s.as_mut()) {
            Some(IoHandle::Stream(s)) => s
                .read(&mut buf)
                .map_err(|e| Error::msg(format!("tcp-recv: {e}")))?,
            _ => return Err(Error::msg("tcp-recv: not a stream")),
        };
        buf.truncate(n);
        let text = String::from_utf8_lossy(&buf).into_owned();
        Ok(arena.alloc(HeapObj::Str(text)))
    }

    pub fn tcp_send(&mut self, arena: &Arena, fd: Value, data: Value) -> Result<Value> {
        let i = fd.as_int().ok_or_else(|| Error::msg("tcp-send fd"))? as usize;
        let bytes = as_str(arena, data)?.as_bytes();
        match self.slots.get_mut(i).and_then(|s| s.as_mut()) {
            Some(IoHandle::Stream(s)) => s
                .write_all(bytes)
                .map_err(|e| Error::msg(format!("tcp-send: {e}")))?,
            _ => return Err(Error::msg("tcp-send: not a stream")),
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
            Some(IoHandle::File(f)) => f.read(&mut buf),
            Some(IoHandle::Stream(s)) => s.read(&mut buf),
            _ => return Err(Error::msg("bad fd")),
        }
        .map_err(|e| Error::msg(format!("read-byte-fd: {e}")))?;
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
            Some(IoHandle::File(f)) => f
                .write_all(&[n])
                .map_err(|e| Error::msg(format!("write-byte-fd: {e}")))?,
            Some(IoHandle::Stream(s)) => s
                .write_all(&[n])
                .map_err(|e| Error::msg(format!("write-byte-fd: {e}")))?,
            _ => return Err(Error::msg("bad fd")),
        }
        Ok(Value::NIL)
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
