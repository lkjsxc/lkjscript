use std::num::NonZeroU64;

use super::{
    LiveLoan, LoanSlot, RootSlot, StructuralBorrow, StructuralBorrowKey, StructuralRootOwnership,
    StructuralRootTable, StructuralRootTableError, StructuralRootTableStats, StructuralValueKey,
};

impl StructuralRootTable {
    pub fn borrow_shared(
        &mut self,
        root: StructuralValueKey,
    ) -> Result<StructuralBorrow, StructuralRootTableError> {
        let root_index = self.live_root_index(root)?;
        let RootSlot::Live { value, .. } = self.roots[root_index] else {
            return Err(StructuralRootTableError::InvariantViolation);
        };
        if value.exclusive_loan {
            return Err(StructuralRootTableError::BorrowConflict);
        }
        let shared = value
            .shared_loans
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let next_stats = self.next_started_loan_stats()?;
        let key = self.allocate_loan(root, false)?;
        let RootSlot::Live { value, .. } = &mut self.roots[root_index] else {
            return Err(StructuralRootTableError::InvariantViolation);
        };
        value.shared_loans = shared;
        self.stats = next_stats;
        Ok(StructuralBorrow::new(key, root, false))
    }

    pub fn borrow_exclusive(
        &mut self,
        root: StructuralValueKey,
    ) -> Result<StructuralBorrow, StructuralRootTableError> {
        let root_index = self.live_root_index(root)?;
        let RootSlot::Live { value, .. } = self.roots[root_index] else {
            return Err(StructuralRootTableError::InvariantViolation);
        };
        if value.ownership != StructuralRootOwnership::Owned
            || value.exclusive_loan
            || value.shared_loans != 0
        {
            return Err(StructuralRootTableError::BorrowConflict);
        }
        let next_stats = self.next_started_loan_stats()?;
        let key = self.allocate_loan(root, true)?;
        let RootSlot::Live { value, .. } = &mut self.roots[root_index] else {
            return Err(StructuralRootTableError::InvariantViolation);
        };
        value.exclusive_loan = true;
        self.stats = next_stats;
        Ok(StructuralBorrow::new(key, root, true))
    }

    pub fn end_borrow(&mut self, borrow: StructuralBorrow) -> Result<(), StructuralRootTableError> {
        let token = self.borrow_token(borrow.key())?;
        let loan_index = usize::try_from(token.slot)
            .map_err(|_| StructuralRootTableError::ArithmeticOverflow)?;
        let (generation, loan) = match self.loans.get(loan_index) {
            Some(LoanSlot::Live {
                generation,
                key,
                value,
            }) if *key == borrow.key() && *generation == token.generation => (*generation, *value),
            _ => return Err(StructuralRootTableError::StaleLoan),
        };
        if loan.root != borrow.root() || loan.exclusive != borrow.is_exclusive() {
            return Err(StructuralRootTableError::StaleLoan);
        }
        let root_index = self.live_root_index(loan.root)?;
        let RootSlot::Live { value, .. } = self.roots[root_index] else {
            return Err(StructuralRootTableError::InvariantViolation);
        };
        if loan.exclusive && !value.exclusive_loan {
            return Err(StructuralRootTableError::InvariantViolation);
        }
        if !loan.exclusive && value.shared_loans == 0 {
            return Err(StructuralRootTableError::InvariantViolation);
        }
        let retires = generation.get() == u64::MAX;
        let next_live = self
            .stats
            .live_loans
            .checked_sub(1)
            .ok_or(StructuralRootTableError::InvariantViolation)?;
        let next_ended = self
            .stats
            .loans_ended
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let next_retired = self
            .stats
            .loan_slots_retired
            .checked_add(u64::from(retires))
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let next = generation.get().checked_add(1).and_then(NonZeroU64::new);
        if next.is_some() {
            self.free_loans
                .try_reserve(1)
                .map_err(|_| StructuralRootTableError::AllocationFailed)?;
        }
        self.loans[loan_index] = match next {
            Some(next) => LoanSlot::Vacant { generation: next },
            None => LoanSlot::Retired,
        };
        if next.is_some() {
            self.free_loans.push(token.slot);
        }
        let RootSlot::Live { value, .. } = &mut self.roots[root_index] else {
            return Err(StructuralRootTableError::InvariantViolation);
        };
        if loan.exclusive {
            value.exclusive_loan = false;
        } else {
            value.shared_loans -= 1;
        }
        self.tokens.remove(&borrow.key().get());
        self.stats.live_loans = next_live;
        self.stats.loans_ended = next_ended;
        self.stats.loan_slots_retired = next_retired;
        Ok(())
    }

    fn allocate_loan(
        &mut self,
        root: StructuralValueKey,
        exclusive: bool,
    ) -> Result<StructuralBorrowKey, StructuralRootTableError> {
        let value = LiveLoan { root, exclusive };
        let (slot, generation, reused) = if let Some(&slot) = self.free_loans.last() {
            let index =
                usize::try_from(slot).map_err(|_| StructuralRootTableError::ArithmeticOverflow)?;
            let LoanSlot::Vacant { generation } = self
                .loans
                .get(index)
                .ok_or(StructuralRootTableError::InvariantViolation)?
            else {
                return Err(StructuralRootTableError::InvariantViolation);
            };
            (slot, *generation, true)
        } else {
            self.loans
                .try_reserve(1)
                .map_err(|_| StructuralRootTableError::AllocationFailed)?;
            let slot = u64::try_from(self.loans.len())
                .map_err(|_| StructuralRootTableError::ArithmeticOverflow)?;
            (slot, NonZeroU64::MIN, false)
        };
        let key = self.allocate_borrow_token(slot, generation)?;
        let index =
            usize::try_from(slot).map_err(|_| StructuralRootTableError::ArithmeticOverflow)?;
        if reused {
            if self.free_loans.pop() != Some(slot) {
                return Err(StructuralRootTableError::InvariantViolation);
            }
            self.loans[index] = LoanSlot::Live {
                generation,
                key,
                value,
            };
        } else {
            self.loans.push(LoanSlot::Live {
                generation,
                key,
                value,
            });
        }
        Ok(key)
    }

    fn next_started_loan_stats(
        &self,
    ) -> Result<StructuralRootTableStats, StructuralRootTableError> {
        let mut stats = self.stats;
        stats.loans_started = stats
            .loans_started
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        stats.live_loans = stats
            .live_loans
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        stats.peak_live_loans = stats.peak_live_loans.max(stats.live_loans);
        stats.loan_slots_reused = stats
            .loan_slots_reused
            .checked_add(u64::from(!self.free_loans.is_empty()))
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        Ok(stats)
    }
}
