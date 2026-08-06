#![allow(unsafe_code)]

use super::*;

impl Mapping {
    pub(in crate::executable) fn allocate_rw(length: usize) -> Result<Self, InstallError> {
        if length == 0 {
            return Err(InstallError::AllocationFailed);
        }
        // SAFETY: SC_PAGESIZE has no pointer contract and returns one
        // process page-size value.
        let page_size = unsafe { sysconf(SC_PAGESIZE) };
        let page_size = usize::try_from(page_size)
            .ok()
            .filter(|value| *value > 0)
            .ok_or(InstallError::AllocationFailed)?;
        let allocation_length = length
            .checked_add(page_size - 1)
            .map(|value| value / page_size * page_size)
            .ok_or(InstallError::AllocationFailed)?;
        // SAFETY: The null hint, anonymous descriptor, zero offset, and
        // nonzero page-rounded length satisfy mmap. The returned mapping is
        // checked before ownership.
        let pointer = unsafe {
            mmap(
                std::ptr::null_mut(),
                allocation_length,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if pointer as isize == -1 {
            return Err(InstallError::AllocationFailed);
        }
        let base = NonNull::new(pointer.cast::<u8>()).ok_or(InstallError::AllocationFailed)?;
        Ok(Self {
            base,
            length,
            allocation_length,
            sealed: false,
            wx_transition_verified: false,
        })
    }

    pub(in crate::executable) fn copy_from(&mut self, bytes: &[u8]) -> Result<(), InstallError> {
        if self.sealed || bytes.len() != self.length {
            return Err(InstallError::RelocationRange);
        }
        // SAFETY: This Mapping uniquely owns a writable mapping of exactly
        // `length` bytes and source/destination cannot overlap.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.base.as_ptr(), self.length);
        }
        Ok(())
    }

    pub(in crate::executable) fn write_absolute64(
        &mut self,
        offset: usize,
        address: usize,
    ) -> Result<(), InstallError> {
        if self.sealed {
            return Err(InstallError::ProtectionFailed);
        }
        let end = offset.checked_add(8).ok_or(InstallError::RelocationRange)?;
        if end > self.length {
            return Err(InstallError::RelocationRange);
        }
        let address = u64::try_from(address).map_err(|_| InstallError::RelocationAddress)?;
        // SAFETY: The checked range is within the uniquely owned RW mapping.
        unsafe {
            std::ptr::copy_nonoverlapping(
                address.to_le_bytes().as_ptr(),
                self.base.as_ptr().add(offset),
                8,
            );
        }
        Ok(())
    }

    pub(in crate::executable) fn address_at(&self, offset: usize) -> Result<usize, InstallError> {
        if offset >= self.length {
            return Err(InstallError::RelocationAddress);
        }
        (self.base.as_ptr() as usize)
            .checked_add(offset)
            .ok_or(InstallError::RelocationAddress)
    }

    pub(in crate::executable) fn seal_rx(&mut self) -> Result<(), InstallError> {
        if self.sealed {
            return Err(InstallError::ProtectionFailed);
        }
        let writable_phase_verified = self.permissions().ok().is_some_and(|permissions| {
            permissions.readable && permissions.writable && !permissions.executable
        });
        // SAFETY: The mapping base came from mmap and remains live. Linux
        // accepts the original length and rounds it to mapped pages.
        let result = unsafe {
            mprotect(
                self.base.as_ptr().cast::<c_void>(),
                self.allocation_length,
                PROT_READ | PROT_EXEC,
            )
        };
        if result != 0 {
            return Err(InstallError::ProtectionFailed);
        }
        self.sealed = true;
        let executable_phase_verified = self.permissions().ok().is_some_and(|permissions| {
            permissions.readable && !permissions.writable && permissions.executable
        });
        self.wx_transition_verified = writable_phase_verified && executable_phase_verified;
        Ok(())
    }

    pub(in crate::executable) fn allocation_length(&self) -> usize {
        self.allocation_length
    }

    pub(in crate::executable) fn wx_transition_verified(&self) -> bool {
        self.wx_transition_verified
    }
}
