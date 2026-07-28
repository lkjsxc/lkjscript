use super::model::RawUniqueKey;
use super::object::{Payload, SlotState};
use super::{ByteVectorKey, BytesKey, PathKey, UniqueLayout, UniqueStore, UniqueStoreError};

impl UniqueStore {
    pub fn byte_vector(&mut self, key: ByteVectorKey) -> Result<&[u8], UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::ByteVector)?;
        match &self.slots[index].state {
            SlotState::Occupied(Payload::ByteVector(bytes)) => Ok(bytes),
            _ => Err(UniqueStoreError::ArithmeticOverflow),
        }
    }

    pub fn byte_vector_mut(&mut self, key: ByteVectorKey) -> Result<&mut [u8], UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::ByteVector)?;
        match &mut self.slots[index].state {
            SlotState::Occupied(Payload::ByteVector(bytes)) => Ok(bytes),
            _ => Err(UniqueStoreError::ArithmeticOverflow),
        }
    }

    pub fn bytes(&mut self, key: BytesKey) -> Result<&[u8], UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::Bytes)?;
        match &self.slots[index].state {
            SlotState::Occupied(Payload::Bytes(bytes)) => Ok(bytes),
            _ => Err(UniqueStoreError::ArithmeticOverflow),
        }
    }

    pub fn path(&mut self, key: PathKey) -> Result<&[u8], UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::Path)?;
        match &self.slots[index].state {
            SlotState::Occupied(Payload::Path(bytes)) => Ok(bytes),
            _ => Err(UniqueStoreError::ArithmeticOverflow),
        }
    }

    pub(super) fn locate(
        &mut self,
        key: RawUniqueKey,
        expected: UniqueLayout,
    ) -> Result<usize, UniqueStoreError> {
        if key.store != self.id {
            return Err(UniqueStoreError::StoreMismatch);
        }
        let index = key.index as usize;
        let Some(slot) = self.slots.get(index) else {
            return self.reject_stale();
        };
        if slot.generation != key.generation {
            return self.reject_stale();
        }
        let SlotState::Occupied(payload) = &slot.state else {
            return self.reject_stale();
        };
        let actual = payload.layout();
        if actual != expected {
            return self.reject_layout(expected, actual);
        }
        Ok(index)
    }

    fn reject_stale<T>(&mut self) -> Result<T, UniqueStoreError> {
        self.stats.stale_failures = self
            .stats
            .stale_failures
            .checked_add(1)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        Err(UniqueStoreError::StaleKey)
    }

    fn reject_layout<T>(
        &mut self,
        expected: UniqueLayout,
        actual: UniqueLayout,
    ) -> Result<T, UniqueStoreError> {
        self.stats.wrong_layout_failures = self
            .stats
            .wrong_layout_failures
            .checked_add(1)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        Err(UniqueStoreError::WrongLayout { expected, actual })
    }
}
