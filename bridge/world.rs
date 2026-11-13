//! The bootstrap K-machine host. Authority is represented by owned descriptors;
//! handles are private registry identities, never source-visible integers.
#[allow(dead_code)]
#[path = "output.rs"]
mod oracle;
use oracle::files;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;

pub fn restore_closed_standard_streams() {
    files::restore_closed_standard_streams();
}

enum Handle {
    Directory(std::fs::File),
    File(std::fs::File),
}

pub struct Host {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: u8,
    arguments: Vec<Vec<u8>>,
    input: Vec<u8>,
    input_position: usize,
    is_stdio: bool,
    handles: BTreeMap<u64, Handle>,
    next_handle: u64,
}

impl Host {
    /// Arguments already exclude argv0. Bytes need not be UTF-8.
    pub fn stdio(arguments: &[OsString]) -> Self {
        let arguments = arguments
            .iter()
            .map(|value| value.as_bytes().to_vec())
            .collect();
        let mut host = Self::with_inputs(arguments, Vec::new());
        host.is_stdio = true;
        host
    }

    pub fn captured() -> Self {
        Self::with_inputs(Vec::new(), Vec::new())
    }

    pub fn with_inputs(arguments: Vec<Vec<u8>>, input: Vec<u8>) -> Self {
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
            status: 0,
            arguments,
            input,
            input_position: 0,
            is_stdio: false,
            handles: BTreeMap::new(),
            next_handle: 1,
        }
    }

    fn insert_handle(&mut self, handle: Handle) -> Result<u64, i64> {
        let identity = self.next_handle;
        self.next_handle = identity.checked_add(1).ok_or(75)?;
        self.handles.insert(identity, handle);
        Ok(identity)
    }
}

impl lkjscript::machine::Host for Host {
    fn write_stdout(&mut self, bytes: &[u8]) -> Result<(), String> {
        if self.is_stdio {
            files::write_all(1, bytes)
                .map_err(|error| std::io::Error::from_raw_os_error(error as i32).to_string())
        } else {
            self.stdout.extend_from_slice(bytes);
            Ok(())
        }
    }

    fn arguments(&self) -> &[Vec<u8>] {
        &self.arguments
    }

    fn read_stdin(&mut self, bytes: &mut [u8]) -> Result<usize, i64> {
        if self.is_stdio {
            files::read(0, bytes)
        } else {
            let count = bytes.len().min(self.input.len() - self.input_position);
            bytes[..count]
                .copy_from_slice(&self.input[self.input_position..self.input_position + count]);
            self.input_position += count;
            Ok(count)
        }
    }

    fn write_stderr(&mut self, bytes: &[u8]) -> Result<(), String> {
        if self.is_stdio {
            files::write_all(2, bytes)
                .map_err(|error| std::io::Error::from_raw_os_error(error as i32).to_string())
        } else {
            self.stderr.extend_from_slice(bytes);
            Ok(())
        }
    }

    fn open_directory(&mut self, path: &[u8]) -> Result<u64, i64> {
        self.insert_handle(Handle::Directory(files::open_directory(path)?))
    }

    fn open_file(&mut self, directory: u64, path: &[u8], create: bool) -> Result<u64, i64> {
        let Some(Handle::Directory(directory)) = self.handles.get(&directory) else {
            return Err(9);
        };
        let file = files::open_file(directory, path, create)?;
        self.insert_handle(Handle::File(file))
    }

    fn read_file(&mut self, handle: u64, bytes: &mut [u8]) -> Result<usize, i64> {
        let Some(Handle::File(file)) = self.handles.get(&handle) else {
            return Err(9);
        };
        files::read(file.as_raw_fd(), bytes)
    }

    fn write_file(&mut self, handle: u64, bytes: &[u8]) -> Result<usize, i64> {
        let Some(Handle::File(file)) = self.handles.get(&handle) else {
            return Err(9);
        };
        files::write(file.as_raw_fd(), bytes)
    }

    fn close_handle(&mut self, handle: u64) {
        self.handles.remove(&handle);
    }

    fn set_exit_status(&mut self, status: u8) {
        self.status = status;
    }
}
