use super::{map_store_error, Error, Result, UniqueRuntime, Value};

impl UniqueRuntime {
    pub(crate) fn shared_bytes(&mut self, value: Value) -> Result<&[u8]> {
        let token = self.validate_view(value, false)?;
        let loan = *self
            .loans
            .get(&token)
            .ok_or_else(|| Error::msg("VM byte view disappeared"))?;
        let key = self
            .store
            .import_byte_vector_key(loan.owner)
            .map_err(map_store_error)?;
        self.store
            .byte_vector_range(key, loan.start, loan.len)
            .map_err(map_store_error)
    }

    pub(crate) fn exclusive_bytes(&mut self, value: Value) -> Result<&mut [u8]> {
        let token = self.validate_view(value, true)?;
        let loan = *self
            .loans
            .get(&token)
            .ok_or_else(|| Error::msg("VM byte view disappeared"))?;
        let key = self
            .store
            .import_byte_vector_key(loan.owner)
            .map_err(map_store_error)?;
        self.store
            .byte_vector_range_mut(key, loan.start, loan.len)
            .map_err(map_store_error)
    }

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

    pub(crate) fn read_u32_little_endian(&mut self, value: Value, index: i64) -> Result<i64> {
        let token = self.validate_view(value, false)?;
        let index = usize::try_from(index)
            .map_err(|_| Error::msg("byte-slice-read-u32-little-endian index out of range"))?;
        let loan = *self
            .loans
            .get(&token)
            .ok_or_else(|| Error::msg("VM byte view disappeared"))?;
        let absolute = checked_word_index(loan, index, "byte-slice-read-u32-little-endian")?;
        let key = self
            .store
            .import_byte_vector_key(loan.owner)
            .map_err(map_store_error)?;
        self.store
            .read_byte_vector_u32_little_endian(key, absolute)
            .map(i64::from)
            .map_err(map_store_error)
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

    pub(crate) fn write_u32_little_endian(
        &mut self,
        value: Value,
        index: i64,
        word: i64,
    ) -> Result<()> {
        let token = self.validate_view(value, true)?;
        let index = usize::try_from(index)
            .map_err(|_| Error::msg("byte-slice-mut-write-u32-little-endian index out of range"))?;
        let word = u32::try_from(word)
            .map_err(|_| Error::msg("byte-slice-mut-write-u32-little-endian value out of range"))?;
        let loan = *self
            .loans
            .get(&token)
            .ok_or_else(|| Error::msg("VM byte view disappeared"))?;
        let absolute = checked_word_index(loan, index, "byte-slice-mut-write-u32-little-endian")?;
        let key = self
            .store
            .import_byte_vector_key(loan.owner)
            .map_err(map_store_error)?;
        self.store
            .write_byte_vector_u32_little_endian(key, absolute, word)
            .map_err(map_store_error)
    }
}

fn checked_word_index(loan: super::Loan, index: usize, operation: &str) -> Result<usize> {
    let end = index
        .checked_add(4)
        .ok_or_else(|| Error::msg(format!("{operation} range overflow")))?;
    if end > loan.len {
        return Err(Error::msg(format!("{operation} out of bounds")));
    }
    loan.start
        .checked_add(index)
        .ok_or_else(|| Error::msg(format!("{operation} range overflow")))
}
