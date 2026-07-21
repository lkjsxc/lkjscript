//! Owned Linux-first OS wrappers (no crates.io deps).
//!
//! `unsafe` is allowed only in this crate. Other workspace crates stay forbid.

#![allow(unsafe_code)]

mod ioctl;
mod poll;
mod termios;

pub use ioctl::{
    ioctl_buf, is_tty, tty_guard_clear, tty_guard_restore, tty_guard_save, IoctlError, STDIN_FD,
    TCGETS, TCSETS, TERMIOS_LEN,
};
pub use poll::{poll_fd, poll_stdin_ready, PollError};
pub use termios::{
    make_raw, tcgetattr_stdin, tcsetattr_stdin_now, Termios, TermiosError,
};
