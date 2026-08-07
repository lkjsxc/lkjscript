#![allow(unsafe_code)]

use super::*;

impl Statement {
    pub fn reset(&self) -> Result<(), SqliteError> {
        self.check("reset", unsafe { sqlite3_reset(self.raw.as_ptr()) })
    }

    pub fn clear_bindings(&self) -> Result<(), SqliteError> {
        self.check("clear bindings", unsafe {
            sqlite3_clear_bindings(self.raw.as_ptr())
        })
    }

    pub fn bind_null(&self, index: i64) -> Result<(), SqliteError> {
        self.check("bind null", unsafe {
            sqlite3_bind_null(self.raw.as_ptr(), bind_index(index)?)
        })
    }

    pub fn bind_i64(&self, index: i64, value: i64) -> Result<(), SqliteError> {
        self.check("bind I64", unsafe {
            sqlite3_bind_int64(self.raw.as_ptr(), bind_index(index)?, value)
        })
    }

    pub fn bind_f64(&self, index: i64, value: f64) -> Result<(), SqliteError> {
        self.check("bind F64", unsafe {
            sqlite3_bind_double(self.raw.as_ptr(), bind_index(index)?, value)
        })
    }

    pub fn bind_text(&self, index: i64, value: &str) -> Result<(), SqliteError> {
        let bytes = c_int::try_from(value.len()).map_err(|_| SqliteError::new("bind text", -1))?;
        self.check("bind text", unsafe {
            sqlite3_bind_text(
                self.raw.as_ptr(),
                bind_index(index)?,
                value.as_ptr().cast(),
                bytes,
                transient(),
            )
        })
    }

    pub fn bind_bytes(&self, index: i64, value: &[u8]) -> Result<(), SqliteError> {
        let bytes = c_int::try_from(value.len()).map_err(|_| SqliteError::new("bind bytes", -1))?;
        self.check("bind bytes", unsafe {
            sqlite3_bind_blob(
                self.raw.as_ptr(),
                bind_index(index)?,
                value.as_ptr().cast(),
                bytes,
                transient(),
            )
        })
    }

    pub fn step(&self) -> Result<Step, SqliteError> {
        match unsafe { sqlite3_step(self.raw.as_ptr()) } {
            SQLITE_ROW => Ok(Step::Row),
            SQLITE_DONE => Ok(Step::Done),
            code => Err(SqliteError::new("step", code)),
        }
    }

    pub fn column_count(&self) -> i64 {
        i64::from(unsafe { sqlite3_column_count(self.raw.as_ptr()) })
    }

    pub fn column_type(&self, index: i64) -> Result<ColumnType, SqliteError> {
        match unsafe { sqlite3_column_type(self.raw.as_ptr(), column_index(index)?) } {
            SQLITE_INTEGER => Ok(ColumnType::Integer),
            SQLITE_FLOAT => Ok(ColumnType::Float),
            SQLITE_TEXT => Ok(ColumnType::Text),
            SQLITE_BLOB => Ok(ColumnType::Blob),
            SQLITE_NULL => Ok(ColumnType::Null),
            code => Err(SqliteError::new("column type", code)),
        }
    }

    pub fn column_i64(&self, index: i64) -> Result<Option<i64>, SqliteError> {
        match self.column_type(index)? {
            ColumnType::Null => Ok(None),
            ColumnType::Integer => Ok(Some(unsafe {
                sqlite3_column_int64(self.raw.as_ptr(), column_index(index)?)
            })),
            _ => Err(SqliteError::new("column I64 type", -1)),
        }
    }

    pub fn column_f64(&self, index: i64) -> Result<Option<f64>, SqliteError> {
        match self.column_type(index)? {
            ColumnType::Null => Ok(None),
            ColumnType::Float => Ok(Some(unsafe {
                sqlite3_column_double(self.raw.as_ptr(), column_index(index)?)
            })),
            _ => Err(SqliteError::new("column F64 type", -1)),
        }
    }

    pub fn column_text(&self, index: i64, max_bytes: usize) -> Result<Option<String>, SqliteError> {
        if self.column_type(index)? == ColumnType::Null {
            return Ok(None);
        }
        if self.column_type(index)? != ColumnType::Text {
            return Err(SqliteError::new("column text type", -1));
        }
        let bytes = column_text_bytes(self.raw, index, max_bytes)?;
        let text =
            String::from_utf8(bytes).map_err(|_| SqliteError::new("column text UTF-8", -1))?;
        Ok(Some(text))
    }

    pub fn column_bytes(
        &self,
        index: i64,
        max_bytes: usize,
    ) -> Result<Option<Vec<u8>>, SqliteError> {
        if self.column_type(index)? == ColumnType::Null {
            return Ok(None);
        }
        if self.column_type(index)? != ColumnType::Blob {
            return Err(SqliteError::new("column bytes type", -1));
        }
        Ok(Some(column_bytes(self.raw, index, max_bytes)?))
    }

    fn check(&self, operation: &'static str, code: c_int) -> Result<(), SqliteError> {
        if code == SQLITE_OK {
            Ok(())
        } else {
            Err(SqliteError::new(operation, unsafe {
                sqlite3_extended_errcode(self.database.as_ptr())
            }))
        }
    }
}
