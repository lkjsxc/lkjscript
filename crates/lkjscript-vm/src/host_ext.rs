//! Strings, resource handles, filesystem, and socket host operations.

use std::os::fd::RawFd;

use lkjscript_core::{Error, GcHeap as Arena, HeapObj, Result, Value};
use lkjscript_sys::OwnedFd;

const STDIN_TOKEN: u32 = 1;
const FIRST_OWNED_TOKEN: u32 = 16;

enum OwnedResource {
    File(OwnedFd),
    Directory(OwnedFd),
    Socket(OwnedFd),
    SqliteConnection {
        connection: lkjscript_sys::SqliteConnection,
        live_statements: usize,
    },
    SqliteStatement {
        statement: lkjscript_sys::SqliteStatement,
        parent: usize,
    },
}

pub struct ResourceTable {
    slots: Vec<Option<OwnedResource>>,
    max_handles: usize,
    limit_exceeded: bool,
}

impl Default for ResourceTable {
    fn default() -> Self {
        Self::new(4_096)
    }
}

impl ResourceTable {
    pub fn new(max_handles: usize) -> Self {
        Self {
            slots: Vec::new(),
            max_handles,
            limit_exceeded: false,
        }
    }

    pub fn allocated_handle_slots(&self) -> usize {
        self.slots.len()
    }

    pub const fn limit_exceeded(&self) -> bool {
        self.limit_exceeded
    }
    pub fn stdin_handle() -> Value {
        Value::from_handle(STDIN_TOKEN)
    }
}

mod files;
mod resources;
mod results;
mod sockets;
mod sqlite_bindings;
mod sqlite_columns;
mod sqlite_lifecycle;
mod streams;
mod strings;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

pub use results::*;
pub use sockets::SocketReceiveError;
pub use strings::*;
