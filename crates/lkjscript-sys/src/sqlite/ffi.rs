use super::*;

pub(super) const SQLITE_OK: c_int = 0;
pub(super) const SQLITE_ROW: c_int = 100;
pub(super) const SQLITE_DONE: c_int = 101;
pub(super) const SQLITE_INTEGER: c_int = 1;
pub(super) const SQLITE_FLOAT: c_int = 2;
pub(super) const SQLITE_TEXT: c_int = 3;
pub(super) const SQLITE_BLOB: c_int = 4;
pub(super) const SQLITE_NULL: c_int = 5;
pub(super) const SQLITE_OPEN_READONLY: c_int = 0x0000_0001;
pub(super) const SQLITE_OPEN_READWRITE: c_int = 0x0000_0002;
pub(super) const SQLITE_OPEN_CREATE: c_int = 0x0000_0004;
pub(super) const SQLITE_OPEN_URI: c_int = 0x0000_0040;
pub(super) const SQLITE_OPEN_MEMORY: c_int = 0x0000_0080;
pub(super) const SQLITE_OPEN_FULLMUTEX: c_int = 0x0001_0000;
pub(super) const SQLITE_OPEN_NOFOLLOW: c_int = 0x0100_0000;
pub(super) const SQLITE_OPEN_ALLOWED: c_int = SQLITE_OPEN_READONLY
    | SQLITE_OPEN_READWRITE
    | SQLITE_OPEN_CREATE
    | SQLITE_OPEN_URI
    | SQLITE_OPEN_MEMORY
    | SQLITE_OPEN_FULLMUTEX
    | SQLITE_OPEN_NOFOLLOW;
pub(super) const SQLITE_TRANSIENT: isize = -1;

#[repr(C)]
pub(super) struct Sqlite3 {
    _opaque: [u8; 0],
}

#[repr(C)]
pub(super) struct Sqlite3Stmt {
    _opaque: [u8; 0],
}

#[repr(C)]
pub(super) struct Sqlite3Backup {
    _opaque: [u8; 0],
}

pub(super) type Destructor = Option<unsafe extern "C" fn(*mut c_void)>;

#[link(name = ":libsqlite3.so.0", kind = "dylib")]
extern "C" {
    pub(super) fn sqlite3_open_v2(
        filename: *const c_char,
        database: *mut *mut Sqlite3,
        flags: c_int,
        vfs: *const c_char,
    ) -> c_int;
    pub(super) fn sqlite3_close_v2(database: *mut Sqlite3) -> c_int;
    pub(super) fn sqlite3_busy_timeout(database: *mut Sqlite3, milliseconds: c_int) -> c_int;
    pub(super) fn sqlite3_prepare_v2(
        database: *mut Sqlite3,
        sql: *const c_char,
        bytes: c_int,
        statement: *mut *mut Sqlite3Stmt,
        tail: *mut *const c_char,
    ) -> c_int;
    pub(super) fn sqlite3_finalize(statement: *mut Sqlite3Stmt) -> c_int;
    pub(super) fn sqlite3_reset(statement: *mut Sqlite3Stmt) -> c_int;
    pub(super) fn sqlite3_clear_bindings(statement: *mut Sqlite3Stmt) -> c_int;
    pub(super) fn sqlite3_bind_null(statement: *mut Sqlite3Stmt, index: c_int) -> c_int;
    pub(super) fn sqlite3_bind_int64(
        statement: *mut Sqlite3Stmt,
        index: c_int,
        value: i64,
    ) -> c_int;
    pub(super) fn sqlite3_bind_double(
        statement: *mut Sqlite3Stmt,
        index: c_int,
        value: f64,
    ) -> c_int;
    pub(super) fn sqlite3_bind_text(
        statement: *mut Sqlite3Stmt,
        index: c_int,
        value: *const c_char,
        bytes: c_int,
        destructor: Destructor,
    ) -> c_int;
    pub(super) fn sqlite3_bind_blob(
        statement: *mut Sqlite3Stmt,
        index: c_int,
        value: *const c_void,
        bytes: c_int,
        destructor: Destructor,
    ) -> c_int;
    pub(super) fn sqlite3_step(statement: *mut Sqlite3Stmt) -> c_int;
    pub(super) fn sqlite3_column_count(statement: *mut Sqlite3Stmt) -> c_int;
    pub(super) fn sqlite3_column_type(statement: *mut Sqlite3Stmt, index: c_int) -> c_int;
    pub(super) fn sqlite3_column_int64(statement: *mut Sqlite3Stmt, index: c_int) -> i64;
    pub(super) fn sqlite3_column_double(statement: *mut Sqlite3Stmt, index: c_int) -> f64;
    pub(super) fn sqlite3_column_text(statement: *mut Sqlite3Stmt, index: c_int) -> *const u8;
    pub(super) fn sqlite3_column_blob(statement: *mut Sqlite3Stmt, index: c_int) -> *const c_void;
    pub(super) fn sqlite3_column_bytes(statement: *mut Sqlite3Stmt, index: c_int) -> c_int;
    pub(super) fn sqlite3_changes(database: *mut Sqlite3) -> c_int;
    pub(super) fn sqlite3_last_insert_rowid(database: *mut Sqlite3) -> i64;
    pub(super) fn sqlite3_extended_errcode(database: *mut Sqlite3) -> c_int;
    pub(super) fn sqlite3_exec(
        database: *mut Sqlite3,
        sql: *const c_char,
        callback: Option<unsafe extern "C" fn()>,
        context: *mut c_void,
        error: *mut *mut c_char,
    ) -> c_int;
    pub(super) fn sqlite3_free(value: *mut c_void);
    pub(super) fn sqlite3_backup_init(
        destination: *mut Sqlite3,
        destination_name: *const c_char,
        source: *mut Sqlite3,
        source_name: *const c_char,
    ) -> *mut Sqlite3Backup;
    pub(super) fn sqlite3_backup_step(backup: *mut Sqlite3Backup, pages: c_int) -> c_int;
    pub(super) fn sqlite3_backup_finish(backup: *mut Sqlite3Backup) -> c_int;
}
