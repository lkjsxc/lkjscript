use super::*;

#[derive(Default)]
pub struct NoopNativeIslandRuntimeServices;

impl NativeStructuralRuntimeServices for NoopNativeIslandRuntimeServices {}
impl NativeRuntimeServices for NoopNativeIslandRuntimeServices {}

impl NativeIslandRuntimeServices for NoopNativeIslandRuntimeServices {
    fn borrow_standard_input(&mut self) -> Result<NativeResource, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn new_byte_vector(&mut self, _size: i64) -> Result<NativeUnique, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn move_byte_vector(
        &mut self,
        _owner: NativeUnique,
    ) -> Result<NativeUnique, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn borrow_byte_vector(
        &mut self,
        _owner: NativeUnique,
        _kind: LoanType,
    ) -> Result<NativeLoan, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn byte_slice_length(&mut self, _loan: NativeLoan) -> Result<i64, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn byte_slice_byte_at(
        &mut self,
        _loan: NativeLoan,
        _index: i64,
    ) -> Result<i64, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn byte_slice_read_u32_little_endian(
        &mut self,
        _loan: NativeLoan,
        _index: i64,
    ) -> Result<i64, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn byte_slice_mut_set_byte(
        &mut self,
        _loan: NativeLoan,
        _index: i64,
        _byte: i64,
    ) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn byte_slice_mut_write_u32_little_endian(
        &mut self,
        _loan: NativeLoan,
        _index: i64,
        _word: i64,
    ) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn end_byte_vector_borrow(&mut self, _loan: NativeLoan) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn drop_byte_vector(&mut self, _owner: NativeUnique) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn clone_static_bytes(&mut self, _bytes: &[u8]) -> Result<NativeUnique, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn copy_static_bytes_slice(
        &mut self,
        _bytes: &[u8],
        _start: i64,
        _len: i64,
    ) -> Result<NativeUnique, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn thaw_static_bytes(&mut self, _bytes: &[u8]) -> Result<NativeUnique, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn move_bytes(&mut self, _owner: NativeUnique) -> Result<NativeUnique, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn borrow_bytes(&mut self, _owner: NativeUnique) -> Result<NativeLoan, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn bytes_length(&mut self, _loan: NativeLoan) -> Result<i64, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn bytes_byte_at(&mut self, _loan: NativeLoan, _index: i64) -> Result<i64, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn clone_bytes(&mut self, _loan: NativeLoan) -> Result<NativeUnique, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn copy_bytes_slice(
        &mut self,
        _loan: NativeLoan,
        _start: i64,
        _len: i64,
    ) -> Result<NativeUnique, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn end_bytes_borrow(&mut self, _loan: NativeLoan) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn drop_bytes(&mut self, _owner: NativeUnique) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn freeze_byte_vector(
        &mut self,
        _owner: NativeUnique,
    ) -> Result<NativeUnique, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn thaw_bytes(&mut self, _owner: NativeUnique) -> Result<NativeUnique, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
}

#[derive(Default)]
pub struct NoopNativeRuntimeServices;

impl NativeRuntimeServices for NoopNativeRuntimeServices {}
