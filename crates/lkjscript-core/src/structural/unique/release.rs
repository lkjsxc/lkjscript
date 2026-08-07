use super::object::SlotState;
use super::{
    ByteVectorKey, BytesKey, PathKey, UniqueLayout, UniqueStore, UniqueStoreError, UniqueStoreStats,
};

impl UniqueStore {
    pub fn free_byte_vector(&mut self, key: ByteVectorKey) -> Result<(), UniqueStoreError> {
        self.release(key.raw(), UniqueLayout::ByteVector)
    }

    /// Transfers byte-vector backing out of this store while discharging the
    /// exact slot obligation. The returned allocation has one Rust owner and
    /// the key is stale after success.
    pub fn take_byte_vector(&mut self, key: ByteVectorKey) -> Result<Vec<u8>, UniqueStoreError> {
        let raw = key.raw();
        let index = self.locate(raw, UniqueLayout::ByteVector)?;
        let retained = match &self.slots[index].state {
            SlotState::Occupied(payload) => payload.retained_bytes()?,
            _ => return Err(UniqueStoreError::ArithmeticOverflow),
        };
        let retire = raw.generation.get() == u32::MAX;
        let next_stats = self.preflight_release(retained, retire)?;
        let replacement = if retire {
            SlotState::Retired
        } else {
            SlotState::Vacant {
                next: self.free_head,
            }
        };
        let removed = std::mem::replace(&mut self.slots[index].state, replacement);
        let super::object::Payload::ByteVector(bytes) = (match removed {
            SlotState::Occupied(payload) => payload,
            state => {
                self.slots[index].state = state;
                return Err(UniqueStoreError::ArithmeticOverflow);
            }
        }) else {
            return Err(UniqueStoreError::ArithmeticOverflow);
        };
        if !retire {
            self.free_head = Some(raw.index);
        }
        self.stats = next_stats;
        Ok(bytes)
    }

    pub fn free_bytes(&mut self, key: BytesKey) -> Result<(), UniqueStoreError> {
        self.release(key.raw(), UniqueLayout::Bytes)
    }

    pub fn take_bytes(&mut self, key: BytesKey) -> Result<Vec<u8>, UniqueStoreError> {
        let raw = key.raw();
        let index = self.locate(raw, UniqueLayout::Bytes)?;
        let retained = match &self.slots[index].state {
            SlotState::Occupied(payload) => payload.retained_bytes()?,
            _ => return Err(UniqueStoreError::ArithmeticOverflow),
        };
        let retire = raw.generation.get() == u32::MAX;
        let next_stats = self.preflight_release(retained, retire)?;
        let replacement = if retire {
            SlotState::Retired
        } else {
            SlotState::Vacant {
                next: self.free_head,
            }
        };
        let removed = std::mem::replace(&mut self.slots[index].state, replacement);
        let super::object::Payload::Bytes(bytes) = (match removed {
            SlotState::Occupied(payload) => payload,
            state => {
                self.slots[index].state = state;
                return Err(UniqueStoreError::ArithmeticOverflow);
            }
        }) else {
            return Err(UniqueStoreError::ArithmeticOverflow);
        };
        if !retire {
            self.free_head = Some(raw.index);
        }
        self.stats = next_stats;
        Ok(bytes)
    }

    pub fn free_path(&mut self, key: PathKey) -> Result<(), UniqueStoreError> {
        self.release(key.raw(), UniqueLayout::Path)
    }

    /// Transfers immutable path bytes across an execution boundary while
    /// discharging the exact path slot. The key is stale after success.
    pub fn take_path(&mut self, key: PathKey) -> Result<Box<[u8]>, UniqueStoreError> {
        let raw = key.raw();
        let index = self.locate(raw, UniqueLayout::Path)?;
        let retained = match &self.slots[index].state {
            SlotState::Occupied(payload) => payload.retained_bytes()?,
            _ => return Err(UniqueStoreError::ArithmeticOverflow),
        };
        let retire = raw.generation.get() == u32::MAX;
        let next_stats = self.preflight_release(retained, retire)?;
        let replacement = if retire {
            SlotState::Retired
        } else {
            SlotState::Vacant {
                next: self.free_head,
            }
        };
        let removed = std::mem::replace(&mut self.slots[index].state, replacement);
        let super::object::Payload::Path(bytes) = (match removed {
            SlotState::Occupied(payload) => payload,
            state => {
                self.slots[index].state = state;
                return Err(UniqueStoreError::ArithmeticOverflow);
            }
        }) else {
            return Err(UniqueStoreError::ArithmeticOverflow);
        };
        if !retire {
            self.free_head = Some(raw.index);
        }
        self.stats = next_stats;
        Ok(bytes)
    }

    fn release(
        &mut self,
        key: super::model::RawUniqueKey,
        expected: UniqueLayout,
    ) -> Result<(), UniqueStoreError> {
        let index = self.locate(key, expected)?;
        let retained = match &self.slots[index].state {
            SlotState::Occupied(payload) => payload.retained_bytes()?,
            _ => return Err(UniqueStoreError::ArithmeticOverflow),
        };
        let retire = key.generation.get() == u32::MAX;
        let next_stats = self.preflight_release(retained, retire)?;
        let replacement = if retire {
            SlotState::Retired
        } else {
            SlotState::Vacant {
                next: self.free_head,
            }
        };
        let removed = std::mem::replace(&mut self.slots[index].state, replacement);
        let payload = match removed {
            SlotState::Occupied(payload) => payload,
            state => {
                self.slots[index].state = state;
                return Err(UniqueStoreError::ArithmeticOverflow);
            }
        };
        if !retire {
            self.free_head = Some(key.index);
        }
        self.stats = next_stats;
        drop(payload);
        Ok(())
    }

    fn preflight_release(
        &self,
        retained: u64,
        retire: bool,
    ) -> Result<UniqueStoreStats, UniqueStoreError> {
        let mut next = self.stats;
        next.frees = next
            .frees
            .checked_add(1)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        next.live_objects = next
            .live_objects
            .checked_sub(1)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        next.live_bytes = next
            .live_bytes
            .checked_sub(retained)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        if retire {
            next.retired_slots = next
                .retired_slots
                .checked_add(1)
                .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        }
        Ok(next)
    }
}
