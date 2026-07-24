//! Linux filesystem open/read/write/access via owned libc externs.

use std::ffi::CString;
use std::os::fd::RawFd;

use crate::fd::{errno, FdError, OwnedFd};

const O_RDONLY: i32 = 0;
const O_WRONLY: i32 = 1;
const O_CREAT: i32 = 0o100;
const O_EXCL: i32 = 0o200;
const O_TRUNC: i32 = 0o1000;
const O_APPEND: i32 = 0o2000;
const O_DIRECTORY: i32 = 0o200000;
const F_OK: i32 = 0;
const MODE_0666: u32 = 0o666;
const ENOENT: i32 = 2;
const ENOTDIR: i32 = 20;
const EINTR: i32 = 4;

extern "C" {
    fn open(pathname: *const i8, flags: i32, mode: u32) -> i32;
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    fn access(pathname: *const i8, mode: i32) -> i32;
    fn fsync(fd: i32) -> i32;
    fn ftruncate(fd: i32, length: i64) -> i32;
    fn rename(oldpath: *const i8, newpath: *const i8) -> i32;
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
    open_with_mode(path, O_WRONLY | O_CREAT | O_TRUNC)
}

pub fn open_append(path: &str) -> Result<OwnedFd, FdError> {
    open_with_mode(path, O_WRONLY | O_CREAT | O_APPEND)
}

pub fn open_create_new(path: &str) -> Result<OwnedFd, FdError> {
    open_with_mode(path, O_WRONLY | O_CREAT | O_EXCL)
}

pub fn open_dir(path: &str) -> Result<OwnedFd, FdError> {
    open_with_mode(path, O_RDONLY | O_DIRECTORY)
}

fn open_with_mode(path: &str, flags: i32) -> Result<OwnedFd, FdError> {
    let c = c_path(path)?;
    let fd = unsafe { open(c.as_ptr(), flags, MODE_0666) };
    if fd < 0 {
        return Err(FdError(errno()));
    }
    Ok(OwnedFd::from_raw(fd))
}

pub fn fsync_fd(fd: RawFd) -> Result<(), FdError> {
    if unsafe { fsync(fd) } < 0 {
        return Err(FdError(errno()));
    }
    Ok(())
}

pub fn truncate_fd(fd: RawFd, length: i64) -> Result<(), FdError> {
    if length < 0 {
        return Err(FdError(-1));
    }
    if unsafe { ftruncate(fd, length) } < 0 {
        return Err(FdError(errno()));
    }
    Ok(())
}

pub fn rename_path(from: &str, to: &str) -> Result<(), FdError> {
    let from = c_path(from)?;
    let to = c_path(to)?;
    if unsafe { rename(from.as_ptr(), to.as_ptr()) } < 0 {
        return Err(FdError(errno()));
    }
    Ok(())
}

pub fn read_fd(fd: RawFd, buf: &mut [u8]) -> Result<usize, FdError> {
    loop {
        let n = unsafe { read(fd, buf.as_mut_ptr(), buf.len()) };
        if n >= 0 {
            return Ok(n as usize);
        }
        let error = errno();
        if error != EINTR {
            return Err(FdError(error));
        }
    }
}

pub fn write_fd(fd: RawFd, buf: &[u8]) -> Result<usize, FdError> {
    loop {
        let n = unsafe { write(fd, buf.as_ptr(), buf.len()) };
        if n >= 0 {
            return Ok(n as usize);
        }
        let error = errno();
        if error != EINTR {
            return Err(FdError(error));
        }
    }
}

pub fn path_exists(path: &str) -> Result<bool, FdError> {
    let c = c_path(path)?;
    let rc = unsafe { access(c.as_ptr(), F_OK) };
    if rc == 0 {
        return Ok(true);
    }
    let error = errno();
    if error == ENOENT || error == ENOTDIR {
        Ok(false)
    } else {
        Err(FdError(error))
    }
}

#[cfg(test)]
mod tests {
    use super::path_exists;

    #[test]
    fn missing_paths_are_false_but_malformed_paths_are_errors() {
        let missing = format!("/tmp/lkjscript-missing-{}-{}", std::process::id(), line!());
        assert_eq!(path_exists(&missing).ok(), Some(false));
        assert!(path_exists("embedded\0nul").is_err());
    }
}
