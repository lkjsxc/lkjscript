//! Owned Linux-first OS wrappers with no crates.io dependencies.
//!
//! Unsafe code is isolated here. Every public safe wrapper must validate the
//! complete memory and type contract required by its FFI call.

#![allow(unsafe_code)]

pub mod executable;
mod fd;
mod file;
mod poll;
mod socket;
mod time;
mod tty;

pub use fd::{close_fd, FdError, OwnedFd};
pub use file::{open_read, open_write, path_exists, read_fd, write_fd};
pub use poll::{poll_fd, PollError};
pub use socket::{
    accept_sock, bind_ipv4_any, listen_sock, recv_sock, send_sock, set_reuseaddr, tcp_socket,
    SockError,
};
pub use time::{now_ms_monotonic, sleep_ms};
pub use tty::{
    is_tty, tty_get, tty_guard_clear, tty_guard_restore, tty_guard_save, tty_set, TtyError,
    STDIN_FD, TTY_STATE_LEN,
};
