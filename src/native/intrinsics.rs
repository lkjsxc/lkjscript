//! Checked collection, text and capability operations over inline native values.
use super::*;

impl Body<'_> {
    fn intrinsic_unit(&mut self, dst: usize) {
        self.a.immediate(A, 0);
        self.write(dst, A);
    }

    fn intrinsic_open_result(&mut self, dst: usize) {
        let ty = self.ty(dst);
        let success = self.layout.field_offset(ty, Some(0), 0);
        let failure = self.layout.field_offset(ty, Some(1), 0);
        let error = self.a.label();
        let done = self.a.label();
        self.local_address(dst, R8);
        self.a.test(A, true);
        self.a.condition(8, error);
        self.a.immediate(C, 0);
        self.a.store(R8, 0, C);
        self.a.store(R8, success as i64, A);
        self.a.jump(done);
        self.a.bind(error);
        self.a.negate(A);
        self.a.immediate(C, 1);
        self.a.store(R8, 0, C);
        self.a.store(R8, failure as i64, A);
        self.a.bind(done);
        self.mark(dst);
    }

    fn intrinsic_io(&mut self, dst: usize, name: &str, args: &[usize]) -> Result<bool, Diagnostic> {
        if name == "world_stdout" || name == "world_stderr" || name == "world_stdin" {
            self.a.immediate(A, 1);
            self.write(dst, A);
            return Ok(true);
        }
        match name {
            "stdout_write" | "stderr_write" => {
                self.read_word(args[1], 0, DI);
                self.read_word(args[1], 1, SI);
                self.a.call(if name == "stdout_write" {
                    self.platform.stdout_write
                } else {
                    self.platform.stderr_write
                });
            }
            "world_arg_count" => self.a.call(self.platform.argument_count),
            "world_arg_len" => {
                self.read(args[1], DI);
                self.a.call(self.platform.argument_length);
            }
            "world_arg_copy" => {
                self.read(args[1], DI);
                self.read_word(args[2], 0, SI);
                self.read_word(args[2], 1, D);
                self.a.call(self.platform.argument_copy);
            }
            "world_exit_status" => {
                self.read(args[1], DI);
                self.a.call(self.platform.set_exit_status);
            }
            "world_directory" => {
                self.read_word(args[1], 0, DI);
                self.read_word(args[1], 1, SI);
                self.a.call(self.platform.directory_open);
                self.intrinsic_open_result(dst);
                return Ok(true);
            }
            "directory_open" => {
                self.read(args[0], DI);
                self.a.load(DI, DI, 0);
                self.read_word(args[1], 0, SI);
                self.read_word(args[1], 1, D);
                self.read(args[2], C);
                self.a.call(self.platform.file_open);
                self.intrinsic_open_result(dst);
                return Ok(true);
            }
            "stdin_read" => {
                self.read_word(args[1], 0, DI);
                self.read_word(args[1], 1, SI);
                self.a.call(self.platform.stdin_read);
            }
            "file_read" | "file_write" => {
                self.read(args[0], DI);
                self.a.load(DI, DI, 0);
                self.read_word(args[1], 0, SI);
                self.read_word(args[1], 1, D);
                self.a.call(if name == "file_read" {
                    self.platform.file_read
                } else {
                    self.platform.file_write
                });
            }
            _ => return Ok(false),
        }
        if name == "world_arg_len" || name == "world_arg_copy" {
            self.a.test(A, true);
            self.trap_if(8, 3);
        }
        self.write(dst, A);
        Ok(true)
    }

    fn intrinsic_vec_push(&mut self, dst: usize, args: &[usize]) {
        let element = self.ty(args[1]);
        let stride = self.layout.stride(element);
        let ready = self.a.label();
        let grow = self.a.label();
        self.read(args[0], R8);
        self.a.store(BP, self.scratch[0], R8);
        self.a.load(R9, R8, 8);
        self.a.store(BP, self.scratch[1], R9);
        self.a.load(A, R8, 16);
        self.a.compare(R9, A);
        self.a.condition(5, ready);
        self.a.test(A, true);
        self.a.condition(5, grow);
        self.a.immediate(A, 2);
        self.a.bind(grow);
        self.a.immediate(C, (!0u64) / 2);
        self.a.compare(A, C);
        self.trap_if(7, 6);
        self.a.shift(A, 4, 1);
        self.a.store(BP, self.scratch[2], A);
        self.a.immediate(C, stride);
        self.a.mul(C);
        self.a.test(D, true);
        self.trap_if(5, 6);
        self.a.mov(SI, A);
        self.a.load(DI, R8, 0);
        self.a.call(self.runtime.reallocate);
        self.a.load(R8, BP, self.scratch[0]);
        self.a.store(R8, 0, A);
        self.a.load(C, BP, self.scratch[2]);
        self.a.store(R8, 16, C);
        self.a.bind(ready);
        self.a.load(R8, BP, self.scratch[0]);
        self.a.load(A, BP, self.scratch[1]);
        self.a.immediate(C, stride);
        self.a.imul(A, C);
        self.a.load(DI, R8, 0);
        self.a.rr(0x01, DI, A, true);
        self.local_address(args[1], SI);
        self.copy_value(DI, SI, element);
        self.a.load(R8, BP, self.scratch[0]);
        self.a.load(A, BP, self.scratch[1]);
        self.a.immediate(C, 1);
        self.a.rr(0x01, A, C, true);
        self.a.store(R8, 8, A);
        self.intrinsic_unit(dst);
    }

    fn intrinsic_vec_pop(&mut self, dst: usize, args: &[usize]) -> Result<(), Diagnostic> {
        let result_ty = self.ty(dst);
        let element = match &self.program.types[result_ty].kind {
            TypeKind::Enum { variants, .. } => variants[1].fields[0].ty,
            _ => return Err(self.problem("vec_pop result is not an enum")),
        };
        let field = self.layout.field_offset(result_ty, Some(1), 0);
        let stride = self.layout.stride(element);
        let empty = self.a.label();
        let done = self.a.label();
        self.read(args[0], R8);
        self.a.load(A, R8, 8);
        self.a.test(A, true);
        self.a.condition(4, empty);
        self.a.immediate(C, 1);
        self.a.rr(0x29, A, C, true);
        self.a.store(R8, 8, A);
        self.a.immediate(C, stride);
        self.a.imul(A, C);
        self.a.load(SI, R8, 0);
        self.a.rr(0x01, SI, A, true);
        self.local_address(dst, DI);
        self.a.immediate(C, 1);
        self.a.store(DI, 0, C);
        self.a.lea(DI, DI, field as i64);
        self.copy_value(DI, SI, element);
        self.a.jump(done);
        self.a.bind(empty);
        self.a.immediate(A, 0);
        self.write_word(dst, 0, A);
        self.a.bind(done);
        self.mark(dst);
        Ok(())
    }

    fn intrinsic_replace(&mut self, dst: usize, args: &[usize]) {
        // Swap a byte at a time so dst may reuse the consumed replacement local.
        self.read(args[0], R8);
        self.local_address(args[1], R9);
        self.local_address(dst, R10);
        let bytes = self.layout.storage(self.ty(dst)).bytes;
        self.a.immediate(C, bytes);
        self.a.immediate(D, 1);
        let again = self.a.label();
        let done = self.a.label();
        self.a.bind(again);
        self.a.test(C, true);
        self.a.condition(4, done);
        self.a.load_width(A, R8, 0, 1, false);
        self.a.load_width(DI, R9, 0, 1, false);
        self.a.store_width(R8, 0, DI, 1);
        self.a.store_width(R10, 0, A, 1);
        self.a.rr(0x01, R8, D, true);
        self.a.rr(0x01, R9, D, true);
        self.a.rr(0x01, R10, D, true);
        self.a.rr(0x29, C, D, true);
        self.a.jump(again);
        self.a.bind(done);
        self.mark(dst);
    }

    fn intrinsic_array_repeat(&mut self, dst: usize, src: usize) -> Result<(), Diagnostic> {
        let (inner, len) = match self.program.types[self.ty(dst)].kind {
            TypeKind::Array { inner, len } => (inner, len),
            _ => return Err(self.problem("array_repeat result is not an array")),
        };
        let stride = self.layout.stride(inner);
        self.local_address(dst, A);
        self.a.store(BP, self.scratch[0], A);
        self.a.immediate(A, len as u64);
        self.a.store(BP, self.scratch[1], A);
        let again = self.a.label();
        let done = self.a.label();
        self.a.bind(again);
        self.a.load(A, BP, self.scratch[1]);
        self.a.test(A, true);
        self.a.condition(4, done);
        self.a.load(DI, BP, self.scratch[0]);
        self.local_address(src, SI);
        self.copy_value(DI, SI, inner);
        self.a.load(A, BP, self.scratch[0]);
        self.a.immediate(C, stride);
        self.a.rr(0x01, A, C, true);
        self.a.store(BP, self.scratch[0], A);
        self.a.load(A, BP, self.scratch[1]);
        self.a.immediate(C, 1);
        self.a.rr(0x29, A, C, true);
        self.a.store(BP, self.scratch[1], A);
        self.a.jump(again);
        self.a.bind(done);
        self.mark(dst);
        Ok(())
    }

    pub(super) fn intrinsic(
        &mut self,
        dst: usize,
        name: &str,
        args: &[usize],
    ) -> Result<(), Diagnostic> {
        if self.intrinsic_io(dst, name, args)? {
            return Ok(());
        }
        match name {
            "assert" => {
                self.read(args[0], A);
                self.a.test(A, true);
                self.trap_if(4, 5);
                self.intrinsic_unit(dst);
            }
            "cast" => self.scalar_cast(dst, args[0])?,
            "array_repeat" => self.intrinsic_array_repeat(dst, args[0])?,
            "replace" => self.intrinsic_replace(dst, args),
            "box_new" => {
                let inner = self.ty(args[0]);
                self.a.immediate(DI, self.layout.storage(inner).bytes);
                self.a.call(self.runtime.allocate);
                self.a.store(BP, self.scratch[0], A);
                self.a.mov(DI, A);
                self.local_address(args[0], SI);
                self.copy_value(DI, SI, inner);
                self.a.load(A, BP, self.scratch[0]);
                self.write(dst, A);
            }
            "vec_new" => {
                self.a.immediate(A, 0);
                self.write_word(dst, 0, A);
                self.write_word(dst, 1, A);
                self.write_word(dst, 2, A);
                self.mark(dst);
            }
            "vec_push" => self.intrinsic_vec_push(dst, args),
            "vec_pop" => self.intrinsic_vec_pop(dst, args)?,
            "vec_len" => {
                let inner = match self.program.types[self.ty(args[0])].kind {
                    TypeKind::Ref { inner, .. } => inner,
                    _ => return Err(self.problem("vec_len input is not a reference")),
                };
                match self.program.types[inner].kind {
                    TypeKind::Array { len, .. } => {
                        self.a.immediate(A, len as u64);
                    }
                    TypeKind::Slice { .. } => self.read_word(args[0], 1, A),
                    TypeKind::Vec { .. } => {
                        self.read(args[0], A);
                        self.a.load(A, A, 8);
                    }
                    _ => return Err(self.problem("vec_len input is not a sequence")),
                }
                self.write(dst, A);
            }
            "clone" => {
                self.read(args[0], SI);
                self.local_address(dst, DI);
                let ty = self.ty(dst);
                self.copy_value(DI, SI, ty);
                self.mark(dst);
            }
            "char_from_u32" => {
                let ty = self.ty(dst);
                let field = self.layout.field_offset(ty, Some(1), 0);
                let valid = self.a.label();
                let done = self.a.label();
                self.read(args[0], A);
                self.local_address(dst, R8);
                self.a.immediate(C, 0);
                self.a.store(R8, 0, C);
                self.a.immediate(C, 0x110000);
                self.a.compare(A, C);
                self.a.condition(3, done);
                self.a.immediate(C, 0xd800);
                self.a.compare(A, C);
                self.a.condition(2, valid);
                self.a.immediate(C, 0xe000);
                self.a.compare(A, C);
                self.a.condition(2, done);
                self.a.bind(valid);
                self.a.immediate(C, 1);
                self.a.store(R8, 0, C);
                self.a.store_width(R8, field as i64, A, 4);
                self.a.bind(done);
                self.mark(dst);
            }
            "str_as_bytes" => {
                self.copy_local(dst, args[0]);
            }
            "str_len" => {
                self.read_word(args[0], 1, A);
                self.write(dst, A);
            }
            "str_from_utf8" => {
                let ty = self.ty(dst);
                let ok_field = self.layout.field_offset(ty, Some(0), 0);
                let err_field = self.layout.field_offset(ty, Some(1), 0);
                let invalid = self.a.label();
                let done = self.a.label();
                self.read_word(args[0], 0, DI);
                self.read_word(args[0], 1, SI);
                self.a.call(self.runtime.utf8_validate);
                self.local_address(dst, R8);
                self.a.test(A, true);
                self.a.condition(4, invalid);
                self.a.immediate(A, 0);
                self.a.store(R8, 0, A);
                self.read_word(args[0], 0, A);
                self.a.store(R8, ok_field as i64, A);
                self.read_word(args[0], 1, A);
                self.a.store(R8, ok_field as i64 + 8, A);
                self.a.jump(done);
                self.a.bind(invalid);
                self.a.immediate(A, 1);
                self.a.store(R8, 0, A);
                self.a.immediate(A, 0);
                self.a.store_width(R8, err_field as i64, A, 1);
                self.a.bind(done);
                self.mark(dst);
            }
            _ => return Err(self.problem(&format!("unsupported checked intrinsic {name}"))),
        }
        Ok(())
    }
}
