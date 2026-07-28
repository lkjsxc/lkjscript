use super::*;

#[derive(Default)]
pub(in crate::executable) struct NoopNativeIslandRuntimeServices;

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

    fn byte_slice_mut_set_byte(
        &mut self,
        _loan: NativeLoan,
        _index: i64,
        _byte: i64,
    ) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn end_byte_vector_borrow(&mut self, _loan: NativeLoan) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }

    fn drop_byte_vector(&mut self, _owner: NativeUnique) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
}

#[derive(Default)]
pub(in crate::executable) struct NoopNativeRuntimeServices;

impl NativeRuntimeServices for NoopNativeRuntimeServices {
    fn collect_references(&mut self, _roots: &mut [NativeRoot]) -> Result<(), NativeServiceError> {
        Ok(())
    }
}
