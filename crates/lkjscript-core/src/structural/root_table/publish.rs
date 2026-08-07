use std::num::NonZeroU64;

use super::super::{DomainClass, RootKey};
use super::{
    LiveRoot, RootSlot, StructuralRootOwnership, StructuralRootTable, StructuralRootTableError,
    StructuralValueKey,
};

impl StructuralRootTable {
    pub fn publish(
        &mut self,
        root: RootKey,
        ownership: StructuralRootOwnership,
    ) -> Result<StructuralValueKey, StructuralRootTableError> {
        self.validate_publication(root, ownership)?;
        if ownership != StructuralRootOwnership::SealedShared {
            self.exclusive_roots
                .try_reserve(1)
                .map_err(|_| StructuralRootTableError::AllocationFailed)?;
        }
        let next_published = self
            .stats
            .roots_published
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let next_live = self
            .stats
            .live_roots
            .checked_add(1)
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let (slot, generation, reused) = self.prepare_root_slot()?;
        let next_reused = self
            .stats
            .root_slots_reused
            .checked_add(u64::from(reused))
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let key = self.allocate_value_token(slot, generation)?;
        let index =
            usize::try_from(slot).map_err(|_| StructuralRootTableError::ArithmeticOverflow)?;
        if reused {
            if self.free_roots.pop() != Some(slot) {
                return Err(StructuralRootTableError::InvariantViolation);
            }
        } else {
            self.roots.push(RootSlot::Vacant {
                generation,
                previous: None,
            });
        }
        self.roots[index] = RootSlot::Live {
            generation,
            key,
            value: LiveRoot {
                root,
                ownership,
                shared_loans: 0,
                exclusive_loan: false,
            },
        };
        if ownership != StructuralRootOwnership::SealedShared && !self.exclusive_roots.insert(root)
        {
            return Err(StructuralRootTableError::InvariantViolation);
        }
        self.stats.roots_published = next_published;
        self.stats.live_roots = next_live;
        self.stats.peak_live_roots = self.stats.peak_live_roots.max(next_live);
        self.stats.root_slots_reused = next_reused;
        Ok(key)
    }

    fn validate_publication(
        &self,
        root: RootKey,
        ownership: StructuralRootOwnership,
    ) -> Result<(), StructuralRootTableError> {
        if root.domain().runtime() != self.runtime {
            return Err(StructuralRootTableError::WrongRuntime);
        }
        let class = root.domain().class();
        let valid = match ownership {
            StructuralRootOwnership::Owned => {
                !matches!(class, DomainClass::Static | DomainClass::RegionSealed)
            }
            StructuralRootOwnership::Static => class == DomainClass::Static,
            StructuralRootOwnership::SealedShared => class == DomainClass::RegionSealed,
        };
        if !valid {
            return Err(StructuralRootTableError::WrongOwnership);
        }
        if ownership != StructuralRootOwnership::SealedShared
            && self.exclusive_roots.contains(&root)
        {
            return Err(StructuralRootTableError::DuplicateOwner);
        }
        Ok(())
    }

    fn prepare_root_slot(&mut self) -> Result<(u64, NonZeroU64, bool), StructuralRootTableError> {
        if let Some(&slot) = self.free_roots.last() {
            let index =
                usize::try_from(slot).map_err(|_| StructuralRootTableError::ArithmeticOverflow)?;
            let RootSlot::Vacant { generation, .. } = self
                .roots
                .get(index)
                .ok_or(StructuralRootTableError::InvariantViolation)?
            else {
                return Err(StructuralRootTableError::InvariantViolation);
            };
            return Ok((slot, *generation, true));
        }
        let slot = u64::try_from(self.roots.len())
            .map_err(|_| StructuralRootTableError::ArithmeticOverflow)?;
        self.roots
            .try_reserve(1)
            .map_err(|_| StructuralRootTableError::AllocationFailed)?;
        Ok((slot, NonZeroU64::MIN, false))
    }
}
