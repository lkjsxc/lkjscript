#![allow(unsafe_code)]

use super::*;

impl Mapping {
    pub(in crate::executable) fn validate_entry(&self, offset: usize) -> Result<(), PreEntryError> {
        if !self.sealed || offset >= self.length {
            return Err(PreEntryError::UnknownEntry);
        }
        Ok(())
    }

    pub(in crate::executable) fn enter(
        &self,
        offset: usize,
        signature: &Signature,
        arguments: &[MachineArgument],
        state: &mut NativeCallState,
    ) -> RawReturn {
        let address = self.base.as_ptr().wrapping_add(offset).cast::<c_void>();
        state.entry_started = true;
        // Native entry begins at this exact boundary. All validation and
        // fallible preparation completed before this unsafe ABI call.
        // SAFETY: InstallableImage can only arise from the verified closed
        // encoder. Installation validates the entry offset/signature and
        // seals the complete mapping RX before this conversion and call.
        unsafe {
            invoke_typed(
                address,
                signature.result(),
                arguments,
                (state as *mut NativeCallState).cast::<c_void>(),
            )
        }
    }

    pub(in crate::executable) fn enter_island(
        &self,
        offset: usize,
        signature: &Signature,
        arguments: &[MachineArgument],
        state: &mut IslandCallState,
    ) -> RawReturn {
        let address = self.base.as_ptr().wrapping_add(offset).cast::<c_void>();
        state.entry_started = true;
        // Native entry begins at this exact boundary. All validation and
        // fallible preparation completed before this unsafe ABI call.
        // SAFETY: collector-free image integrity binds every relocation and
        // entry to the noncollecting state/runtime table before RX sealing.
        unsafe {
            invoke_typed(
                address,
                signature.result(),
                arguments,
                (state as *mut IslandCallState).cast::<c_void>(),
            )
        }
    }

    pub(in crate::executable) fn permissions(
        &self,
    ) -> Result<MappingPermissions, PermissionProbeError> {
        let maps = std::fs::read_to_string("/proc/self/maps")
            .map_err(|_| PermissionProbeError::ProcMapsUnavailable)?;
        let address = self.base.as_ptr() as usize;
        for line in maps.lines() {
            let mut fields = line.split_whitespace();
            let range = match fields.next() {
                Some(value) => value,
                None => continue,
            };
            let permissions = match fields.next() {
                Some(value) => value,
                None => continue,
            };
            let mut bounds = range.split('-');
            let start = match bounds
                .next()
                .and_then(|value| usize::from_str_radix(value, 16).ok())
            {
                Some(value) => value,
                None => continue,
            };
            let end = match bounds
                .next()
                .and_then(|value| usize::from_str_radix(value, 16).ok())
            {
                Some(value) => value,
                None => continue,
            };
            if start <= address && address < end {
                let bytes = permissions.as_bytes();
                if bytes.len() < 3 {
                    return Err(PermissionProbeError::MalformedPermissions);
                }
                return Ok(MappingPermissions {
                    readable: bytes[0] == b'r',
                    writable: bytes[1] == b'w',
                    executable: bytes[2] == b'x',
                });
            }
        }
        Err(PermissionProbeError::MappingNotFound)
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        // SAFETY: This Mapping owns this still-live mmap range exactly once.
        let _ = unsafe { munmap(self.base.as_ptr().cast::<c_void>(), self.allocation_length) };
    }
}
