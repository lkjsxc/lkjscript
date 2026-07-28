use std::sync::MutexGuard;

use lkjscript_core::{UniqueStore, UniqueStoreStats};

use super::{stats::add_stats, *};
use crate::OwnerMetrics;

impl PartitionedUniqueStore {
    pub fn metrics(&self) -> ResourceResult<(OwnerMetrics, UniqueStoreStats)> {
        let metadata = self.metadata()?;
        let mut unique = UniqueStoreStats::default();
        for partition in 0..self.stores.len() {
            add_stats(&mut unique, self.store(partition)?.stats());
        }
        Ok((metadata.homes.metrics(), unique))
    }

    pub fn verify_empty(&self) -> ResourceResult<()> {
        let metadata = self.metadata()?;
        if !metadata.keys.is_empty() {
            return Err(ResourceError::new(
                "unique-leak",
                "session unique owner leaked",
            ));
        }
        for partition in 0..self.stores.len() {
            if self.store(partition)?.assert_no_leaks().is_err() {
                return Err(ResourceError::new(
                    "unique-leak",
                    "session unique owner leaked",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn require<'a>(
        &self,
        metadata: &'a Metadata,
        value: HomedByteVector,
    ) -> ResourceResult<&'a Metadata> {
        if metadata.keys.get(&value.owner) != Some(&(value.partition, value.key)) {
            return Err(ResourceError::new("owner-stale", "owner key is stale"));
        }
        Ok(metadata)
    }

    pub(super) fn release_locked(
        &self,
        metadata: &mut Metadata,
        value: HomedByteVector,
    ) -> ResourceResult<()> {
        let proof = metadata.homes.prove_no_live_loan(value.owner)?;
        self.store(value.partition)?
            .free_byte_vector(value.key)
            .map_err(unique_error)?;
        metadata.homes.remove(value.owner, proof)?;
        metadata.keys.remove(&value.owner);
        Ok(())
    }

    pub(super) fn metadata(&self) -> ResourceResult<MutexGuard<'_, Metadata>> {
        self.metadata
            .lock()
            .map_err(|_| ResourceError::new("poison", "unique metadata poisoned"))
    }

    pub(super) fn store(&self, partition: usize) -> ResourceResult<MutexGuard<'_, UniqueStore>> {
        self.stores[partition]
            .lock()
            .map_err(|_| ResourceError::new("poison", "unique partition poisoned"))
    }
}
