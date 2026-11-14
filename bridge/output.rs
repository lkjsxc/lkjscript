//! The rustc-only conformance host. Source programs receive capabilities but
//! cannot construct them; only the wrapper calls enter with the language entry.
#![allow(unexpected_cfgs)]
use std::cell::Cell;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::rc::Rc;

pub struct World {
    arguments: Vec<Vec<u8>>,
    status: Rc<Cell<u8>>,
}
pub struct Stdout {
    _seal: (),
}
pub struct Stdin {
    _seal: (),
}
pub struct Stderr {
    _seal: (),
}
pub struct Directory {
    handle: std::fs::File,
}
pub struct File {
    handle: Option<std::fs::File>,
}

impl World {
    pub fn stdout(&mut self) -> Stdout {
        Stdout { _seal: () }
    }
    pub fn stdin(&mut self) -> Stdin {
        Stdin { _seal: () }
    }
    pub fn stderr(&mut self) -> Stderr {
        Stderr { _seal: () }
    }
    pub fn args(&mut self) -> Vec<Vec<u8>> {
        self.arguments.clone()
    }
    pub fn __lkjscript_arg_count(&mut self) -> usize {
        self.arguments.len()
    }
    pub fn __lkjscript_arg_len(&mut self, index: usize) -> usize {
        self.arguments[index].len()
    }
    pub fn __lkjscript_arg_copy(&mut self, index: usize, bytes: &mut [u8]) -> usize {
        let argument = &self.arguments[index];
        let count = argument.len().min(bytes.len());
        bytes[..count].copy_from_slice(&argument[..count]);
        count
    }
    pub fn directory(&mut self, path: &str) -> Result<Directory, i64> {
        self.__lkjscript_directory(&files::path_bytes(path)?)
    }
    pub fn __lkjscript_directory(&mut self, path: &[u8]) -> Result<Directory, i64> {
        Ok(Directory {
            handle: files::open_directory(path)?,
        })
    }
    pub fn set_exit_status(&mut self, status: u8) {
        self.status.set(status);
    }
}
impl Stdout {
    pub fn write(&mut self, bytes: &[u8]) {
        if files::write_all(1, bytes).is_err() {
            trap("R0007");
        }
    }
}
impl Stderr {
    pub fn write(&mut self, bytes: &[u8]) {
        if files::write_all(2, bytes).is_err() {
            trap("R0007");
        }
    }
}
impl Stdin {
    pub fn __lkjscript_read(&mut self, bytes: &mut [u8]) -> i64 {
        count(files::read(0, bytes))
    }
    pub fn read(&mut self, bytes: &mut [u8]) -> Result<usize, i64> {
        files::read(0, bytes)
    }
    pub fn read_to_end(&mut self) -> Result<Vec<u8>, i64> {
        files::read_to_end(0)
    }
}
impl Directory {
    fn descriptor(&self) -> i32 {
        self.handle.as_raw_fd()
    }
    pub fn open(&mut self, path: &str) -> Result<File, i64> {
        self.__lkjscript_open(&files::path_bytes(path)?, false)
    }
    pub fn create(&mut self, path: &str) -> Result<File, i64> {
        self.__lkjscript_open(&files::path_bytes(path)?, true)
    }
    pub fn __lkjscript_open(&mut self, path: &[u8], create: bool) -> Result<File, i64> {
        Ok(File {
            handle: Some(files::open_file(&self.handle, path, create)?),
        })
    }
}
impl File {
    fn descriptor(&self) -> i32 {
        self.handle.as_ref().map_or(-1, |file| file.as_raw_fd())
    }
    pub fn __lkjscript_read(&mut self, bytes: &mut [u8]) -> i64 {
        count(self.read(bytes))
    }
    pub fn __lkjscript_write(&mut self, bytes: &[u8]) -> i64 {
        count(self.write(bytes))
    }
    pub fn read(&mut self, bytes: &mut [u8]) -> Result<usize, i64> {
        files::read(self.descriptor(), bytes)
    }
    pub fn write(&mut self, bytes: &[u8]) -> Result<usize, i64> {
        files::write(self.descriptor(), bytes)
    }
    pub fn read_to_end(&mut self) -> Result<Vec<u8>, i64> {
        files::read_to_end(self.descriptor())
    }
    pub fn write_all(&mut self, bytes: &[u8]) -> Result<(), i64> {
        files::write_all(self.descriptor(), bytes)
    }
}

