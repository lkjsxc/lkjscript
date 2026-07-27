use super::*;

impl ResourceTable {
    pub fn sqlite_column_count(&self, handle: Value) -> Result<i64> {
        Ok(self
            .sqlite_statement(handle, "sqlite-column-count")?
            .column_count())
    }

    pub fn sqlite_column_type(&self, handle: Value, index: i64) -> Result<i64> {
        let value = match self
            .sqlite_statement(handle, "sqlite-column-type")?
            .column_type(index)
            .map_err(|error| Error::msg(format!("sys-sqlite-column-type: {error}")))?
        {
            lkjscript_sys::ColumnType::Integer => 1,
            lkjscript_sys::ColumnType::Float => 2,
            lkjscript_sys::ColumnType::Text => 3,
            lkjscript_sys::ColumnType::Blob => 4,
            lkjscript_sys::ColumnType::Null => 5,
        };
        Ok(value)
    }

    pub fn sqlite_column_i64(&self, handle: Value, index: i64) -> Result<Option<i64>> {
        self.sqlite_statement(handle, "sqlite-column-i64")?
            .column_i64(index)
            .map_err(|error| Error::msg(format!("sys-sqlite-column-i64: {error}")))
    }

    pub fn sqlite_column_f64(&self, handle: Value, index: i64) -> Result<Option<f64>> {
        self.sqlite_statement(handle, "sqlite-column-f64")?
            .column_f64(index)
            .map_err(|error| Error::msg(format!("sys-sqlite-column-f64: {error}")))
    }

    pub fn sqlite_column_text(
        &self,
        handle: Value,
        index: i64,
        max: usize,
    ) -> Result<Option<String>> {
        self.sqlite_statement(handle, "sqlite-column-string")?
            .column_text(index, max)
            .map_err(|error| Error::msg(format!("sys-sqlite-column-text: {error}")))
    }

    pub fn sqlite_column_bytes(
        &self,
        handle: Value,
        index: i64,
        max: usize,
    ) -> Result<Option<Vec<u8>>> {
        self.sqlite_statement(handle, "sqlite-column-bytes")?
            .column_bytes(index, max)
            .map_err(|error| Error::msg(format!("sys-sqlite-column-bytes: {error}")))
    }

    pub fn sqlite_changes(&self, handle: Value) -> Result<i64> {
        Ok(self
            .sqlite_connection(handle, "sqlite-change-count")?
            .changes())
    }

    pub fn sqlite_last_insert_rowid(&self, handle: Value) -> Result<i64> {
        Ok(self
            .sqlite_connection(handle, "sqlite-last-insert-rowid")?
            .last_insert_rowid())
    }

    pub fn sqlite_extended_result_code(&self, handle: Value) -> Result<i64> {
        Ok(self
            .sqlite_connection(handle, "sqlite-extended-result-code")?
            .extended_result_code())
    }

    pub fn sqlite_backup(&self, handle: Value, path: &[u8], flags: i64) -> Result<Value> {
        self.sqlite_connection(handle, "backup-sqlite")?
            .backup_to(path, flags)
            .map_err(|error| Error::msg(format!("sys-sqlite-backup: {error}")))?;
        Ok(Value::UNIT)
    }
}
