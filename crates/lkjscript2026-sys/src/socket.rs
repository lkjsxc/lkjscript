//! Linux TCP socket primitives via owned libc externs.

use std::os::fd::RawFd;

const AF_INET: i32 = 2;
const SOCK_STREAM: i32 = 1;
const SOL_SOCKET: i32 = 1;
const SO_REUSEADDR: i32 = 2;
const INADDR_ANY: u32 = 0;

#[repr(C)]
struct InAddr {
    s_addr: u32,
}

#[repr(C)]
struct SockAddrIn {
    sin_family: u16,
    sin_port: u16,
    sin_addr: InAddr,
    sin_zero: [u8; 8],
}

#[derive(Debug)]
pub struct SockError(pub i32);

impl std::fmt::Display for SockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "socket errno {}", self.0)
    }
}

extern "C" {
    fn socket(domain: i32, ty: i32, protocol: i32) -> i32;
    fn setsockopt(
        sockfd: i32,
        level: i32,
        optname: i32,
        optval: *const u8,
        optlen: u32,
    ) -> i32;
    fn bind(sockfd: i32, addr: *const SockAddrIn, addrlen: u32) -> i32;
    fn listen(sockfd: i32, backlog: i32) -> i32;
    fn accept(sockfd: i32, addr: *mut SockAddrIn, addrlen: *mut u32) -> i32;
    fn recv(sockfd: i32, buf: *mut u8, len: usize, flags: i32) -> isize;
    fn send(sockfd: i32, buf: *const u8, len: usize, flags: i32) -> isize;
    fn close(fd: i32) -> i32;
    fn __errno_location() -> *mut i32;
}

fn errno() -> i32 {
    unsafe { *__errno_location() }
}

fn htons(port: u16) -> u16 {
    port.to_be()
}

/// Owned socket fd; Drop closes. Safe to hold outside this crate.
pub struct OwnedSock(RawFd);

impl OwnedSock {
    pub fn as_raw(&self) -> RawFd {
        self.0
    }
}

impl Drop for OwnedSock {
    fn drop(&mut self) {
        let _ = close_fd(self.0);
    }
}

pub fn tcp_socket() -> Result<OwnedSock, SockError> {
    let fd = unsafe { socket(AF_INET, SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(SockError(errno()));
    }
    Ok(OwnedSock(fd))
}

pub fn set_reuseaddr(fd: RawFd) -> Result<(), SockError> {
    let yes: i32 = 1;
    let rc = unsafe {
        setsockopt(
            fd,
            SOL_SOCKET,
            SO_REUSEADDR,
            (&yes as *const i32).cast(),
            std::mem::size_of_val(&yes) as u32,
        )
    };
    if rc < 0 {
        return Err(SockError(errno()));
    }
    Ok(())
}

pub fn bind_ipv4_any(fd: RawFd, port: u16) -> Result<(), SockError> {
    let addr = SockAddrIn {
        sin_family: AF_INET as u16,
        sin_port: htons(port),
        sin_addr: InAddr {
            s_addr: INADDR_ANY.to_be(),
        },
        sin_zero: [0; 8],
    };
    let rc = unsafe {
        bind(
            fd,
            &addr,
            std::mem::size_of::<SockAddrIn>() as u32,
        )
    };
    if rc < 0 {
        return Err(SockError(errno()));
    }
    Ok(())
}

pub fn listen_sock(fd: RawFd, backlog: i32) -> Result<(), SockError> {
    let rc = unsafe { listen(fd, backlog) };
    if rc < 0 {
        return Err(SockError(errno()));
    }
    Ok(())
}

pub fn accept_sock(fd: RawFd) -> Result<OwnedSock, SockError> {
    let mut addr = SockAddrIn {
        sin_family: 0,
        sin_port: 0,
        sin_addr: InAddr { s_addr: 0 },
        sin_zero: [0; 8],
    };
    let mut len = std::mem::size_of::<SockAddrIn>() as u32;
    let client = unsafe { accept(fd, &mut addr, &mut len) };
    if client < 0 {
        return Err(SockError(errno()));
    }
    Ok(OwnedSock(client))
}

pub fn recv_sock(fd: RawFd, buf: &mut [u8]) -> Result<usize, SockError> {
    let n = unsafe { recv(fd, buf.as_mut_ptr(), buf.len(), 0) };
    if n < 0 {
        return Err(SockError(errno()));
    }
    Ok(n as usize)
}

pub fn send_sock(fd: RawFd, buf: &[u8]) -> Result<usize, SockError> {
    let n = unsafe { send(fd, buf.as_ptr(), buf.len(), 0) };
    if n < 0 {
        return Err(SockError(errno()));
    }
    Ok(n as usize)
}

pub fn close_fd(fd: RawFd) -> Result<(), SockError> {
    let rc = unsafe { close(fd) };
    if rc < 0 {
        return Err(SockError(errno()));
    }
    Ok(())
}
