use super::{ByteVectorKey, UniqueStore, UniqueStoreError};

impl UniqueStore {
    pub fn read_byte_vector_u32_little_endian(
        &mut self,
        key: ByteVectorKey,
        index: usize,
    ) -> Result<u32, UniqueStoreError> {
        let bytes = self.byte_vector_range(key, index, 4)?;
        let word: [u8; 4] = bytes
            .try_into()
            .map_err(|_| UniqueStoreError::ArithmeticOverflow)?;
        Ok(u32::from_le_bytes(word))
    }

    pub fn write_byte_vector_u32_little_endian(
        &mut self,
        key: ByteVectorKey,
        index: usize,
        value: u32,
    ) -> Result<(), UniqueStoreError> {
        let bytes = self.byte_vector_range_mut(key, index, 4)?;
        bytes.copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
}
