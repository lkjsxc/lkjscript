//! Bounded owned SQLite C API boundary for generic language operations.

use std::ffi::{c_char, c_int, c_void, CString};
use std::fmt;
use std::ptr::{self, NonNull};

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_INTEGER: c_int = 1;
const SQLITE_FLOAT: c_int = 2;
const SQLITE_TEXT: c_int = 3;
const SQLITE_BLOB: c_int = 4;
const SQLITE_NULL: c_int = 5;
const SQLITE_OPEN_READONLY: c_int = 0x0000_0001;
const SQLITE_OPEN_READWRITE: c_int = 0x0000_0002;
const SQLITE_OPEN_CREATE: c_int = 0x0000_0004;
const SQLITE_OPEN_URI: c_int = 0x0000_0040;
const SQLITE_OPEN_MEMORY: c_int = 0x0000_0080;
const SQLITE_OPEN_FULLMUTEX: c_int = 0x0001_0000;
const SQLITE_OPEN_NOFOLLOW: c_int = 0x0100_0000;
const SQLITE_OPEN_ALLOWED: c_int = SQLITE_OPEN_READONLY
    | SQLITE_OPEN_READWRITE
    | SQLITE_OPEN_CREATE
    | SQLITE_OPEN_URI
    | SQLITE_OPEN_MEMORY
    | SQLITE_OPEN_FULLMUTEX
    | SQLITE_OPEN_NOFOLLOW;
const SQLITE_TRANSIENT: isize = -1;

#[repr(C)]
struct Sqlite3 {
    _opaque: [u8; 0],
}

#[repr(C)]
struct Sqlite3Stmt {
    _opaque: [u8; 0],
}

#[repr(C)]
struct Sqlite3Backup {
    _opaque: [u8; 0],
}

type Destructor = Option<unsafe extern "C" fn(*mut c_void)>;

#[link(name = ":libsqlite3.so.0", kind = "dylib")]
extern "C" {
    fn sqlite3_open_v2(
        filename: *const c_char,
        database: *mut *mut Sqlite3,
        flags: c_int,
        vfs: *const c_char,
    ) -> c_int;
    fn sqlite3_close_v2(database: *mut Sqlite3) -> c_int;
    fn sqlite3_busy_timeout(database: *mut Sqlite3, milliseconds: c_int) -> c_int;
    fn sqlite3_prepare_v2(
        database: *mut Sqlite3,
        sql: *const c_char,
        bytes: c_int,
        statement: *mut *mut Sqlite3Stmt,
        tail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_finalize(statement: *mut Sqlite3Stmt) -> c_int;
    fn sqlite3_reset(statement: *mut Sqlite3Stmt) -> c_int;
    fn sqlite3_clear_bindings(statement: *mut Sqlite3Stmt) -> c_int;
    fn sqlite3_bind_null(statement: *mut Sqlite3Stmt, index: c_int) -> c_int;
    fn sqlite3_bind_int64(statement: *mut Sqlite3Stmt, index: c_int, value: i64) -> c_int;
    fn sqlite3_bind_double(statement: *mut Sqlite3Stmt, index: c_int, value: f64) -> c_int;
    fn sqlite3_bind_text(
        statement: *mut Sqlite3Stmt,
        index: c_int,
        value: *const c_char,
        bytes: c_int,
        destructor: Destructor,
    ) -> c_int;
    fn sqlite3_bind_blob(
        statement: *mut Sqlite3Stmt,
        index: c_int,
        value: *const c_void,
        bytes: c_int,
        destructor: Destructor,
    ) -> c_int;
    fn sqlite3_step(statement: *mut Sqlite3Stmt) -> c_int;
    fn sqlite3_column_count(statement: *mut Sqlite3Stmt) -> c_int;
    fn sqlite3_column_type(statement: *mut Sqlite3Stmt, index: c_int) -> c_int;
    fn sqlite3_column_int64(statement: *mut Sqlite3Stmt, index: c_int) -> i64;
    fn sqlite3_column_double(statement: *mut Sqlite3Stmt, index: c_int) -> f64;
    fn sqlite3_column_text(statement: *mut Sqlite3Stmt, index: c_int) -> *const u8;
    fn sqlite3_column_blob(statement: *mut Sqlite3Stmt, index: c_int) -> *const c_void;
    fn sqlite3_column_bytes(statement: *mut Sqlite3Stmt, index: c_int) -> c_int;
    fn sqlite3_changes(database: *mut Sqlite3) -> c_int;
    fn sqlite3_last_insert_rowid(database: *mut Sqlite3) -> i64;
    fn sqlite3_extended_errcode(database: *mut Sqlite3) -> c_int;
    fn sqlite3_exec(
        database: *mut Sqlite3,
        sql: *const c_char,
        callback: Option<unsafe extern "C" fn()>,
        context: *mut c_void,
        error: *mut *mut c_char,
    ) -> c_int;
    fn sqlite3_free(value: *mut c_void);
    fn sqlite3_backup_init(
        destination: *mut Sqlite3,
        destination_name: *const c_char,
        source: *mut Sqlite3,
        source_name: *const c_char,
    ) -> *mut Sqlite3Backup;
    fn sqlite3_backup_step(backup: *mut Sqlite3Backup, pages: c_int) -> c_int;
    fn sqlite3_backup_finish(backup: *mut Sqlite3Backup) -> c_int;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqliteError {
    operation: &'static str,
    code: c_int,
}

impl SqliteError {
    const fn new(operation: &'static str, code: c_int) -> Self {
        Self { operation, code }
    }

    pub const fn code(self) -> i32 {
        self.code
    }
}

impl fmt::Display for SqliteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "sqlite {} failed with result code {}",
            self.operation, self.code
        )
    }
}

