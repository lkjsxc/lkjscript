#![allow(unsafe_code)]

use super::*;

impl Connection {
    pub fn open(path: &[u8], flags: i64) -> Result<Self, SqliteError> {
        let flags = checked_open_flags(flags)?;
        if !crate::native_path::validate(path) {
            return Err(SqliteError::new("open path", -1));
        }
        let path = CString::new(path).map_err(|_| SqliteError::new("open path", -1))?;
        let mut raw = ptr::null_mut();
        let code = unsafe { sqlite3_open_v2(path.as_ptr(), &mut raw, flags, ptr::null()) };
        let Some(raw) = NonNull::new(raw) else {
            return Err(SqliteError::new("open", code));
        };
        if code != SQLITE_OK {
            unsafe {
                let _ = sqlite3_close_v2(raw.as_ptr());
            }
            return Err(SqliteError::new("open", code));
        }
        Ok(Self { raw })
    }

    pub fn busy_timeout(&self, milliseconds: i64) -> Result<(), SqliteError> {
        let milliseconds =
            c_int::try_from(milliseconds).map_err(|_| SqliteError::new("busy timeout", -1))?;
        if milliseconds < 0 {
            return Err(SqliteError::new("busy timeout", -1));
        }
        self.check("busy timeout", unsafe {
            sqlite3_busy_timeout(self.raw.as_ptr(), milliseconds)
        })
    }

    pub fn exec(&self, sql: &str) -> Result<(), SqliteError> {
        let sql = CString::new(sql).map_err(|_| SqliteError::new("exec SQL", -1))?;
        let mut message = ptr::null_mut();
        let code = unsafe {
            sqlite3_exec(
                self.raw.as_ptr(),
                sql.as_ptr(),
                None,
                ptr::null_mut(),
                &mut message,
            )
        };
        if !message.is_null() {
            unsafe { sqlite3_free(message.cast()) };
        }
        self.check("exec", code)
    }

    pub fn prepare(&self, sql: &str) -> Result<Statement, SqliteError> {
        let sql = CString::new(sql).map_err(|_| SqliteError::new("prepare SQL", -1))?;
        let bytes = c_int::try_from(sql.as_bytes().len())
            .map_err(|_| SqliteError::new("prepare SQL", -1))?;
        let mut raw = ptr::null_mut();
        let code = unsafe {
            sqlite3_prepare_v2(
                self.raw.as_ptr(),
                sql.as_ptr(),
                bytes,
                &mut raw,
                ptr::null_mut(),
            )
        };
        self.check("prepare", code)?;
        let raw = NonNull::new(raw).ok_or(SqliteError::new("prepare", -1))?;
        Ok(Statement {
            raw,
            database: self.raw,
        })
    }

    pub fn changes(&self) -> i64 {
        i64::from(unsafe { sqlite3_changes(self.raw.as_ptr()) })
    }

    pub fn last_insert_rowid(&self) -> i64 {
        unsafe { sqlite3_last_insert_rowid(self.raw.as_ptr()) }
    }

    pub fn extended_result_code(&self) -> i64 {
        i64::from(unsafe { sqlite3_extended_errcode(self.raw.as_ptr()) })
    }

    pub fn backup_to(&self, destination: &[u8], flags: i64) -> Result<(), SqliteError> {
        let destination = Self::open(destination, flags)?;
        let main = c"main";
        let backup = unsafe {
            sqlite3_backup_init(
                destination.raw.as_ptr(),
                main.as_ptr(),
                self.raw.as_ptr(),
                main.as_ptr(),
            )
        };
        let backup = NonNull::new(backup).ok_or_else(|| destination.error("backup init"))?;
        let step = unsafe { sqlite3_backup_step(backup.as_ptr(), -1) };
        let finish = unsafe { sqlite3_backup_finish(backup.as_ptr()) };
        if step != SQLITE_DONE {
            return Err(destination.error("backup step"));
        }
        destination.check("backup finish", finish)
    }

    fn error(&self, operation: &'static str) -> SqliteError {
        SqliteError::new(operation, unsafe {
            sqlite3_extended_errcode(self.raw.as_ptr())
        })
    }

    fn check(&self, operation: &'static str, code: c_int) -> Result<(), SqliteError> {
        if code == SQLITE_OK {
            Ok(())
        } else {
            Err(SqliteError::new(operation, code))
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe {
            let _ = sqlite3_close_v2(self.raw.as_ptr());
        }
    }
}
