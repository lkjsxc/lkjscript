use std::num::NonZeroU64;

use super::model::RawUniqueKey;
use super::object::{Payload, Slot, SlotState};
use super::{ByteVectorKey, BytesKey, PathKey, UniqueStore, UniqueStoreError, UniqueStoreStats};

enum SlotPlan {
    New { index: u64 },
    Reuse { index: u64, generation: NonZeroU64 },
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
        self.tokens
            .try_reserve(1)
            .map_err(|_| UniqueStoreError::StorageCapacity)?;
        let word = self.next_token()?;
        if self.tokens.contains_key(&word) {
            return Err(UniqueStoreError::ArithmeticOverflow);
        }
        let (index, generation) = match slot_plan {
            SlotPlan::New { index } => {
                let generation = NonZeroU64::MIN;
                self.slots.push(Slot::occupied(generation, payload));
                (index, generation)
            }
            SlotPlan::Reuse { index, generation } => {
                let host_index =
                    usize::try_from(index).map_err(|_| UniqueStoreError::ArithmeticOverflow)?;
                let Some(slot) = self.slots.get_mut(host_index) else {
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
        let key = RawUniqueKey {
            store: self.id,
            index,
            generation,
            word,
        };
        self.tokens.insert(word, key);
        self.stats = next_stats;
        Ok(key)
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
        next.live_objects = next
            .live_objects
            .checked_add(1)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        next.live_bytes = next
            .live_bytes
            .checked_add(retained)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
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
            let index = u64::try_from(self.slots.len())
                .map_err(|_| UniqueStoreError::ArithmeticOverflow)?;
            return Ok(SlotPlan::New { index });
        };
        let slot = self
            .slots
            .get(usize::try_from(index).map_err(|_| UniqueStoreError::ArithmeticOverflow)?)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        if !matches!(&slot.state, SlotState::Vacant { .. }) {
            return Err(UniqueStoreError::ArithmeticOverflow);
        }
        let value = slot
            .generation
            .get()
            .checked_add(1)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        let generation = NonZeroU64::new(value).ok_or(UniqueStoreError::ArithmeticOverflow)?;
        next_stats.reused_slots = next_stats
            .reused_slots
            .checked_add(1)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        Ok(SlotPlan::Reuse { index, generation })
    }
}
