use super::{StructuralRootTable, StructuralRootTableError, StructuralRootTableStats};
use crate::structural::StructuralRuntimeId;

impl StructuralRootTable {
    pub fn new(runtime: StructuralRuntimeId) -> Result<Self, StructuralRootTableError> {
        Ok(Self {
            runtime,
            roots: Vec::new(),
            free_roots: Vec::new(),
            exclusive_roots: std::collections::HashSet::new(),
            loans: Vec::new(),
            free_loans: Vec::new(),
            stats: StructuralRootTableStats::default(),
        })
    }

    pub const fn runtime(&self) -> StructuralRuntimeId {
        self.runtime
    }

    pub const fn stats(&self) -> StructuralRootTableStats {
        self.stats
    }

    pub(in crate::structural) fn retained_bytes_estimate(
        &self,
    ) -> Result<u64, StructuralRootTableError> {
        let roots = u64::try_from(self.roots.capacity())
            .ok()
            .and_then(|capacity| {
                capacity.checked_mul(std::mem::size_of::<super::RootSlot>() as u64)
            })
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let free_roots = u64::try_from(self.free_roots.capacity())
            .ok()
            .and_then(|capacity| capacity.checked_mul(std::mem::size_of::<u32>() as u64))
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let exclusive_roots = u64::try_from(self.exclusive_roots.capacity())
            .ok()
            .and_then(|capacity| {
                capacity.checked_mul(std::mem::size_of::<crate::structural::RootKey>() as u64)
            })
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let loans = u64::try_from(self.loans.capacity())
            .ok()
            .and_then(|capacity| {
                capacity.checked_mul(std::mem::size_of::<super::LoanSlot>() as u64)
            })
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let free_loans = u64::try_from(self.free_loans.capacity())
            .ok()
            .and_then(|capacity| capacity.checked_mul(std::mem::size_of::<u32>() as u64))
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        roots
            .checked_add(free_roots)
            .and_then(|total| total.checked_add(exclusive_roots))
            .and_then(|total| total.checked_add(loans))
            .and_then(|total| total.checked_add(free_loans))
            .ok_or(StructuralRootTableError::ArithmeticOverflow)
    }
}
