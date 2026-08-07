use std::num::NonZeroU64;

use super::super::RootKey;
use super::{
    RootSlot, StructuralRootOwnership, StructuralRootTable, StructuralRootTableError,
    StructuralValueKey, TerminalState,
};

impl StructuralRootTable {
    pub fn take_owned(
        &mut self,
        key: StructuralValueKey,
    ) -> Result<RootKey, StructuralRootTableError> {
        let next_moved = self
            .stats
            .roots_moved
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let root = self.remove(key, StructuralRootOwnership::Owned, TerminalState::Moved)?;
        self.stats.roots_moved = next_moved;
        Ok(root)
    }

    pub fn drop_owned(
        &mut self,
        key: StructuralValueKey,
    ) -> Result<RootKey, StructuralRootTableError> {
        let next_dropped = self
            .stats
            .roots_dropped
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let root = self.remove(key, StructuralRootOwnership::Owned, TerminalState::Dropped)?;
        self.stats.roots_dropped = next_dropped;
        Ok(root)
    }

    pub fn release_sealed(
        &mut self,
        key: StructuralValueKey,
    ) -> Result<RootKey, StructuralRootTableError> {
        let next_released = self
            .stats
            .roots_released
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let root = self.remove(
            key,
            StructuralRootOwnership::SealedShared,
            TerminalState::Dropped,
        )?;
        self.stats.roots_released = next_released;
        Ok(root)
    }

    pub fn unregister_static(
        &mut self,
        key: StructuralValueKey,
    ) -> Result<RootKey, StructuralRootTableError> {
        self.remove(key, StructuralRootOwnership::Static, TerminalState::Dropped)
    }

    fn remove(
        &mut self,
        key: StructuralValueKey,
        ownership: StructuralRootOwnership,
        terminal: TerminalState,
    ) -> Result<RootKey, StructuralRootTableError> {
        let token = self.value_token(key)?;
        let index = self.live_root_index(key)?;
        let RootSlot::Live {
            generation,
            key: current,
            value,
        } = self.roots[index]
        else {
            return Err(StructuralRootTableError::InvariantViolation);
        };
        if current != key || generation != token.generation {
            return Err(StructuralRootTableError::InvariantViolation);
        }
        if value.ownership != ownership {
            return Err(StructuralRootTableError::WrongOwnership);
        }
        if value.shared_loans != 0 || value.exclusive_loan {
            return Err(StructuralRootTableError::LiveLoan);
        }
        if ownership != StructuralRootOwnership::SealedShared
            && !self.exclusive_roots.contains(&value.root)
        {
            return Err(StructuralRootTableError::InvariantViolation);
        }
        let retires = generation.get() == u64::MAX;
        let next_live = self
            .stats
            .live_roots
            .checked_sub(1)
            .ok_or(StructuralRootTableError::InvariantViolation)?;
        let next_retired = self
            .stats
            .root_slots_retired
            .checked_add(u64::from(retires))
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let next = generation.get().checked_add(1).and_then(NonZeroU64::new);
        if next.is_some() {
            self.free_roots
                .try_reserve(1)
                .map_err(|_| StructuralRootTableError::AllocationFailed)?;
        }
        self.roots[index] = match next {
            Some(next) => RootSlot::Vacant {
                generation: next,
                previous: Some((key, generation, terminal)),
            },
            None => RootSlot::Retired { generation, key },
        };
        if next.is_some() {
            self.free_roots.push(token.slot);
        }
        if ownership != StructuralRootOwnership::SealedShared {
            self.exclusive_roots.remove(&value.root);
        }
        self.stats.live_roots = next_live;
        self.stats.root_slots_retired = next_retired;
        Ok(value.root)
    }
}
