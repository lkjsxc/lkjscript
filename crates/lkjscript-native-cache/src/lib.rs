#![forbid(unsafe_code)]
//! Bounded local content-addressed storage for verified native images.

mod error;
mod key;
mod limits;
mod model;
mod publication;
mod storage;

pub use error::CacheError;
pub use key::artifact_key;
pub use limits::CacheLimits;
pub use model::{ArtifactKey, CacheContext, CacheTier, Lookup, MissReason, Publication};
pub use storage::NativeArtifactCache;

#[cfg(test)]
mod tests;
