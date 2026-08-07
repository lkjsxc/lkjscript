use std::num::NonZeroU32;

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
        let will_reuse = !self.free_roots.is_empty();
        let next_reused = self
            .stats
            .root_slots_reused
            .checked_add(u64::from(will_reuse))
            .ok_or(StructuralRootTableError::ArithmeticOverflow)?;
        let (slot, generation) = self.allocate_root_slot()?;
        let index =
            usize::try_from(slot).map_err(|_| StructuralRootTableError::ArithmeticOverflow)?;
        self.roots[index] = RootSlot::Live {
            generation,
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
        Ok(StructuralValueKey::from_parts(slot, generation))
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

    fn allocate_root_slot(&mut self) -> Result<(u32, NonZeroU32), StructuralRootTableError> {
        if let Some(slot) = self.free_roots.pop() {
            let index =
                usize::try_from(slot).map_err(|_| StructuralRootTableError::ArithmeticOverflow)?;
            let RootSlot::Vacant { generation, .. } = self.roots[index] else {
                return Err(StructuralRootTableError::InvariantViolation);
            };
            return Ok((slot, generation));
        }
        let slot = u32::try_from(self.roots.len())
            .map_err(|_| StructuralRootTableError::ArithmeticOverflow)?;
        self.roots
            .try_reserve(1)
            .map_err(|_| StructuralRootTableError::AllocationFailed)?;
        let generation = NonZeroU32::new(1).ok_or(StructuralRootTableError::InvariantViolation)?;
        self.roots.push(RootSlot::Vacant {
            generation,
            previous: None,
        });
        Ok((slot, generation))
    }
}
