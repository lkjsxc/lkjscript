use super::{PoolId, PoolPartition, PoolSlotState, TypedPool};
use crate::structural::{RootClass, StructuralError};

impl<T> TypedPool<T> {
    pub fn get(&self, id: PoolId<T>) -> Result<&T, StructuralError> {
        let index = self.validate_id(id)?;
        self.slots[index]
            .value
            .as_ref()
            .ok_or(StructuralError::SlotVacant)
    }

    pub fn get_mut(&mut self, id: PoolId<T>) -> Result<&mut T, StructuralError> {
        let index = self.validate_id(id)?;
        self.slots[index]
            .value
            .as_mut()
            .ok_or(StructuralError::SlotVacant)
    }

    pub fn get_in_partition(
        &self,
        partition: &PoolPartition<T>,
        id: PoolId<T>,
    ) -> Result<&T, StructuralError> {
        if !partition.contains(id) {
            return Err(StructuralError::WrongPartition);
        }
        self.get(id)
    }

    pub fn validate(&self) -> Result<(), StructuralError> {
        for &slot in &self.free {
            let entry = self
                .slots
                .get(slot as usize)
                .ok_or(StructuralError::SlotVacant)?;
            if entry.state != PoolSlotState::Vacant || entry.value.is_some() {
                return Err(StructuralError::SlotVacant);
            }
        }
        let live = self
            .slots
            .iter()
            .filter(|entry| entry.state == PoolSlotState::Live)
            .count() as u64;
        if live != self.metrics.get().live_slots {
            return Err(StructuralError::ArithmeticOverflow);
        }
        Ok(())
    }

    pub(super) fn validate_id(&self, id: PoolId<T>) -> Result<usize, StructuralError> {
        let key = id.key;
        if key.domain() != self.domain {
            return Err(StructuralError::WrongPool);
        }
        if key.class() != RootClass::PoolElement {
            return Err(self.stale(key));
        }
        if key.layout() != self.layout {
            return Err(StructuralError::WrongLayout);
        }
        if key.semantic_type() != self.semantic_type {
            return Err(StructuralError::WrongSemanticType);
        }
        let Some(entry) = self.slots.get(key.slot() as usize) else {
            return Err(self.stale(key));
        };
        if entry.generation != key.generation() || entry.state != PoolSlotState::Live {
            return Err(self.stale(key));
        }
        Ok(key.slot() as usize)
    }

    fn stale(&self, key: crate::structural::RootKey) -> StructuralError {
        let mut metrics = self.metrics.get();
        metrics.stale_failures = metrics.stale_failures.saturating_add(1);
        self.metrics.set(metrics);
        StructuralError::StaleRoot(key)
    }
}
