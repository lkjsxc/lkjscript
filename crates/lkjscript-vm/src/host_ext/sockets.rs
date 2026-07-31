use super::*;

pub enum SocketReceiveError {
    Network(Error),
    Utf8(lkjscript_core::Utf8Failure),
}

impl ResourceTable {
    pub fn sys_socket(&mut self) -> Result<Value> {
        self.acquire_owned(
            ResourceKind::TcpListener,
            NETWORK_PROVIDER,
            "open-tcp-socket",
            || {
                lkjscript_sys::tcp_socket()
                    .map(OwnedResource::Socket)
                    .map_err(|error| Error::msg(format!("sys-socket: {error}")))
            },
        )
    }

    pub fn sys_bind(&self, handle: Value, port: i64) -> Result<Value> {
        let raw = self.socket_raw(handle, ResourceKind::TcpListener, "bind-tcp")?;
        let port = u16::try_from(port).map_err(|_| Error::msg("sys-bind port out of range"))?;
        lkjscript_sys::set_reuseaddr(raw)
            .map_err(|error| Error::msg(format!("sys-bind reuse: {error}")))?;
        lkjscript_sys::bind_ipv4_any(raw, port)
            .map_err(|error| Error::msg(format!("sys-bind: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sys_listen(&self, handle: Value, backlog: i64) -> Result<Value> {
        let raw = self.socket_raw(handle, ResourceKind::TcpListener, "listen-tcp")?;
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
        let raw = self.socket_raw(handle, ResourceKind::TcpListener, "accept-tcp")?;
        self.acquire_owned(
            ResourceKind::TcpStream,
            NETWORK_PROVIDER,
            "accept-tcp",
            || {
                lkjscript_sys::accept_sock(raw)
                    .map(OwnedResource::Socket)
                    .map_err(|error| Error::msg(format!("sys-accept: {error}")))
            },
        )
    }

    pub fn sys_recv(&self, handle: Value) -> std::result::Result<String, SocketReceiveError> {
        let raw = self
            .socket_raw(handle, ResourceKind::TcpStream, "receive-string")
            .map_err(SocketReceiveError::Network)?;
        let mut buffer = vec![0_u8; 4096];
        let received = lkjscript_sys::recv_sock(raw, &mut buffer).map_err(|error| {
            SocketReceiveError::Network(Error::msg(format!("sys-recv: {error}")))
        })?;
        buffer.truncate(received);
        let text = lkjscript_core::validate_utf8(&buffer).map_err(SocketReceiveError::Utf8)?;
        Ok(text.to_owned())
    }

    pub fn sys_send(&self, handle: Value, data: &str) -> Result<i64> {
        let raw = self.socket_raw(handle, ResourceKind::TcpStream, "send-string")?;
        let bytes = data.as_bytes();
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
