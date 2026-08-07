use std::num::NonZeroU32;

use super::model::RawUniqueKey;
use super::object::{Payload, Slot, SlotState};
use super::{ByteVectorKey, BytesKey, PathKey, UniqueStore, UniqueStoreError, UniqueStoreStats};

enum SlotPlan {
    New { index: u32 },
    Reuse { index: u32, generation: NonZeroU32 },
}

impl UniqueStore {
    pub fn allocate_byte_vector(
        &mut self,
        bytes: Vec<u8>,
    ) -> Result<ByteVectorKey, UniqueStoreError> {
        self.allocate(Payload::ByteVector(bytes))
            .map(ByteVectorKey::from_raw)
    }

    /// Checks explicit store policy before allocating a byte-vector backing buffer.
    /// The actual retained capacity is checked again when the buffer is published.
    pub fn check_byte_vector_allocation(&self, bytes: usize) -> Result<(), UniqueStoreError> {
        let bytes = u64::try_from(bytes).map_err(|_| UniqueStoreError::StorageCapacity)?;
        self.check_allocation(bytes)
    }

    pub fn allocate_bytes(&mut self, bytes: Vec<u8>) -> Result<BytesKey, UniqueStoreError> {
        self.allocate(Payload::Bytes(bytes)).map(BytesKey::from_raw)
    }

    pub fn allocate_path(&mut self, bytes: Box<[u8]>) -> Result<PathKey, UniqueStoreError> {
        self.allocate(Payload::Path(bytes)).map(PathKey::from_raw)
    }

    pub(super) fn allocate(&mut self, payload: Payload) -> Result<RawUniqueKey, UniqueStoreError> {
        let retained = payload.retained_bytes()?;
        let (slot_plan, next_stats) = self.preflight_allocation(retained)?;
        if matches!(slot_plan, SlotPlan::New { .. }) {
            self.slots
                .try_reserve(1)
                .map_err(|_| UniqueStoreError::StorageCapacity)?;
        }
        let (index, generation) = match slot_plan {
            SlotPlan::New { index } => {
                let generation = NonZeroU32::MIN;
                self.slots.push(Slot::occupied(generation, payload));
                (index, generation)
            }
            SlotPlan::Reuse { index, generation } => {
                let Some(slot) = self.slots.get_mut(index as usize) else {
                    return Err(UniqueStoreError::ArithmeticOverflow);
                };
                let next = match &slot.state {
                    SlotState::Vacant { next } => *next,
                    _ => return Err(UniqueStoreError::ArithmeticOverflow),
                };
                self.free_head = next;
                slot.generation = generation;
                slot.state = SlotState::Occupied(payload);
                (index, generation)
            }
        };
        self.stats = next_stats;
        Ok(RawUniqueKey {
            store: self.id,
            index,
            generation,
        })
    }

    pub(super) fn check_allocation(&self, retained: u64) -> Result<(), UniqueStoreError> {
        self.preflight_allocation(retained).map(|_| ())
    }

    fn preflight_allocation(
        &self,
        retained: u64,
    ) -> Result<(SlotPlan, UniqueStoreStats), UniqueStoreError> {
        let mut next = self.stats;
        next.allocations = next
            .allocations
            .checked_add(1)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        if next.allocations > self.limits.max_allocations() {
            return Err(UniqueStoreError::AllocationLimit);
        }
        next.live_objects = next
            .live_objects
            .checked_add(1)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        if next.live_objects > self.limits.max_objects() {
            return Err(UniqueStoreError::ObjectLimit);
        }
        next.live_bytes = next
            .live_bytes
            .checked_add(retained)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        if next.live_bytes > self.limits.max_bytes() {
            return Err(UniqueStoreError::ByteLimit);
        }
        next.allocated_bytes = next
            .allocated_bytes
            .checked_add(retained)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        next.peak_live_objects = next.peak_live_objects.max(next.live_objects);
        next.peak_live_bytes = next.peak_live_bytes.max(next.live_bytes);
        let plan = self.preflight_slot(&mut next)?;
        Ok((plan, next))
    }

    fn preflight_slot(
        &self,
        next_stats: &mut UniqueStoreStats,
    ) -> Result<SlotPlan, UniqueStoreError> {
        let Some(index) = self.free_head else {
            let index = u32::try_from(self.slots.len()).map_err(|_| UniqueStoreError::SlotLimit)?;
            if index >= self.limits.max_slots() {
                return Err(UniqueStoreError::SlotLimit);
            }
            return Ok(SlotPlan::New { index });
        };
        let slot = self
            .slots
            .get(index as usize)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        if !matches!(&slot.state, SlotState::Vacant { .. }) {
            return Err(UniqueStoreError::ArithmeticOverflow);
        }
        let value = slot
            .generation
            .get()
            .checked_add(1)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        let generation = NonZeroU32::new(value).ok_or(UniqueStoreError::ArithmeticOverflow)?;
        if generation > self.limits.max_generation() {
            return Err(UniqueStoreError::ArithmeticOverflow);
        }
        next_stats.reused_slots = next_stats
            .reused_slots
            .checked_add(1)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        Ok(SlotPlan::Reuse { index, generation })
    }
}
