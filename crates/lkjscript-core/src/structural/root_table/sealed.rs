use std::num::NonZeroU32;

use super::super::{DomainClass, RootClass, RootKey};
use super::{
    LiveRoot, RootSlot, StructuralRootOwnership, StructuralRootTable, StructuralRootTableError,
    StructuralValueKey,
};

impl StructuralRootTable {
    pub(in crate::structural) fn preflight_owned_to_sealed(
        &self,
        key: StructuralValueKey,
        sealed_root: RootKey,
    ) -> Result<(), StructuralRootTableError> {
        let index = self.live_root_index(key)?;
        let RootSlot::Live { generation, value } = self.roots[index] else {
            return Err(StructuralRootTableError::InvariantViolation);
        };
        self.validate_sealed_replacement(value, sealed_root)?;
        if value.shared_loans != 0 || value.exclusive_loan {
            return Err(StructuralRootTableError::LiveLoan);
        }
        self.next_rekey_generation(generation)?;
        self.stats
            .roots_published
            .checked_add(1)
            .and_then(|_| self.stats.roots_moved.checked_add(1))
            .and_then(|_| self.stats.root_slots_reused.checked_add(1))
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        Ok(())
    }

    pub(in crate::structural) fn replace_owned_with_sealed(
        &mut self,
        key: StructuralValueKey,
        sealed_root: RootKey,
    ) -> Result<StructuralValueKey, StructuralRootTableError> {
        self.preflight_owned_to_sealed(key, sealed_root)?;
        let index = self.live_root_index(key)?;
        let RootSlot::Live { generation, .. } = self.roots[index] else {
            return Err(StructuralRootTableError::InvariantViolation);
        };
        let next = self.next_rekey_generation(generation)?;
        let next_published = self
            .stats
            .roots_published
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let next_moved = self
            .stats
            .roots_moved
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let next_reused = self
            .stats
            .root_slots_reused
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        self.roots[index] = RootSlot::Live {
            generation: next,
            value: LiveRoot {
                root: sealed_root,
                ownership: StructuralRootOwnership::SealedShared,
                shared_loans: 0,
                exclusive_loan: false,
            },
        };
        self.stats.roots_published = next_published;
        self.stats.roots_moved = next_moved;
        self.stats.root_slots_reused = next_reused;
        Ok(StructuralValueKey::from_parts(key.slot(), next))
    }

    pub(in crate::structural) fn move_sealed(
        &mut self,
        key: StructuralValueKey,
    ) -> Result<StructuralValueKey, StructuralRootTableError> {
        let index = self.live_root_index(key)?;
        let RootSlot::Live { generation, value } = self.roots[index] else {
            return Err(StructuralRootTableError::InvariantViolation);
        };
        if value.ownership != StructuralRootOwnership::SealedShared {
            return Err(StructuralRootTableError::WrongOwnership);
        }
        if value.shared_loans != 0 || value.exclusive_loan {
            return Err(StructuralRootTableError::LiveLoan);
        }
        let next = self.next_rekey_generation(generation)?;
        let next_moved = self
            .stats
            .roots_moved
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let next_reused = self
            .stats
            .root_slots_reused
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        self.roots[index] = RootSlot::Live {
            generation: next,
            value,
        };
        self.stats.roots_moved = next_moved;
        self.stats.root_slots_reused = next_reused;
        Ok(StructuralValueKey::from_parts(key.slot(), next))
    }

    fn validate_sealed_replacement(
        &self,
        current: LiveRoot,
        sealed: RootKey,
    ) -> Result<(), StructuralRootTableError> {
        if current.ownership != StructuralRootOwnership::Owned {
            return Err(StructuralRootTableError::WrongOwnership);
        }
        let unique = current.root;
        if unique.domain().class() != DomainClass::Unique
            || sealed.domain().class() != DomainClass::RegionSealed
            || sealed.class() != RootClass::SealedPublic
            || sealed.domain().runtime() != self.runtime
            || unique.domain().runtime() != sealed.domain().runtime()
            || unique.domain().slot() != sealed.domain().slot()
            || unique.domain().generation() != sealed.domain().generation()
            || unique.slot() != sealed.slot()
            || unique.generation() != sealed.generation()
            || unique.layout() != sealed.layout()
            || unique.semantic_type() != sealed.semantic_type()
        {
            return Err(StructuralRootTableError::WrongOwnership);
        }
        Ok(())
    }

    fn next_rekey_generation(
        &self,
        generation: NonZeroU32,
    ) -> Result<NonZeroU32, StructuralRootTableError> {
        if generation.get() >= self.limits.max_generation {
            return Err(StructuralRootTableError::GenerationExhausted);
        }
        NonZeroU32::new(generation.get() + 1).ok_or(StructuralRootTableError::GenerationExhausted)
    }
}
