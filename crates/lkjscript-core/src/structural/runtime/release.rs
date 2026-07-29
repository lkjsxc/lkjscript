use std::num::NonZeroU32;

use super::{SlotState, StructuralRuntime};
use crate::structural::{DomainKey, StructuralError};

impl StructuralRuntime {
    pub(in crate::structural) fn preflight_release(
        &mut self,
        keys: &[DomainKey],
    ) -> Result<(), StructuralError> {
        let mut reusable = 0_usize;
        for (index, &key) in keys.iter().enumerate() {
            self.require_live(key)?;
            if keys[..index].contains(&key) {
                return Err(StructuralError::DuplicateDependency);
            }
            if key.generation().get() < self.limits.max_generation {
                reusable = reusable
                    .checked_add(1)
                    .ok_or(StructuralError::ArithmeticOverflow)?;
            }
        }
        self.free
            .try_reserve(reusable)
            .map_err(|_| StructuralError::AllocationFailed)
    }

    pub(in crate::structural) fn release(&mut self, key: DomainKey) -> Result<(), StructuralError> {
        self.require_live(key)?;
        let index = key.slot() as usize;
        let generation = self.slots[index].generation.get();
        if generation >= self.limits.max_generation {
            self.slots[index].state = SlotState::Retired;
            self.metrics.slots_retired = self.metrics.slots_retired.saturating_add(1);
        } else {
            let next = generation
                .checked_add(1)
                .and_then(NonZeroU32::new)
                .ok_or(StructuralError::GenerationExhausted)?;
            self.slots[index].generation = next;
            self.slots[index].state = SlotState::Vacant;
            self.free.push(key.slot());
        }
        self.metrics.domains_released = self.metrics.domains_released.saturating_add(1);
        self.metrics.live_domains = self.metrics.live_domains.saturating_sub(1);
        Ok(())
    }
}
