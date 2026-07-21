//! stdin poll(2) via owned libc extern.

use std::os::fd::RawFd;

const STDIN_FILENO: RawFd = 0;
const POLLIN: i16 = 1;
const POLLERR: i16 = 8;
const POLLHUP: i16 = 16;

#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

#[derive(Debug)]
pub struct PollError(pub i32);

impl std::fmt::Display for PollError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "poll errno {}", self.0)
    }
}

extern "C" {
    fn poll(fds: *mut PollFd, nfds: u64, timeout: i32) -> i32;
    fn __errno_location() -> *mut i32;
}

fn errno() -> i32 {
    unsafe { *__errno_location() }
}

/// Non-blocking readiness check for stdin (`timeout` 0).
pub fn poll_stdin_ready() -> Result<bool, PollError> {
    let mut pfd = PollFd {
        fd: STDIN_FILENO,
        events: POLLIN | POLLHUP,
        revents: 0,
    };
    let n = unsafe { poll(&mut pfd, 1, 0) };
    if n < 0 {
        return Err(PollError(errno()));
    }
    if n == 0 {
        return Ok(false);
    }
    let ev = pfd.revents;
    Ok(ev & (POLLIN | POLLHUP | POLLERR) != 0)
}
