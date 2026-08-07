//! Deterministic storage for uniquely owned byte payloads.

mod access;
mod allocation;
mod cloning;
mod error;
mod model;
mod mutation;
mod object;
mod release;
mod transfer;
mod word;

pub use error::{InvalidUniqueKeyWord, UniqueStoreError, UniqueStoreLeak};
pub use model::{
    ByteVectorKey, BytesKey, PathKey, StaticBytes, UniqueKeyWord, UniqueLayout, UniqueStore,
    UniqueStoreId, UniqueStoreStats,
};

#[cfg(test)]
mod tests;
