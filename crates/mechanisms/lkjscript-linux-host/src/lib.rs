//! Safe Linux topology, scheduler, affinity, and worker-binding facade.
//!
//! The private Linux ABI file is machine-registered under `linux-host-io`.

#![allow(unsafe_code)]

mod linux_host;

pub use linux_host::*;
