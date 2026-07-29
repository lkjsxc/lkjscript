use super::{
    LoanSlot, RootSlot, StructuralRootTable, StructuralRootTableError, StructuralValueKey,
};

impl StructuralRootTable {
    pub fn validate(&self) -> Result<(), StructuralRootTableError> {
        let live_roots = self
            .roots
            .iter()
            .filter(|slot| matches!(slot, RootSlot::Live { .. }))
            .count();
        let live_loans = self
            .loans
            .iter()
            .filter(|slot| matches!(slot, LoanSlot::Live { .. }))
            .count();
        if u32::try_from(live_roots).ok() != Some(self.stats.live_roots)
            || u32::try_from(live_loans).ok() != Some(self.stats.live_loans)
        {
            return Err(StructuralRootTableError::InvariantViolation);
        }
        self.validate_free_root_slots()?;
        self.validate_free_loan_slots()?;
        for (slot, root) in self.roots.iter().enumerate() {
            let RootSlot::Live { generation, value } = root else {
                continue;
            };
            if value.root.domain().runtime() != self.runtime {
                return Err(StructuralRootTableError::InvariantViolation);
            }
            let slot =
                u32::try_from(slot).map_err(|_| StructuralRootTableError::InvariantViolation)?;
            let key = StructuralValueKey::from_parts(slot, *generation);
            let (shared, exclusive) = self.loans.iter().fold((0_u32, false), |counts, loan| {
                let LoanSlot::Live { value, .. } = loan else {
                    return counts;
                };
                if value.root != key {
                    return counts;
                }
                (
                    counts.0 + u32::from(!value.exclusive),
                    counts.1 || value.exclusive,
                )
            });
            if shared != value.shared_loans || exclusive != value.exclusive_loan {
                return Err(StructuralRootTableError::InvariantViolation);
            }
            if exclusive && shared != 0 {
                return Err(StructuralRootTableError::InvariantViolation);
            }
        }
        Ok(())
    }

    pub fn assert_no_live_roots(&self) -> Result<(), StructuralRootTableError> {
        if self.stats.live_loans != 0 {
            return Err(StructuralRootTableError::LiveLoan);
        }
        if self.stats.live_roots != 0 {
            return Err(StructuralRootTableError::LiveRoot);
        }
        self.validate()
    }

    fn validate_free_root_slots(&self) -> Result<(), StructuralRootTableError> {
        for (position, slot) in self.free_roots.iter().enumerate() {
            let index =
                usize::try_from(*slot).map_err(|_| StructuralRootTableError::InvariantViolation)?;
            if !matches!(self.roots.get(index), Some(RootSlot::Vacant { .. }))
                || self.free_roots[..position].contains(slot)
            {
                return Err(StructuralRootTableError::InvariantViolation);
            }
        }
        Ok(())
    }

    fn validate_free_loan_slots(&self) -> Result<(), StructuralRootTableError> {
        for (position, slot) in self.free_loans.iter().enumerate() {
            let index =
                usize::try_from(*slot).map_err(|_| StructuralRootTableError::InvariantViolation)?;
            if !matches!(self.loans.get(index), Some(LoanSlot::Vacant { .. }))
                || self.free_loans[..position].contains(slot)
            {
                return Err(StructuralRootTableError::InvariantViolation);
            }
        }
        Ok(())
    }
}
