//! Native immutable byte operations required by the LKJCI1 compiler workload.
//! A value is [tag=7, length, data pointer]; every producer owns its copied bytes.
use super::Emitter;
use super::asm::{R::*, *};

impl Emitter<'_> {
    /// Inputs follow the external-function stack ABI. Ax/Cx are its first two
    /// argument values; the result is a value pointer in Ax.
    pub(super) fn bytes_external(&mut self, name: &str) -> bool {
        match name {
            "core.bytes.length" => {
                self.a.load(Ax, Ax, 8);
                self.a.call(self.runtime.integer);
            }
            "core.bytes.get" => {
                self.a.load(Cx, Cx, 8);
                self.a.load(Dx, Ax, 8);
                self.a.bin(0x39, Cx, Dx);
                // Unsigned comparison also rejects every negative I64 index.
                self.a.when(AE, self.runtime.bytes_index);
                self.a.load(Ax, Ax, 16);
                self.a.bin(0x01, Ax, Cx);
                self.a.load_byte(Ax, Ax);
                self.a.call(self.runtime.integer);
            }
            "core.bytes.from-list" => self.bytes_from_list(),
            "core.bytes.from-text" => {
                self.a.load(Bx, Ax, 24);
                self.a.load(R12, Ax, 32);
                self.bytes_owned_copy();
            }
            "core.bytes.copy" => {
                self.a.load(Bx, Ax, 8);
                self.a.load(R12, Ax, 16);
                self.bytes_owned_copy();
            }
            "core.bytes.slice" => {
                self.a.load(Cx, Cx, 8);
                self.argument(Dx, 0);
                self.a.load(Dx, Dx, 8);
                self.a.bin(0x39, Dx, Cx);
                self.a.when(B, self.runtime.bytes_range);
                self.a.load(Bx, Ax, 8);
                self.a.bin(0x39, Bx, Dx);
                self.a.when(B, self.runtime.bytes_range);
                self.a.load(R12, Ax, 16);
                self.a.bin(0x01, R12, Cx);
                self.a.mov(Bx, Dx);
                self.a.bin(0x29, Bx, Cx);
                self.bytes_owned_copy();
            }
            "core.bytes.concat" => self.bytes_concat(),
            "core.bytes.equal" => self.bytes_equal(),
            "core.bytes.blake3" => {
                self.a.mov(R12, Ax);
                self.a.imm(Bx, 32);
                self.bytes_allocate();
                self.a.load(Si, R12, 8);
                self.a.group(7, Si, 256 * 1024 * 1024 + 1);
                self.a.when(AE, self.runtime.bytes_capacity);
                self.a.load(Di, R12, 16);
                self.a.imm(Dx, 0);
                self.a.imm(Cx, 8);
                self.a.lea(R8, R13, 24);
                self.a.call(self.runtime.blake3);
                self.a.mov(Ax, R13);
            }
            _ => return self.buffer_external(name),
        }
        true
    }

    /// Bx is the validated byte length. R13 receives the new object pointer.
    /// The bound leaves addition/alignment headroom and agrees with the VM's
    /// finite value bound, independently of its cumulative allocation quota.
    fn bytes_allocate(&mut self) {
        self.a.group(7, Bx, 256 * 1024 * 1024 + 1);
        self.a.when(AE, self.runtime.bytes_capacity);
        self.a.lea(Di, Bx, 24);
        self.a.call(self.runtime.alloc);
        self.a.mov(R13, Ax);
        self.a.imm(Cx, 7);
        self.a.store(Ax, 0, Cx);
        self.a.store(Ax, 8, Bx);
        self.a.lea(Cx, Ax, 24);
        self.a.store(Ax, 16, Cx);
    }

    /// Copy Bx octets from R8 to R9, leaving both pointers at the end. Empty
    /// inputs perform no dereference, including an empty slice at length.
    fn bytes_copy_loop(&mut self) {
        self.a.imm(Cx, 0);
        let next = self.a.label();
        let done = self.a.label();
        self.a.bind(next);
        self.a.bin(0x39, Cx, Bx);
        self.a.when(AE, done);
        self.a.load_byte(Dx, R8);
        self.a.store_byte(R9, Dx);
        self.a.group(0, R8, 1);
        self.a.group(0, R9, 1);
        self.a.group(0, Cx, 1);
        self.a.jump(next);
        self.a.bind(done);
    }

    pub(super) fn bytes_owned_copy(&mut self) {
        self.bytes_allocate();
        self.a.mov(R8, R12);
        self.a.lea(R9, R13, 24);
        self.bytes_copy_loop();
        self.a.mov(Ax, R13);
    }

    fn bytes_from_list(&mut self) {
        self.a.mov(R12, Ax);
        self.a.load(Bx, Ax, 8);
        self.bytes_allocate();
        self.a.lea(R8, R12, 16);
        self.a.lea(R9, R13, 24);
        self.a.imm(Cx, 0);
        let next = self.a.label();
        let done = self.a.label();
        self.a.bind(next);
        self.a.bin(0x39, Cx, Bx);
        self.a.when(AE, done);
        self.a.load(Dx, R8, 0);
        self.a.load(Dx, Dx, 8);
        self.a.group(7, Dx, 256);
        self.a.when(AE, self.runtime.bytes_octet);
        self.a.store_byte(R9, Dx);
        self.a.group(0, R8, 8);
        self.a.group(0, R9, 1);
        self.a.group(0, Cx, 1);
        self.a.jump(next);
        self.a.bind(done);
        self.a.mov(Ax, R13);
    }

    fn bytes_concat(&mut self) {
        self.a.mov(R12, Ax);
        self.a.load(Bx, Ax, 8);
        self.a.load(Dx, Cx, 8);
        self.a.bin(0x01, Bx, Dx);
        self.a.when(B, self.runtime.bytes_capacity);
        self.a.push(Cx);
        self.bytes_allocate();
        self.a.pop(R10);
        self.a.load(Bx, R12, 8);
        self.a.load(R8, R12, 16);
        self.a.lea(R9, R13, 24);
        self.bytes_copy_loop();
        self.a.load(Bx, R10, 8);
        self.a.load(R8, R10, 16);
        self.bytes_copy_loop();
        self.a.mov(Ax, R13);
    }

    fn bytes_equal(&mut self) {
        self.a.load(Bx, Ax, 8);
        self.a.load(Dx, Cx, 8);
        self.a.bin(0x39, Bx, Dx);
        let unequal = self.a.label();
        let equal = self.a.label();
        let done = self.a.label();
        let next = self.a.label();
        self.a.when(NE, unequal);
        self.a.load(R8, Ax, 16);
        self.a.load(R9, Cx, 16);
        self.a.imm(Cx, 0);
        self.a.bind(next);
        self.a.bin(0x39, Cx, Bx);
        self.a.when(AE, equal);
        self.a.load_byte(Dx, R8);
        self.a.load_byte(Ax, R9);
        self.a.bin(0x39, Ax, Dx);
        self.a.when(NE, unequal);
        self.a.group(0, R8, 1);
        self.a.group(0, R9, 1);
        self.a.group(0, Cx, 1);
        self.a.jump(next);
        self.a.bind(equal);
        self.a.imm(Ax, 1);
        self.a.jump(done);
        self.a.bind(unequal);
        self.a.imm(Ax, 0);
        self.a.bind(done);
        self.a.call(self.runtime.boolean);
    }
}
