use super::*;

impl EvalUniqueRuntime {
    pub(in crate::eval) fn read_u32_little_endian(
        &mut self,
        value: &EvalValue,
        index: usize,
    ) -> Result<i64, Flow> {
        let loan = self.shared_loan(value)?;
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

    pub(in crate::eval) fn write_u32_little_endian(
        &mut self,
        value: &EvalValue,
        index: usize,
        word: u32,
    ) -> Result<(), Flow> {
        let loan = self.mutable_loan(value)?;
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

fn checked_word_index(loan: Loan, index: usize, operation: &str) -> Result<usize, Flow> {
    let end = index
        .checked_add(4)
        .ok_or_else(|| Flow::Trap(format!("{operation} range overflow")))?;
    if end > loan.len {
        return Err(Flow::Trap(format!("{operation} out of bounds")));
    }
    loan.start
        .checked_add(index)
        .ok_or_else(|| Flow::Trap(format!("{operation} range overflow")))
}
