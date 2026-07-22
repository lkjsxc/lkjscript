//! ioctl(2) and tty exit-restore guard.

use std::sync::Mutex;

use crate::poll::errno;

/// Linux TCGETS / TCSETS (asm-generic/ioctls.h).
pub const TCGETS: u64 = 0x5401;
pub const TCSETS: u64 = 0x5402;
pub const STDIN_FD: i32 = 0;
pub const TERMIOS_LEN: usize = 60;

#[derive(Debug)]
pub struct IoctlError(pub i32);

impl std::fmt::Display for IoctlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ioctl errno {}", self.0)
    }
}

extern "C" {
    fn ioctl(fd: i32, request: u64, argp: *mut u8) -> i32;
    fn isatty(fd: i32) -> i32;
}

static GUARD: Mutex<Option<Vec<u8>>> = Mutex::new(None);

pub fn is_tty(fd: i32) -> bool {
    unsafe { isatty(fd) != 0 }
}

pub fn ioctl_buf(fd: i32, request: u64, buf: &mut [u8]) -> Result<(), IoctlError> {
    let rc = unsafe { ioctl(fd, request, buf.as_mut_ptr()) };
    if rc < 0 {
        return Err(IoctlError(errno()));
    }
    Ok(())
}

pub fn tty_guard_save(buf: &[u8]) {
    if let Ok(mut g) = GUARD.lock() {
        *g = Some(buf.to_vec());
    }
}

pub fn tty_guard_clear() {
    if let Ok(mut g) = GUARD.lock() {
        *g = None;
    }
}

/// Best-effort restore of the saved cooked termios (process exit safety).
pub fn tty_guard_restore() {
    let saved = match GUARD.lock() {
        Ok(mut g) => g.take(),
        Err(_) => None,
    };
    if let Some(mut buf) = saved {
        if buf.len() == TERMIOS_LEN && is_tty(STDIN_FD) {
            let _ = ioctl_buf(STDIN_FD, TCSETS, &mut buf);
        }
    }
}
