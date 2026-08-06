#![allow(unsafe_code)]

use super::*;

impl Drop for Statement {
    fn drop(&mut self) {
        unsafe {
            let _ = sqlite3_finalize(self.raw.as_ptr());
        }
    }
}

pub(super) fn checked_open_flags(flags: i64) -> Result<c_int, SqliteError> {
    let flags = c_int::try_from(flags).map_err(|_| SqliteError::new("open flags", -1))?;
    if flags & !SQLITE_OPEN_ALLOWED != 0 {
        return Err(SqliteError::new("open flags", -1));
    }
    let modes = flags & (SQLITE_OPEN_READONLY | SQLITE_OPEN_READWRITE);
    if modes != SQLITE_OPEN_READONLY && modes != SQLITE_OPEN_READWRITE {
        return Err(SqliteError::new("open flags", -1));
    }
    if flags & SQLITE_OPEN_CREATE != 0 && modes != SQLITE_OPEN_READWRITE {
        return Err(SqliteError::new("open flags", -1));
    }
    Ok(flags)
}

pub(super) fn bind_index(index: i64) -> Result<c_int, SqliteError> {
    let index = c_int::try_from(index).map_err(|_| SqliteError::new("bind index", -1))?;
    if index < 1 {
        return Err(SqliteError::new("bind index", -1));
    }
    Ok(index)
}

pub(super) fn column_index(index: i64) -> Result<c_int, SqliteError> {
    let index = c_int::try_from(index).map_err(|_| SqliteError::new("column index", -1))?;
    if index < 0 {
        return Err(SqliteError::new("column index", -1));
    }
    Ok(index)
}

pub(super) fn transient() -> Destructor {
    unsafe { std::mem::transmute(SQLITE_TRANSIENT) }
}

pub(super) fn column_text_bytes(
    statement: NonNull<Sqlite3Stmt>,
    index: i64,
    max_bytes: usize,
) -> Result<Vec<u8>, SqliteError> {
    let index = column_index(index)?;
    let length = unsafe { sqlite3_column_bytes(statement.as_ptr(), index) };
    let length = usize::try_from(length).map_err(|_| SqliteError::new("column text", -1))?;
    if length > max_bytes {
        return Err(SqliteError::new("column text limit", -1));
    }
    let pointer = unsafe { sqlite3_column_text(statement.as_ptr(), index) };
    if pointer.is_null() && length != 0 {
        return Err(SqliteError::new("column text", -1));
    }
    if length == 0 {
        return Ok(Vec::new());
    }
    let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
    Ok(bytes.to_vec())
}

pub(super) fn column_bytes(
    statement: NonNull<Sqlite3Stmt>,
    index: i64,
    max_bytes: usize,
) -> Result<Vec<u8>, SqliteError> {
    let index = column_index(index)?;
    let length = unsafe { sqlite3_column_bytes(statement.as_ptr(), index) };
    let length = usize::try_from(length).map_err(|_| SqliteError::new("column bytes", -1))?;
    if length > max_bytes {
        return Err(SqliteError::new("column bytes limit", -1));
    }
    let pointer = unsafe { sqlite3_column_blob(statement.as_ptr(), index) };
    if pointer.is_null() && length != 0 {
        return Err(SqliteError::new("column bytes", -1));
    }
    if length == 0 {
        return Ok(Vec::new());
    }
    let bytes = unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), length) };
    Ok(bytes.to_vec())
}
