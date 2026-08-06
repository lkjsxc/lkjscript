#![allow(unsafe_code)]

//! Linux kernel entropy via getrandom, with no userspace fallback.

use crate::fd::{errno, FdError};

const EINTR: i32 = 4;
const EIO: i32 = 5;

extern "C" {
    fn getrandom(buf: *mut u8, buflen: usize, flags: u32) -> isize;
}

/// Fill every byte from Linux `getrandom(2)`, retrying only interruptions.
pub fn random_fill(destination: &mut [u8]) -> Result<(), FdError> {
    let mut filled = 0_usize;
    while filled < destination.len() {
        let count = unsafe {
            getrandom(
                destination[filled..].as_mut_ptr(),
                destination.len() - filled,
                0,
            )
        };
        if count > 0 {
            let count = usize::try_from(count).map_err(|_| FdError(EIO))?;
            filled = filled.checked_add(count).ok_or(FdError(EIO))?;
            continue;
        }
        if count == 0 {
            return Err(FdError(EIO));
        }
        let error = errno();
        if error != EINTR {
            return Err(FdError(error));
        }
    }
    Ok(())
}
