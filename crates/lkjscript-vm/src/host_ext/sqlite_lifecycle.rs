use super::*;

impl ResourceTable {
    pub fn close(&mut self, handle: Value) -> Result<Value> {
        let index = self.owned_index(handle, "sys-close")?;
        match self.slots.get(index).and_then(Option::as_ref) {
            Some(OwnedResource::SqliteConnection { .. })
            | Some(OwnedResource::SqliteStatement { .. }) => Err(Error::msg(
                "sys-close: SQLite handles require their SQLite close operation",
            )),
            Some(_) => self.close_slot(index, "sys-close"),
            None => Err(Error::msg("sys-close: stale or already closed handle")),
        }
    }

    pub fn sqlite_open(&mut self, path: &str, flags: i64) -> Result<Value> {
        self.ensure_capacity()?;
        let connection = lkjscript_sys::SqliteConnection::open(path, flags)
            .map_err(|error| Error::msg(format!("sys-sqlite-open: {error}")))?;
        self.push(OwnedResource::SqliteConnection {
            connection,
            live_statements: 0,
        })
    }

    pub fn sqlite_close(&mut self, handle: Value) -> Result<Value> {
        let index = self.owned_index(handle, "sys-sqlite-close")?;
        match self.slots.get(index).and_then(Option::as_ref) {
            Some(OwnedResource::SqliteConnection {
                live_statements, ..
            }) if *live_statements > 0 => Err(Error::msg(
                "sys-sqlite-close: live statements must be finalized",
            )),
            Some(OwnedResource::SqliteConnection { .. }) => {
                self.close_slot(index, "sys-sqlite-close")
            }
            Some(_) => Err(Error::msg(
                "sys-sqlite-close: handle is not a SQLite connection",
            )),
            None => Err(Error::msg("sys-sqlite-close: stale or unknown handle")),
        }
    }

    pub fn sqlite_busy_timeout(&self, handle: Value, milliseconds: i64) -> Result<Value> {
        self.sqlite_connection(handle, "sys-sqlite-busy-timeout")?
            .busy_timeout(milliseconds)
            .map_err(|error| Error::msg(format!("sys-sqlite-busy-timeout: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sqlite_exec(&self, handle: Value, sql: &str) -> Result<Value> {
        self.sqlite_connection(handle, "sys-sqlite-exec")?
            .exec(sql)
            .map_err(|error| Error::msg(format!("sys-sqlite-exec: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sqlite_prepare(&mut self, handle: Value, sql: &str) -> Result<Value> {
        self.ensure_capacity()?;
        let parent = self.owned_index(handle, "sys-sqlite-prepare")?;
        let statement = self
            .sqlite_connection_at(parent, "sys-sqlite-prepare")?
            .prepare(sql)
            .map_err(|error| Error::msg(format!("sys-sqlite-prepare: {error}")))?;
        let live_statements = self.sqlite_live_statements_at_mut(parent, "sys-sqlite-prepare")?;
        *live_statements = live_statements.saturating_add(1);
        self.push(OwnedResource::SqliteStatement { statement, parent })
    }

    pub fn sqlite_finalize(&mut self, handle: Value) -> Result<Value> {
        let index = self.owned_index(handle, "sys-sqlite-finalize")?;
        let parent = match self.slots.get(index).and_then(Option::as_ref) {
            Some(OwnedResource::SqliteStatement { parent, .. }) => *parent,
            Some(_) => {
                return Err(Error::msg(
                    "sys-sqlite-finalize: handle is not a SQLite statement",
                ))
            }
            None => return Err(Error::msg("sys-sqlite-finalize: stale or unknown handle")),
        };
        let _statement = self
            .slots
            .get_mut(index)
            .and_then(Option::take)
            .ok_or_else(|| Error::msg("sys-sqlite-finalize: stale or unknown handle"))?;
        let live_statements = self.sqlite_live_statements_at_mut(parent, "sys-sqlite-finalize")?;
        *live_statements = live_statements.saturating_sub(1);
        Ok(Value::UNIT)
    }
}
