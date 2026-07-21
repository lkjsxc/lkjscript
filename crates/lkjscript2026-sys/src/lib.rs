//! Owned Linux-first OS wrappers (no crates.io deps).
//!
//! `unsafe` is allowed only in this crate. Other workspace crates stay forbid.

#![allow(unsafe_code)]

mod fd;
mod file;
mod ioctl;
mod poll;
mod socket;
mod termios;

pub use fd::{close_fd, FdError, OwnedFd};
pub use file::{open_read, open_write, path_exists, read_fd, write_fd};
pub use ioctl::{
    ioctl_buf, is_tty, tty_guard_clear, tty_guard_restore, tty_guard_save, IoctlError, STDIN_FD,
    TCGETS, TCSETS, TERMIOS_LEN,
};
pub use poll::{poll_fd, poll_stdin_ready, PollError};
pub use socket::{
    accept_sock, bind_ipv4_any, listen_sock, recv_sock, send_sock, set_reuseaddr, tcp_socket,
    SockError,
};
pub use termios::{
    make_raw, tcgetattr_stdin, tcsetattr_stdin_now, Termios, TermiosError,
};
