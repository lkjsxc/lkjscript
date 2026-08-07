use super::{
    LoanSlot, RootSlot, StructuralRootOwnership, StructuralRootTable, StructuralRootTableError,
};

impl StructuralRootTable {
    pub fn validate(&self) -> Result<(), StructuralRootTableError> {
        let live_roots = u64::try_from(
            self.roots
                .iter()
                .filter(|slot| matches!(slot, RootSlot::Live { .. }))
                .count(),
        )
        .map_err(|_| StructuralRootTableError::InvariantViolation)?;
        let live_loans = u64::try_from(
            self.loans
                .iter()
                .filter(|slot| matches!(slot, LoanSlot::Live { .. }))
                .count(),
        )
        .map_err(|_| StructuralRootTableError::InvariantViolation)?;
        if live_roots != self.stats.live_roots || live_loans != self.stats.live_loans {
            return Err(StructuralRootTableError::InvariantViolation);
        }
        self.validate_free_root_slots()?;
        self.validate_free_loan_slots()?;
        let exclusive_count = self
            .roots
            .iter()
            .filter(|slot| {
                matches!(
                    slot,
                    RootSlot::Live { value, .. }
                        if value.ownership != StructuralRootOwnership::SealedShared
                )
            })
            .count();
        if exclusive_count != self.exclusive_roots.len() {
            return Err(StructuralRootTableError::InvariantViolation);
        }
        for (slot, root) in self.roots.iter().enumerate() {
            let RootSlot::Live {
                generation,
                key,
                value,
            } = root
            else {
                continue;
            };
            let slot =
                u64::try_from(slot).map_err(|_| StructuralRootTableError::InvariantViolation)?;
            let token = self.value_token(*key)?;
            if token.slot != slot
                || token.generation != *generation
                || value.root.domain().runtime() != self.runtime
                || (value.ownership != StructuralRootOwnership::SealedShared
                    && !self.exclusive_roots.contains(&value.root))
            {
                return Err(StructuralRootTableError::InvariantViolation);
            }
            let (shared, exclusive) = self.loans.iter().fold((0_u64, false), |counts, loan| {
                let LoanSlot::Live { value, .. } = loan else {
                    return counts;
                };
                if value.root != *key {
                    return counts;
                }
                (
                    counts.0 + u64::from(!value.exclusive),
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
        for (slot, loan) in self.loans.iter().enumerate() {
            let LoanSlot::Live {
                generation, key, ..
            } = loan
            else {
                continue;
            };
            let token = self.borrow_token(*key)?;
            if token.slot
                != u64::try_from(slot).map_err(|_| StructuralRootTableError::InvariantViolation)?
                || token.generation != *generation
            {
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
        let mut seen = validation_flags(self.roots.len())?;
        for slot in &self.free_roots {
            let index =
                usize::try_from(*slot).map_err(|_| StructuralRootTableError::InvariantViolation)?;
            let duplicate = *seen
                .get(index)
                .ok_or(StructuralRootTableError::InvariantViolation)?;
            if !matches!(self.roots.get(index), Some(RootSlot::Vacant { .. })) || duplicate {
                return Err(StructuralRootTableError::InvariantViolation);
            }
            seen[index] = true;
        }
        Ok(())
    }

    fn validate_free_loan_slots(&self) -> Result<(), StructuralRootTableError> {
        let mut seen = validation_flags(self.loans.len())?;
        for slot in &self.free_loans {
            let index =
                usize::try_from(*slot).map_err(|_| StructuralRootTableError::InvariantViolation)?;
            let duplicate = *seen
                .get(index)
                .ok_or(StructuralRootTableError::InvariantViolation)?;
            if !matches!(self.loans.get(index), Some(LoanSlot::Vacant { .. })) || duplicate {
                return Err(StructuralRootTableError::InvariantViolation);
            }
            seen[index] = true;
        }
        Ok(())
    }
}

fn validation_flags(length: usize) -> Result<Vec<bool>, StructuralRootTableError> {
    let mut flags = Vec::new();
    flags
        .try_reserve_exact(length)
        .map_err(|_| StructuralRootTableError::AllocationFailed)?;
    flags.resize(length, false);
    Ok(flags)
}
