// Interpreter effects use the same owned capabilities as source programs.
// Handle numbers belong to this registry and never expose machine descriptors.
use crate::{driver, interpreter, ir};
use std::collections::BTreeMap;

enum Handle {
    Directory(Directory),
    File(File),
    Child(Child),
}

struct Streams {
    input: File,
    output: File,
    errors: File,
}

pub struct Host<'world, 'program> {
    world: &'world mut World,
    process: Process,
    clock: Clock,
    handles: BTreeMap<u64, Handle>,
    next_handle: u64,
    arguments: Vec<Vec<u8>>,
    capture: Option<Streams>,
    checked: Option<&'program ir::CheckedProgram>,
    entry: usize,
    runtime_source: Option<String>,
    executable: Option<File>,
    executable_error: Option<String>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: u8,
}

fn temporary(world: &mut World) -> Result<File, String> {
    match world.__lkjscript_file(0u8) {
        Ok(file) => Ok(file),
        Err(error) => Err(format!("capture file failed (errno {error})")),
    }
}

fn write_all(file: &mut File, bytes: &[u8]) -> Result<(), String> {
    let mut at = 0usize;
    while at < bytes.len() {
        let result = file.__lkjscript_write(&bytes[at..]);
        if result < 0 {
            return Err(format!("write failed (errno {})", -result));
        }
        if result == 0 {
            return Err("write made no progress".to_string());
        }
        at += result as usize;
    }
    Ok(())
}

fn read_capture(file: &mut File) -> Result<Vec<u8>, String> {
    let rewind = file.__lkjscript_rewind();
    if rewind < 0 {
        return Err(format!("capture rewind failed (errno {})", -rewind));
    }
    match file.read_to_end() {
        Ok(bytes) => Ok(bytes),
        Err(error) => Err(format!("capture read failed (errno {error})")),
    }
}

fn count(value: i64) -> Result<usize, i64> {
    if value < 0 {
        Err(-value)
    } else {
        Ok(value as usize)
    }
}

fn status(value: i64) -> Result<(), i64> {
    if value < 0 { Err(-value) } else { Ok(()) }
}

impl<'world, 'program> Host<'world, 'program> {
    pub fn new(
        world: &'world mut World,
        arguments: Vec<Vec<u8>>,
        capture: bool,
        checked: Option<&'program ir::CheckedProgram>,
        entry: usize,
        runtime_source: Option<String>,
    ) -> Result<Self, String> {
        let process = world.process();
        let clock = world.clock();
        let capture = if capture {
            Some(Streams {
                input: temporary(world)?,
                output: temporary(world)?,
                errors: temporary(world)?,
            })
        } else {
            None
        };
        Ok(Self {
            world,
            process,
            clock,
            handles: BTreeMap::new(),
            next_handle: 1,
            arguments,
            capture,
            checked,
            entry,
            runtime_source,
            executable: None,
            executable_error: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
            status: 0,
        })
    }

    pub fn finish(&mut self) -> Result<(), String> {
        self.handles = BTreeMap::new();
        if let Some(streams) = self.capture.as_mut() {
            self.stdout = read_capture(&mut streams.output)?;
            self.stderr = read_capture(&mut streams.errors)?;
        }
        if let Some(problem) = self.executable_error.take() {
            return Err(problem);
        }
        Ok(())
    }

    fn program_executable(&mut self) -> Result<File, i64> {
        if self.executable.is_none() {
            let checked = match self.checked {
                Some(checked) => checked,
                None => return Err(38),
            };
            let bytes = match driver::interpreter_image(
                self.world,
                checked,
                self.entry,
                &self.runtime_source,
            ) {
                Ok(bytes) => bytes,
                Err(problem) => {
                    self.executable_error = Some(problem);
                    return Err(8);
                }
            };
            let mut file = self.world.__lkjscript_file(0u8)?;
            file.write_all(&bytes)?;
            file.set_executable()?;
            self.executable = Some(file);
        }
        match self.executable.as_mut() {
            Some(file) => file.__lkjscript_readonly(),
            None => Err(8),
        }
    }

    fn insert_handle(&mut self, handle: Handle) -> Result<u64, i64> {
        let identity = self.next_handle;
        self.next_handle = identity.checked_add(1).ok_or(75i64)?;
        self.handles.insert(identity, handle);
        Ok(identity)
    }

