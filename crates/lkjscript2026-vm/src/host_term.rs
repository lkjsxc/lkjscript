//! TTY restore on exit (time ops live in sys + thin VM opcodes).

pub fn restore_tty() {
    lkjscript2026_sys::tty_guard_restore();
}
