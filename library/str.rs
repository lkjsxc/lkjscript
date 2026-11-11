//! Checked immutable UTF-8 views use the safe std::str::from_utf8 operation.
//! The current error value is an opaque failure marker.
#[derive(Copy, Clone)]
pub struct Utf8Error;
