use super::*;

/// Safe runtime boundary. Implementations receive only copied typed values.
pub trait NativeIslandRuntimeServices: NativeStructuralRuntimeServices {
    fn borrow_standard_input(&mut self) -> Result<NativeResource, NativeServiceError>;
    fn new_byte_vector(&mut self, size: i64) -> Result<NativeUnique, NativeServiceError>;
    fn move_byte_vector(&mut self, owner: NativeUnique)
        -> Result<NativeUnique, NativeServiceError>;
    fn borrow_byte_vector(
        &mut self,
        owner: NativeUnique,
        kind: LoanType,
    ) -> Result<NativeLoan, NativeServiceError>;
    fn byte_slice_length(&mut self, loan: NativeLoan) -> Result<i64, NativeServiceError>;
    fn byte_slice_byte_at(
        &mut self,
        loan: NativeLoan,
        index: i64,
    ) -> Result<i64, NativeServiceError>;
    fn byte_slice_read_u32_little_endian(
        &mut self,
        loan: NativeLoan,
        index: i64,
    ) -> Result<i64, NativeServiceError>;
    fn byte_slice_mut_set_byte(
        &mut self,
        loan: NativeLoan,
        index: i64,
        byte: i64,
    ) -> Result<(), NativeServiceError>;
    fn byte_slice_mut_write_u32_little_endian(
        &mut self,
        loan: NativeLoan,
        index: i64,
        word: i64,
    ) -> Result<(), NativeServiceError>;
    fn end_byte_vector_borrow(&mut self, loan: NativeLoan) -> Result<(), NativeServiceError>;
    fn drop_byte_vector(&mut self, owner: NativeUnique) -> Result<(), NativeServiceError>;
    fn clone_static_bytes(&mut self, bytes: &[u8]) -> Result<NativeUnique, NativeServiceError>;
    fn copy_static_bytes_slice(
        &mut self,
        bytes: &[u8],
        start: i64,
        len: i64,
    ) -> Result<NativeUnique, NativeServiceError>;
    fn thaw_static_bytes(&mut self, bytes: &[u8]) -> Result<NativeUnique, NativeServiceError>;
    fn move_bytes(&mut self, owner: NativeUnique) -> Result<NativeUnique, NativeServiceError>;
    fn borrow_bytes(&mut self, owner: NativeUnique) -> Result<NativeLoan, NativeServiceError>;
    fn bytes_length(&mut self, loan: NativeLoan) -> Result<i64, NativeServiceError>;
    fn bytes_byte_at(&mut self, loan: NativeLoan, index: i64) -> Result<i64, NativeServiceError>;
    fn clone_bytes(&mut self, loan: NativeLoan) -> Result<NativeUnique, NativeServiceError>;
    fn copy_bytes_slice(
        &mut self,
        loan: NativeLoan,
        start: i64,
        len: i64,
    ) -> Result<NativeUnique, NativeServiceError>;
    fn end_bytes_borrow(&mut self, loan: NativeLoan) -> Result<(), NativeServiceError>;
    fn drop_bytes(&mut self, owner: NativeUnique) -> Result<(), NativeServiceError>;
    fn freeze_byte_vector(
        &mut self,
        owner: NativeUnique,
    ) -> Result<NativeUnique, NativeServiceError>;
    fn thaw_bytes(&mut self, owner: NativeUnique) -> Result<NativeUnique, NativeServiceError>;
}

pub trait NativeRuntimeServices {
    fn heap_operation(
        &mut self,
        _site: &HeapRuntimeSite,
        _arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
}
