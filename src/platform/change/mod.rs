//! Private Graph 5 generic semantic-change substrate.

#![allow(
    unused_imports,
    reason = "private change exports become crate consumers at the Graph 5 repository cutover"
)]

mod delta;
mod derived;
mod overlay;

pub use delta::{CanonicalDelta, ExactEdit, PrimitiveEdit};
pub use derived::{DerivedDelta, DerivedValueEdit, RelationDelta, derive_local_delta};
pub use overlay::KernelOverlay;

#[cfg(test)]
mod tests;
