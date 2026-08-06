//! Safe Linux topology, scheduler, affinity, and worker-binding facade.
//!
//! Unsafe code is lint-confined to the private Linux ABI module.

mod linux_host;

pub use linux_host::*;
