//! Rust child lifetime and the temporary native result channel.
use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

unsafe extern "C" {
    fn prctl(option: i32, ...) -> i32;
    fn getppid() -> i32;
    fn dup2(old: i32, new: i32) -> i32;
    fn fcntl(fd: i32, operation: i32, ...) -> i32;
}

pub fn prepare_child(command: &mut Command) {
    let parent = std::process::id() as i32;
    // Only async-signal-safe syscalls run after fork. Process-group timeout
    // handling normally kills children too; this also covers parent death.
    unsafe {
        command.pre_exec(move || {
            if prctl(1, 15_usize, 0_usize, 0_usize, 0_usize) != 0 {
                return Err(io::Error::last_os_error());
            }
            if getppid() != parent {
                return Err(io::Error::from_raw_os_error(3));
            }
            Ok(())
        });
    }
}

// A caller may have closed a standard stream. Creating a control channel must
// preserve that absence instead of occupying fd0, fd1 or fd2 in the child.
fn above_standard_streams(
    socket: std::os::unix::net::UnixStream,
) -> io::Result<std::os::unix::net::UnixStream> {
    use std::os::fd::{AsRawFd, FromRawFd};
    if socket.as_raw_fd() >= 3 {
        return Ok(socket);
    }
    let fd = unsafe { fcntl(socket.as_raw_fd(), 1030, 3) }; // F_DUPFD_CLOEXEC
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { std::os::unix::net::UnixStream::from_raw_fd(fd) })
}

/// The result packet has its own descriptor; stderr remains a program stream.
/// Drop the Command before reading to EOF so its retained writer cannot keep
/// an absent packet open after the child has terminated.
pub fn result_channel(command: &mut Command) -> io::Result<std::os::unix::net::UnixStream> {
    use std::os::fd::AsRawFd;
    let (reader, writer) = std::os::unix::net::UnixStream::pair()?;
    let reader = above_standard_streams(reader)?;
    let writer = above_standard_streams(writer)?;
    unsafe {
        command.pre_exec(move || {
            let fd = writer.as_raw_fd();
            if fd == 3 {
                if fcntl(fd, 2, 0) < 0 {
                    return Err(io::Error::last_os_error());
                } // F_SETFD
            } else if dup2(fd, 3) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    Ok(reader)
}
