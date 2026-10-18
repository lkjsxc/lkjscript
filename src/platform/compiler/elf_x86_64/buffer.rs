//! Affine byte builders in the process-owned native heap. Admission establishes
//! exclusive mutation; borrows only inspect. Geometric growth bounds retained
//! old capacity to less than twice the current capacity until process exit.
use super::Emitter;
use super::asm::{R::*, *};

impl Emitter<'_> {
    pub(super) fn buffer_external(&mut self, name: &str) -> bool {
        match name {
            "core.buffer.empty" => {
                self.a.imm(Di, 32);
                self.a.call(self.runtime.alloc);
                self.a.imm(Cx, 8);
                self.a.store(Ax, 0, Cx);
                self.a.imm(Cx, 0);
                for offset in [8, 16, 24] {
                    self.a.store(Ax, offset, Cx);
                }
            }
            "core.buffer.push" => self.buffer_push(),
            "core.buffer.length" => {
                self.a.load(Ax, Ax, 8);
                self.a.call(self.runtime.integer);
            }
            "core.buffer.get" => {
                // Unlike bytes.get, the buffer intrinsic takes index first.
                self.a.load(Ax, Ax, 8);
                self.a.load(Dx, Cx, 8);
                self.a.bin(0x39, Ax, Dx);
                self.a.when(AE, self.runtime.buffer_index);
                self.a.load(Cx, Cx, 16);
                self.a.bin(0x01, Cx, Ax);
                self.a.load_byte(Ax, Cx);
                self.a.call(self.runtime.integer);
            }
            "core.buffer.freeze" => {
                // Bytes never shares mutable builder storage.
                self.a.load(Bx, Ax, 8);
                self.a.load(R12, Ax, 16);
                self.bytes_owned_copy();
            }
            "core.buffer.discard" => {
                let unit = self.constant(super::Constant::Unit);
                self.a.address(Ax, unit);
            }
            _ => return false,
        }
        true
    }

    fn buffer_push(&mut self) {
        self.a.load(R13, Ax, 8);
        self.a.group(7, R13, 256);
        self.a.when(AE, self.runtime.buffer_octet);
        self.a.mov(R12, Cx);
        self.a.load(Bx, R12, 8);
        self.a.group(7, Bx, 256 * 1024 * 1024);
        self.a.when(AE, self.runtime.buffer_capacity);
        self.a.load(Ax, R12, 24);
        self.a.bin(0x39, Bx, Ax);
        let space = self.a.label();
        self.a.when(B, space);
        self.a.shift(4, Ax, 1);
        self.a.group(7, Ax, 8);
        let capacity = self.a.label();
        self.a.when(AE, capacity);
        self.a.imm(Ax, 8);
        self.a.bind(capacity);
        self.a.store(R12, 24, Ax);
        self.a.mov(Di, Ax);
        self.a.call(self.runtime.alloc);
        self.a.load(R8, R12, 16);
        self.a.store(R12, 16, Ax);
        self.a.mov(R9, Ax);
        self.a.imm(Cx, 0);
        let copy = self.a.label();
        self.a.bind(copy);
        self.a.bin(0x39, Cx, Bx);
        self.a.when(AE, space);
        self.a.load_byte(Dx, R8);
        self.a.store_byte(R9, Dx);
        self.a.group(0, R8, 1);
        self.a.group(0, R9, 1);
        self.a.group(0, Cx, 1);
        self.a.jump(copy);
        self.a.bind(space);
        self.a.load(Ax, R12, 16);
        self.a.bin(0x01, Ax, Bx);
        self.a.store_byte(Ax, R13);
        self.a.group(0, Bx, 1);
        self.a.store(R12, 8, Bx);
        self.a.mov(Ax, R12);
    }
}
