//! Bounded deterministic storage for uniquely owned byte payloads.

mod access;
mod allocation;
mod error;
mod limits;
mod model;
mod object;
mod release;
mod transfer;

pub use error::{InvalidUniqueStoreLimits, UniqueStoreError, UniqueStoreLeak};
pub use limits::UniqueStoreLimits;
pub use model::{
    ByteVectorKey, BytesKey, PathKey, StaticBytes, UniqueLayout, UniqueStore, UniqueStoreId,
    UniqueStoreStats,
};

#[cfg(test)]
mod tests;