    fn write_stream(&mut self, kind: u8, bytes: &[u8]) -> Result<(), String> {
        let mut file = match self.world.__lkjscript_file(kind) {
            Ok(file) => file,
            Err(error) => return Err(format!("open output stream failed (errno {error})")),
        };
        write_all(&mut file, bytes)
    }
}

impl interpreter::Host for Host<'_, '_> {
    fn write_stdout(&mut self, bytes: &[u8]) -> Result<(), String> {
        match self.capture.as_mut() {
            Some(streams) => write_all(&mut streams.output, bytes),
            None => self.write_stream(2u8, bytes),
        }
    }

    fn write_stderr(&mut self, bytes: &[u8]) -> Result<(), String> {
        match self.capture.as_mut() {
            Some(streams) => write_all(&mut streams.errors, bytes),
            None => self.write_stream(3u8, bytes),
        }
    }

    fn argument_count(&self) -> usize {
        self.arguments.len()
    }

    fn argument(&self, index: usize) -> Option<Vec<u8>> {
        match self.arguments.get(index) {
            Some(argument) => Some(argument.clone()),
            None => None,
        }
    }

    fn read_stdin(&mut self, bytes: &mut [u8]) -> Result<usize, i64> {
        match self.capture.as_mut() {
            Some(streams) => count(streams.input.__lkjscript_read(bytes)),
            None => {
                let mut input = self.world.stdin();
                count(input.__lkjscript_read(bytes))
            }
        }
    }

    fn open_directory(&mut self, path: &[u8]) -> Result<u64, i64> {
        let directory = self.world.__lkjscript_directory(path)?;
        self.insert_handle(Handle::Directory(directory))
    }

    fn open_file(&mut self, directory: u64, path: &[u8], create: bool) -> Result<u64, i64> {
        let opened = match self.handles.get_mut(&directory) {
            Some(Handle::Directory(directory)) => directory.__lkjscript_open(path, create),
            _ => return Err(9),
        };
        self.insert_handle(Handle::File(opened?))
    }

    fn duplicate_file(&mut self, handle: u64) -> Result<u64, i64> {
        let file = match self.handles.get_mut(&handle) {
            Some(Handle::File(file)) => file.__lkjscript_duplicate(),
            _ => return Err(9),
        }?;
        self.insert_handle(Handle::File(file))
    }

    fn readonly_file(&mut self, handle: u64) -> Result<u64, i64> {
        let file = match self.handles.get_mut(&handle) {
            Some(Handle::File(file)) => file.__lkjscript_readonly(),
            _ => return Err(9),
        }?;
        self.insert_handle(Handle::File(file))
    }

    fn read_file(&mut self, handle: u64, bytes: &mut [u8]) -> Result<usize, i64> {
        match self.handles.get_mut(&handle) {
            Some(Handle::File(file)) => count(file.__lkjscript_read(bytes)),
            _ => Err(9),
        }
    }

    fn write_file(&mut self, handle: u64, bytes: &[u8]) -> Result<usize, i64> {
        match self.handles.get_mut(&handle) {
            Some(Handle::File(file)) => count(file.__lkjscript_write(bytes)),
            _ => Err(9),
        }
    }

    fn close_handle(&mut self, handle: u64) {
        self.handles.remove(&handle);
    }
    fn set_exit_status(&mut self, status: u8) {
        self.status = status;
    }

    fn world_file(&mut self, kind: u8) -> Result<u64, i64> {
        if kind == 4u8 {
            let file = self.program_executable()?;
            return self.insert_handle(Handle::File(file));
        }
        let file = match self.capture.as_mut() {
            Some(streams) => match kind {
                1 => streams.input.__lkjscript_duplicate(),
                2 => streams.output.__lkjscript_duplicate(),
                3 => streams.errors.__lkjscript_duplicate(),
                _ => self.world.__lkjscript_file(kind),
            },
            None => self.world.__lkjscript_file(kind),
        }?;
        self.insert_handle(Handle::File(file))
    }

    fn open_subdirectory(&mut self, directory: u64, path: &[u8]) -> Result<u64, i64> {
        let opened = match self.handles.get_mut(&directory) {
            Some(Handle::Directory(directory)) => directory.__lkjscript_open_directory(path),
            _ => return Err(9),
        };
        self.insert_handle(Handle::Directory(opened?))
    }

