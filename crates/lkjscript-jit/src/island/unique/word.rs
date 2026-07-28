use super::*;

impl JitUniqueRuntime {
    pub(crate) fn read_u32_little_endian(
        &mut self,
        value: NativeLoan,
        index: i64,
    ) -> Result<i64, NativeServiceError> {
        if value.loan_type() != LoanType::ByteSlice {
            return Err(self.reject());
        }
        let index = usize::try_from(index).map_err(|_| self.reject())?;
        let loan = self.loan(value)?;
        let absolute = self.checked_word_index(loan, index)?;
        let key = self
            .store
            .import_byte_vector_key(loan.owner)
            .map_err(|_| self.reject())?;
        let word = self
            .store
            .read_byte_vector_u32_little_endian(key, absolute)
            .map_err(|_| self.reject())?;
        self.stats.byte_reads = self.stats.byte_reads.saturating_add(4);
        Ok(i64::from(word))
    }

    pub(crate) fn write_u32_little_endian(
        &mut self,
        value: NativeLoan,
        index: i64,
        word: i64,
    ) -> Result<(), NativeServiceError> {
        if value.loan_type() != LoanType::ByteSliceMut {
            return Err(self.reject());
        }
        let index = usize::try_from(index).map_err(|_| self.reject())?;
        let word = u32::try_from(word).map_err(|_| self.reject())?;
        let loan = self.loan(value)?;
        let absolute = self.checked_word_index(loan, index)?;
        let key = self
            .store
            .import_byte_vector_key(loan.owner)
            .map_err(|_| self.reject())?;
        self.store
            .write_byte_vector_u32_little_endian(key, absolute, word)
            .map_err(|_| self.reject())?;
        self.stats.byte_writes = self.stats.byte_writes.saturating_add(4);
        Ok(())
    }

    fn checked_word_index(
        &mut self,
        loan: Loan,
        index: usize,
    ) -> Result<usize, NativeServiceError> {
        let Some(end) = index.checked_add(4) else {
            return Err(self.reject());
        };
        if end > loan.len {
            return Err(self.reject());
        }
        loan.start.checked_add(index).ok_or_else(|| self.reject())
    }
}
