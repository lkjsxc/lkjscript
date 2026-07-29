use super::SealedRegionStore;
use crate::structural::{DomainKey, StructuralError};

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub(super) fn preflight_owner_counts(
        &self,
        incoming: &[(DomainKey, u32)],
        existing: &[(DomainKey, u32)],
    ) -> Result<(), StructuralError> {
        for &(_, count) in incoming {
            if count
                .checked_add(1)
                .is_none_or(|owners| owners > self.limits.max_region_owners)
            {
                return Err(StructuralError::OwnerOverflow);
            }
        }
        for &(key, count) in existing {
            let record = &self.records[self.record_index(key)?].1;
            if record
                .owners
                .checked_add(count)
                .is_none_or(|owners| owners > self.limits.max_region_owners)
            {
                return Err(StructuralError::OwnerOverflow);
            }
        }
        Ok(())
    }
}
