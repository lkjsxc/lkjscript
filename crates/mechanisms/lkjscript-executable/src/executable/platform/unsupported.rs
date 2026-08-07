use crate::executable::{
    InstallError, IslandCallState, MachineArgument, MappingPermissions, NativeCallState,
    NativeStackError, PermissionProbeError, PreEntryError, RawReturn, Signature,
};

#[derive(Clone, Copy)]
pub(in crate::executable) struct NativeStackBounds;

pub(in crate::executable) fn native_stack_bounds() -> Option<NativeStackBounds> {
    None
}

pub(in crate::executable) fn native_stack_reservation_fits(
    _rbp: *mut u8,
    _frame_bytes: usize,
    _bounds: NativeStackBounds,
) -> Result<(), NativeStackError> {
    Err(NativeStackError::ThreadExtentUnavailable)
}

pub(in crate::executable) fn native_stack_requirement_fits(
    _required_bytes: usize,
    _bounds: NativeStackBounds,
) -> Result<(), NativeStackError> {
    Err(NativeStackError::ThreadExtentUnavailable)
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

    pub(in crate::executable) fn validate_entry(
        &self,
        _offset: usize,
    ) -> Result<(), PreEntryError> {
        Err(PreEntryError::UnknownEntry)
    }

    pub(in crate::executable) fn enter(
        &self,
        _offset: usize,
        _signature: &Signature,
        _arguments: &[MachineArgument],
        _state: &mut NativeCallState,
    ) -> RawReturn {
        unreachable!("unsupported platform cannot install a prepared invocation")
    }

    pub(in crate::executable) fn enter_island(
        &self,
        _offset: usize,
        _signature: &Signature,
        _arguments: &[MachineArgument],
        _state: &mut IslandCallState,
    ) -> RawReturn {
        unreachable!("unsupported platform cannot install a prepared invocation")
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
