//! Checked bulk byte movement without per-octet boxed scalar temporaries.
use super::Emitter;
use super::asm::{R::*, *};

impl Emitter<'_> {
    pub(super) fn buffer_append_slice(&mut self) {
        // Input arguments are boxed Bytes, start, end, and the consumed builder.
        self.a.load(Bx, Cx, 8);
        self.argument(Dx, 1);
        self.a.load(Dx, Dx, 8);
        self.a.group(7, Bx, 0);
        self.a.when(L, self.runtime.bytes_range);
        self.a.bin(0x39, Dx, Bx);
        self.a.when(L, self.runtime.bytes_range);
        self.a.load(Di, Ax, 8);
        self.a.bin(0x39, Di, Dx);
        self.a.when(B, self.runtime.bytes_range);
        self.a.load(R13, Ax, 16);
        self.a.bin(0x01, R13, Bx);
        self.a.bin(0x29, Dx, Bx);
        self.a.mov(Bx, Dx);
        self.argument(R12, 0);
        self.buffer_append_window();
    }

    pub(super) fn buffer_append_i64_le(&mut self) {
        self.a.load(Bx, Ax, 8);
        self.a.push(Bx);
        self.a.mov(R13, Sp);
        self.a.mov(R12, Cx);
        self.a.imm(Bx, 8);
        self.buffer_append_window();
        self.a.group(0, Sp, 8);
    }

    /// R12 owns the builder; R13/Bx identify an immutable source window. The
    /// source remains live across allocation. No builder header is published
    /// until the complete checked reservation and old-data copy succeed.
    fn buffer_append_window(&mut self) {
        self.a.load(Dx, R12, 8);
        self.a.bin(0x01, Dx, Bx);
        self.a.when(O, self.runtime.buffer_capacity);
        self.a.group(7, Dx, 0);
        self.a.when(L, self.runtime.buffer_capacity);
        self.a.load(Ax, R12, 24);
        self.a.bin(0x39, Dx, Ax);
        let space = self.a.label();
        self.a.when(LE, space);
        super::buffer::grow_capacity(&mut self.a);
        self.a.bin(0x39, Ax, Dx);
        let capacity = self.a.label();
        self.a.when(AE, capacity);
        self.a.mov(Ax, Dx);
        self.a.bind(capacity);
        self.a.push(Ax);
        self.a.mov(Di, Ax);
        self.a.call(self.runtime.alloc);
        self.a.pop(Dx);
        self.a.load(Si, R12, 16);
        self.a.load(Cx, R12, 8);
        self.a.mov(Di, Ax);
        self.runtime.copy_bytes(&mut self.a);
        self.a.store(R12, 16, Ax);
        self.a.store(R12, 24, Dx);
        self.a.bind(space);
        self.a.load(Dx, R12, 8);
        self.a.load(Di, R12, 16);
        self.a.bin(0x01, Di, Dx);
        self.a.mov(Si, R13);
        self.a.mov(Cx, Bx);
        self.runtime.copy_bytes(&mut self.a);
        self.a.bin(0x01, Dx, Bx);
        self.a.store(R12, 8, Dx);
        self.a.mov(Ax, R12);
    }

    pub(super) fn bytes_read_i64_le(&mut self) {
        self.a.load(Bx, Cx, 8);
        self.a.group(7, Bx, 0);
        self.a.when(L, self.runtime.bytes_range);
        self.a.load(Dx, Ax, 8);
        self.a.bin(0x39, Dx, Bx);
        self.a.when(B, self.runtime.bytes_range);
        self.a.bin(0x29, Dx, Bx);
        self.a.group(7, Dx, 8);
        self.a.when(B, self.runtime.bytes_range);
        self.a.load(R12, Ax, 16);
        self.a.bin(0x01, R12, Bx);
        self.a.load(Ax, R12, 0);
        self.a.call(self.runtime.integer);
    }
}
