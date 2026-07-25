//! Byte-buffer and bounded terminal/poll host helpers.

use crate::host_ext::ResourceTable;
use lkjscript_core::{
    Error, GcHeap as Arena, HeapObj, Result, Value, MAX_BUFFER_BYTES, MAX_BULK_IO_BYTES,
};

mod borrowing;
mod operations;
mod storage;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

pub use borrowing::*;
pub(crate) use operations::*;
pub use storage::*;
