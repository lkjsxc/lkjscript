#![allow(unsafe_code)]

#[cfg(target_os = "linux")]
use std::ffi::{c_int, c_void};
#[cfg(target_os = "linux")]
use std::mem::size_of;
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
#[cfg(target_os = "linux")]
use std::os::unix::net::UnixStream;

#[cfg(target_os = "linux")]
use crate::{HostError, HostResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalPrincipal {
    pub process: u32,
    pub user: u32,
    pub group: u32,
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct LinuxCredentials {
    process: c_int,
    user: u32,
    group: u32,
}

#[cfg(target_os = "linux")]
const SOL_SOCKET: c_int = 1;
#[cfg(target_os = "linux")]
const SO_PEERCRED: c_int = 17;

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn getsockopt(
        socket: c_int,
        level: c_int,
        option: c_int,
        value: *mut c_void,
        length: *mut u32,
    ) -> c_int;
}

/// Returns kernel-authenticated credentials for one connected Linux Unix socket.
#[cfg(target_os = "linux")]
pub fn local_peer_principal(stream: &UnixStream) -> HostResult<LocalPrincipal> {
    let mut credentials = LinuxCredentials {
        process: 0,
        user: 0,
        group: 0,
    };
    let mut length = u32::try_from(size_of::<LinuxCredentials>()).map_err(|_| HostError::Io {
        operation: "size Linux peer credentials".to_string(),
        message: "credential structure exceeds u32".to_string(),
    })?;
    // SAFETY: `credentials` and `length` are initialized writable values for
    // exactly their declared sizes. The borrowed stream keeps the descriptor
    // live for this synchronous call. No pointer escapes the call.
    let status = unsafe {
        getsockopt(
            stream.as_raw_fd(),
            SOL_SOCKET,
            SO_PEERCRED,
            (&mut credentials as *mut LinuxCredentials).cast(),
            &mut length,
        )
    };
    if status != 0 || usize::try_from(length).ok() != Some(size_of::<LinuxCredentials>()) {
        return Err(HostError::Io {
            operation: "read Unix peer credentials".to_string(),
            message: std::io::Error::last_os_error().to_string(),
        });
    }
    let process = u32::try_from(credentials.process).map_err(|_| HostError::Io {
        operation: "validate Unix peer process".to_string(),
        message: "kernel returned negative process identity".to_string(),
    })?;
    Ok(LocalPrincipal {
        process,
        user: credentials.user,
        group: credentials.group,
    })
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::*;

    #[test]
    fn unix_pair_reports_kernel_peer_process() -> Result<(), Box<dyn std::error::Error>> {
        let (left, right) = UnixStream::pair()?;
        let left_peer = local_peer_principal(&left)?;
        let right_peer = local_peer_principal(&right)?;
        assert_eq!(left_peer.process, std::process::id());
        assert_eq!(left_peer, right_peer);
        Ok(())
    }
}
