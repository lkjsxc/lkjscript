#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]
#![allow(
    clippy::result_large_err,
    reason = "typed diagnostics remain complete values at deterministic boundaries"
)]
#![forbid(unsafe_code)]

//! The lkjscript source language, package authority, component runtime, and capability adapters.
//!
//! Rust embedders place complete semantic operations inside [`platform::stack::run`].
//! Construct, use and destroy recursive values within that closure, returning flat results
//! such as bytes or status. Built-in binaries and runtime workers already supply semantic
//! stacks sized from physical memory; individual library functions use their caller's stack.

/// Exact product snapshot; no component implies stability or compatibility.
pub const PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Machine-readable policy for every component of the product identifier.
pub const PRODUCT_VERSION_POLICY: &str = "opaque-triplet";

pub mod platform;
pub mod release_container;
