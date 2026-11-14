//! Independent capability execution over typed cells and host-owned handles.
use super::*;

impl<H: Host> Interpreter<'_, H> {
    fn io_reference(
        &self,
        values: &[(usize, Value)],
        index: usize,
        mutable: bool,
    ) -> Result<Access, RunError> {
        let Some((ty, Value::Reference(reference))) = values.get(index) else {
            return Err(self.invalid("I/O argument must be a reference"));
        };
        if !matches!(self.kind(*ty)?, TypeKind::Ref { mutable: m, .. } if *m == mutable) {
            return Err(self.invalid("I/O reference mutability differs"));
        }
        let access = self.reference_access(reference)?;
        self.access(&access, mutable, true)?;
        Ok(access)
    }
    fn io_authority(
        &self,
        values: &[(usize, Value)],
        expected: TypeKind,
    ) -> Result<Access, RunError> {
        let access = self.io_reference(values, 0, true)?;
        if *self.kind(access.ty)? != expected {
            return Err(self.invalid("I/O requires the declared capability"));
        }
        Ok(access)
    }
    fn io_buffer(
        &self,
        values: &[(usize, Value)],
        index: usize,
        mutable: bool,
    ) -> Result<Access, RunError> {
        let access = self.io_reference(values, index, mutable)?;
        if !matches!(
            self.kind(access.ty)?,
            TypeKind::Slice {
                inner,
                mutable: false
            } if *inner == ir::U8
        ) {
            return Err(self.invalid("I/O buffer must be a byte slice"));
        }
        for element in self.sequence(&access)? {
            let child = Access {
                extent: None,
                cell: *element,
                ty: ir::U8,
                permissions: access.permissions.clone(),
                writable: mutable,
            };
            self.access(&child, mutable, true)?;
        }
        Ok(access)
    }
    fn io_number(
        &self,
        values: &[(usize, Value)],
        index: usize,
        expected: usize,
    ) -> Result<u64, RunError> {
        match values.get(index) {
            Some((ty, Value::Bits(value))) if *ty == expected => Ok(*value),
            _ => Err(self.invalid("I/O scalar argument type differs")),
        }
    }
    fn io_result_type(&self, actual: usize, expected: usize) -> Result<(), RunError> {
        if actual == expected {
            Ok(())
        } else {
            Err(self.invalid("I/O result type differs"))
        }
    }
    fn io_open_types(&self, result: usize, success: TypeKind) -> Result<(usize, usize), RunError> {
        let TypeKind::Enum {
            variants,
            copy: false,
        } = self.kind(result)?
        else {
            return Err(self.invalid("open result must be an affine Result enum"));
        };
        if variants.len() != 2 || variants[0].fields.len() != 1 || variants[1].fields.len() != 1 {
            return Err(self.invalid("open result needs one success and one error field"));
        }
        let ok = variants[0].fields[0].ty;
        let error = variants[1].fields[0].ty;
        if *self.kind(ok)? != success || error != ir::I64 {
            return Err(self.invalid("open result must contain its capability or i64 errno"));
        }
        Ok((ok, error))
    }
    fn io_open_result(
        &mut self,
        types: (usize, usize),
        result: Result<u64, i64>,
        kind: TypeKind,
    ) -> Result<Value, RunError> {
        let (tag, ty, value) = match result {
            Ok(handle) if handle != 0 => (
                0,
                types.0,
                match kind {
                    TypeKind::Directory => Value::Directory(handle),
                    TypeKind::File => Value::File(handle),
                    TypeKind::Child => Value::Child(handle),
                    _ => return Err(self.invalid("open result is not a handle capability")),
                },
            ),
            Err(errno) if errno > 0 => (1, types.1, Value::Bits(errno as u64)),
            _ => return Err(self.trap(7, "host returned an invalid open result")),
        };
        let child = self.allocate(ty, Some(value), false)?;
        Ok(Value::Aggregate {
            variant: Some(tag),
            fields: vec![child],
        })
    }
    fn io_handle(&self, access: &Access) -> Result<u64, RunError> {
        match self.value(access.cell)? {
            Value::Directory(handle) | Value::File(handle) | Value::Child(handle) => Ok(*handle),
            _ => Err(self.invalid("I/O capability has no live handle")),
        }
    }
    fn io_write_buffer(&mut self, access: &Access, bytes: &[u8]) -> Result<(), RunError> {
        let cells = self.sequence_cells(access)?;
        if bytes.len() > cells.len() {
            return Err(self.trap(7, "host read exceeded its buffer"));
        }
        for index in 0..bytes.len() {
            self.cells[cells[index].index].value = Some(Value::Bits(bytes[index] as u64));
        }
        Ok(())
    }
    fn io_count(&self, result: Result<usize, i64>, extent: usize) -> Result<u64, RunError> {
        match result {
            Ok(count) if count <= extent && count <= 9223372036854775807i64 as usize => {
                Ok(count as u64)
            }
            Err(errno) if errno > 0 => Ok((-errno) as u64),
            _ => Err(self.trap(7, "host returned an invalid I/O count")),
        }
    }
    fn io_status(&self, result: Result<(), i64>) -> Result<Value, RunError> {
        match result {
            Ok(()) => Ok(Value::Bits(0)),
            Err(errno) if errno > 0 => Ok(Value::Bits((-errno) as u64)),
            _ => Err(self.trap(7, "host returned an invalid I/O status")),
        }
    }
    fn io_file(&self, values: &[(usize, Value)], index: usize) -> Result<u64, RunError> {
        let access = self.io_reference(values, index, false)?;
        if !matches!(self.kind(access.ty)?, TypeKind::File) {
            return Err(self.invalid("process stream requires a File capability"));
        }
        self.io_handle(&access)
    }
    pub(super) fn io_intrinsic(
        &mut self,
        result: usize,
        name: ir::IntrinsicOp,
        values: &[(usize, Value)],
    ) -> Result<Option<Value>, RunError> {
        let value = match name {
            ir::IntrinsicOp::FileDuplicate | ir::IntrinsicOp::FileReadOnly if values.len() == 1 => {
                let authority = self.io_authority(values, TypeKind::File)?;
                let types = self.io_open_types(result, TypeKind::File)?;
                let handle = self.io_handle(&authority)?;
                let opened = if name == ir::IntrinsicOp::FileReadOnly {
                    self.host.readonly_file(handle)
                } else {
                    self.host.duplicate_file(handle)
                };
                self.io_open_result(types, opened, TypeKind::File)?
            }
            ir::IntrinsicOp::WorldProcess | ir::IntrinsicOp::WorldClock if values.len() == 1 => {
                self.io_authority(values, TypeKind::World)?;
                let (expected, value) = if name == ir::IntrinsicOp::WorldProcess {
                    (TypeKind::Process, Value::Process)
                } else {
                    (TypeKind::Clock, Value::Clock)
                };
                if *self.kind(result)? != expected {
                    return Err(self.invalid("derived capability result differs"));
                }
                value
            }
            ir::IntrinsicOp::WorldFile if values.len() == 2 => {
                self.io_authority(values, TypeKind::World)?;
                let types = self.io_open_types(result, TypeKind::File)?;
                let kind = self.io_number(values, 1, ir::U8)?;
                let opened = self.host.world_file(kind as u8);
                self.io_open_result(types, opened, TypeKind::File)?
            }
            ir::IntrinsicOp::DirectoryOpenDirectory if values.len() == 2 => {
                let authority = self.io_authority(values, TypeKind::Directory)?;
                let types = self.io_open_types(result, TypeKind::Directory)?;
                let path = self.io_buffer(values, 1, false)?;
                let path = self.bytes(&path)?;
                let handle = self.io_handle(&authority)?;
                let opened = self.host.open_subdirectory(handle, &path);
                self.io_open_result(types, opened, TypeKind::Directory)?
            }
            ir::IntrinsicOp::DirectoryRead if values.len() == 2 => {
                let authority = self.io_authority(values, TypeKind::Directory)?;
                self.io_result_type(result, ir::I64)?;
                let buffer = self.io_buffer(values, 1, true)?;
                let mut bytes = self.bytes(&buffer)?;
                let handle = self.io_handle(&authority)?;
                let read = self.host.read_directory(handle, &mut bytes);
                let count = self.io_count(read, bytes.len())?;
                if (count as i64) >= 0 {
                    self.io_write_buffer(&buffer, &bytes[..count as usize])?;
                }
                Value::Bits(count)
            }
            ir::IntrinsicOp::DirectoryRewind
            | ir::IntrinsicOp::FileRewind
            | ir::IntrinsicOp::FileExecutable
                if values.len() == 1 =>
            {
                let kind = if name == ir::IntrinsicOp::DirectoryRewind {
                    TypeKind::Directory
                } else {
                    TypeKind::File
                };
                let authority = self.io_authority(values, kind)?;
                self.io_result_type(result, ir::I64)?;
                let handle = self.io_handle(&authority)?;
                let outcome = if name == ir::IntrinsicOp::FileExecutable {
                    self.host.file_executable(handle)
                } else {
                    self.host.rewind_handle(handle)
                };
                self.io_status(outcome)?
            }
            ir::IntrinsicOp::DirectoryCreate | ir::IntrinsicOp::DirectoryRemove
                if values.len()
                    == if name == ir::IntrinsicOp::DirectoryCreate {
                        2
                    } else {
                        3
                    } =>
            {
                let authority = self.io_authority(values, TypeKind::Directory)?;
                self.io_result_type(result, ir::I64)?;
                let path = self.io_buffer(values, 1, false)?;
                let path = self.bytes(&path)?;
                let handle = self.io_handle(&authority)?;
                let outcome = if name == ir::IntrinsicOp::DirectoryCreate {
                    self.host.create_directory(handle, &path)
                } else {
                    let is_directory = self.io_number(values, 2, ir::BOOL)? != 0;
                    self.host.remove_entry(handle, &path, is_directory)
                };
                self.io_status(outcome)?
            }
            ir::IntrinsicOp::DirectoryRename if values.len() == 4 => {
                let from = self.io_authority(values, TypeKind::Directory)?;
                self.io_result_type(result, ir::I64)?;
                let to = self.io_reference(values, 2, true)?;
                if !matches!(self.kind(to.ty)?, TypeKind::Directory) {
                    return Err(self.invalid("rename destination requires a Directory capability"));
                }
                let old_name = self.io_buffer(values, 1, false)?;
                let new_name = self.io_buffer(values, 3, false)?;
                let old_name = self.bytes(&old_name)?;
                let new_name = self.bytes(&new_name)?;
                let from = self.io_handle(&from)?;
                let to = self.io_handle(&to)?;
                let outcome = self.host.rename_entry(from, &old_name, to, &new_name);
                self.io_status(outcome)?
            }
            ir::IntrinsicOp::ClockRead if values.len() == 2 => {
                self.io_authority(values, TypeKind::Clock)?;
                self.io_result_type(result, ir::I64)?;
                let buffer = self.io_buffer(values, 1, true)?;
                let mut bytes = self.bytes(&buffer)?;
                if bytes.len() < 16 {
                    Value::Bits((-22i64) as u64)
                } else {
                    let outcome = self.host.monotonic_time(&mut bytes[..16]);
                    if outcome.is_ok() {
                        self.io_write_buffer(&buffer, &bytes[..16])?;
                    }
                    self.io_status(outcome)?
                }
            }
            ir::IntrinsicOp::ProcessSpawn if values.len() == 7 => {
                self.io_authority(values, TypeKind::Process)?;
                let types = self.io_open_types(result, TypeKind::Child)?;
                let executable = self.io_file(values, 1)?;
                let arguments = self.io_buffer(values, 2, false)?;
                let arguments = self.bytes(&arguments)?;
                let stdin = self.io_file(values, 3)?;
                let stdout = self.io_file(values, 4)?;
                let stderr = self.io_file(values, 5)?;
                let result = self.io_file(values, 6)?;
                let child = if arguments.is_empty() || arguments[arguments.len() - 1] != 0 {
                    Err(22)
                } else {
                    self.host
                        .spawn_process(executable, &arguments, stdin, stdout, stderr, result)
                };
                self.io_open_result(types, child, TypeKind::Child)?
            }
            ir::IntrinsicOp::ChildWait if values.len() == 2 => {
                let authority = self.io_authority(values, TypeKind::Child)?;
                self.io_result_type(result, ir::I64)?;
                let nonblocking = self.io_number(values, 1, ir::BOOL)? != 0;
                let child = self.io_handle(&authority)?;
                match self.host.wait_process(child, nonblocking) {
                    Ok(None) => Value::Bits(0),
                    Ok(Some(status)) if status >= 0 && status < 9223372036854775807i64 => {
                        Value::Bits((status + 1) as u64)
                    }
                    Err(errno) if errno > 0 => Value::Bits((-errno) as u64),
                    _ => return Err(self.trap(7, "host returned an invalid child status")),
                }
            }
            ir::IntrinsicOp::ChildTerminate if values.len() == 1 => {
                let authority = self.io_authority(values, TypeKind::Child)?;
                self.io_result_type(result, ir::I64)?;
                let child = self.io_handle(&authority)?;
                let outcome = self.host.terminate_process(child);
                self.io_status(outcome)?
            }
            ir::IntrinsicOp::WorldStdin | ir::IntrinsicOp::WorldStderr if values.len() == 1 => {
                self.io_authority(values, TypeKind::World)?;
                let (expected, value) = if name == ir::IntrinsicOp::WorldStdin {
                    (TypeKind::Stdin, Value::Stdin)
                } else {
                    (TypeKind::Stderr, Value::Stderr)
                };
                if *self.kind(result)? != expected {
                    return Err(self.invalid("derived capability result differs"));
                }
                value
            }
            ir::IntrinsicOp::WorldArgCount if values.len() == 1 => {
                self.io_authority(values, TypeKind::World)?;
                self.io_result_type(result, ir::U64)?;
                Value::Bits(self.host.argument_count() as u64)
            }
            ir::IntrinsicOp::WorldArgLen | ir::IntrinsicOp::WorldArgCopy
                if values.len()
                    == if name == ir::IntrinsicOp::WorldArgLen {
                        2
                    } else {
                        3
                    } =>
            {
                self.io_authority(values, TypeKind::World)?;
                self.io_result_type(result, ir::U64)?;
                let index = self.io_number(values, 1, ir::U64)?;
                let index = index as usize;
                let argument = self
                    .host
                    .argument(index)
                    .ok_or_else(|| self.trap(3, "argument index out of bounds"))?;
                if name == ir::IntrinsicOp::WorldArgLen {
                    Value::Bits(argument.len() as u64)
                } else {
                    let buffer = self.io_buffer(values, 2, true)?;
                    let count = argument.len().min(self.sequence(&buffer)?.len());
                    let bytes = argument[..count].to_vec();
                    self.io_write_buffer(&buffer, &bytes)?;
                    Value::Bits(count as u64)
                }
            }
            ir::IntrinsicOp::WorldExitStatus if values.len() == 2 => {
                self.io_authority(values, TypeKind::World)?;
                self.io_result_type(result, ir::UNIT)?;
                let status = self.io_number(values, 1, ir::U8)?;
                self.host.set_exit_status(status as u8);
                Value::Unit
            }
            ir::IntrinsicOp::WorldDirectory if values.len() == 2 => {
                self.io_authority(values, TypeKind::World)?;
                let types = self.io_open_types(result, TypeKind::Directory)?;
                let path = self.io_buffer(values, 1, false)?;
                let path = self.bytes(&path)?;
                let opened = self.host.open_directory(&path);
                self.io_open_result(types, opened, TypeKind::Directory)?
            }
            ir::IntrinsicOp::DirectoryOpen if values.len() == 3 => {
                let authority = self.io_authority(values, TypeKind::Directory)?;
                let types = self.io_open_types(result, TypeKind::File)?;
                let path = self.io_buffer(values, 1, false)?;
                let path = self.bytes(&path)?;
                let create = self.io_number(values, 2, ir::BOOL)? != 0;
                let handle = self.io_handle(&authority)?;
                let opened = self.host.open_file(handle, &path, create);
                self.io_open_result(types, opened, TypeKind::File)?
            }
            ir::IntrinsicOp::StdinRead | ir::IntrinsicOp::FileRead if values.len() == 2 => {
                let authority = self.io_authority(
                    values,
                    if name == ir::IntrinsicOp::StdinRead {
                        TypeKind::Stdin
                    } else {
                        TypeKind::File
                    },
                )?;
                self.io_result_type(result, ir::I64)?;
                let buffer = self.io_buffer(values, 1, true)?;
                let mut bytes = self.bytes(&buffer)?;
                let read = if name == ir::IntrinsicOp::StdinRead {
                    self.host.read_stdin(&mut bytes)
                } else {
                    let handle = self.io_handle(&authority)?;
                    self.host.read_file(handle, &mut bytes)
                };
                let count = self.io_count(read, bytes.len())?;
                if (count as i64) >= 0 {
                    self.io_write_buffer(&buffer, &bytes[..count as usize])?;
                }
                Value::Bits(count)
            }
            ir::IntrinsicOp::FileWrite if values.len() == 2 => {
                let authority = self.io_authority(values, TypeKind::File)?;
                self.io_result_type(result, ir::I64)?;
                let buffer = self.io_buffer(values, 1, false)?;
                let bytes = self.bytes(&buffer)?;
                let handle = self.io_handle(&authority)?;
                let written = self.host.write_file(handle, &bytes);
                Value::Bits(self.io_count(written, bytes.len())?)
            }
            ir::IntrinsicOp::StderrWrite if values.len() == 2 => {
                self.io_authority(values, TypeKind::Stderr)?;
                self.io_result_type(result, ir::UNIT)?;
                let buffer = self.io_buffer(values, 1, false)?;
                let bytes = self.bytes(&buffer)?;
                if let Err(message) = self.host.write_stderr(&bytes) {
                    return Err(self.trap(7, &message));
                }
                Value::Unit
            }
            _ => return Ok(None),
        };
        Ok(Some(value))
    }
}
