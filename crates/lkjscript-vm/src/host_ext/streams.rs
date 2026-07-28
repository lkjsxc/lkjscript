use super::resource_token::decode_parts;
use super::*;

impl ResourceTable {
    pub fn read_into(&self, handle: Value, destination: &mut [u8]) -> Result<usize> {
        let (kind, payload) = self.owned_payload_for(
            handle,
            &[ResourceKind::FileReader, ResourceKind::TcpStream],
            "read-into",
            "expected file-reader or tcp-stream",
        )?;
        match (kind, payload) {
            (ResourceKind::FileReader, OwnedResource::File(file)) => {
                lkjscript_sys::read_fd(file.as_raw(), destination)
                    .map_err(|error| Error::msg(format!("sys-read-into: {error}")))
            }
            (ResourceKind::TcpStream, OwnedResource::Socket(socket)) => {
                lkjscript_sys::recv_sock(socket.as_raw(), destination)
                    .map_err(|error| Error::msg(format!("sys-read-into: {error}")))
            }
            _ => Err(Error::msg("read-into: invalid readable resource payload")),
        }
    }

    pub fn write_from(&self, handle: Value, source: &[u8]) -> Result<usize> {
        let (kind, payload) = self.owned_payload_for(
            handle,
            &[
                ResourceKind::FileWriter,
                ResourceKind::FileAppender,
                ResourceKind::TcpStream,
            ],
            "write-from",
            "expected file-writer, file-appender, or tcp-stream",
        )?;
        match (kind, payload) {
            (ResourceKind::FileWriter | ResourceKind::FileAppender, OwnedResource::File(file)) => {
                lkjscript_sys::write_fd(file.as_raw(), source)
                    .map_err(|error| Error::msg(format!("sys-write-from: {error}")))
            }
            (ResourceKind::TcpStream, OwnedResource::Socket(socket)) => {
                lkjscript_sys::send_sock(socket.as_raw(), source)
                    .map_err(|error| Error::msg(format!("sys-write-from: {error}")))
            }
            _ => Err(Error::msg("write-from: invalid writable resource payload")),
        }
    }

    pub fn read_byte(&mut self, handle: Value) -> Result<i64> {
        let mut buffer = [0_u8; 1];
        let count = match self.standard_input(handle, "read-resource-byte")? {
            true => lkjscript_sys::read_fd(lkjscript_sys::STDIN_FD, &mut buffer)
                .map_err(|error| Error::msg(format!("sys-read-byte: {error}")))?,
            false => {
                let (kind, payload) = self.owned_payload_for(
                    handle,
                    &[ResourceKind::FileReader, ResourceKind::TcpStream],
                    "read-resource-byte",
                    "expected file-reader or tcp-stream",
                )?;
                match (kind, payload) {
                    (ResourceKind::FileReader, OwnedResource::File(file)) => {
                        lkjscript_sys::read_fd(file.as_raw(), &mut buffer)
                            .map_err(|error| Error::msg(format!("sys-read-byte: {error}")))?
                    }
                    (ResourceKind::TcpStream, OwnedResource::Socket(socket)) => {
                        lkjscript_sys::recv_sock(socket.as_raw(), &mut buffer)
                            .map_err(|error| Error::msg(format!("sys-read-byte: {error}")))?
                    }
                    _ => return Err(Error::msg("read-resource-byte: invalid resource payload")),
                }
            }
        };
        if count == 0 {
            Ok(-1)
        } else {
            Ok(i64::from(buffer[0]))
        }
    }

    pub fn write_byte(&mut self, handle: Value, byte: i64) -> Result<Value> {
        let byte =
            u8::try_from(byte).map_err(|_| Error::msg("sys-write-byte byte out of range"))?;
        let (kind, payload) = self.owned_payload_for(
            handle,
            &[
                ResourceKind::FileWriter,
                ResourceKind::FileAppender,
                ResourceKind::TcpStream,
            ],
            "write-resource-byte",
            "expected file-writer, file-appender, or tcp-stream",
        )?;
        match (kind, payload) {
            (ResourceKind::FileWriter | ResourceKind::FileAppender, OwnedResource::File(file)) => {
                lkjscript_sys::write_fd(file.as_raw(), &[byte])
                    .map_err(|error| Error::msg(format!("sys-write-byte: {error}")))?;
            }
            (ResourceKind::TcpStream, OwnedResource::Socket(socket)) => {
                lkjscript_sys::send_sock(socket.as_raw(), &[byte])
                    .map_err(|error| Error::msg(format!("sys-write-byte: {error}")))?;
            }
            _ => return Err(Error::msg("write-resource-byte: invalid resource payload")),
        }
        Ok(Value::UNIT)
    }

    pub fn poll_readable(&self, handle: Value, timeout_ms: i32, operation: &str) -> Result<bool> {
        let raw = self.raw_fd(handle, operation)?;
        lkjscript_sys::poll_fd(raw, timeout_ms)
            .map_err(|error| Error::msg(format!("{operation}: {error}")))
    }

    fn standard_input(&self, handle: Value, operation: &str) -> Result<bool> {
        let parts = decode_parts(handle, operation)?;
        match self.table.resolve_token_parts(
            parts,
            ResourceKind::InputStream,
            STDIO_PROVIDER,
            self.table.scope(),
            ResourceOwnership::Borrowed,
        ) {
            Ok(key) => self
                .table
                .borrowed(
                    &key,
                    ResourceKind::InputStream,
                    STDIO_PROVIDER,
                    self.table.scope(),
                )
                .map(|payload| matches!(payload, OwnedResource::StandardInput))
                .map_err(|error| self.access_error(operation, error)),
            Err(ResourceTableError::WrongKind { .. }) => Ok(false),
            Err(error) => Err(self.access_error(operation, error)),
        }
    }
}
