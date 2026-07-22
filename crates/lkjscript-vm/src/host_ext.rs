//! Strings, resource handles, filesystem, and socket host operations.

use std::os::fd::RawFd;

use lkjscript_core::{Error, HeapObj, Result, Value};
use lkjscript_sys::OwnedFd;

use crate::arena::Arena;

const STDIN_TOKEN: u32 = 1;
const FIRST_OWNED_TOKEN: u32 = 16;

pub fn as_str(arena: &Arena, value: Value) -> Result<&str> {
    match arena.get(value)? {
        HeapObj::Str(text) | HeapObj::Symbol(text) => Ok(text.as_str()),
        _ => Err(Error::msg("expected string")),
    }
}

pub fn str_len(arena: &Arena, value: Value) -> Result<Value> {
    Ok(Value::from_int(as_str(arena, value)?.len() as i64))
}

pub fn str_ref(arena: &Arena, string: Value, index: Value) -> Result<Value> {
    let index = index.as_int().ok_or_else(|| Error::msg("str-ref index"))?;
    let index = usize::try_from(index).map_err(|_| Error::msg("str-ref index out of range"))?;
    let byte = *as_str(arena, string)?
        .as_bytes()
        .get(index)
        .ok_or_else(|| Error::msg("str-ref out of bounds"))?;
    Ok(Value::from_int(i64::from(byte)))
}

pub fn str_append(arena: &mut Arena, left: Value, right: Value) -> Result<Value> {
    let mut output = as_str(arena, left)?.to_string();
    output.push_str(as_str(arena, right)?);
    Ok(arena.alloc(HeapObj::Str(output)))
}

pub fn str_slice(arena: &mut Arena, string: Value, start: Value, end: Value) -> Result<Value> {
    let start = start
        .as_int()
        .ok_or_else(|| Error::msg("str-slice start"))?;
    let end = end.as_int().ok_or_else(|| Error::msg("str-slice end"))?;
    let start = usize::try_from(start).map_err(|_| Error::msg("str-slice start out of range"))?;
    let end = usize::try_from(end).map_err(|_| Error::msg("str-slice end out of range"))?;
    let bytes = as_str(arena, string)?.as_bytes();
    if start > end || end > bytes.len() {
        return Err(Error::msg("str-slice out of bounds"));
    }
    let text = std::str::from_utf8(&bytes[start..end])
        .map_err(|_| Error::msg("str-slice splits UTF-8"))?;
    Ok(arena.alloc(HeapObj::Str(text.to_string())))
}

pub fn str_from_byte(arena: &mut Arena, byte: Value) -> Result<Value> {
    let number = byte.as_int().ok_or_else(|| Error::msg("str-from-byte"))?;
    let byte = u8::try_from(number).map_err(|_| Error::msg("str-from-byte out of range"))?;
    Ok(arena.alloc(HeapObj::Str(String::from(char::from(byte)))))
}

enum OwnedResource {
    File(OwnedFd),
    Socket(OwnedFd),
}

#[derive(Default)]
pub struct ResourceTable {
    slots: Vec<Option<OwnedResource>>,
}

impl ResourceTable {
    pub fn stdin_handle() -> Value {
        Value::from_handle(STDIN_TOKEN)
    }

    pub fn sys_open_read(&mut self, path: &str) -> Result<Value> {
        let file = lkjscript_sys::open_read(path)
            .map_err(|error| Error::msg(format!("sys-open-read: {error}")))?;
        self.push(OwnedResource::File(file))
    }

    pub fn sys_open_write(&mut self, path: &str) -> Result<Value> {
        let file = lkjscript_sys::open_write(path)
            .map_err(|error| Error::msg(format!("sys-open-write: {error}")))?;
        self.push(OwnedResource::File(file))
    }

    pub fn sys_path_exists(path: &str) -> Result<Value> {
        let exists = lkjscript_sys::path_exists(path)
            .map_err(|error| Error::msg(format!("sys-path-exists: {error}")))?;
        Ok(Value::from_bool(exists))
    }

    pub fn sys_socket(&mut self) -> Result<Value> {
        let socket = lkjscript_sys::tcp_socket()
            .map_err(|error| Error::msg(format!("sys-socket: {error}")))?;
        self.push(OwnedResource::Socket(socket))
    }

