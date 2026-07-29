use super::RegionStore;
use crate::structural::{DomainKey, StructuralError};

impl<T: Copy, D: Copy> RegionStore<T, D> {
    pub(super) fn preflight_release(&self, work: &[DomainKey]) -> Result<(), StructuralError> {
        for &key in work {
            if self.records[self.record_index(key)?].1.loans != 0 {
                return Err(StructuralError::LiveLoan);
            }
        }
        Ok(())
    }

    pub(super) fn failure_buffer<E>(
        &self,
        work: &[DomainKey],
        extra: Option<DomainKey>,
    ) -> Result<Vec<E>, StructuralError> {
        let mut drops = 0_usize;
        for key in work.iter().copied().chain(extra) {
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
