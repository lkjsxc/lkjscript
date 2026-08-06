//! Residual Linux-first host I/O and SQLite wrappers with no dependencies.
//!
//! Unsafe code is confined by lint to the private FFI modules that need it.
//! Every public safe wrapper validates the complete memory and type contract
//! required by its FFI call.

mod fd;
mod file;
mod native_path;
mod poll;
mod random;
mod socket;
mod sqlite;
mod time;
mod tty;

pub use fd::{close_fd, FdError, OwnedFd};
pub use file::{
    fsync_fd, open_append, open_create_new, open_dir, open_read, open_write, path_exists, read_fd,
    rename_path, truncate_fd, write_fd,
};
pub use poll::{poll_fd, PollError};
pub use random::random_fill;
pub use socket::{
    accept_sock, bind_ipv4_any, listen_sock, recv_sock, send_sock, set_reuseaddr, tcp_socket,
    SockError,
};
pub use sqlite::{
    ColumnType, Connection as SqliteConnection, SqliteError, Statement as SqliteStatement,
    Step as SqliteStep,
};
pub use time::{now_ms_monotonic, sleep_ms};
pub use tty::{
    is_tty, tty_get, tty_guard_clear, tty_guard_restore, tty_guard_save, tty_set, TtyError,
    STDIN_FD, TTY_STATE_LEN,
};
