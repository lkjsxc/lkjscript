use crate::executable::{
    InstallError, InvocationError, MappingPermissions, NativeCallState, NativeValue,
    PermissionProbeError, RawReturn, Signature,
};

pub(in crate::executable) fn native_stack_bounds() -> Option<(usize, usize)> {
    None
}

pub(in crate::executable) fn native_stack_reservation_fits(
    _rbp: *mut u8,
    _frame_bytes: usize,
    _guard_bytes: usize,
    _stack_low: usize,
    _stack_high: usize,
) -> bool {
    false
}

#[derive(Debug)]
pub(in crate::executable) struct Mapping;

impl Mapping {
    pub(in crate::executable) fn allocate_rw(_length: usize) -> Result<Self, InstallError> {
        Err(InstallError::UnsupportedPlatform)
    }

    pub(in crate::executable) fn copy_from(&mut self, _bytes: &[u8]) -> Result<(), InstallError> {
        Err(InstallError::UnsupportedPlatform)
    }

    pub(in crate::executable) fn write_absolute64(
        &mut self,
        _offset: usize,
        _address: usize,
    ) -> Result<(), InstallError> {
        Err(InstallError::UnsupportedPlatform)
    }

    pub(in crate::executable) fn address_at(&self, _offset: usize) -> Result<usize, InstallError> {
        Err(InstallError::UnsupportedPlatform)
    }

    pub(in crate::executable) fn seal_rx(&mut self) -> Result<(), InstallError> {
        Err(InstallError::UnsupportedPlatform)
    }

    pub(in crate::executable) fn invoke(
        &self,
        _offset: usize,
        _signature: &Signature,
        _arguments: &[NativeValue],
        _state: &mut NativeCallState,
    ) -> Result<RawReturn, InvocationError> {
        Err(InvocationError::UnsupportedSignature)
    }

    pub(in crate::executable) fn permissions(
        &self,
    ) -> Result<MappingPermissions, PermissionProbeError> {
        Err(PermissionProbeError::UnsupportedPlatform)
    }

    pub(in crate::executable) fn allocation_length(&self) -> usize {
        0
    }

    pub(in crate::executable) fn wx_transition_verified(&self) -> bool {
        false
    }
}
