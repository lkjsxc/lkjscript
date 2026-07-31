//! Pure session-owned stable-index mark/sweep heap shared by VM and JIT.

mod allocation;
#[cfg(test)]
mod bridge_tests;
mod collection;
#[cfg(test)]
mod enum_tests;
mod model;
mod mutation;
#[cfg(test)]
mod root_tests;
mod snapshot;
#[cfg(test)]
mod symbol_tests;

pub use model::{GcConfig, GcHeap, GcLimit, GcStats};

#[cfg(test)]
mod tests;
