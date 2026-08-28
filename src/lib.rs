#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]
#![allow(
    clippy::result_large_err,
    reason = "typed diagnostics remain complete values at deterministic boundaries"
)]
#![forbid(unsafe_code)]

//! The lkjscript source language, package authority, component runtime, and capability adapters.

pub const PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod platform;
