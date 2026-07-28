use super::*;

impl NativeIslandRuntimeServices for JitIslandServices {
    fn borrow_standard_input(&mut self) -> Result<NativeResource, NativeServiceError> {
        self.native_stdin()
    }

    fn new_byte_vector(&mut self, size: i64) -> Result<NativeUnique, NativeServiceError> {
        self.unique.allocate(size)
    }

    fn move_byte_vector(
        &mut self,
        owner: NativeUnique,
    ) -> Result<NativeUnique, NativeServiceError> {
        self.unique.move_owner(owner)
    }

    fn borrow_byte_vector(
        &mut self,
        owner: NativeUnique,
        kind: LoanType,
    ) -> Result<NativeLoan, NativeServiceError> {
        self.unique.borrow(owner, kind)
    }

    fn byte_slice_length(&mut self, loan: NativeLoan) -> Result<i64, NativeServiceError> {
        self.unique.length(loan)
    }

    fn byte_slice_byte_at(
        &mut self,
        loan: NativeLoan,
        index: i64,
    ) -> Result<i64, NativeServiceError> {
        self.unique.byte_at(loan, index)
    }

    fn byte_slice_read_u32_little_endian(
        &mut self,
        loan: NativeLoan,
        index: i64,
    ) -> Result<i64, NativeServiceError> {
        self.unique.read_u32_little_endian(loan, index)
    }

    fn byte_slice_mut_set_byte(
        &mut self,
        loan: NativeLoan,
        index: i64,
        byte: i64,
    ) -> Result<(), NativeServiceError> {
        self.unique.set_byte(loan, index, byte)
    }

    fn byte_slice_mut_write_u32_little_endian(
        &mut self,
        loan: NativeLoan,
        index: i64,
        word: i64,
    ) -> Result<(), NativeServiceError> {
        self.unique.write_u32_little_endian(loan, index, word)
    }

    fn end_byte_vector_borrow(&mut self, loan: NativeLoan) -> Result<(), NativeServiceError> {
        self.unique.end_borrow(loan)
    }

    fn drop_byte_vector(&mut self, owner: NativeUnique) -> Result<(), NativeServiceError> {
        self.unique.drop_owner(owner)
    }
}