    pub fn sys_bind(&self, handle: Value, port: Value) -> Result<Value> {
        let raw = self.socket_raw(handle, "sys-bind")?;
        let port = port.as_int().ok_or_else(|| Error::msg("sys-bind port"))?;
        let port = u16::try_from(port).map_err(|_| Error::msg("sys-bind port out of range"))?;
        lkjscript_sys::set_reuseaddr(raw)
            .map_err(|error| Error::msg(format!("sys-bind reuse: {error}")))?;
        lkjscript_sys::bind_ipv4_any(raw, port)
            .map_err(|error| Error::msg(format!("sys-bind: {error}")))?;
        Ok(Value::NIL)
    }

    pub fn sys_listen(&self, handle: Value, backlog: Value) -> Result<Value> {
        let raw = self.socket_raw(handle, "sys-listen")?;
        let backlog = backlog
            .as_int()
            .ok_or_else(|| Error::msg("sys-listen backlog"))?;
        let backlog =
            i32::try_from(backlog).map_err(|_| Error::msg("sys-listen backlog out of range"))?;
        if backlog < 0 {
            return Err(Error::msg("sys-listen backlog out of range"));
        }
        lkjscript_sys::listen_sock(raw, backlog)
            .map_err(|error| Error::msg(format!("sys-listen: {error}")))?;
        Ok(Value::NIL)
    }

    pub fn sys_accept(&mut self, handle: Value) -> Result<Value> {
        let raw = self.socket_raw(handle, "sys-accept")?;
        let client = lkjscript_sys::accept_sock(raw)
            .map_err(|error| Error::msg(format!("sys-accept: {error}")))?;
        self.push(OwnedResource::Socket(client))
    }

    pub fn sys_recv(&self, arena: &mut Arena, handle: Value) -> Result<Value> {
        let raw = self.socket_raw(handle, "sys-recv")?;
        let mut buffer = vec![0_u8; 4096];
        let received = lkjscript_sys::recv_sock(raw, &mut buffer)
            .map_err(|error| Error::msg(format!("sys-recv: {error}")))?;
        buffer.truncate(received);
        let text = String::from_utf8_lossy(&buffer).into_owned();
        Ok(arena.alloc(HeapObj::Str(text)))
    }

    pub fn sys_send(&self, arena: &Arena, handle: Value, data: Value) -> Result<Value> {
        let raw = self.socket_raw(handle, "sys-send")?;
        let bytes = as_str(arena, data)?.as_bytes();
        let mut sent = 0_usize;
        while sent < bytes.len() {
            let count = lkjscript_sys::send_sock(raw, &bytes[sent..])
                .map_err(|error| Error::msg(format!("sys-send: {error}")))?;
            if count == 0 {
                return Err(Error::msg("sys-send: zero-byte progress"));
            }
            sent += count;
        }
        let sent = i64::try_from(sent).map_err(|_| Error::msg("sys-send count out of range"))?;
        Ok(Value::from_int(sent))
    }

    pub fn close(&mut self, handle: Value) -> Result<Value> {
        let index = self.owned_index(handle, "sys-close")?;
        let slot = self
            .slots
            .get_mut(index)
            .ok_or_else(|| Error::msg("sys-close: unknown handle"))?;
        if slot.take().is_none() {
            return Err(Error::msg("sys-close: stale or already closed handle"));
        }
        Ok(Value::NIL)
    }

    pub fn read_byte(&mut self, handle: Value) -> Result<Value> {
        let mut buffer = [0_u8; 1];
        let count = if handle.as_handle() == Some(STDIN_TOKEN) {
            lkjscript_sys::read_fd(lkjscript_sys::STDIN_FD, &mut buffer)
                .map_err(|error| Error::msg(format!("sys-read-byte: {error}")))?
        } else {
            let index = self.owned_index(handle, "sys-read-byte")?;
            match self.slots.get_mut(index).and_then(Option::as_mut) {
                Some(OwnedResource::File(file)) => {
                    lkjscript_sys::read_fd(file.as_raw(), &mut buffer)
                        .map_err(|error| Error::msg(format!("sys-read-byte: {error}")))?
                }
                Some(OwnedResource::Socket(socket)) => {
                    lkjscript_sys::recv_sock(socket.as_raw(), &mut buffer)
                        .map_err(|error| Error::msg(format!("sys-read-byte: {error}")))?
                }
                None => return Err(Error::msg("sys-read-byte: stale or unknown handle")),
            }
        };
        if count == 0 {
            Ok(Value::from_int(-1))
        } else {
            Ok(Value::from_int(i64::from(buffer[0])))
        }
    }

