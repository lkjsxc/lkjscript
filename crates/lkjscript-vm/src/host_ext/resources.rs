use super::*;

impl ResourceTable {
    pub(crate) fn raw_fd(&self, handle: Value, operation: &str) -> Result<RawFd> {
        if handle.as_handle() == Some(STDIN_TOKEN) {
            return Ok(lkjscript_sys::STDIN_FD);
        }
        let index = self.owned_index(handle, operation)?;
        match self.slots.get(index).and_then(Option::as_ref) {
            Some(OwnedResource::File(file)) => Ok(file.as_raw()),
            Some(OwnedResource::Directory(directory)) => Ok(directory.as_raw()),
            Some(OwnedResource::Socket(socket)) => Ok(socket.as_raw()),
            Some(OwnedResource::SqliteConnection { .. })
            | Some(OwnedResource::SqliteStatement { .. }) => Err(Error::msg(format!(
                "{operation}: handle is not a file, directory, or socket"
            ))),
            None => Err(Error::msg(format!("{operation}: stale or unknown handle"))),
        }
    }

    pub(crate) fn close_slot(&mut self, index: usize, operation: &str) -> Result<Value> {
        let slot = self
            .slots
            .get_mut(index)
            .ok_or_else(|| Error::msg(format!("{operation}: unknown handle")))?;
        if slot.take().is_none() {
            return Err(Error::msg(format!(
                "{operation}: stale or already closed handle"
            )));
        }
        Ok(Value::UNIT)
    }

    pub(crate) fn sqlite_connection(
        &self,
        handle: Value,
        operation: &str,
    ) -> Result<&lkjscript_sys::SqliteConnection> {
        let index = self.owned_index(handle, operation)?;
        self.sqlite_connection_at(index, operation)
    }

    pub(crate) fn sqlite_connection_at(
        &self,
        index: usize,
        operation: &str,
    ) -> Result<&lkjscript_sys::SqliteConnection> {
        match self.slots.get(index).and_then(Option::as_ref) {
            Some(OwnedResource::SqliteConnection { connection, .. }) => Ok(connection),
            Some(_) => Err(Error::msg(format!(
                "{operation}: handle is not a SQLite connection"
            ))),
            None => Err(Error::msg(format!("{operation}: stale or unknown handle"))),
        }
    }

    pub(crate) fn sqlite_live_statements_at_mut(
        &mut self,
        index: usize,
        operation: &str,
    ) -> Result<&mut usize> {
        match self.slots.get_mut(index).and_then(Option::as_mut) {
            Some(OwnedResource::SqliteConnection {
                live_statements, ..
            }) => Ok(live_statements),
            Some(_) => Err(Error::msg(format!(
                "{operation}: handle is not a SQLite connection"
            ))),
            None => Err(Error::msg(format!("{operation}: stale or unknown handle"))),
        }
    }

    pub(crate) fn sqlite_statement(
        &self,
        handle: Value,
        operation: &str,
    ) -> Result<&lkjscript_sys::SqliteStatement> {
        let index = self.owned_index(handle, operation)?;
        match self.slots.get(index).and_then(Option::as_ref) {
            Some(OwnedResource::SqliteStatement { statement, parent }) => {
                if !matches!(
                    self.slots.get(*parent),
                    Some(Some(OwnedResource::SqliteConnection { .. }))
                ) {
                    return Err(Error::msg(format!(
                        "{operation}: parent SQLite connection is closed"
                    )));
                }
                Ok(statement)
            }
            Some(_) => Err(Error::msg(format!(
                "{operation}: handle is not a SQLite statement"
            ))),
            None => Err(Error::msg(format!("{operation}: stale or unknown handle"))),
        }
    }

    pub(crate) fn socket_raw(&self, handle: Value, operation: &str) -> Result<RawFd> {
        let index = self.owned_index(handle, operation)?;
        match self.slots.get(index).and_then(Option::as_ref) {
            Some(OwnedResource::Socket(socket)) => Ok(socket.as_raw()),
            Some(OwnedResource::File(_))
            | Some(OwnedResource::Directory(_))
            | Some(OwnedResource::SqliteConnection { .. })
            | Some(OwnedResource::SqliteStatement { .. }) => {
                Err(Error::msg(format!("{operation}: handle is not a socket")))
            }
            None => Err(Error::msg(format!("{operation}: stale or unknown handle"))),
        }
    }

    pub(crate) fn file_raw(&self, handle: Value, operation: &str) -> Result<RawFd> {
        let index = self.owned_index(handle, operation)?;
        match self.slots.get(index).and_then(Option::as_ref) {
            Some(OwnedResource::File(file)) => Ok(file.as_raw()),
            Some(OwnedResource::Directory(_)) => {
                Err(Error::msg(format!("{operation}: handle is a directory")))
            }
            Some(OwnedResource::Socket(_))
            | Some(OwnedResource::SqliteConnection { .. })
            | Some(OwnedResource::SqliteStatement { .. }) => {
                Err(Error::msg(format!("{operation}: handle is not a file")))
            }
            None => Err(Error::msg(format!("{operation}: stale or unknown handle"))),
        }
    }

    pub(crate) fn sync_raw(&self, handle: Value, operation: &str) -> Result<RawFd> {
        let index = self.owned_index(handle, operation)?;
        match self.slots.get(index).and_then(Option::as_ref) {
            Some(OwnedResource::File(file)) => Ok(file.as_raw()),
            Some(OwnedResource::Directory(directory)) => Ok(directory.as_raw()),
            Some(OwnedResource::Socket(_))
            | Some(OwnedResource::SqliteConnection { .. })
            | Some(OwnedResource::SqliteStatement { .. }) => Err(Error::msg(format!(
                "{operation}: handle is not a file or directory"
            ))),
            None => Err(Error::msg(format!("{operation}: stale or unknown handle"))),
        }
    }

    pub(crate) fn owned_index(&self, handle: Value, operation: &str) -> Result<usize> {
        let token = handle
            .as_handle()
            .ok_or_else(|| Error::msg(format!("{operation}: expected Handle")))?;
        let index = token
            .checked_sub(FIRST_OWNED_TOKEN)
            .ok_or_else(|| Error::msg(format!("{operation}: borrowed or invalid handle")))?;
        usize::try_from(index).map_err(|_| Error::msg(format!("{operation}: invalid handle")))
    }

    pub(super) fn push(&mut self, handle: OwnedResource) -> Result<Value> {
        self.ensure_capacity()?;
        let index = u32::try_from(self.slots.len())
            .map_err(|_| Error::msg("resource handle table exhausted"))?;
        let token = FIRST_OWNED_TOKEN
            .checked_add(index)
            .ok_or_else(|| Error::msg("resource handle token exhausted"))?;
        self.slots.push(Some(handle));
        Ok(Value::from_handle(token))
    }

    pub(crate) fn ensure_capacity(&mut self) -> Result<()> {
        if self.slots.len() >= self.max_handles {
            self.limit_exceeded = true;
            Err(Error::msg("resource handle limit exceeded"))
        } else {
            Ok(())
        }
    }
}
