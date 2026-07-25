use super::*;

impl ResourceTable {
    pub fn sys_socket(&mut self) -> Result<Value> {
        self.ensure_capacity()?;
        let socket = lkjscript_sys::tcp_socket()
            .map_err(|error| Error::msg(format!("sys-socket: {error}")))?;
        self.push(OwnedResource::Socket(socket))
    }

    pub fn sys_bind(&self, handle: Value, port: i64) -> Result<Value> {
        let raw = self.socket_raw(handle, "sys-bind")?;
        let port = u16::try_from(port).map_err(|_| Error::msg("sys-bind port out of range"))?;
        lkjscript_sys::set_reuseaddr(raw)
            .map_err(|error| Error::msg(format!("sys-bind reuse: {error}")))?;
        lkjscript_sys::bind_ipv4_any(raw, port)
            .map_err(|error| Error::msg(format!("sys-bind: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sys_listen(&self, handle: Value, backlog: i64) -> Result<Value> {
        let raw = self.socket_raw(handle, "sys-listen")?;
        let backlog =
            i32::try_from(backlog).map_err(|_| Error::msg("sys-listen backlog out of range"))?;
        if backlog < 0 {
            return Err(Error::msg("sys-listen backlog out of range"));
        }
        lkjscript_sys::listen_sock(raw, backlog)
            .map_err(|error| Error::msg(format!("sys-listen: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sys_accept(&mut self, handle: Value) -> Result<Value> {
        self.ensure_capacity()?;
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
        arena.alloc(HeapObj::Str(text))
    }

    pub fn sys_send(&self, arena: &Arena, handle: Value, data: Value) -> Result<i64> {
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
        Ok(sent)
    }
}
