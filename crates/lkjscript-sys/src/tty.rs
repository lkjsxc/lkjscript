//! Fixed Linux terminal operations and best-effort exit restoration.

use std::sync::Mutex;

use crate::poll::errno;

const TCGETS: u64 = 0x5401;
const TCSETS: u64 = 0x5402;
pub const STDIN_FD: i32 = 0;
pub const TTY_STATE_LEN: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtyError {
    InvalidStateLength { actual: usize },
    GuardPoisoned,
    Os(i32),
}

impl std::fmt::Display for TtyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStateLength { actual } => write!(
                formatter,
                "terminal state must be {TTY_STATE_LEN} bytes, got {actual}"
            ),
            Self::GuardPoisoned => formatter.write_str("terminal guard lock poisoned"),
            Self::Os(errno) => write!(formatter, "terminal ioctl errno {errno}"),
        }
    }
}

extern "C" {
    fn ioctl(fd: i32, request: u64, argument: *mut u8) -> i32;
    fn isatty(fd: i32) -> i32;
}

static GUARD: Mutex<Option<[u8; TTY_STATE_LEN]>> = Mutex::new(None);

pub fn is_tty(fd: i32) -> bool {
    unsafe { isatty(fd) != 0 }
}

pub fn tty_get(fd: i32, state: &mut [u8]) -> Result<(), TtyError> {
    validate_state_length(state.len())?;
    ioctl_fixed(fd, TCGETS, state.as_mut_ptr())
}

pub fn tty_set(fd: i32, state: &[u8]) -> Result<(), TtyError> {
    validate_state_length(state.len())?;
    ioctl_fixed(fd, TCSETS, state.as_ptr().cast_mut())
}

pub fn tty_guard_save(state: &[u8]) -> Result<(), TtyError> {
    validate_state_length(state.len())?;
    let mut saved = [0; TTY_STATE_LEN];
    saved.copy_from_slice(state);
    let mut guard = GUARD.lock().map_err(|_| TtyError::GuardPoisoned)?;
    *guard = Some(saved);
    Ok(())
}

pub fn tty_guard_clear() -> Result<(), TtyError> {
    let mut guard = GUARD.lock().map_err(|_| TtyError::GuardPoisoned)?;
    *guard = None;
    Ok(())
}

/// Restore a saved state during process shutdown; errors cannot be surfaced.
pub fn tty_guard_restore() {
    let saved = match GUARD.lock() {
        Ok(mut guard) => guard.take(),
        Err(_) => None,
    };
    if let Some(state) = saved {
        if is_tty(STDIN_FD) {
            let _ = tty_set(STDIN_FD, &state);
        }
    }
}

fn validate_state_length(actual: usize) -> Result<(), TtyError> {
    if actual == TTY_STATE_LEN {
        Ok(())
    } else {
        Err(TtyError::InvalidStateLength { actual })
    }
}

fn ioctl_fixed(fd: i32, request: u64, argument: *mut u8) -> Result<(), TtyError> {
    let result = unsafe { ioctl(fd, request, argument) };
    if result < 0 {
        return Err(TtyError::Os(errno()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{tty_get, tty_set, TtyError, TTY_STATE_LEN};

    #[test]
    fn rejects_wrong_state_lengths_before_ffi() {
        let mut short = [0; TTY_STATE_LEN - 1];
        let long = [0; TTY_STATE_LEN + 1];
        assert_eq!(
            tty_get(-1, &mut short),
            Err(TtyError::InvalidStateLength {
                actual: TTY_STATE_LEN - 1
            })
        );
        assert_eq!(
            tty_set(-1, &long),
            Err(TtyError::InvalidStateLength {
                actual: TTY_STATE_LEN + 1
            })
        );
    }

    #[test]
    fn exact_state_length_reaches_only_the_fixed_request() {
        let mut state = [0; TTY_STATE_LEN];
        assert!(matches!(tty_get(-1, &mut state), Err(TtyError::Os(_))));
        assert!(matches!(tty_set(-1, &state), Err(TtyError::Os(_))));
    }
}
