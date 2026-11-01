//! Caller-owned native phase contexts. Slab lists, not passive counters, own memory.
//! R14 selects a Context64; R15 is zero or a transient invocation recovery frame.
//! Only the closed pure binary-filter boundary exports recovery and promotion.
use super::asm::{R::*, *};
use super::runtime::Runtime;
use super::{Emitter, Result};

const FRAME: i32 = 128;
const SAVED: [R; 6] = [Bp, Bx, R12, R13, R14, R15];

pub(super) fn process_context(a: &mut Asm) {
    a.imm(R15, 0);
    a.group(5, Sp, 64);
    a.mov(R14, Sp);
    a.imm(Ax, 0);
    for offset in (0..64).step_by(8) {
        a.store(R14, offset, Ax);
    }
}

fn add_observation(a: &mut Asm, offset: i32, amount: R, bit: i32) {
    let exact = a.label();
    a.load(R8, R14, offset);
    a.bin(0x01, R8, amount);
    a.when(AE, exact);
    a.imm(R8, u64::MAX);
    a.load(R9, R14, 40);
    a.group(1, R9, bit);
    a.store(R14, 40, R9);
    a.bind(exact);
    a.store(R14, offset, R8);
}

fn subtract_observation(a: &mut Asm, offset: i32, amount: R, bit: i32) {
    let done = a.label();
    let saturated = a.label();
    a.load(R8, Bx, 40);
    a.mov(R9, R8);
    a.group(4, R9, bit);
    a.when(NE, done);
    a.load(Ax, Bx, offset);
    a.bin(0x29, Ax, amount);
    a.when(B, saturated);
    a.store(Bx, offset, Ax);
    a.jump(done);
    a.bind(saturated);
    a.group(1, R8, bit);
    a.store(Bx, 40, R8);
    a.bind(done);
}

pub(super) fn emit_runtime(a: &mut Asm, runtime: &Runtime) {
    emit_poll(a, runtime);
    emit_copy(a, runtime);
    emit_abort(a, runtime);
    emit_allocate(a, runtime);
    emit_dispose(a, runtime);
}

fn emit_poll(a: &mut Asm, runtime: &Runtime) {
    let done = a.label();
    a.bind(runtime.poll);
    a.bytes(&[0x9c]); // Preserve incoming flags; R11 is reserved volatile scratch.
    a.bin(0x85, R15, R15);
    a.when(E, done);
    a.load(R11, R15, 16);
    a.bin(0x85, R11, R11);
    a.when(E, done);
    a.load(R11, R11, 0);
    a.bin(0x85, R11, R11);
    a.when(E, done);
    a.imm(Ax, 1);
    a.imm(Dx, 0);
    a.jump(runtime.abort);
    a.bind(done);
    a.bytes(&[0x9d]);
    a.ret();
}

fn emit_copy(a: &mut Asm, runtime: &Runtime) {
    let next = a.label();
    let sized = a.label();
    let done = a.label();
    a.bind(runtime.copy);
    a.bytes(&[0x9c]);
    a.push(R8);
    a.push(R9);
    a.mov(R8, Cx);
    a.bind(next);
    runtime.checkpoint(a);
    a.bin(0x85, R8, R8);
    a.when(E, done);
    a.mov(Cx, R8);
    a.group(7, Cx, 65536);
    a.when(6, sized);
    a.imm(Cx, 65536);
    a.bind(sized);
    a.mov(R9, Cx);
    a.bytes(&[0xfc, 0xf3, 0xa4]);
    a.bin(0x29, R8, R9);
    a.jump(next);
    a.bind(done);
    a.pop(R9);
    a.pop(R8);
    a.bytes(&[0x9d]);
    a.ret();
}

fn emit_abort(a: &mut Asm, runtime: &Runtime) {
    a.bind(runtime.abort);
    a.bin(0x85, R15, R15);
    a.when(E, runtime.capacity);
    a.store(R15, 64, Ax);
    a.store(R15, 72, Dx);
    a.load(Sp, R15, 0);
    a.load(R11, R15, 8);
    a.bytes(&[0x41, 0xff, 0xe3]);
}

