//! Linux glibc-shaped termios via owned libc externs.

use std::io::IsTerminal;
use std::os::fd::RawFd;

const NCCS: usize = 32;
const STDIN_FILENO: RawFd = 0;
const TCSANOW: i32 = 0;

const IGNBRK: u32 = 1;
const BRKINT: u32 = 2;
const PARMRK: u32 = 8;
const ISTRIP: u32 = 32;
const INLCR: u32 = 64;
const IGNCR: u32 = 128;
const ICRNL: u32 = 256;
const IXON: u32 = 1024;
const OPOST: u32 = 1;
const ECHO: u32 = 8;
const ECHONL: u32 = 64;
const ICANON: u32 = 2;
const ISIG: u32 = 1;
const IEXTEN: u32 = 32768;
const CSIZE: u32 = 48;
const PARENB: u32 = 256;
const CS8: u32 = 48;
const VMIN: usize = 6;
const VTIME: usize = 5;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Termios {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_line: u8,
    pub c_cc: [u8; NCCS],
    pub c_ispeed: u32,
    pub c_ospeed: u32,
}

#[derive(Debug)]
pub struct TermiosError(pub i32);

impl std::fmt::Display for TermiosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "termios errno {}", self.0)
    }
}

extern "C" {
    fn tcgetattr(fd: i32, termios_p: *mut Termios) -> i32;
    fn tcsetattr(fd: i32, optional_actions: i32, termios_p: *const Termios) -> i32;
}

fn errno() -> i32 {
    // libc errno via __errno_location on Linux glibc/musl
    extern "C" {
        fn __errno_location() -> *mut i32;
    }
    unsafe { *__errno_location() }
}

/// Apply cfmakeraw-equivalent flag clearing in place.
pub fn make_raw(t: &mut Termios) {
    t.c_iflag &= !(IGNBRK | BRKINT | PARMRK | ISTRIP | INLCR | IGNCR | ICRNL | IXON);
    t.c_oflag &= !OPOST;
    t.c_lflag &= !(ECHO | ECHONL | ICANON | ISIG | IEXTEN);
    t.c_cflag &= !(CSIZE | PARENB);
    t.c_cflag |= CS8;
    t.c_cc[VMIN] = 1;
    t.c_cc[VTIME] = 0;
}

pub fn tcgetattr_stdin() -> Result<Option<Termios>, TermiosError> {
    if !std::io::stdin().is_terminal() {
        return Ok(None);
    }
    let mut t = Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_line: 0,
        c_cc: [0; NCCS],
        c_ispeed: 0,
        c_ospeed: 0,
    };
    let rc = unsafe { tcgetattr(STDIN_FILENO, &mut t) };
    if rc != 0 {
        return Err(TermiosError(errno()));
    }
    Ok(Some(t))
}

pub fn tcsetattr_stdin_now(t: &Termios) -> Result<(), TermiosError> {
    let rc = unsafe { tcsetattr(STDIN_FILENO, TCSANOW, t) };
    if rc != 0 {
        return Err(TermiosError(errno()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn termios_size_matches_linux_glibc() {
        assert_eq!(std::mem::size_of::<Termios>(), 60);
    }
}
