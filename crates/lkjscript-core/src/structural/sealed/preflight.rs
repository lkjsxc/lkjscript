use super::SealedRegionStore;
use crate::structural::{DomainKey, StructuralError};

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub(super) fn failure_buffer<E>(&self, keys: &[DomainKey]) -> Result<Vec<E>, StructuralError> {
        let mut drops = 0_usize;
        for &key in keys {
            drops = drops
                .checked_add(
                    self.records[self.record_index(key)?]
                        .1
                        .drops
                        .as_slice()
                        .len(),
                )
                .ok_or(StructuralError::ArithmeticOverflow)?;
        }
        let mut failures = Vec::new();
        failures
            .try_reserve_exact(drops)
            .map_err(|_| StructuralError::AllocationFailed)?;
        Ok(failures)
    }
}
