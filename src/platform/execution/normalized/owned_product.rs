//! Fixed products use the shared, loan-aware composite custody mechanism.
pub(super) use super::owned_storage::OwnedStorage as OwnedProduct;
#[cfg(test)]
pub(super) use super::owned_storage::StorageObservation;
