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
    handle: std::fs::File,
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
    pub fn open(&mut self, path: &str) -> Result<File, i64> {
        self.__lkjscript_open(&files::path_bytes(path)?, false)
    }
    pub fn create(&mut self, path: &str) -> Result<File, i64> {
        self.__lkjscript_open(&files::path_bytes(path)?, true)
    }
    pub fn __lkjscript_open(&mut self, path: &[u8], create: bool) -> Result<File, i64> {
        Ok(File {
            handle: files::open_file(&self.handle, path, create)?,
        })
    }
}
impl File {
    pub fn __lkjscript_read(&mut self, bytes: &mut [u8]) -> i64 {
        count(self.read(bytes))
    }
    pub fn __lkjscript_write(&mut self, bytes: &[u8]) -> i64 {
        count(self.write(bytes))
    }
    pub fn read(&mut self, bytes: &mut [u8]) -> Result<usize, i64> {
        files::read(self.handle.as_raw_fd(), bytes)
    }
    pub fn write(&mut self, bytes: &[u8]) -> Result<usize, i64> {
        files::write(self.handle.as_raw_fd(), bytes)
    }
    pub fn read_to_end(&mut self) -> Result<Vec<u8>, i64> {
        files::read_to_end(self.handle.as_raw_fd())
    }
    pub fn write_all(&mut self, bytes: &[u8]) -> Result<(), i64> {
        files::write_all(self.handle.as_raw_fd(), bytes)
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
}
