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
                inner: ir::U8,
                mutable: false
            }
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
        directory: bool,
    ) -> Result<Value, RunError> {
        let (tag, ty, value) = match result {
            Ok(handle) if handle != 0 => (
                0,
                types.0,
                if directory {
                    Value::Directory(handle)
                } else {
                    Value::File(handle)
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
            Value::Directory(handle) | Value::File(handle) => Ok(*handle),
            _ => Err(self.invalid("I/O capability has no live handle")),
        }
    }
    fn io_write_buffer(&mut self, access: &Access, bytes: &[u8]) -> Result<(), RunError> {
        let cells = self.sequence(access)?;
        if bytes.len() > cells.len() {
            return Err(self.trap(7, "host read exceeded its buffer"));
        }
        let cells = cells[..bytes.len()].to_vec();
        for (cell, byte) in cells.iter().zip(bytes) {
            self.cells[cell.index].value = Some(Value::Bits(u64::from(*byte)));
        }
        Ok(())
    }
    fn io_count(&self, result: Result<usize, i64>, extent: usize) -> Result<u64, RunError> {
        match result {
            Ok(count) if count <= extent && count <= i64::MAX as usize => Ok(count as u64),
            Err(errno) if errno > 0 => Ok((-errno) as u64),
            _ => Err(self.trap(7, "host returned an invalid I/O count")),
        }
    }
    pub(super) fn io_intrinsic(
        &mut self,
        result: usize,
        name: &str,
        values: &[(usize, Value)],
    ) -> Result<Option<Value>, RunError> {
        let value = match name {
            "world_stdin" | "world_stderr" if values.len() == 1 => {
                self.io_authority(values, TypeKind::World)?;
                let (expected, value) = if name == "world_stdin" {
                    (TypeKind::Stdin, Value::Stdin)
                } else {
                    (TypeKind::Stderr, Value::Stderr)
                };
                if *self.kind(result)? != expected {
                    return Err(self.invalid("derived capability result differs"));
                }
                value
            }
            "world_arg_count" if values.len() == 1 => {
                self.io_authority(values, TypeKind::World)?;
                self.io_result_type(result, ir::U64)?;
                Value::Bits(self.host.arguments().len() as u64)
            }
            "world_arg_len" | "world_arg_copy"
                if values.len() == if name == "world_arg_len" { 2 } else { 3 } =>
            {
                self.io_authority(values, TypeKind::World)?;
                self.io_result_type(result, ir::U64)?;
                let index = self.io_number(values, 1, ir::U64)?;
                let index = usize::try_from(index)
                    .map_err(|_| self.trap(3, "argument index out of bounds"))?;
                let argument = self
                    .host
                    .arguments()
                    .get(index)
                    .ok_or_else(|| self.trap(3, "argument index out of bounds"))?;
                if name == "world_arg_len" {
                    Value::Bits(argument.len() as u64)
                } else {
                    let buffer = self.io_buffer(values, 2, true)?;
                    let count = argument.len().min(self.sequence(&buffer)?.len());
                    let bytes = argument[..count].to_vec();
                    self.io_write_buffer(&buffer, &bytes)?;
                    Value::Bits(count as u64)
                }
            }
            "world_exit_status" if values.len() == 2 => {
                self.io_authority(values, TypeKind::World)?;
                self.io_result_type(result, ir::UNIT)?;
                let status = self.io_number(values, 1, ir::U8)?;
                self.host.set_exit_status(status as u8);
                Value::Unit
            }
            "world_directory" if values.len() == 2 => {
                self.io_authority(values, TypeKind::World)?;
                let types = self.io_open_types(result, TypeKind::Directory)?;
                let path = self.io_buffer(values, 1, false)?;
                let path = self.bytes(&path)?;
                let opened = self.host.open_directory(&path);
                self.io_open_result(types, opened, true)?
            }
            "directory_open" if values.len() == 3 => {
                let authority = self.io_authority(values, TypeKind::Directory)?;
                let types = self.io_open_types(result, TypeKind::File)?;
                let path = self.io_buffer(values, 1, false)?;
                let path = self.bytes(&path)?;
                let create = self.io_number(values, 2, ir::BOOL)? != 0;
                let handle = self.io_handle(&authority)?;
                let opened = self.host.open_file(handle, &path, create);
                self.io_open_result(types, opened, false)?
            }
            "stdin_read" | "file_read" if values.len() == 2 => {
                let authority = self.io_authority(
                    values,
                    if name == "stdin_read" {
                        TypeKind::Stdin
                    } else {
                        TypeKind::File
                    },
                )?;
                self.io_result_type(result, ir::I64)?;
                let buffer = self.io_buffer(values, 1, true)?;
                let mut bytes = self.bytes(&buffer)?;
                let read = if name == "stdin_read" {
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
            "file_write" if values.len() == 2 => {
                let authority = self.io_authority(values, TypeKind::File)?;
                self.io_result_type(result, ir::I64)?;
                let buffer = self.io_buffer(values, 1, false)?;
                let bytes = self.bytes(&buffer)?;
                let handle = self.io_handle(&authority)?;
                let written = self.host.write_file(handle, &bytes);
                Value::Bits(self.io_count(written, bytes.len())?)
            }
            "stderr_write" if values.len() == 2 => {
                self.io_authority(values, TypeKind::Stderr)?;
                self.io_result_type(result, ir::UNIT)?;
                let buffer = self.io_buffer(values, 1, false)?;
                let bytes = self.bytes(&buffer)?;
                self.host
                    .write_stderr(&bytes)
                    .map_err(|message| self.trap(7, message))?;
                Value::Unit
            }
            _ => return Ok(None),
        };
        Ok(Some(value))
    }
}