    pub fn write_byte(&mut self, handle: Value, byte: Value) -> Result<Value> {
        let index = self.owned_index(handle, "sys-write-byte")?;
        let byte = byte
            .as_int()
            .ok_or_else(|| Error::msg("sys-write-byte byte"))?;
        let byte =
            u8::try_from(byte).map_err(|_| Error::msg("sys-write-byte byte out of range"))?;
        match self.slots.get_mut(index).and_then(Option::as_mut) {
            Some(OwnedResource::File(file)) => {
                lkjscript_sys::write_fd(file.as_raw(), &[byte])
                    .map_err(|error| Error::msg(format!("sys-write-byte: {error}")))?;
            }
            Some(OwnedResource::Socket(socket)) => {
                lkjscript_sys::send_sock(socket.as_raw(), &[byte])
                    .map_err(|error| Error::msg(format!("sys-write-byte: {error}")))?;
            }
            None => return Err(Error::msg("sys-write-byte: stale or unknown handle")),
        }
        Ok(Value::NIL)
    }

    pub(crate) fn raw_fd(&self, handle: Value, operation: &str) -> Result<RawFd> {
        if handle.as_handle() == Some(STDIN_TOKEN) {
            return Ok(lkjscript_sys::STDIN_FD);
        }
        let index = self.owned_index(handle, operation)?;
        match self.slots.get(index).and_then(Option::as_ref) {
            Some(OwnedResource::File(file)) => Ok(file.as_raw()),
            Some(OwnedResource::Socket(socket)) => Ok(socket.as_raw()),
            None => Err(Error::msg(format!("{operation}: stale or unknown handle"))),
        }
    }

    fn socket_raw(&self, handle: Value, operation: &str) -> Result<RawFd> {
        let index = self.owned_index(handle, operation)?;
        match self.slots.get(index).and_then(Option::as_ref) {
            Some(OwnedResource::Socket(socket)) => Ok(socket.as_raw()),
            Some(OwnedResource::File(_)) => {
                Err(Error::msg(format!("{operation}: handle is not a socket")))
            }
            None => Err(Error::msg(format!("{operation}: stale or unknown handle"))),
        }
    }

    fn owned_index(&self, handle: Value, operation: &str) -> Result<usize> {
        let token = handle
            .as_handle()
            .ok_or_else(|| Error::msg(format!("{operation}: expected Handle")))?;
        let index = token
            .checked_sub(FIRST_OWNED_TOKEN)
            .ok_or_else(|| Error::msg(format!("{operation}: borrowed or invalid handle")))?;
        usize::try_from(index).map_err(|_| Error::msg(format!("{operation}: invalid handle")))
    }

    fn push(&mut self, handle: OwnedResource) -> Result<Value> {
        let index = u32::try_from(self.slots.len())
            .map_err(|_| Error::msg("resource handle table exhausted"))?;
        let token = FIRST_OWNED_TOKEN
            .checked_add(index)
            .ok_or_else(|| Error::msg("resource handle token exhausted"))?;
        self.slots.push(Some(handle));
        Ok(Value::from_handle(token))
    }
}

pub fn result_ok(arena: &mut Arena, value: Value) -> Value {
    arena.alloc(HeapObj::ResultOk(value))
}

pub fn result_err(arena: &mut Arena, value: Value) -> Value {
    arena.alloc(HeapObj::ResultErr(value))
}

pub fn language_result(arena: &mut Arena, result: Result<Value>) -> Value {
    match result {
        Ok(value) => result_ok(arena, value),
        Err(error) => {
            let message = arena.alloc(HeapObj::Str(error.to_string()));
            result_err(arena, message)
        }
    }
}

pub fn is_ok(arena: &Arena, value: Value) -> Result<Value> {
    match arena.get(value)? {
        HeapObj::ResultOk(_) => Ok(Value::TRUE),
        HeapObj::ResultErr(_) => Ok(Value::FALSE),
        _ => Err(Error::msg("is-ok: expected Result")),
    }
}

