use super::{map_store_error, Error, Result, UniqueRuntime, Value};

impl UniqueRuntime {
    pub(crate) fn len(&self, value: Value) -> Result<i64> {
        let token = self.validate_view(value, false)?;
        let loan = self
            .loans
            .get(&token)
            .ok_or_else(|| Error::msg("VM byte view disappeared"))?;
        i64::try_from(loan.len).map_err(|_| Error::msg("byte-slice length out of range"))
    }

    pub(crate) fn byte_at(&mut self, value: Value, index: i64) -> Result<i64> {
        let token = self.validate_view(value, false)?;
        let index = usize::try_from(index)
            .map_err(|_| Error::msg("byte-slice-byte-at index out of range"))?;
        let loan = *self
            .loans
            .get(&token)
            .ok_or_else(|| Error::msg("VM byte view disappeared"))?;
        let key = self
            .store
            .import_byte_vector_key(loan.owner)
            .map_err(map_store_error)?;
        let bytes = self
            .store
            .byte_vector_range(key, loan.start, loan.len)
            .map_err(map_store_error)?;
        bytes
            .get(index)
            .copied()
            .map(i64::from)
            .ok_or_else(|| Error::msg("byte-slice-byte-at out of bounds"))
    }

    pub(crate) fn set_byte(&mut self, value: Value, index: i64, byte: i64) -> Result<()> {
        let token = self.validate_view(value, true)?;
        let index = usize::try_from(index)
            .map_err(|_| Error::msg("byte-slice-mut-set-byte index out of range"))?;
        let byte = u8::try_from(byte)
            .map_err(|_| Error::msg("byte-slice-mut-set-byte value out of range"))?;
        let loan = *self
            .loans
            .get(&token)
            .ok_or_else(|| Error::msg("VM byte view disappeared"))?;
        let key = self
            .store
            .import_byte_vector_key(loan.owner)
            .map_err(map_store_error)?;
        let bytes = self
            .store
            .byte_vector_range_mut(key, loan.start, loan.len)
            .map_err(map_store_error)?;
        let slot = bytes
            .get_mut(index)
            .ok_or_else(|| Error::msg("byte-slice-mut-set-byte out of bounds"))?;
        *slot = byte;
        Ok(())
    }
}
