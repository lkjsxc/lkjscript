use super::access::checked_range;
use super::object::{Payload, SlotState};
use super::{ByteVectorKey, UniqueLayout, UniqueStore, UniqueStoreError, UniqueStoreStats};

impl UniqueStore {
    pub fn resize_byte_vector(
        &mut self,
        key: ByteVectorKey,
        new_len: usize,
        value: u8,
    ) -> Result<(), UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::ByteVector)?;
        let old_capacity = match &self.slots[index].state {
            SlotState::Occupied(Payload::ByteVector(bytes)) => bytes.capacity(),
            _ => return Err(UniqueStoreError::ArithmeticOverflow),
        };
        if new_len <= old_capacity {
            let SlotState::Occupied(Payload::ByteVector(bytes)) = &mut self.slots[index].state
            else {
                return Err(UniqueStoreError::ArithmeticOverflow);
            };
            bytes.resize(new_len, value);
            return Ok(());
        }

        let old_retained =
            u64::try_from(old_capacity).map_err(|_| UniqueStoreError::ArithmeticOverflow)?;
        let requested = u64::try_from(new_len).map_err(|_| UniqueStoreError::ArithmeticOverflow)?;
        self.preflight_retained_growth(old_retained, requested)?;

        let mut replacement = Vec::new();
        replacement
            .try_reserve_exact(new_len)
            .map_err(|_| UniqueStoreError::StorageCapacity)?;
        let new_retained = u64::try_from(replacement.capacity())
            .map_err(|_| UniqueStoreError::ArithmeticOverflow)?;
        let next_stats = self.preflight_retained_growth(old_retained, new_retained)?;
        let SlotState::Occupied(Payload::ByteVector(bytes)) = &self.slots[index].state else {
            return Err(UniqueStoreError::ArithmeticOverflow);
        };
        replacement.extend_from_slice(bytes);
        replacement.resize(new_len, value);
        let SlotState::Occupied(Payload::ByteVector(bytes)) = &mut self.slots[index].state else {
            return Err(UniqueStoreError::ArithmeticOverflow);
        };
        *bytes = replacement;
        self.stats = next_stats;
        Ok(())
    }

    pub fn fill_byte_vector(
        &mut self,
        key: ByteVectorKey,
        value: u8,
    ) -> Result<(), UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::ByteVector)?;
        let SlotState::Occupied(Payload::ByteVector(bytes)) = &mut self.slots[index].state else {
            return Err(UniqueStoreError::ArithmeticOverflow);
        };
        bytes.fill(value);
        Ok(())
    }

    pub fn fill_byte_vector_range(
        &mut self,
        key: ByteVectorKey,
        start: usize,
        len: usize,
        value: u8,
    ) -> Result<(), UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::ByteVector)?;
        let available = match &self.slots[index].state {
            SlotState::Occupied(Payload::ByteVector(bytes)) => bytes.len(),
            _ => return Err(UniqueStoreError::ArithmeticOverflow),
        };
        let range = checked_range(start, len, available)?;
        let SlotState::Occupied(Payload::ByteVector(bytes)) = &mut self.slots[index].state else {
            return Err(UniqueStoreError::ArithmeticOverflow);
        };
        bytes[range].fill(value);
        Ok(())
    }

    pub fn copy_byte_vector_range(
        &mut self,
        key: ByteVectorKey,
        source_start: usize,
        destination_start: usize,
        len: usize,
    ) -> Result<(), UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::ByteVector)?;
        let available = match &self.slots[index].state {
            SlotState::Occupied(Payload::ByteVector(bytes)) => bytes.len(),
            _ => return Err(UniqueStoreError::ArithmeticOverflow),
        };
        let source = checked_range(source_start, len, available)?;
        checked_range(destination_start, len, available)?;
        let SlotState::Occupied(Payload::ByteVector(bytes)) = &mut self.slots[index].state else {
            return Err(UniqueStoreError::ArithmeticOverflow);
        };
        bytes.copy_within(source, destination_start);
        Ok(())
    }

    fn preflight_retained_growth(
        &self,
        old_retained: u64,
        new_retained: u64,
    ) -> Result<UniqueStoreStats, UniqueStoreError> {
        let growth = new_retained
            .checked_sub(old_retained)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        let mut next = self.stats;
        next.live_bytes = next
            .live_bytes
            .checked_sub(old_retained)
            .and_then(|bytes| bytes.checked_add(new_retained))
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        if next.live_bytes > self.limits.max_bytes() {
            return Err(UniqueStoreError::ByteLimit);
        }
        next.allocated_bytes = next
            .allocated_bytes
            .checked_add(growth)
            .ok_or(UniqueStoreError::ArithmeticOverflow)?;
        next.peak_live_bytes = next.peak_live_bytes.max(next.live_bytes);
        Ok(next)
    }
}
