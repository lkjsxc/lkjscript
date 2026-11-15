// Interpreter effects use the same owned capabilities as source programs.
// Handle numbers belong to this registry and never expose machine descriptors.
use crate::{driver, interpreter, ir};
use std::collections::BTreeMap;

enum Handle {
    Directory(u64),
    File(File),
    ImportedFile(File, u64),
    RemoteFile(u64),
    RemoteChild(u64),
    Listener(u64),
    Readiness(u64),
    Stop(u64, bool),
}

struct Streams {
    input: File,
    output: File,
    errors: File,
}

pub struct Host<'world, 'program> {
    world: &'world mut World,
    clock: Clock,
    network: crate::host_network::Network,
    handles: BTreeMap<u64, Handle>,
    next_handle: u64,
    arguments: Vec<Vec<u8>>,
    capture: Option<Streams>,
    // Native self-image authority; interpretation separately retains the original IR.
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
            clock,
            network: crate::host_network::Network::new(),
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
        self.network.finish();
        if let Some(streams) = self.capture.as_mut() {
            self.stdout = read_capture(&mut streams.output)?;
            self.stderr = read_capture(&mut streams.errors)?;
        }
        if let Some(problem) = self.executable_error.take() {
            return Err(problem);
        }
        Ok(())
    }

    fn directory_handle(&self, handle: u64) -> Result<u64, i64> {
        match self.handles.get(&handle) {
            Some(Handle::Directory(descriptor)) => Ok(*descriptor),
            _ => Err(9),
        }
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
        self.next_handle = match identity.checked_add(1) {
            Some(next) => next,
            None => { self.dispose_handle(handle); return Err(75); }
        };
        self.handles.insert(identity, handle);
        Ok(identity)
    }

    fn dispose_handle(&mut self, handle: Handle) {
        match handle {
            Handle::Directory(descriptor)
            | Handle::ImportedFile(_, descriptor)
            | Handle::RemoteFile(descriptor)
            | Handle::Listener(descriptor)
            | Handle::Readiness(descriptor)
            | Handle::Stop(descriptor, _) => self.network.close_remote(descriptor),
            Handle::RemoteChild(identity) => self.network.drop_remote_child(identity),
            _ => {}
        }
    }

    fn remote_file(&mut self, handle: u64) -> Result<u64, i64> {
        let value = self.handles.remove(&handle).ok_or(9i64)?;
        match value {
            Handle::File(file) => {
                let result = self.network.import_owned(self.world, &file);
                match result {
                    Ok(descriptor) => {
                        self.handles.insert(handle, Handle::ImportedFile(file, descriptor));
                        Ok(descriptor)
                    }
                    Err(error) => {
                        self.handles.insert(handle, Handle::File(file));
                        Err(error)
                    }
                }
            }
            Handle::ImportedFile(file, descriptor) => {
                self.handles.insert(handle, Handle::ImportedFile(file, descriptor));
                Ok(descriptor)
            }
            Handle::RemoteFile(descriptor) => {
                self.handles.insert(handle, Handle::RemoteFile(descriptor));
                Ok(descriptor)
            }
            other => {
                self.handles.insert(handle, other);
                Err(9)
            }
        }
    }

    fn stop_handle(&self, handle: u64) -> Result<u64, i64> {
        match self.handles.get(&handle) {
            Some(Handle::Stop(descriptor, _)) => Ok(*descriptor),
            _ => Err(9),
        }
    }

    fn selector_handle(&self, handle: u64) -> Result<u64, i64> {
        match self.handles.get(&handle) {
            Some(Handle::Readiness(descriptor)) => Ok(*descriptor),
            _ => Err(9),
        }
    }

    fn listener_handle(&self, handle: u64) -> Result<u64, i64> {
        match self.handles.get(&handle) {
            Some(Handle::Listener(descriptor)) => Ok(*descriptor),
            _ => Err(9),
        }
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

    fn connect_socket(&mut self, address: &[u8]) -> Result<u64, i64> {
        let descriptor = self.network.open_stream(self.world, address)?;
        self.insert_handle(Handle::RemoteFile(descriptor))
    }

    fn open_listener(&mut self, address: &[u8]) -> Result<u64, i64> {
        let descriptor = self.network.open_listener(self.world, address)?;
        self.insert_handle(Handle::Listener(descriptor))
    }

    fn accept_listener(&mut self, listener: u64) -> Result<u64, i64> {
        let listener = self.listener_handle(listener)?;
        let descriptor = self.network.take_connection(listener, false)?;
        self.insert_handle(Handle::RemoteFile(descriptor))
    }

    fn accept_listener_nodelay(&mut self, listener: u64) -> Result<u64, i64> {
        let listener = self.listener_handle(listener)?;
        let descriptor = self.network.take_connection(listener, true)?;
        self.insert_handle(Handle::RemoteFile(descriptor))
    }

    fn listener_address(&mut self, listener: u64, bytes: &mut [u8]) -> Result<(), i64> {
        let descriptor = self.listener_handle(listener)?;
        self.network.address_of(descriptor, bytes)
    }

    fn fill_entropy(&mut self, bytes: &mut [u8]) -> Result<(), i64> {
        self.network.random_bytes(self.world, bytes)
    }
    fn realtime_seconds(&mut self) -> Result<i64, i64> {
        self.network.realtime_value(self.world)
    }
    fn private_file(&mut self, handle: u64) -> Result<(), i64> {
        let descriptor = self.remote_file(handle)?;
        self.network.private_remote(descriptor)
    }
    fn create_stop(&mut self) -> Result<u64, i64> {
        let descriptor = self.network.acquire_stop(self.world)?;
        self.insert_handle(Handle::Stop(descriptor, false))
    }
    fn close_stop(&mut self, handle: u64) {
        if let Some(value) = self.handles.remove(&handle) { self.dispose_handle(value); }
    }
    fn stop_requested(&mut self, handle: u64) -> Result<bool, i64> {
        let descriptor = match self.handles.get(&handle) {
            Some(Handle::Stop(descriptor, sticky)) => { if *sticky { return Ok(true); } *descriptor },
            _ => return Err(9),
        };
        let observed = self.network.consume_stop(descriptor)?;
        if observed {
            if let Some(Handle::Stop(_, sticky)) = self.handles.get_mut(&handle) { *sticky = true; }
        }
        Ok(observed)
    }
    fn stop_target(&mut self, handle: u64) -> Result<u32, i64> {
        let descriptor = self.stop_handle(handle)?;
        self.network.signal_endpoint(descriptor)
    }
    fn readiness_watch_stop(&mut self, readiness: u64, handle: u64, token: usize) -> Result<(), i64> {
        let selector = self.selector_handle(readiness)?;
        let descriptor = self.stop_handle(handle)?;
        self.network.register_interest(selector, descriptor, token, 1u8)
    }
    fn readiness_unwatch_stop(&mut self, readiness: u64, handle: u64) -> Result<(), i64> {
        let selector = self.selector_handle(readiness)?;
        let descriptor = self.stop_handle(handle)?;
        self.network.remove_interest(selector, descriptor)
    }
    fn request_stop_process(&mut self, child: u64) -> Result<(), i64> {
        match self.handles.get(&child) {
            Some(Handle::RemoteChild(identity)) => self.network.stop_remote_child(*identity),
            _ => Err(10),
        }
    }

    fn create_readiness(&mut self) -> Result<u64, i64> {
        let descriptor = self.network.create_selector(self.world)?;
        self.insert_handle(Handle::Readiness(descriptor))
    }

    fn close_readiness(&mut self, readiness: u64) {
        if let Some(value) = self.handles.remove(&readiness) {
            self.dispose_handle(value);
        }
    }

    fn set_file_nonblocking(&mut self, handle: u64, enabled: bool) -> Result<(), i64> {
        let descriptor = self.remote_file(handle)?;
        self.network.set_status_flags(descriptor, enabled)
    }

    fn set_listener_nonblocking(&mut self, handle: u64, enabled: bool) -> Result<(), i64> {
        let descriptor = self.listener_handle(handle)?;
        self.network.set_status_flags(descriptor, enabled)
    }

    fn readiness_watch_file(&mut self, readiness: u64, handle: u64, token: usize, interest: u8) -> Result<(), i64> {
        let selector = self.selector_handle(readiness)?;
        let descriptor = self.remote_file(handle)?;
        self.network.register_interest(selector, descriptor, token, interest)
    }

    fn readiness_watch_listener(&mut self, readiness: u64, handle: u64, token: usize) -> Result<(), i64> {
        let selector = self.selector_handle(readiness)?;
        let descriptor = self.listener_handle(handle)?;
        self.network.register_interest(selector, descriptor, token, 1u8)
    }

    fn readiness_unwatch_file(&mut self, readiness: u64, handle: u64) -> Result<(), i64> {
        let selector = self.selector_handle(readiness)?;
        let descriptor = self.remote_file(handle)?;
        self.network.remove_interest(selector, descriptor)
    }

    fn readiness_unwatch_listener(&mut self, readiness: u64, handle: u64) -> Result<(), i64> {
        let selector = self.selector_handle(readiness)?;
        let descriptor = self.listener_handle(handle)?;
        self.network.remove_interest(selector, descriptor)
    }

    fn readiness_wait(&mut self, readiness: u64, deadline_ns: usize) -> Result<Vec<(usize, u8)>, i64> {
        let selector = self.selector_handle(readiness)?;
        self.network.wait_events(selector, deadline_ns)
    }

    fn open_directory(&mut self, path: &[u8]) -> Result<u64, i64> {
        let directory = self.network.open_root(self.world, path)?;
        self.insert_handle(Handle::Directory(directory))
    }

    fn open_file(&mut self, directory: u64, path: &[u8], create: bool) -> Result<u64, i64> {
        let directory = self.directory_handle(directory)?;
        let opened = self.network.open_relative_file(directory, path, create)?;
        self.insert_handle(Handle::RemoteFile(opened))
    }

    fn open_file_storage(&mut self, directory: u64, path: &[u8], append: bool) -> Result<u64, i64> {
        let directory = self.directory_handle(directory)?;
        let opened = self.network.open_storage(directory, path, append)?;
        self.insert_handle(Handle::RemoteFile(opened))
    }

    fn open_file_private(&mut self, directory: u64, path: &[u8]) -> Result<u64, i64> {
        let directory = self.directory_handle(directory)?;
        let opened = self.network.open_private(directory, path)?;
        self.insert_handle(Handle::RemoteFile(opened))
    }

    fn sync_file(&mut self, handle: u64) -> Result<(), i64> {
        let descriptor = self.remote_file(handle)?;
        self.network.sync_remote(descriptor)
    }

    fn sync_directory(&mut self, handle: u64) -> Result<(), i64> {
        let directory = self.directory_handle(handle)?;
        let descriptor = self.network.open_relative_directory(directory, b".\0")?;
        let outcome = self.network.sync_remote(descriptor);
        self.network.close_remote(descriptor);
        outcome
    }

    fn truncate_file(&mut self, handle: u64, length: usize) -> Result<(), i64> {
        let descriptor = self.remote_file(handle)?;
        self.network.resize_remote(descriptor, length)
    }

    fn lock_file_exclusive(&mut self, handle: u64) -> Result<(), i64> {
        let descriptor = self.remote_file(handle)?;
        self.network.lock_remote(descriptor)
    }

    fn duplicate_file(&mut self, handle: u64) -> Result<u64, i64> {
        let duplicated = match self.handles.get_mut(&handle) {
            Some(Handle::File(file)) | Some(Handle::ImportedFile(file, _)) => {
                Handle::File(file.__lkjscript_duplicate()?)
            }
            Some(Handle::RemoteFile(descriptor)) => {
                Handle::RemoteFile(self.network.duplicate_remote(*descriptor)?)
            }
            _ => return Err(9),
        };
        self.insert_handle(duplicated)
    }

    fn readonly_file(&mut self, handle: u64) -> Result<u64, i64> {
        let opened = match self.handles.get_mut(&handle) {
            Some(Handle::File(file)) | Some(Handle::ImportedFile(file, _)) => {
                Handle::File(file.__lkjscript_readonly()?)
            }
            Some(Handle::RemoteFile(descriptor)) => {
                Handle::RemoteFile(self.network.readonly_remote(*descriptor)?)
            }
            _ => return Err(9),
        };
        self.insert_handle(opened)
    }

    fn read_file(&mut self, handle: u64, bytes: &mut [u8]) -> Result<usize, i64> {
        match self.handles.get_mut(&handle) {
            Some(Handle::File(file)) | Some(Handle::ImportedFile(file, _)) => count(file.__lkjscript_read(bytes)),
            Some(Handle::RemoteFile(descriptor)) => self.network.read_remote(*descriptor, bytes),
            _ => Err(9),
        }
    }

    fn write_file(&mut self, handle: u64, bytes: &[u8]) -> Result<usize, i64> {
        match self.handles.get_mut(&handle) {
            Some(Handle::File(file)) | Some(Handle::ImportedFile(file, _)) => count(file.__lkjscript_write(bytes)),
            Some(Handle::RemoteFile(descriptor)) => self.network.write_remote(*descriptor, bytes),
            _ => Err(9),
        }
    }

    fn close_handle(&mut self, handle: u64) {
        if let Some(value) = self.handles.remove(&handle) {
            self.dispose_handle(value);
        }
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
        let directory = self.directory_handle(directory)?;
        let opened = self.network.open_relative_directory(directory, path)?;
        self.insert_handle(Handle::Directory(opened))
    }

    fn read_directory(&mut self, directory: u64, bytes: &mut [u8]) -> Result<usize, i64> {
        let directory = self.directory_handle(directory)?;
        self.network.read_entries(directory, bytes)
    }

    fn rewind_handle(&mut self, handle: u64) -> Result<(), i64> {
        match self.handles.get_mut(&handle) {
            Some(Handle::Directory(directory)) => self.network.rewind_remote(*directory),
            Some(Handle::File(file)) | Some(Handle::ImportedFile(file, _)) => status(file.__lkjscript_rewind()),
            Some(Handle::RemoteFile(descriptor)) => self.network.rewind_remote(*descriptor),
            _ => Err(9),
        }
    }

    fn create_directory(&mut self, directory: u64, name: &[u8]) -> Result<(), i64> {
        let directory = self.directory_handle(directory)?;
        self.network.make_directory(directory, name)
    }

    fn remove_entry(&mut self, directory: u64, name: &[u8], is_directory: bool) -> Result<(), i64> {
        let directory = self.directory_handle(directory)?;
        self.network.unlink_entry(directory, name, is_directory)
    }

    fn rename_entry(&mut self, from: u64, old: &[u8], to: u64, new: &[u8]) -> Result<(), i64> {
        let from = self.directory_handle(from)?;
        let to = self.directory_handle(to)?;
        self.network.move_entry(from, old, to, new)
    }

    fn file_executable(&mut self, file: u64) -> Result<(), i64> {
        match self.handles.get_mut(&file) {
            Some(Handle::File(file)) | Some(Handle::ImportedFile(file, _)) => status(file.__lkjscript_executable()),
            Some(Handle::RemoteFile(descriptor)) => self.network.executable_remote(*descriptor),
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
        let executable = self.remote_file(executable)?;
        let input = self.remote_file(input)?;
        let output = self.remote_file(output)?;
        let errors = self.remote_file(errors)?;
        let result = self.remote_file(result)?;
        let identity = self.network.spawn_remote(executable, arguments, input, output, errors, result)?;
        self.insert_handle(Handle::RemoteChild(identity))
    }

    fn wait_process(&mut self, child: u64, nonblocking: bool) -> Result<Option<i64>, i64> {
        let remote = match self.handles.get(&child) {
            Some(Handle::RemoteChild(identity)) => Some(*identity),
            _ => None,
        };
        if let Some(identity) = remote {
            let status = self.network.wait_remote_child(identity, nonblocking)?;
            if status.is_some() { self.handles.remove(&child); }
            return Ok(status);
        }
        Err(10)
    }

    fn terminate_process(&mut self, child: u64) -> Result<(), i64> {
        match self.handles.get_mut(&child) {
            Some(Handle::RemoteChild(identity)) => self.network.terminate_remote_child(*identity),
            _ => Err(10),
        }
    }

    fn drop_process(&mut self, child: u64) {
        if let Some(value) = self.handles.remove(&child) {
            self.dispose_handle(value);
        }
    }
}
