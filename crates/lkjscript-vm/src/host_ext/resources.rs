use super::resource_token::decode_parts;
use super::*;

impl ResourceTable {
    pub(crate) fn raw_fd(&self, handle: Value, operation: &str) -> Result<RawFd> {
        let parts = decode_parts(handle, operation)?;
        match self.table.resolve_token_parts(
            parts,
            ResourceKind::InputStream,
            STDIO_PROVIDER,
            self.table.scope(),
            ResourceOwnership::Borrowed,
        ) {
            Ok(key) => {
                return match self.table.borrowed(
                    &key,
                    ResourceKind::InputStream,
                    STDIO_PROVIDER,
                    self.table.scope(),
                ) {
                    Ok(OwnedResource::StandardInput) => Ok(lkjscript_sys::STDIN_FD),
                    Ok(_) => Err(Error::msg(format!(
                        "{operation}: invalid standard input payload"
                    ))),
                    Err(error) => Err(self.access_error(operation, error)),
                };
            }
            Err(ResourceTableError::WrongKind { .. }) => {}
            Err(error) => return Err(self.access_error(operation, error)),
        }
        let (kind, payload) = self.owned_payload_for(
            handle,
            &[
                ResourceKind::FileReader,
                ResourceKind::TcpListener,
                ResourceKind::TcpStream,
            ],
            operation,
            "typed resource kind is not pollable",
        )?;
        match (kind, payload) {
            (ResourceKind::FileReader, OwnedResource::File(descriptor))
            | (
                ResourceKind::TcpListener | ResourceKind::TcpStream,
                OwnedResource::Socket(descriptor),
            ) => Ok(descriptor.as_raw()),
            _ => Err(Error::msg(format!(
                "{operation}: invalid pollable resource payload"
            ))),
        }
    }

    pub(crate) fn sqlite_connection(
        &self,
        handle: Value,
        operation: &str,
    ) -> Result<&lkjscript_sys::SqliteConnection> {
        let payload = self.owned_exact_payload(
            handle,
            ResourceKind::SqliteConnection,
            SQLITE_PROVIDER,
            operation,
        )?;
        match payload {
            OwnedResource::SqliteConnection(connection) => Ok(connection),
            _ => Err(Error::msg(format!(
                "{operation}: invalid SQLite connection payload"
            ))),
        }
    }

    pub(crate) fn sqlite_statement(
        &self,
        handle: Value,
        operation: &str,
    ) -> Result<&lkjscript_sys::SqliteStatement> {
        let payload = self.owned_exact_payload(
            handle,
            ResourceKind::SqliteStatement,
            SQLITE_PROVIDER,
            operation,
        )?;
        match payload {
            OwnedResource::SqliteStatement(statement) => Ok(statement),
            _ => Err(Error::msg(format!(
                "{operation}: invalid SQLite statement payload"
            ))),
        }
    }

    pub(crate) fn socket_raw(
        &self,
        handle: Value,
        expected: ResourceKind,
        operation: &str,
    ) -> Result<RawFd> {
        let payload = self.owned_exact_payload(handle, expected, NETWORK_PROVIDER, operation)?;
        match payload {
            OwnedResource::Socket(socket) => Ok(socket.as_raw()),
            _ => Err(Error::msg(format!(
                "{operation}: invalid {} payload",
                expected.as_str()
            ))),
        }
    }

    pub(crate) fn file_raw(&self, handle: Value, operation: &str) -> Result<RawFd> {
        let (_, payload) = self.owned_payload_for(
            handle,
            &[ResourceKind::FileWriter, ResourceKind::FileAppender],
            operation,
            "expected file-writer or file-appender",
        )?;
        match payload {
            OwnedResource::File(file) => Ok(file.as_raw()),
            _ => Err(Error::msg(format!(
                "{operation}: invalid writable file payload"
            ))),
        }
    }

    pub(crate) fn sync_raw(&self, handle: Value, operation: &str) -> Result<RawFd> {
        let (kind, payload) = self.owned_payload_for(
            handle,
            &[
                ResourceKind::FileWriter,
                ResourceKind::FileAppender,
                ResourceKind::Directory,
            ],
            operation,
            "expected file-writer, file-appender, or directory",
        )?;
        match (kind, payload) {
            (ResourceKind::FileWriter | ResourceKind::FileAppender, OwnedResource::File(file)) => {
                Ok(file.as_raw())
            }
            (ResourceKind::Directory, OwnedResource::Directory(directory)) => {
                Ok(directory.as_raw())
            }
            _ => Err(Error::msg(format!(
                "{operation}: invalid syncable resource payload"
            ))),
        }
    }

    pub(super) fn owned_payload_for(
        &self,
        handle: Value,
        allowed: &[ResourceKind],
        operation: &str,
        expected: &str,
    ) -> Result<(ResourceKind, &OwnedResource)> {
        let (key, kind, provider) = self.resolve_owned_any(handle, operation)?;
        if !allowed.contains(&kind) {
            return Err(Error::msg(format!("{operation}: {expected}")));
        }
        let payload = self
            .table
            .owned(&key, kind, provider, self.table.scope())
            .map_err(|error| self.access_error(operation, error))?;
        Ok((kind, payload))
    }

    pub(super) fn owned_exact_payload(
        &self,
        handle: Value,
        kind: ResourceKind,
        provider: ProviderId,
        operation: &str,
    ) -> Result<&OwnedResource> {
        let key =
            self.resolve_exact(handle, kind, provider, ResourceOwnership::Owned, operation)?;
        self.table
            .owned(&key, kind, provider, self.table.scope())
            .map_err(|error| self.access_error(operation, error))
    }
}
