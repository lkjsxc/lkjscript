//! TTY restoration at the outer execution boundary.

use lkjscript_core::{Error, Result};

pub fn restore_tty() -> Result<()> {
    lkjscript_sys::tty_guard_restore()
        .map_err(|error| Error::host(format!("terminal restore: {error}")))
}
