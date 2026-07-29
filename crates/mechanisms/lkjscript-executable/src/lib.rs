//! Safe installation and invocation facade for verified native images.
//!
//! Unsafe executable mapping, generated entry, and native runtime bridge code
//! is confined to machine-registered files behind this crate's safe API.

#![allow(unsafe_code)]

use std::sync::OnceLock;
use std::time::Instant;

mod executable;

pub use executable::*;

fn now_ms_monotonic() -> i64 {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    let elapsed = ORIGIN.get_or_init(Instant::now).elapsed().as_millis();
    i64::try_from(elapsed).unwrap_or(i64::MAX)
}
