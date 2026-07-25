//! Bounded owned SQLite C API boundary for generic language operations.

use std::ffi::{c_char, c_int, c_void, CString};
use std::fmt;
use std::ptr::{self, NonNull};

mod connection;
mod ffi;
mod statement;
#[cfg(test)]
mod tests;
mod values;

use ffi::*;
use values::*;

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

pub struct Statement {
    raw: NonNull<Sqlite3Stmt>,
    database: NonNull<Sqlite3>,
}
