use super::OwnerHomeTable;
use crate::{DataOwnerId, ResourceError, ResourceResult};

impl OwnerHomeTable {
    pub(super) fn epoch(&self, owner: DataOwnerId) -> ResourceResult<u64> {
        self.epochs
            .get(&owner)
            .copied()
            .ok_or_else(|| ResourceError::new("owner-stale", "owner epoch is unknown"))
    }

    pub(super) fn next_epoch(&self, owner: DataOwnerId) -> ResourceResult<u64> {
        let epoch = self.epoch(owner)?;
        if epoch >= self.max_epoch {
            return Err(ResourceError::new(
                "owner-epoch-overflow",
                "owner proof epoch exhausted",
            ));
        }
        epoch.checked_add(1).ok_or_else(|| {
            ResourceError::new("owner-epoch-overflow", "owner proof epoch exhausted")
        })
    }
}