fn emit_allocate(a: &mut Asm, runtime: &Runtime) {
    let observed = a.label();
    let attempts_exact = a.label();
    let nonzero = a.label();
    let sized = a.label();
    let available = a.label();
    a.bind(runtime.alloc);
    runtime.checkpoint(a);
    a.bin(0x85, R15, R15);
    a.when(E, observed);
    a.load(R8, R15, 24);
    a.load(R9, R8, 144);
    a.group(0, R9, 1);
    a.when(AE, attempts_exact);
    a.imm(R9, u64::MAX);
    a.imm(R10, 1);
    a.store(R8, 152, R10);
    a.bind(attempts_exact);
    a.store(R8, 144, R9);
    a.bind(observed);
    a.load(Ax, R14, 48);
    a.group(7, Ax, 0);
    a.when(NE, runtime.capacity);
    a.group(7, Di, 0);
    a.when(L, runtime.capacity);
    a.group(0, Di, 7);
    a.when(O, runtime.capacity);
    a.group(4, Di, -8);
    a.when(NE, nonzero);
    a.imm(Di, 8);
    a.bind(nonzero);
    a.load(Ax, R14, 8);
    a.mov(R10, Ax);
    a.bin(0x01, R10, Di);
    a.when(B, runtime.capacity);
    a.load(R9, R14, 16);
    a.bin(0x39, R10, R9);
    a.when(6, available);
    a.mov(Si, Di);
    a.group(0, Si, 64);
    a.when(O, runtime.capacity);
    a.group(7, Si, 65536);
    a.when(AE, sized);
    a.imm(Si, 65536);
    a.bind(sized);
    a.group(0, Si, 4095);
    a.when(O, runtime.capacity);
    a.group(4, Si, -4096);
    a.push(Di);
    a.push(Si);
    a.imm(Ax, 9);
    a.imm(Di, 0);
    a.imm(Dx, 3);
    a.imm(R10, 0x22);
    a.imm(R8, u64::MAX);
    a.imm(R9, 0);
    a.syscall();
    a.pop(Si);
    a.pop(Di);
    a.group(7, Ax, -4095);
    a.when(AE, runtime.capacity);
    // Link before further checked arithmetic so recovery owns the mapping.
    a.load(R8, R14, 0);
    a.store(Ax, 0, R8);
    a.store(Ax, 8, Si);
    a.store(Ax, 16, R14);
    // A successful mmap is page-aligned, so base + 64 cannot wrap a u64
    // page base. The checked mapping extent is at least 64 KiB.
    a.lea(R10, Ax, 64);
    a.store(Ax, 24, R10);
    a.store(R14, 0, R10);
    a.mov(R9, Ax);
    a.bin(0x01, R9, Si);
    a.when(B, runtime.capacity);
    a.store(R14, 16, R9);
    a.mov(Ax, R10);
    a.bin(0x01, R10, Di);
    a.when(B, runtime.capacity);
    a.store(R14, 8, R10);
    a.imm(Cx, 1);
    add_observation(a, 24, Cx, 1);
    add_observation(a, 32, Si, 2);
    a.ret();
    a.bind(available);
    a.store(R14, 8, R10);
    a.ret();
}

fn emit_dispose(a: &mut Asm, runtime: &Runtime) {
    let next = a.label();
    let closed = a.label();
    let failed = a.label();
    let done = a.label();
    a.bind(runtime.dispose);
    a.push(Bx);
    a.push(R12);
    a.push(R13);
    a.mov(Bx, Di);
    a.imm(Ax, 1);
    a.store(Bx, 48, Ax);
    a.imm(Ax, 0);
    for offset in [8, 16, 56] {
        a.store(Bx, offset, Ax);
    }
    a.bind(next);
    a.load(Di, Bx, 0);
    a.bin(0x85, Di, Di);
    a.when(E, closed);
    a.group(5, Di, 64);
    a.when(B, failed);
    a.load(R12, Di, 0);
    a.load(R13, Di, 8);
    a.mov(Si, R13);
    a.imm(Ax, 11);
    a.syscall();
    a.group(7, Ax, 0);
    a.when(NE, failed);
    a.store(Bx, 0, R12);
    a.imm(Cx, 1);
    subtract_observation(a, 24, Cx, 1);
    subtract_observation(a, 32, R13, 2);
    a.jump(next);
    a.bind(closed);
    a.imm(Ax, 0);
    for offset in [0, 8, 16, 24, 32, 40, 56] {
        a.store(Bx, offset, Ax);
    }
    a.imm(Cx, 2);
    a.store(Bx, 48, Cx);
    a.jump(done);
    a.bind(failed);
    a.imm(Ax, 4);
    a.bind(done);
    a.pop(R13);
    a.pop(R12);
    a.pop(Bx);
    a.ret();
}

