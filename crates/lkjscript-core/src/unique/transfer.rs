use super::object::SlotState;
use super::{ByteVectorKey, BytesKey, UniqueLayout, UniqueStore, UniqueStoreError};

impl UniqueStore {
    pub fn freeze_byte_vector(&mut self, key: ByteVectorKey) -> Result<BytesKey, UniqueStoreError> {
        let raw = key.raw();
        let index = self.locate(raw, UniqueLayout::ByteVector)?;
        let transfers = self
            .stats
            .transfers
            .checked_add(1)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        let SlotState::Occupied(payload) = &mut self.slots[index].state else {
            return Err(UniqueStoreError::ArithmeticOverflow);
        };
        payload.freeze();
        self.stats.transfers = transfers;
        Ok(BytesKey::from_raw(raw))
    }

    pub fn thaw_dynamic_bytes(&mut self, key: BytesKey) -> Result<ByteVectorKey, UniqueStoreError> {
        let raw = key.raw();
        let index = self.locate(raw, UniqueLayout::Bytes)?;
        let transfers = self
            .stats
            .transfers
            .checked_add(1)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        let SlotState::Occupied(payload) = &mut self.slots[index].state else {
            return Err(UniqueStoreError::ArithmeticOverflow);
        };
        payload.thaw();
        self.stats.transfers = transfers;
        Ok(ByteVectorKey::from_raw(raw))
    }
}
