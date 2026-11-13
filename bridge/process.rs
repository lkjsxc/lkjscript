//! Rust bootstrap child resources, result channels and worker lifetime.
use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

unsafe extern "C" {
    fn prctl(option: i32, ...) -> i32;
    fn getppid() -> i32;
    fn dup2(old: i32, new: i32) -> i32;
    fn fcntl(fd: i32, operation: i32, ...) -> i32;
    fn prlimit(pid: i32, resource: u32, new: *const Limit, old: *mut Limit) -> i32;
}
#[repr(C)]
struct Limit {
    current: u64,
    maximum: u64,
}

/// A runner budget is host policy, not a property of the accepted source.
/// An explicit environment setting can enlarge it or inherit Linux's full
/// extent; a tighter limit imposed by the operator always remains effective.
pub fn prepare_native_child(command: &mut Command) -> Result<(), String> {
    prepare_budgeted_child(command, 268435456).map(|_| ())
}
fn prepare_budgeted_child(command: &mut Command, default: u64) -> Result<u64, String> {
    let text = std::env::var("LKJ_RUN_MEMORY_BYTES").unwrap_or_else(|_| default.to_string());
    let budget = if text == "unlimited" {
        u64::MAX
    } else {
        text.parse::<u64>().ok().filter(|n| *n > 0).ok_or_else(|| {
            "E_HOST LKJ_RUN_MEMORY_BYTES must be positive bytes or unlimited".to_string()
        })?
    };
    prepare_child(command);
    unsafe {
        command.pre_exec(move || {
            let mut limit = Limit {
                current: 0,
                maximum: 0,
            };
            if prlimit(0, 9, std::ptr::null(), &mut limit) != 0 {
                return Err(io::Error::last_os_error());
            }
            let no_core = Limit {
                current: 0,
                maximum: 0,
            };
            if prlimit(0, 4, &no_core, std::ptr::null_mut()) != 0 {
                return Err(io::Error::last_os_error());
            }
            limit.current = limit.current.min(budget);
            if prlimit(0, 9, &limit, std::ptr::null_mut()) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    Ok(budget)
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

/// K execution and every batch run in a fresh process so the Rust allocator
/// and captured program streams cannot consume the compiler host. Compile/check
/// commands have no runner budget. Batch retains its streaming result protocol.
pub fn execution_worker() -> Result<Option<std::process::ExitCode>, String> {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitCode, Stdio};
    if std::env::var("LKJ_EXECUTION_WORKER").as_deref() == Ok("1") {
        return Ok(None);
    }
    let argv = std::env::args_os().skip(1).collect::<Vec<_>>();
    let Some(verb) = argv.first() else {
        return Ok(None);
    };
    let end = argv
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(argv.len());
    let selected = argv[..end].windows(2).find_map(|pair| {
        if pair[0] == "--engine" || pair[0] == "--engines" {
            pair[1].to_str()
        } else {
            None
        }
    });
    let needs_worker =
        verb == "batch" || ((verb == "run" || verb == "test") && selected == Some("k"));
    if !needs_worker {
        return Ok(None);
    }
    let executable = std::env::current_exe().map_err(|error| format!("E_HOST {error}"))?;
    let mut command = Command::new(executable);
    command
        .args(&argv)
        .env("LKJ_EXECUTION_WORKER", "1")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let budget = prepare_budgeted_child(&mut command, 536870912)?;
    let status = command
        .status()
        .map_err(|error| format!("E_HOST execution worker: {error}"))?;
    // Program streams belong to the child directly: proxying stderr would hide
    // prompts and change write errors when the caller closes a pipe or fd2.
    if let Some(signal) = status.signal() {
        return Err(format!(
            "E_HOST execution worker terminated by signal {signal} under Linux address-space budget {budget} bytes"
        ));
    }
    Ok(Some(ExitCode::from(status.code().unwrap_or(1) as u8)))
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
