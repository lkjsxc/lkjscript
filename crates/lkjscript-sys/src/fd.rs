#![allow(unsafe_code)]

//! Owned OS file descriptors; Drop closes.

use std::os::fd::RawFd;

#[derive(Debug)]
pub struct FdError(pub i32);

impl std::fmt::Display for FdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fd errno {}", self.0)
    }
}

extern "C" {
    fn close(fd: i32) -> i32;
    fn __errno_location() -> *mut i32;
}

pub(crate) fn errno() -> i32 {
    unsafe { *__errno_location() }
}

/// Owned fd; Drop closes. Safe to hold outside this crate.
pub struct OwnedFd(RawFd);

impl OwnedFd {
    pub(crate) fn from_raw(fd: RawFd) -> Self {
        Self(fd)
    }

    pub fn as_raw(&self) -> RawFd {
        self.0
    }
}

impl Drop for OwnedFd {
    fn drop(&mut self) {
        let _ = close_fd(self.0);
    }
}

pub fn close_fd(fd: RawFd) -> Result<(), FdError> {
    let rc = unsafe { close(fd) };
    if rc < 0 {
        return Err(FdError(errno()));
    }
    Ok(())
}
