use super::*;

impl ResourceTable {
    pub fn read_into(&self, handle: Value, destination: &mut [u8]) -> Result<usize> {
        let index = self.owned_index(handle, "read-into")?;
        match self.slots.get(index).and_then(Option::as_ref) {
            Some(OwnedResource::File {
                descriptor: file,
                kind: ResourceKind::FileReader,
            }) => lkjscript_sys::read_fd(file.as_raw(), destination)
                .map_err(|error| Error::msg(format!("sys-read-into: {error}"))),
            Some(OwnedResource::Socket {
                descriptor: socket,
                kind: ResourceKind::TcpStream,
            }) => lkjscript_sys::recv_sock(socket.as_raw(), destination)
                .map_err(|error| Error::msg(format!("sys-read-into: {error}"))),
            Some(_) => Err(Error::msg("read-into: expected file-reader or tcp-stream")),
            None => Err(Error::msg("read-into: stale or unknown resource")),
        }
    }

    pub fn write_from(&self, handle: Value, source: &[u8]) -> Result<usize> {
        let index = self.owned_index(handle, "write-from")?;
        match self.slots.get(index).and_then(Option::as_ref) {
            Some(OwnedResource::File {
                descriptor: file,
                kind: ResourceKind::FileWriter | ResourceKind::FileAppender,
            }) => lkjscript_sys::write_fd(file.as_raw(), source)
                .map_err(|error| Error::msg(format!("sys-write-from: {error}"))),
            Some(OwnedResource::Socket {
                descriptor: socket,
                kind: ResourceKind::TcpStream,
            }) => lkjscript_sys::send_sock(socket.as_raw(), source)
                .map_err(|error| Error::msg(format!("sys-write-from: {error}"))),
            Some(_) => Err(Error::msg(
                "write-from: expected file-writer, file-appender, or tcp-stream",
            )),
            None => Err(Error::msg("write-from: stale or unknown resource")),
        }
    }

    pub fn read_byte(&mut self, handle: Value) -> Result<i64> {
        let mut buffer = [0_u8; 1];
        let count = if handle.as_handle() == Some(STDIN_TOKEN) {
            lkjscript_sys::read_fd(lkjscript_sys::STDIN_FD, &mut buffer)
                .map_err(|error| Error::msg(format!("sys-read-byte: {error}")))?
        } else {
            let index = self.owned_index(handle, "read-resource-byte")?;
            match self.slots.get_mut(index).and_then(Option::as_mut) {
                Some(OwnedResource::File {
                    descriptor: file,
                    kind: ResourceKind::FileReader,
                }) => lkjscript_sys::read_fd(file.as_raw(), &mut buffer)
                    .map_err(|error| Error::msg(format!("sys-read-byte: {error}")))?,
                Some(OwnedResource::Socket {
                    descriptor: socket,
                    kind: ResourceKind::TcpStream,
                }) => lkjscript_sys::recv_sock(socket.as_raw(), &mut buffer)
                    .map_err(|error| Error::msg(format!("sys-read-byte: {error}")))?,
                Some(_) => {
                    return Err(Error::msg(
                        "read-resource-byte: expected file-reader or tcp-stream",
                    ));
                }
                None => return Err(Error::msg("read-resource-byte: stale resource")),
            }
        };
        if count == 0 {
            Ok(-1)
        } else {
            Ok(i64::from(buffer[0]))
        }
    }

    pub fn write_byte(&mut self, handle: Value, byte: i64) -> Result<Value> {
        let index = self.owned_index(handle, "write-resource-byte")?;
        let byte =
            u8::try_from(byte).map_err(|_| Error::msg("sys-write-byte byte out of range"))?;
        match self.slots.get_mut(index).and_then(Option::as_mut) {
            Some(OwnedResource::File {
                descriptor: file,
                kind: ResourceKind::FileWriter | ResourceKind::FileAppender,
            }) => {
                lkjscript_sys::write_fd(file.as_raw(), &[byte])
                    .map_err(|error| Error::msg(format!("sys-write-byte: {error}")))?;
            }
            Some(OwnedResource::Socket {
                descriptor: socket,
                kind: ResourceKind::TcpStream,
            }) => {
                lkjscript_sys::send_sock(socket.as_raw(), &[byte])
                    .map_err(|error| Error::msg(format!("sys-write-byte: {error}")))?;
            }
            Some(_) => {
                return Err(Error::msg(concat!(
                    "write-resource-byte: expected file-writer, file-appender, ",
                    "or tcp-stream"
                )));
            }
            None => return Err(Error::msg("write-resource-byte: stale resource")),
        }
        Ok(Value::UNIT)
    }

    pub fn poll_readable(&self, handle: Value, timeout_ms: i32, operation: &str) -> Result<bool> {
        let raw = self.raw_fd(handle, operation)?;
        lkjscript_sys::poll_fd(raw, timeout_ms)
            .map_err(|error| Error::msg(format!("{operation}: {error}")))
    }
}
