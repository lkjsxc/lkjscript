use super::*;

impl ResourceTable {
    pub fn sqlite_open(&mut self, path: &[u8], flags: i64) -> Result<Value> {
        self.acquire_owned(
            ResourceKind::SqliteConnection,
            SQLITE_PROVIDER,
            "open-sqlite",
            || {
                lkjscript_sys::SqliteConnection::open(path, flags)
                    .map(OwnedResource::SqliteConnection)
                    .map_err(|error| Error::msg(format!("sys-sqlite-open: {error}")))
            },
        )
    }

    pub fn sqlite_close(&mut self, handle: Value) -> Result<Value> {
        let key = self.resolve_exact(
            handle,
            ResourceKind::SqliteConnection,
            SQLITE_PROVIDER,
            ResourceOwnership::Owned,
            "close-sqlite",
        )?;
        self.close_owned_key(
            key,
            ResourceKind::SqliteConnection,
            SQLITE_PROVIDER,
            "close-sqlite",
        )
    }

    pub fn sqlite_busy_timeout(&self, handle: Value, milliseconds: i64) -> Result<Value> {
        self.sqlite_connection(handle, "set-sqlite-busy-timeout")?
            .busy_timeout(milliseconds)
            .map_err(|error| Error::msg(format!("sys-sqlite-busy-timeout: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sqlite_exec(&self, handle: Value, sql: &str) -> Result<Value> {
        self.sqlite_connection(handle, "execute-sqlite")?
            .exec(sql)
            .map_err(|error| Error::msg(format!("sys-sqlite-exec: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sqlite_prepare(&mut self, handle: Value, sql: &str) -> Result<Value> {
        let parent = self.resolve_exact(
            handle,
            ResourceKind::SqliteConnection,
            SQLITE_PROVIDER,
            ResourceOwnership::Owned,
            "prepare-sqlite",
        )?;
        let reservation = self.reserve_owned_child(
            &parent,
            ResourceKind::SqliteConnection,
            ResourceKind::SqliteStatement,
            SQLITE_PROVIDER,
            "prepare-sqlite",
        )?;
        let parent_payload = reservation
            .parent_payload()
            .map_err(|error| Error::msg(format!("prepare-sqlite: {error}")))?;
        let connection = match parent_payload {
            Some(OwnedResource::SqliteConnection(connection)) => connection,
            _ => return Err(Error::msg("prepare-sqlite: invalid parent payload")),
        };
        let statement = connection
            .prepare(sql)
            .map_err(|error| Error::msg(format!("sys-sqlite-prepare: {error}")))?;
        let key = reservation.commit(OwnedResource::SqliteStatement(statement));
        self.publish_owned(key, ResourceKind::SqliteStatement, SQLITE_PROVIDER)
    }

    pub fn sqlite_finalize(&mut self, handle: Value) -> Result<Value> {
        let key = self.resolve_exact(
            handle,
            ResourceKind::SqliteStatement,
            SQLITE_PROVIDER,
            ResourceOwnership::Owned,
            "finalize-sqlite-statement",
        )?;
        self.close_owned_key(
            key,
            ResourceKind::SqliteStatement,
            SQLITE_PROVIDER,
            "finalize-sqlite-statement",
        )
    }
}