    fn read_directory(&mut self, directory: u64, bytes: &mut [u8]) -> Result<usize, i64> {
        match self.handles.get_mut(&directory) {
            Some(Handle::Directory(directory)) => count(directory.__lkjscript_read(bytes)),
            _ => Err(9),
        }
    }

    fn rewind_handle(&mut self, handle: u64) -> Result<(), i64> {
        match self.handles.get_mut(&handle) {
            Some(Handle::Directory(directory)) => status(directory.__lkjscript_rewind()),
            Some(Handle::File(file)) => status(file.__lkjscript_rewind()),
            _ => Err(9),
        }
    }

    fn create_directory(&mut self, directory: u64, name: &[u8]) -> Result<(), i64> {
        match self.handles.get_mut(&directory) {
            Some(Handle::Directory(directory)) => {
                status(directory.__lkjscript_create_directory(name))
            }
            _ => Err(9),
        }
    }

    fn remove_entry(&mut self, directory: u64, name: &[u8], is_directory: bool) -> Result<(), i64> {
        match self.handles.get_mut(&directory) {
            Some(Handle::Directory(directory)) => {
                status(directory.__lkjscript_remove(name, is_directory))
            }
            _ => Err(9),
        }
    }

    fn rename_entry(&mut self, from: u64, old: &[u8], to: u64, new: &[u8]) -> Result<(), i64> {
        let mut directory = match self.handles.remove(&from) {
            Some(Handle::Directory(directory)) => directory,
            Some(other) => {
                self.handles.insert(from, other);
                return Err(9);
            }
            None => return Err(9),
        };
        let result = if from == to {
            match directory.__lkjscript_open_directory(b".\0") {
                Ok(mut target) => status(directory.__lkjscript_rename(old, &mut target, new)),
                Err(error) => Err(error),
            }
        } else {
            match self.handles.get_mut(&to) {
                Some(Handle::Directory(target)) => {
                    status(directory.__lkjscript_rename(old, target, new))
                }
                _ => Err(9),
            }
        };
        self.handles.insert(from, Handle::Directory(directory));
        result
    }

    fn file_executable(&mut self, file: u64) -> Result<(), i64> {
        match self.handles.get_mut(&file) {
            Some(Handle::File(file)) => status(file.__lkjscript_executable()),
            _ => Err(9),
        }
    }

    fn monotonic_time(&mut self, bytes: &mut [u8]) -> Result<(), i64> {
        status(self.clock.__lkjscript_read(bytes))
    }

    fn spawn_process(
        &mut self,
        executable: u64,
        arguments: &[u8],
        input: u64,
        output: u64,
        errors: u64,
        result: u64,
    ) -> Result<u64, i64> {
        let child = {
            let executable = match self.handles.get(&executable) {
                Some(Handle::File(file)) => file,
                _ => return Err(9),
            };
            let input = match self.handles.get(&input) {
                Some(Handle::File(file)) => file,
                _ => return Err(9),
            };
            let output = match self.handles.get(&output) {
                Some(Handle::File(file)) => file,
                _ => return Err(9),
            };
            let errors = match self.handles.get(&errors) {
                Some(Handle::File(file)) => file,
                _ => return Err(9),
            };
            let result = match self.handles.get(&result) {
                Some(Handle::File(file)) => file,
                _ => return Err(9),
            };
            self.process
                .__lkjscript_spawn(executable, arguments, input, output, errors, result)?
        };
        self.insert_handle(Handle::Child(child))
    }

    fn wait_process(&mut self, child: u64, nonblocking: bool) -> Result<Option<i64>, i64> {
        let value = match self.handles.get_mut(&child) {
            Some(Handle::Child(process)) => process.__lkjscript_wait(nonblocking),
            _ => return Err(10),
        };
        if value < 0 {
            Err(-value)
        } else if value == 0 {
            Ok(None)
        } else {
            self.handles.remove(&child);
            Ok(Some(value - 1))
        }
    }

    fn terminate_process(&mut self, child: u64) -> Result<(), i64> {
        match self.handles.get_mut(&child) {
            Some(Handle::Child(process)) => status(process.__lkjscript_terminate()),
            _ => Err(10),
        }
    }

    fn drop_process(&mut self, child: u64) {
        self.handles.remove(&child);
    }
}
