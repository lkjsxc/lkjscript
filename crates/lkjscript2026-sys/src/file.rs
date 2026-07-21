//! Linux filesystem open/read/write/access via owned libc externs.

use std::ffi::CString;
use std::os::fd::RawFd;

use crate::fd::{errno, OwnedFd, FdError};

const O_RDONLY: i32 = 0;
const O_WRONLY: i32 = 1;
const O_CREAT: i32 = 0o100;
const O_TRUNC: i32 = 0o1000;
const F_OK: i32 = 0;
const MODE_0666: u32 = 0o666;

extern "C" {
    fn open(pathname: *const i8, flags: i32, mode: u32) -> i32;
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    fn access(pathname: *const i8, mode: i32) -> i32;
}

fn c_path(path: &str) -> Result<CString, FdError> {
    CString::new(path).map_err(|_| FdError(-1))
}

pub fn open_read(path: &str) -> Result<OwnedFd, FdError> {
    let c = c_path(path)?;
    let fd = unsafe { open(c.as_ptr(), O_RDONLY, 0) };
    if fd < 0 {
        return Err(FdError(errno()));
    }
    Ok(OwnedFd::from_raw(fd))
}

pub fn open_write(path: &str) -> Result<OwnedFd, FdError> {
    let c = c_path(path)?;
    let flags = O_WRONLY | O_CREAT | O_TRUNC;
    let fd = unsafe { open(c.as_ptr(), flags, MODE_0666) };
    if fd < 0 {
        return Err(FdError(errno()));
    }
    Ok(OwnedFd::from_raw(fd))
}

pub fn read_fd(fd: RawFd, buf: &mut [u8]) -> Result<usize, FdError> {
    let n = unsafe { read(fd, buf.as_mut_ptr(), buf.len()) };
    if n < 0 {
        return Err(FdError(errno()));
    }
    Ok(n as usize)
}

pub fn write_fd(fd: RawFd, buf: &[u8]) -> Result<usize, FdError> {
    let n = unsafe { write(fd, buf.as_ptr(), buf.len()) };
    if n < 0 {
        return Err(FdError(errno()));
    }
    Ok(n as usize)
}

pub fn path_exists(path: &str) -> Result<bool, FdError> {
    let c = c_path(path)?;
    let rc = unsafe { access(c.as_ptr(), F_OK) };
    if rc == 0 {
        Ok(true)
    } else {
        Ok(false)
    }
}
