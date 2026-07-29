#![allow(clippy::expect_used)]

use super::*;

mod access;
mod cloning;
mod limits;
mod mutation;
mod packed;
mod range;
mod transfer;

fn store_with(
    id: u64,
    max_objects: u32,
    max_bytes: u64,
    max_slots: u32,
    max_allocations: u64,
    max_generation: u32,
) -> UniqueStore {
    let id = UniqueStoreId::new(id).expect("nonzero test store identity");
    let limits = UniqueStoreLimits::new(
        max_objects,
        max_bytes,
        max_slots,
        max_allocations,
        max_generation,
    )
    .expect("valid test limits");
    UniqueStore::new(id, limits)
}

fn bytes_with_capacity(capacity: usize, values: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(capacity);
    bytes.extend_from_slice(values);
    bytes
}
