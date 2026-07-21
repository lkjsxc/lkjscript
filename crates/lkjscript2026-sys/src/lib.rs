//! Owned Linux-first OS wrappers (no crates.io deps).
//!
//! `unsafe` is allowed only in this crate. Other workspace crates stay forbid.

#![allow(unsafe_code)]

mod poll;
mod termios;

pub use poll::{poll_stdin_ready, PollError};
pub use termios::{
    make_raw, tcgetattr_stdin, tcsetattr_stdin_now, Termios, TermiosError,
};