pub fn unwrap_ok(arena: &Arena, value: Value) -> Result<Value> {
    match arena.get(value)? {
        HeapObj::ResultOk(inner) => Ok(*inner),
        HeapObj::ResultErr(error) => {
            let message = match arena.get(*error) {
                Ok(HeapObj::Str(message)) => format!("unwrap-ok: {message}"),
                _ => "unwrap-ok on Err".to_string(),
            };
            Err(Error::msg(message))
        }
        _ => Err(Error::msg("unwrap-ok: expected Result")),
    }
}

pub fn unwrap_err(arena: &Arena, value: Value) -> Result<Value> {
    match arena.get(value)? {
        HeapObj::ResultErr(inner) => Ok(*inner),
        HeapObj::ResultOk(_) => Err(Error::msg("unwrap-err on Ok")),
        _ => Err(Error::msg("unwrap-err: expected Result")),
    }
}

pub fn str_from_i64(arena: &mut Arena, number: Value) -> Result<Value> {
    let number = number.as_int().ok_or_else(|| Error::msg("str-from-i64"))?;
    Ok(arena.alloc(HeapObj::Str(number.to_string())))
}

pub fn str_from_f64(arena: &mut Arena, number: Value) -> Result<Value> {
    let number = if let Some(integer) = number.as_int() {
        integer as f64
    } else {
        match arena.get(number)? {
            HeapObj::Float(float) => *float,
            _ => return Err(Error::msg("str-from-f64")),
        }
    };
    Ok(arena.alloc(HeapObj::Str(number.to_string())))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use lkjscript_core::{Error, Value};

    use crate::arena::Arena;

    use super::{as_str, is_ok, language_result, unwrap_err, unwrap_ok, ResourceTable};

    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

    struct TempFile(PathBuf);

    impl TempFile {
        fn new() -> std::io::Result<Self> {
            let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("lkjscript-handle-{}-{id}", std::process::id()));
            fs::write(&path, b"x")?;
            Ok(Self(path))
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn integer_and_borrowed_handles_cannot_be_closed() {
        let mut table = ResourceTable::default();
        assert!(table.close(Value::from_int(16)).is_err());
        assert!(table.close(ResourceTable::stdin_handle()).is_err());
    }

    #[test]
    fn closed_tokens_are_never_reused() -> std::io::Result<()> {
        let file = TempFile::new()?;
        let path = file.0.to_string_lossy();
        let mut table = ResourceTable::default();
        let first = table.sys_open_read(&path).ok();
        assert!(first.is_some());
        let first = first.unwrap_or(Value::NIL);
        assert_ne!(first, ResourceTable::stdin_handle());
        assert!(table.close(first).is_ok());
        assert!(table.close(first).is_err());
        assert!(table.read_byte(first).is_err());

        let second = table.sys_open_read(&path).ok();
        assert!(second.is_some());
        let second = second.unwrap_or(Value::NIL);
        assert_ne!(first, second);
        assert!(table.close(second).is_ok());
        Ok(())
    }

    #[test]
    fn file_handles_cannot_be_used_as_sockets() -> std::io::Result<()> {
        let file = TempFile::new()?;
        let path = file.0.to_string_lossy();
        let mut table = ResourceTable::default();
        let handle = table.sys_open_read(&path).ok().unwrap_or(Value::NIL);
        assert!(table.sys_listen(handle, Value::from_int(1)).is_err());
        Ok(())
    }

    #[test]
    fn socket_ranges_are_checked_before_os_calls() {
        let mut table = ResourceTable::default();
        let socket = table.sys_socket().ok().unwrap_or(Value::NIL);
        assert!(table.sys_bind(socket, Value::from_int(-1)).is_err());
        assert!(table.sys_bind(socket, Value::from_int(65_536)).is_err());
        assert!(table.sys_listen(socket, Value::from_int(-1)).is_err());
        assert!(table.close(socket).is_ok());
    }

    #[test]
    fn language_results_preserve_operation_error_text() {
        let mut arena = Arena::default();
        let result = language_result(&mut arena, Err(Error::msg("sys-example: failure")));
        assert_eq!(is_ok(&arena, result).ok(), Some(Value::FALSE));
        let error = unwrap_err(&arena, result).ok().unwrap_or(Value::NIL);
        assert_eq!(as_str(&arena, error).ok(), Some("sys-example: failure"));
        let unwrapped = unwrap_ok(&arena, result)
            .err()
            .map(|error| error.to_string());
        assert_eq!(
            unwrapped.as_deref(),
            Some("unwrap-ok: sys-example: failure")
        );
    }
}