pub(super) fn recover_trap(a: &mut Asm, runtime: &Runtime, status: u64, detail: u64) {
    let process = a.label();
    a.bin(0x85, R15, R15);
    a.when(E, process);
    a.imm(Ax, status);
    a.imm(Dx, detail);
    a.jump(runtime.abort);
    a.bind(process);
}

pub(super) fn emit_exports(e: &mut Emitter<'_>, entry: usize) -> Result<(Label, Label)> {
    let invoke = e.a.label();
    let release = e.a.label();
    let entry = e.label(entry)?;
    emit_release(&mut e.a, &e.runtime, release);
    emit_invoke(&mut e.a, &e.runtime, entry, invoke, release);
    Ok((invoke, release))
}

fn emit_release(a: &mut Asm, runtime: &Runtime, release: Label) {
    let invalid = a.label();
    let success = a.label();
    let done = a.label();
    a.bind(release);
    a.push(Bx);
    a.push(R12);
    a.mov(Bx, Di);
    a.bin(0x85, Bx, Bx);
    a.when(E, invalid);
    a.mov(Ax, Bx);
    a.group(4, Ax, 7);
    a.when(NE, invalid);
    a.mov(Ax, Bx);
    a.group(0, Ax, 160);
    a.when(B, invalid);
    a.lea(Di, Bx, 64);
    a.call(runtime.dispose);
    a.mov(R12, Ax);
    a.mov(Di, Bx);
    a.call(runtime.dispose);
    a.bin(0x09, Ax, R12);
    a.bin(0x85, Ax, Ax);
    a.when(E, success);
    a.imm(Ax, 4);
    a.store(Bx, 128, Ax);
    a.jump(done);
    a.bind(success);
    a.store(Bx, 128, Ax);
    a.store(Bx, 136, Ax);
    a.jump(done);
    a.bind(invalid);
    a.imm(Ax, 5);
    a.bind(done);
    a.pop(R12);
    a.pop(Bx);
    a.ret();
}