pub struct Process {
    _seal: (),
}
pub struct Clock {
    _seal: (),
}
pub struct Child {
    handle: Option<std::process::Child>,
}
impl Drop for Child {
    fn drop(&mut self) {
        if let Some(child) = self.handle.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
impl World {
    pub fn process(&mut self) -> Process {
        Process { _seal: () }
    }
    pub fn clock(&mut self) -> Clock {
        Clock { _seal: () }
    }
    pub fn __lkjscript_file(&mut self, kind: u8) -> Result<File, i64> {
        Ok(File {
            handle: files::world_file(kind)?,
        })
    }
    pub fn temporary_file(&mut self) -> Result<File, i64> {
        self.__lkjscript_file(0)
    }
    pub fn stdin_file(&mut self) -> Result<File, i64> {
        self.__lkjscript_file(1)
    }
    pub fn stdout_file(&mut self) -> Result<File, i64> {
        self.__lkjscript_file(2)
    }
    pub fn stderr_file(&mut self) -> Result<File, i64> {
        self.__lkjscript_file(3)
    }
    pub fn executable(&mut self) -> Result<File, i64> {
        self.__lkjscript_file(4)
    }
}
impl Clock {
    pub fn __lkjscript_read(&mut self, bytes: &mut [u8]) -> i64 {
        files::monotonic_time(bytes).map_or_else(|e| -e, |()| 0)
    }
    pub fn nanoseconds(&mut self) -> Result<usize, i64> {
        let mut bytes = [0u8; 16];
        files::monotonic_time(&mut bytes)?;
        Ok(
            usize::from_le_bytes(bytes[..8].try_into().unwrap()) * 1_000_000_000
                + usize::from_le_bytes(bytes[8..].try_into().unwrap()),
        )
    }
}
impl Process {
    pub fn __lkjscript_spawn(
        &mut self,
        executable: &File,
        arguments: &[u8],
        input: &File,
        output: &File,
        errors: &File,
        result: &File,
    ) -> Result<Child, i64> {
        Ok(Child {
            handle: Some(files::spawn_process(
                executable.handle.as_ref().ok_or(9)?,
                arguments,
                [
                    input.handle.as_ref(),
                    output.handle.as_ref(),
                    errors.handle.as_ref(),
                    result.handle.as_ref(),
                ],
            )?),
        })
    }
    pub fn spawn_file(
        &mut self,
        executable: &File,
        arguments: &[Vec<u8>],
        input: &File,
        output: &File,
        errors: &File,
        result: &File,
    ) -> Result<Child, i64> {
        let mut bytes = b"lkjscript\0".to_vec();
        for argument in arguments {
            if argument.contains(&0) {
                return Err(22);
            }
            bytes.extend_from_slice(argument);
            bytes.push(0);
        }
        self.__lkjscript_spawn(executable, &bytes, input, output, errors, result)
    }
}
impl Child {
    pub fn __lkjscript_wait(&mut self, nonblocking: bool) -> i64 {
        let Some(child) = self.handle.as_mut() else {
            return -10;
        };
        match files::wait_process(child, nonblocking) {
            Err(error) => -error,
            Ok(None) => 0,
            Ok(Some(status)) => {
                self.handle = None;
                status + 1
            }
        }
    }
    pub fn __lkjscript_terminate(&mut self) -> i64 {
        let Some(child) = self.handle.as_mut() else {
            return -10;
        };
        child.kill().map_or_else(|e| -files::error(e), |()| 0)
    }
    pub fn wait(&mut self) -> Result<i64, i64> {
        let status = self.__lkjscript_wait(false);
        if status < 0 {
            Err(-status)
        } else {
            Ok(files::process_status(status - 1))
        }
    }
    pub fn try_wait(&mut self) -> Result<Option<i64>, i64> {
        let status = self.__lkjscript_wait(true);
        if status < 0 {
            Err(-status)
        } else if status == 0 {
            Ok(None)
        } else {
            Ok(Some(files::process_status(status - 1)))
        }
    }
    pub fn terminate(&mut self) -> Result<(), i64> {
        let status = self.__lkjscript_terminate();
        if status < 0 { Err(-status) } else { Ok(()) }
    }
}
impl File {
    pub fn __lkjscript_rewind(&mut self) -> i64 {
        files::rewind(self.descriptor()).map_or_else(|e| -e, |()| 0)
    }
    pub fn __lkjscript_executable(&mut self) -> i64 {
        files::executable(self.descriptor()).map_or_else(|e| -e, |()| 0)
    }
    pub fn __lkjscript_duplicate(&mut self) -> Result<File, i64> {
        Ok(File {
            handle: self.handle.as_ref().map(files::duplicate).transpose()?,
        })
    }
    pub fn __lkjscript_readonly(&mut self) -> Result<File, i64> {
        Ok(File {
            handle: self.handle.as_ref().map(files::readonly).transpose()?,
        })
    }
    pub fn readonly(&mut self) -> Result<File, i64> {
        self.__lkjscript_readonly()
    }
    pub fn duplicate(&mut self) -> Result<File, i64> {
        self.__lkjscript_duplicate()
    }
    pub fn rewind(&mut self) -> Result<(), i64> {
        files::rewind(self.descriptor())
    }
    pub fn set_executable(&mut self) -> Result<(), i64> {
        files::executable(self.descriptor())
    }
}
impl Directory {
    pub fn __lkjscript_open_directory(&mut self, path: &[u8]) -> Result<Directory, i64> {
        Ok(Directory {
            handle: files::open_subdirectory(&self.handle, path)?,
        })
    }
    pub fn open_directory(&mut self, path: &str) -> Result<Directory, i64> {
        self.__lkjscript_open_directory(&files::path_bytes(path)?)
    }
    pub fn __lkjscript_read(&mut self, bytes: &mut [u8]) -> i64 {
        count(files::read_directory(self.descriptor(), bytes))
    }
    pub fn __lkjscript_rewind(&mut self) -> i64 {
        files::rewind(self.descriptor()).map_or_else(|e| -e, |()| 0)
    }
    pub fn __lkjscript_create_directory(&mut self, name: &[u8]) -> i64 {
        files::create_directory(&self.handle, name).map_or_else(|e| -e, |()| 0)
    }
    pub fn __lkjscript_remove(&mut self, name: &[u8], directory: bool) -> i64 {
        files::remove_entry(&self.handle, name, directory).map_or_else(|e| -e, |()| 0)
    }
    pub fn __lkjscript_rename(&mut self, old: &[u8], to: &mut Directory, new: &[u8]) -> i64 {
        files::rename_entry(&self.handle, old, &to.handle, new).map_or_else(|e| -e, |()| 0)
    }
    fn parent(&mut self, path: &str) -> Result<(Directory, Vec<u8>), i64> {
        let split = path.rfind('/').map_or(0, |at| at + 1);
        let parent = if split == 0 { "." } else { &path[..split] };
        let name = &path[split..];
        if name.is_empty() || name == "." || name == ".." {
            return Err(22);
        }
        let owner = self.open_directory(parent)?;
        Ok((owner, files::path_bytes(name)?))
    }
    pub fn create_directory(&mut self, path: &str) -> Result<(), i64> {
        let (parent, name) = self.parent(path)?;
        files::create_directory(&parent.handle, &name)
    }
    pub fn remove(&mut self, path: &str) -> Result<(), i64> {
        let (parent, name) = self.parent(path)?;
        files::remove_entry(&parent.handle, &name, false)
    }
    pub fn remove_directory(&mut self, path: &str) -> Result<(), i64> {
        let (parent, name) = self.parent(path)?;
        files::remove_entry(&parent.handle, &name, true)
    }
    pub fn rename(&mut self, old: &str, new: &str) -> Result<(), i64> {
        let (from, old) = self.parent(old)?;
        let (to, new) = self.parent(new)?;
        files::rename_entry(&from.handle, &old, &to.handle, &new)
    }
    pub fn list(&mut self) -> Result<Vec<Vec<u8>>, i64> {
        let directory = self.open_directory(".")?;
        let mut bytes = [0u8; 8192];
        let mut names = Vec::new();
        loop {
            let count = files::read_directory(directory.handle.as_raw_fd(), &mut bytes)?;
            if count == 0 {
                return Ok(names);
            }
            let mut at = 0;
            while at < count {
                if count - at < 20 {
                    return Err(5);
                }
                let record = u16::from_le_bytes([bytes[at + 16], bytes[at + 17]]) as usize;
                if record < 20 || record > count - at {
                    return Err(5);
                }
                let name = bytes[at + 19..at + record]
                    .split(|b| *b == 0)
                    .next()
                    .unwrap();
                if name != b"." && name != b".." {
                    names.push(name.to_vec());
                }
                at += record;
            }
        }
    }
}

fn count(result: Result<usize, i64>) -> i64 {
    match result {
        Ok(count) => count as i64,
        Err(error) => -error,
    }
}

#[cfg(lkjscript_control)]
fn control(code: u64, status: u8) {
    let mut bytes = [0u8; 24];
    bytes[..8].copy_from_slice(&code.to_le_bytes());
    bytes[16..].copy_from_slice(&u64::from(status).to_le_bytes());
    let _ = files::write_all(3, &bytes);
}
#[cfg(not(lkjscript_control))]
fn control(_code: u64, _status: u8) {}

#[allow(clippy::exit)]
fn trap(code: &str) -> ! {
    let number = code
        .strip_prefix('R')
        .and_then(|digits| digits.parse::<u64>().ok())
        .unwrap_or(4);
    control(number, 0);
    #[cfg(not(lkjscript_control))]
    {
        let message = format!("LKJSCRIPT_RUST_TRAP {code}\n");
        // Legacy Rust helpers have no control pipe. A closed stderr must not
        // turn the language trap into a second Rust panic.
        let _ = files::write_all(2, message.as_bytes());
    }
    std::process::exit(101);
}

/// The test host owns fd3 only when compiled with --cfg lkjscript_control. Its record
/// distinguishes a trap from arbitrary program stderr or a normal status101.
#[allow(clippy::exit)]
pub fn observe(entry: impl FnOnce() -> u8) {
    files::restore_closed_standard_streams();
    // The language traps without unwinding. The oracle hook exits before Rust
    // unwinds so output/destructor observations obey that declared departure.
    std::panic::set_hook(Box::new(|info| {
        let text = info.to_string();
        let kind = if text.contains("overflow") {
            "R0001"
        } else if text.contains("divide by zero") || text.contains("divisor of zero") {
            "R0002"
        } else if text.contains("index out of bounds")
            || text.contains("out of range for slice")
            || text.contains("slice index starts at")
            || text.contains("byte range starts at")
            || text.contains("is not a char boundary")
            || text.contains("is out of bounds for string")
        {
            "R0003"
        } else if text.contains("assertion") {
            "R0005"
        } else {
            "R0004"
        };
        trap(kind);
    }));
    let status = entry();
    control(0, status);
    if status != 0 {
        std::process::exit(i32::from(status));
    }
}

pub fn enter(entry: fn(World)) {
    observe(|| {
        let status = Rc::new(Cell::new(0));
        let arguments = std::env::args_os()
            .skip(1)
            .map(|argument| argument.as_bytes().to_vec())
            .collect();
        entry(World {
            arguments,
            status: status.clone(),
        });
        status.get()
    });
}

/// The shared Linux x86-64 host boundary. Oracle harnesses paste this file as a
/// module; the IR-interpreter host imports the same helper rather than maintaining a
/// second path resolver. No source program can name descriptors or this module.
#[allow(unsafe_code)]
pub(crate) mod files {
    use std::ffi::CStr;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::sync::atomic::{AtomicU8, Ordering};

    unsafe extern "C" {
        fn syscall(number: i64, ...) -> i64;
        fn __errno_location() -> *mut i32;
    }

    // Rust startup replaces inherited closed stdio with /dev/null. Record the
    // Linux state before that bootstrap policy runs, then restore it once at
    // the host entry so Rust and standalone ELF expose the same capabilities.
    static CLOSED_STANDARD_STREAMS: AtomicU8 = AtomicU8::new(0);

    extern "C" fn remember_closed_standard_streams() {
        let mut mask = 0_u8;
        let mut descriptor = 0_i64;
        while descriptor < 3 {
            loop {
                let result = unsafe { syscall(72, descriptor, 1_i64, 0_i64) }; // F_GETFD
                if result >= 0 {
                    break;
                }
                let error = unsafe { *__errno_location() };
                if error == 4 {
                    continue;
                }
                if error == 9 {
                    mask |= 1_u8 << descriptor;
                }
                break;
            }
            descriptor += 1;
        }
        CLOSED_STANDARD_STREAMS.store(mask, Ordering::Relaxed);
    }

    #[used]
    #[unsafe(link_section = ".init_array")]
    static RECORD_STANDARD_STREAMS: extern "C" fn() = remember_closed_standard_streams;

    pub fn restore_closed_standard_streams() {
        let mask = CLOSED_STANDARD_STREAMS.swap(0, Ordering::Relaxed);
        let mut descriptor = 0_i64;
        while descriptor < 3 {
            if mask & (1_u8 << descriptor) != 0 {
                // close must not be retried, including after EINTR.
                unsafe { syscall(3, descriptor) };
            }
            descriptor += 1;
        }
    }

    #[repr(C)]
    struct OpenHow {
        flags: u64,
        mode: u64,
        resolve: u64,
    }

    fn retry(mut call: impl FnMut() -> i64) -> Result<i64, i64> {
        loop {
            let result = call();
            if result >= 0 {
                return Ok(result);
            }
            let error = i64::from(std::io::Error::last_os_error().raw_os_error().unwrap_or(5));
            if error != 4 {
                return Err(error);
            }
        }
    }

    fn path(path: &[u8]) -> Result<&CStr, i64> {
        CStr::from_bytes_with_nul(path).map_err(|_| 22)
    }

    pub fn path_bytes(path: &str) -> Result<Vec<u8>, i64> {
        if path.as_bytes().contains(&0) {
            return Err(22);
        }
        let mut bytes = path.as_bytes().to_vec();
        bytes.push(0);
        Ok(bytes)
    }

    fn owned(descriptor: i64) -> Result<std::fs::File, i64> {
        let descriptor = if descriptor < 3 {
            // Never retarget a standard stream which was closed by the operator.
            let promoted = retry(|| unsafe { syscall(72, descriptor, 1030_i64, 3_i64) });
            // Do not retry close: Linux releases ownership even on EINTR.
            unsafe { syscall(3, descriptor) };
            promoted?
        } else {
            descriptor
        };
        // Only a newly opened (or promoted) descriptor reaches this constructor.
        Ok(unsafe { std::fs::File::from_raw_fd(descriptor as i32) })
    }

    pub fn open_directory(bytes: &[u8]) -> Result<std::fs::File, i64> {
        let path = path(bytes)?;
        // World explicitly chooses the root. O_PATH needs no read permission.
        let descriptor =
            retry(|| unsafe { syscall(257, -100_i64, path.as_ptr(), 0x290000_i64, 0_i64) })?;
        owned(descriptor)
    }

    pub fn open_file(
        directory: &std::fs::File,
        bytes: &[u8],
        create: bool,
    ) -> Result<std::fs::File, i64> {
        let path = path(bytes)?;
        let how = OpenHow {
            flags: if create { 0x80241 } else { 0x80000 },
            mode: if create { 0o666 } else { 0 },
            resolve: 0x0a, // RESOLVE_BENEATH | RESOLVE_NO_MAGICLINKS
        };
        // No fallback: an older kernel reports ENOSYS instead of weakening
        // authority by resolving a path outside the held directory.
        let descriptor = retry(|| unsafe {
            syscall(
                437,
                i64::from(directory.as_raw_fd()),
                path.as_ptr(),
                &how as *const OpenHow,
                std::mem::size_of::<OpenHow>() as i64,
            )
        })?;
        owned(descriptor)
    }

    pub fn read(descriptor: i32, bytes: &mut [u8]) -> Result<usize, i64> {
        retry(|| unsafe {
            syscall(
                0,
                i64::from(descriptor),
                bytes.as_mut_ptr(),
                bytes.len() as u64,
            )
        })
        .map(|count| count as usize)
    }

    pub fn write(descriptor: i32, bytes: &[u8]) -> Result<usize, i64> {
        retry(|| unsafe { syscall(1, i64::from(descriptor), bytes.as_ptr(), bytes.len() as u64) })
            .map(|count| count as usize)
    }

    pub fn read_to_end(descriptor: i32) -> Result<Vec<u8>, i64> {
        let mut bytes = Vec::new();
        let mut buffer = [0; 8192];
        loop {
            let count = read(descriptor, &mut buffer)?;
            if count == 0 {
                return Ok(bytes);
            }
            bytes.extend_from_slice(&buffer[..count]);
        }
    }

    pub fn write_all(descriptor: i32, mut bytes: &[u8]) -> Result<(), i64> {
        while !bytes.is_empty() {
            let count = write(descriptor, bytes)?;
            if count == 0 {
                return Err(5);
            }
            bytes = &bytes[count..];
        }
        Ok(())
    }
    pub fn error(error: std::io::Error) -> i64 {
        i64::from(error.raw_os_error().unwrap_or(5))
    }
    pub fn world_file(kind: u8) -> Result<Option<std::fs::File>, i64> {
        let descriptor = match kind {
            0 => retry(|| unsafe { syscall(319, c"lkjscript".as_ptr(), 1_i64) }),
            1..=3 => retry(|| unsafe { syscall(72, i64::from(kind) - 1, 1030_i64, 3_i64) }),
            4 => retry(|| unsafe {
                syscall(
                    257,
                    -100_i64,
                    c"/proc/self/exe".as_ptr(),
                    0x80000_i64,
                    0_i64,
                )
            }),
            _ => return Err(22),
        };
        if matches!(kind, 1..=3) && descriptor == Err(9) {
            return Ok(None);
        }
        owned(descriptor?).map(Some)
    }
    pub fn rewind(descriptor: i32) -> Result<(), i64> {
        retry(|| unsafe { syscall(8, i64::from(descriptor), 0_i64, 0_i64) }).map(|_| ())
    }
    pub fn executable(descriptor: i32) -> Result<(), i64> {
        retry(|| unsafe { syscall(91, i64::from(descriptor), 0o755_i64) }).map(|_| ())
    }
    pub fn monotonic_time(bytes: &mut [u8]) -> Result<(), i64> {
        if bytes.len() < 16 {
            return Err(22);
        }
        retry(|| unsafe { syscall(228, 1_i64, bytes.as_mut_ptr()) }).map(|_| ())
    }
    pub fn open_subdirectory(
        directory: &std::fs::File,
        bytes: &[u8],
    ) -> Result<std::fs::File, i64> {
        let path = path(bytes)?;
        let how = OpenHow {
            flags: 0x90000,
            mode: 0,
            resolve: 0x0a,
        };
        owned(retry(|| unsafe {
            syscall(
                437,
                i64::from(directory.as_raw_fd()),
                path.as_ptr(),
                &how as *const OpenHow,
                24_i64,
            )
        })?)
    }
    pub fn read_directory(descriptor: i32, bytes: &mut [u8]) -> Result<usize, i64> {
        retry(|| unsafe {
            syscall(
                217,
                i64::from(descriptor),
                bytes.as_mut_ptr(),
                bytes.len() as u64,
            )
        })
        .map(|n| n as usize)
    }
    fn name(bytes: &[u8]) -> Result<&CStr, i64> {
        let name = path(bytes)?;
        if name.to_bytes().is_empty()
            || name.to_bytes().contains(&b'/')
            || name.to_bytes() == b"."
            || name.to_bytes() == b".."
        {
            return Err(22);
        }
        Ok(name)
    }
    pub fn create_directory(directory: &std::fs::File, bytes: &[u8]) -> Result<(), i64> {
        let name = name(bytes)?;
        retry(|| unsafe {
            syscall(
                258,
                i64::from(directory.as_raw_fd()),
                name.as_ptr(),
                0o777_i64,
            )
        })
        .map(|_| ())
    }
    pub fn remove_entry(
        directory: &std::fs::File,
        bytes: &[u8],
        is_directory: bool,
    ) -> Result<(), i64> {
        let name = name(bytes)?;
        retry(|| unsafe {
            syscall(
                263,
                i64::from(directory.as_raw_fd()),
                name.as_ptr(),
                if is_directory { 0x200_i64 } else { 0 },
            )
        })
        .map(|_| ())
    }
    pub fn rename_entry(
        from: &std::fs::File,
        old: &[u8],
        to: &std::fs::File,
        new: &[u8],
    ) -> Result<(), i64> {
        let old = name(old)?;
        let new = name(new)?;
        retry(|| unsafe {
            syscall(
                264,
                i64::from(from.as_raw_fd()),
                old.as_ptr(),
                i64::from(to.as_raw_fd()),
                new.as_ptr(),
            )
        })
        .map(|_| ())
    }
    pub fn readonly(file: &std::fs::File) -> Result<std::fs::File, i64> {
        let path = std::ffi::CString::new(format!("/proc/self/fd/{}", file.as_raw_fd())).unwrap();
        owned(retry(|| unsafe {
            syscall(257, -100_i64, path.as_ptr(), 0x80000_i64, 0_i64)
        })?)
    }
    pub fn duplicate(file: &std::fs::File) -> Result<std::fs::File, i64> {
        owned(retry(|| unsafe {
            syscall(72, i64::from(file.as_raw_fd()), 1030_i64, 4_i64)
        })?)
    }
    pub fn spawn_process(
        executable: &std::fs::File,
        arguments: &[u8],
        streams: [Option<&std::fs::File>; 4],
    ) -> Result<std::process::Child, i64> {
        use std::os::unix::ffi::OsStringExt;
        use std::os::unix::process::CommandExt;
        if arguments.is_empty() || arguments[arguments.len() - 1] != 0 {
            return Err(22);
        }
        let argv: Vec<std::ffi::OsString> = arguments[..arguments.len() - 1]
            .split(|byte| *byte == 0)
            .map(|value| std::ffi::OsString::from_vec(value.to_vec()))
            .collect();
        let executable = duplicate(executable)?;
        let result = streams[3].map(duplicate).transpose()?;
        let mut command =
            std::process::Command::new(format!("/proc/self/fd/{}", executable.as_raw_fd()));
        command.arg0(&argv[0]).args(&argv[1..]).env_clear();
        command.stdin(match streams[0] {
            Some(file) => std::process::Stdio::from(duplicate(file)?),
            None => std::process::Stdio::inherit(),
        });
        command.stdout(match streams[1] {
            Some(file) => std::process::Stdio::from(duplicate(file)?),
            None => std::process::Stdio::inherit(),
        });
        command.stderr(match streams[2] {
            Some(file) => std::process::Stdio::from(duplicate(file)?),
            None => std::process::Stdio::inherit(),
        });
        let strings: Vec<std::ffi::CString> = arguments[..arguments.len() - 1]
            .split(|byte| *byte == 0)
            .map(|value| std::ffi::CString::new(value).unwrap())
            .collect();
        let mut pointers: Vec<usize> = strings
            .iter()
            .map(|value| value.as_ptr() as usize)
            .collect();
        pointers.push(0);
        let executable_descriptor = executable.as_raw_fd();
        let result_descriptor = result.as_ref().map(|file| file.as_raw_fd());
        let mut closed = 0u8;
        for (index, stream) in streams.iter().enumerate() {
            if stream.is_none() {
                closed |= 1 << index;
            }
        }
        unsafe {
            command.pre_exec(move || {
                if syscall(157, 1_i64, 9_i64, 0_i64, 0_i64, 0_i64) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                for descriptor in 0..4 {
                    if closed & (1 << descriptor) != 0 {
                        syscall(3, descriptor as i64);
                    }
                }
                if let Some(result) = result_descriptor {
                    if syscall(33, i64::from(result), 3_i64) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                let _strings = &strings;
                syscall(
                    322,
                    i64::from(executable_descriptor),
                    c"".as_ptr(),
                    pointers.as_ptr(),
                    0usize,
                    0x1000_i64,
                );
                Err(std::io::Error::last_os_error())
            });
        }
        command.spawn().map_err(error)
    }
    pub fn wait_process(
        child: &mut std::process::Child,
        nonblocking: bool,
    ) -> Result<Option<i64>, i64> {
        use std::os::unix::process::ExitStatusExt;
        let status = if nonblocking {
            child.try_wait().map_err(error)?
        } else {
            Some(child.wait().map_err(error)?)
        };
        Ok(status.map(|value| i64::from(value.into_raw())))
    }
    pub fn process_status(status: i64) -> i64 {
        let signal = status & 127;
        if signal == 0 {
            (status >> 8) & 255
        } else {
            128 + signal
        }
    }
}
