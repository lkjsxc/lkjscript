//! Wait / clock host effects; tty restore on exit.

use std::thread;
use std::time::{Duration, Instant};

use lkjscript2026_core::{Error, Result, Value};

static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn start() -> Instant {
    *START.get_or_init(Instant::now)
}

pub fn restore_tty() {
    lkjscript2026_sys::tty_guard_restore();
}

pub fn now_ms() -> Result<Value> {
    let ms = start().elapsed().as_millis() as i64;
    Ok(Value::from_int(ms))
}

pub fn wait_ms(v: Value) -> Result<Value> {
    let n = v
        .as_int()
        .ok_or_else(|| Error::msg("wait-ms expects int"))?;
    let n = n.max(0) as u64;
    thread::sleep(Duration::from_millis(n));
    Ok(Value::NIL)
}