fn emit_invoke(a: &mut Asm, runtime: &Runtime, entry: Label, invoke: Label, release: Label) {
    let invalid = a.label();
    let invalid_result = a.label();
    let length_empty = a.label();
    let recover = a.label();
    let cleanup_failed = a.label();
    let recovered = a.label();
    let copied = a.label();
    let done = a.label();
    a.bind(invoke);
    for register in SAVED {
        a.push(register);
    }
    a.group(5, Sp, FRAME);
    a.mov(R15, Sp);
    a.store(R15, 0, Sp);
    a.address(Ax, recover);
    a.store(R15, 8, Ax);
    for (offset, register) in [(16, Cx), (24, Dx), (32, Di), (40, Si)] {
        a.store(R15, offset, register);
    }
    a.imm(Ax, 0);
    for offset in [48, 56, 64, 72, 80, 88] {
        a.store(R15, offset, Ax);
    }
    a.group(7, Si, 0);
    a.when(L, invalid);
    a.bin(0x85, Dx, Dx);
    a.when(E, invalid);
    a.mov(Ax, Dx);
    a.group(4, Ax, 7);
    a.when(NE, invalid);
    a.mov(Ax, Dx);
    a.group(0, Ax, 160);
    a.when(B, invalid);
    a.mov(Ax, Cx);
    a.group(4, Ax, 7);
    a.when(NE, invalid);
    a.mov(Ax, Cx);
    a.group(0, Ax, 8);
    a.when(B, invalid);
    a.bin(0x85, Si, Si);
    a.when(E, length_empty);
    a.bin(0x85, Di, Di);
    a.when(E, invalid);
    a.bind(length_empty);
    a.mov(Ax, Di);
    a.bin(0x01, Ax, Si);
    a.when(B, invalid);
    for offset in [0, 8, 16, 56, 64, 72, 80, 120] {
        a.load(Ax, Dx, offset);
        a.bin(0x85, Ax, Ax);
        a.when(NE, invalid);
    }
    for offset in [48, 112] {
        let allowed = a.label();
        a.load(Ax, Dx, offset);
        a.group(7, Ax, 0);
        a.when(E, allowed);
        a.group(7, Ax, 2);
        a.when(NE, invalid);
        a.bind(allowed);
    }
    a.imm(Ax, 0);
    for offset in (0..160).step_by(8) {
        a.store(Dx, offset, Ax);
    }
    a.mov(R14, Dx);
    runtime.checkpoint(a);
    a.imm(Ax, 7);
    a.store(R15, 96, Ax);
    a.load(Ax, R15, 40);
    a.store(R15, 104, Ax);
    a.load(Ax, R15, 32);
    a.store(R15, 112, Ax);
    a.lea(Ax, R15, 96);
    a.push(Ax);
    a.call(entry);
    a.group(0, Sp, 8);
    a.store(R15, 48, Ax);
    a.load(Bx, Ax, 40);
    a.store(R15, 80, Bx);
    a.load(R12, Bx, 8);
    a.group(7, R12, 0);
    a.when(L, invalid_result);
    a.store(R15, 88, R12);
    a.load(R14, R15, 24);
    a.group(0, R14, 64);
    a.imm(Di, 24);
    a.call(runtime.alloc);
    a.store(R15, 56, Ax);
    a.load(Bx, R15, 48);
    a.load(Cx, Bx, 24);
    a.load(Cx, Cx, 8);
    a.store(Ax, 0, Cx);
    a.load(Cx, R15, 88);
    a.store(Ax, 8, Cx);
    a.imm(Dx, 0);
    a.store(Ax, 16, Dx);
    a.bin(0x85, Cx, Cx);
    a.when(E, copied);
    a.mov(Di, Cx);
    a.call(runtime.alloc);
    a.load(Bx, R15, 56);
    a.store(Bx, 16, Ax);
    a.mov(Di, Ax);
    a.load(Bx, R15, 80);
    a.load(Si, Bx, 16);
    a.load(Cx, R15, 88);
    runtime.copy_bytes(a);
    a.bind(copied);
    runtime.checkpoint(a);
    a.load(Di, R15, 24);
    a.call(runtime.dispose);
    a.bin(0x85, Ax, Ax);
    a.when(NE, cleanup_failed);
    a.load(Bx, R15, 24);
    a.load(Cx, R15, 56);
    a.store(Bx, 120, Cx);
    a.imm(Ax, 0);
    a.store(Bx, 128, Ax);
    a.store(Bx, 136, Ax);
    a.jump(done);
    a.bind(cleanup_failed);
    a.imm(Ax, 4);
    a.store(R15, 64, Ax);
    a.store(R15, 72, Ax);
    a.bind(recover);
    a.load(Di, R15, 24);
    a.call(release);
    a.bin(0x85, Ax, Ax);
    a.when(NE, recovered);
    a.load(Ax, R15, 64);
    a.bind(recovered);
    a.load(Bx, R15, 24);
    a.store(Bx, 128, Ax);
    a.load(Cx, R15, 72);
    a.store(Bx, 136, Cx);
    a.jump(done);
    a.bind(invalid_result);
    a.imm(Ax, 3);
    a.imm(Dx, 0);
    a.jump(runtime.abort);
    a.bind(invalid);
    a.imm(Ax, 5);
    a.bind(done);
    a.lea(Sp, R15, FRAME);
    for register in SAVED.into_iter().rev() {
        a.pop(register);
    }
    a.ret();
}

pub(super) fn write_table(bytes: &mut [u8], invoke: u64, release: u64) {
    bytes[256..264].copy_from_slice(b"LKJPH001");
    for (offset, value) in [
        (264, 64),
        (272, 160),
        (280, 24),
        (288, invoke),
        (296, release),
        (304, 1),
        (312, 0),
    ] {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}
