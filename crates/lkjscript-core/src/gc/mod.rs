//! Pure session-owned stable-index mark/sweep heap shared by VM and JIT.

mod allocation;
mod collection;
mod model;
mod mutation;
mod snapshot;

pub use model::{GcConfig, GcHeap, GcLimit, GcStats};

#[cfg(test)]
mod tests;
