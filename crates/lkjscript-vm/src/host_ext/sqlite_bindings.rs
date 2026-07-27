use super::*;

impl ResourceTable {
    pub fn sqlite_reset(&self, handle: Value) -> Result<Value> {
        self.sqlite_statement(handle, "reset-sqlite-statement")?
            .reset()
            .map_err(|error| Error::msg(format!("sys-sqlite-reset: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sqlite_clear_bindings(&self, handle: Value) -> Result<Value> {
        self.sqlite_statement(handle, "clear-sqlite-bindings")?
            .clear_bindings()
            .map_err(|error| Error::msg(format!("sys-sqlite-clear-bindings: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sqlite_bind_null(&self, handle: Value, index: i64) -> Result<Value> {
        self.sqlite_statement(handle, "bind-sqlite-null")?
            .bind_null(index)
            .map_err(|error| Error::msg(format!("sys-sqlite-bind-null: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sqlite_bind_i64(&self, handle: Value, index: i64, value: i64) -> Result<Value> {
        self.sqlite_statement(handle, "bind-sqlite-i64")?
            .bind_i64(index, value)
            .map_err(|error| Error::msg(format!("sys-sqlite-bind-i64: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sqlite_bind_f64(&self, handle: Value, index: i64, value: f64) -> Result<Value> {
        self.sqlite_statement(handle, "bind-sqlite-f64")?
            .bind_f64(index, value)
            .map_err(|error| Error::msg(format!("sys-sqlite-bind-f64: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sqlite_bind_text(&self, handle: Value, index: i64, value: &str) -> Result<Value> {
        self.sqlite_statement(handle, "bind-sqlite-string")?
            .bind_text(index, value)
            .map_err(|error| Error::msg(format!("sys-sqlite-bind-text: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sqlite_bind_bytes(&self, handle: Value, index: i64, value: &[u8]) -> Result<Value> {
        self.sqlite_statement(handle, "bind-sqlite-bytes")?
            .bind_bytes(index, value)
            .map_err(|error| Error::msg(format!("sys-sqlite-bind-bytes: {error}")))?;
        Ok(Value::UNIT)
    }

    pub fn sqlite_step(&self, handle: Value) -> Result<i64> {
        match self
            .sqlite_statement(handle, "step-sqlite")?
            .step()
            .map_err(|error| Error::msg(format!("sys-sqlite-step: {error}")))?
        {
            lkjscript_sys::SqliteStep::Row => Ok(100),
            lkjscript_sys::SqliteStep::Done => Ok(101),
        }
    }
}
