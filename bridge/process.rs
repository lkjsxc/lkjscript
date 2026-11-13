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

const STACK_FALLBACK: u64 = 64 * 1024 * 1024;

fn budget(name: &str) -> Result<Option<u64>, String> {
    let text = match std::env::var(name) {
        Ok(text) => text,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(_) => return Err(format!("E_HOST {name} must be positive bytes or unlimited")),
    };
    if text == "unlimited" {
        return Ok(None);
    }
    text.parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .map(Some)
        .ok_or_else(|| format!("E_HOST {name} must be positive bytes or unlimited"))
}

fn inherited_limit(resource: u32) -> Result<Limit, String> {
    let mut limit = Limit {
        current: 0,
        maximum: 0,
    };
    if unsafe { prlimit(0, resource, std::ptr::null(), &mut limit) } != 0 {
        return Err(format!(
            "E_HOST resource limit: {}",
            io::Error::last_os_error()
        ));
    }
    Ok(limit)
}

/// The model receives an execution input; its core never reads host policy.
/// A finite inherited stack limit is always authoritative. Only an unlimited
/// inherited extent needs a finite provisioning fallback.
pub fn machine_resources() -> Result<lkjscript::machine::Resources, String> {
    let inherited = inherited_limit(3)?.current;
    let stack = match budget("LKJ_RUN_STACK_BYTES")? {
        Some(requested) => inherited.min(requested),
        None if inherited == u64::MAX => STACK_FALLBACK,
        None => inherited,
    };
    let stack_bytes = usize::try_from(stack)
        .map_err(|_| "E_HOST stack extent exceeds host representation".to_string())?;
    Ok(lkjscript::machine::Resources { stack_bytes })
}

/// Budgets are explicit host choices. An absent memory setting inherits Linux
/// controls, and an explicit setting can only tighten an inherited soft limit.
pub fn prepare_native_child(command: &mut Command) -> Result<(), String> {
    prepare_budgeted_child(command, true).map(|_| ())
}
fn prepare_budgeted_child(command: &mut Command, native_stack: bool) -> Result<u64, String> {
    let requested_memory = budget("LKJ_RUN_MEMORY_BYTES")?.unwrap_or(u64::MAX);
    let selected_memory = inherited_limit(9)?.current.min(requested_memory);
    let requested_stack = if native_stack {
        budget("LKJ_RUN_STACK_BYTES")?
    } else {
        // A tiny model extent must not shrink the Rust worker's startup stack.
        None
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
            limit.current = limit.current.min(requested_memory);
            if prlimit(0, 9, &limit, std::ptr::null_mut()) != 0 {
                return Err(io::Error::last_os_error());
            }
            if let Some(requested_stack) = requested_stack {
                if prlimit(0, 3, std::ptr::null(), &mut limit) != 0 {
                    return Err(io::Error::last_os_error());
                }
                limit.current = limit.current.min(requested_stack);
                if prlimit(0, 3, &limit, std::ptr::null_mut()) != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    Ok(selected_memory)
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
    let budget = prepare_budgeted_child(&mut command, false)?;
    let status = command
        .status()
        .map_err(|error| format!("E_HOST execution worker: {error}"))?;
    // Program streams belong to the child directly: proxying stderr would hide
    // prompts and change write errors when the caller closes a pipe or fd2.
    if let Some(signal) = status.signal() {
        let extent = if budget == u64::MAX {
            "unlimited (inherited)".to_string()
        } else {
            format!("{budget} bytes")
        };
        return Err(format!(
            "E_HOST execution worker terminated by signal {signal} under Linux address-space budget {extent}"
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