impl std::error::Error for SqliteError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Row,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Integer,
    Float,
    Text,
    Blob,
    Null,
}

pub struct Connection {
    raw: NonNull<Sqlite3>,
}

impl Connection {
    pub fn open(path: &str, flags: i64) -> Result<Self, SqliteError> {
        let flags = checked_open_flags(flags)?;
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

    pub fn backup_to(&self, destination: &str, flags: i64) -> Result<(), SqliteError> {
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

pub struct Statement {
    raw: NonNull<Sqlite3Stmt>,
    database: NonNull<Sqlite3>,
}

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
            std::str::from_utf8(&bytes).map_err(|_| SqliteError::new("column text UTF-8", -1))?;
        Ok(Some(text.to_owned()))
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

impl Drop for Statement {
    fn drop(&mut self) {
        unsafe {
            let _ = sqlite3_finalize(self.raw.as_ptr());
        }
    }
}

fn checked_open_flags(flags: i64) -> Result<c_int, SqliteError> {
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

fn bind_index(index: i64) -> Result<c_int, SqliteError> {
    let index = c_int::try_from(index).map_err(|_| SqliteError::new("bind index", -1))?;
    if index < 1 {
        return Err(SqliteError::new("bind index", -1));
    }
    Ok(index)
}

fn column_index(index: i64) -> Result<c_int, SqliteError> {
    let index = c_int::try_from(index).map_err(|_| SqliteError::new("column index", -1))?;
    if index < 0 {
        return Err(SqliteError::new("column index", -1));
    }
    Ok(index)
}

fn transient() -> Destructor {
    unsafe { std::mem::transmute(SQLITE_TRANSIENT) }
}

fn column_text_bytes(
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

fn column_bytes(
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{Connection, Step};

    const OPEN_RW_CREATE_FULLMUTEX: i64 = 0x0001_0006;

    #[test]
    fn memory_database_prepares_binds_and_reads_exact_values() {
        let connection = Connection::open(":memory:", OPEN_RW_CREATE_FULLMUTEX).expect("open");
        connection
            .exec("CREATE TABLE sample (number INTEGER, text TEXT, bytes BLOB)")
            .expect("schema");
        let insert = connection
            .prepare("INSERT INTO sample VALUES (?1, ?2, ?3)")
            .expect("prepare insert");
        insert.bind_i64(1, i64::MIN).expect("integer");
        insert.bind_text(2, "日本語").expect("text");
        insert.bind_bytes(3, &[0, 255]).expect("bytes");
        assert_eq!(insert.step().expect("step"), Step::Done);
        let query = connection
            .prepare("SELECT number, text, bytes FROM sample")
            .expect("prepare query");
        assert_eq!(query.step().expect("row"), Step::Row);
        assert_eq!(query.column_i64(0).expect("number"), Some(i64::MIN));
        assert_eq!(
            query.column_text(1, 1_000).expect("text"),
            Some("日本語".into())
        );
        assert_eq!(
            query.column_bytes(2, 1_000).expect("bytes"),
            Some(vec![0, 255])
        );
        assert_eq!(query.step().expect("done"), Step::Done);
    }

    #[test]
    fn file_database_backup_restores_durable_rows() {
        let root = std::env::temp_dir().join(format!(
            "lkjscript-sqlite-{}-{}",
            std::process::id(),
            line!()
        ));
        let source_path = root.with_extension("source.sqlite");
        let backup_path = root.with_extension("backup.sqlite");
        let source = Connection::open(
            source_path.to_str().expect("UTF-8 source path"),
            OPEN_RW_CREATE_FULLMUTEX,
        )
        .expect("open source");
        source.busy_timeout(100).expect("busy timeout");
        source
            .exec("CREATE TABLE sample (number INTEGER); INSERT INTO sample VALUES (9)")
            .expect("write source");
        source
            .backup_to(
                backup_path.to_str().expect("UTF-8 backup path"),
                OPEN_RW_CREATE_FULLMUTEX,
            )
            .expect("backup");
        drop(source);
        let restored = Connection::open(
            backup_path.to_str().expect("UTF-8 backup path"),
            OPEN_RW_CREATE_FULLMUTEX,
        )
        .expect("open backup");
        let query = restored
            .prepare("SELECT number FROM sample")
            .expect("prepare restored query");
        assert_eq!(query.step().expect("restored row"), Step::Row);
        assert_eq!(query.column_i64(0).expect("restored number"), Some(9));
        drop(query);
        drop(restored);
        let _ = std::fs::remove_file(source_path);
        let _ = std::fs::remove_file(backup_path);
    }
}
