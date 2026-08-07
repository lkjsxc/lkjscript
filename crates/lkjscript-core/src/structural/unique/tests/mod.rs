#![allow(clippy::expect_used)]

use super::*;

mod access;
mod cloning;
mod mutation;
mod opaque;
mod range;
mod transfer;

fn store(id: u64) -> UniqueStore {
    let id = UniqueStoreId::new(id).expect("nonzero test store identity");
    UniqueStore::new(id)
}

fn bytes_with_capacity(capacity: usize, values: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(capacity);
    bytes.extend_from_slice(values);
    bytes
}
