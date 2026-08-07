use std::ops::Range;

use super::model::RawUniqueKey;
use super::object::{Payload, SlotState};
use super::{
    ByteVectorKey, BytesKey, PathKey, UniqueKeyWord, UniqueLayout, UniqueStore, UniqueStoreError,
};

impl UniqueStore {
    pub fn import_byte_vector_key(
        &mut self,
        word: UniqueKeyWord,
    ) -> Result<ByteVectorKey, UniqueStoreError> {
        let raw = self.bind(word)?;
        self.locate(raw, UniqueLayout::ByteVector)?;
        Ok(ByteVectorKey::from_raw(raw))
    }

    pub fn import_bytes_key(&mut self, word: UniqueKeyWord) -> Result<BytesKey, UniqueStoreError> {
        let raw = self.bind(word)?;
        self.locate(raw, UniqueLayout::Bytes)?;
        Ok(BytesKey::from_raw(raw))
    }

    pub fn import_path_key(&mut self, word: UniqueKeyWord) -> Result<PathKey, UniqueStoreError> {
        let raw = self.bind(word)?;
        self.locate(raw, UniqueLayout::Path)?;
        Ok(PathKey::from_raw(raw))
    }

    pub fn byte_vector(&mut self, key: ByteVectorKey) -> Result<&[u8], UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::ByteVector)?;
        match &self.slots[index].state {
            SlotState::Occupied(Payload::ByteVector(bytes)) => Ok(bytes),
            _ => Err(UniqueStoreError::ArithmeticOverflow),
        }
    }

    pub fn byte_vector_range(
        &mut self,
        key: ByteVectorKey,
        start: usize,
        len: usize,
    ) -> Result<&[u8], UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::ByteVector)?;
        match &self.slots[index].state {
            SlotState::Occupied(Payload::ByteVector(bytes)) => {
                Ok(&bytes[checked_range(start, len, bytes.len())?])
            }
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

    pub fn byte_vector_range_mut(
        &mut self,
        key: ByteVectorKey,
        start: usize,
        len: usize,
    ) -> Result<&mut [u8], UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::ByteVector)?;
        match &mut self.slots[index].state {
            SlotState::Occupied(Payload::ByteVector(bytes)) => {
                let range = checked_range(start, len, bytes.len())?;
                Ok(&mut bytes[range])
            }
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

    pub fn bytes_range(
        &mut self,
        key: BytesKey,
        start: usize,
        len: usize,
    ) -> Result<&[u8], UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::Bytes)?;
        match &self.slots[index].state {
            SlotState::Occupied(Payload::Bytes(bytes)) => {
                Ok(&bytes[checked_range(start, len, bytes.len())?])
            }
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

    pub fn path_range(
        &mut self,
        key: PathKey,
        start: usize,
        len: usize,
    ) -> Result<&[u8], UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::Path)?;
        match &self.slots[index].state {
            SlotState::Occupied(Payload::Path(bytes)) => {
                Ok(&bytes[checked_range(start, len, bytes.len())?])
            }
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
        let index = usize::try_from(key.index).map_err(|_| UniqueStoreError::ArithmeticOverflow)?;
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

pub(super) fn checked_range(
    start: usize,
    len: usize,
    available: usize,
) -> Result<Range<usize>, UniqueStoreError> {
    let end = start
        .checked_add(len)
        .ok_or(UniqueStoreError::RangeOverflow { start, len })?;
    if end > available {
        return Err(UniqueStoreError::RangeOutOfBounds {
            start,
            len,
            available,
        });
    }
    Ok(start..end)
}
