//! Linux process stacks grow toward twice physical memory, within caller hard limits.
use super::asm::{Asm, R::*, *};

pub(super) fn initialize(a: &mut Asm) {
    let failed = a.label();
    let selected = a.label();
    let done = a.label();
    let ready = a.label();
    // Linux x86-64 sysinfo is 112 bytes; the following 16 bytes hold rlimit.
    a.group(5, Sp, 128);
    a.mov(Di, Sp);
    a.imm(Ax, 99); // sysinfo
    a.syscall();
    a.group(7, Ax, 0);
    a.when(L, failed);
    a.load(R10, Sp, 32); // totalram
    a.load32(R8, Sp, 104); // mem_unit
    a.imul(R10, R8);
    a.when(O, failed);
    a.bin(0x01, R10, R10);
    a.when(O, failed);
    a.group(7, R10, 0);
    a.when(LE, failed);
    a.imm(Di, 3); // RLIMIT_STACK
    a.lea(Si, Sp, 112);
    a.imm(Ax, 97); // getrlimit
    a.syscall();
    a.group(7, Ax, 0);
    a.when(L, failed);
    a.load(R8, Sp, 120); // externally imposed hard limit
    a.bin(0x39, R10, R8);
    a.when(BE, selected);
    a.mov(R10, R8);
    a.bind(selected);
    a.load(R8, Sp, 112);
    a.bin(0x39, R10, R8);
    a.when(BE, done); // preserve an already larger soft limit
    a.store(Sp, 112, R10);
    a.imm(Di, 3);
    a.lea(Si, Sp, 112);
    a.imm(Ax, 160); // setrlimit, keeping the caller's hard limit
    a.syscall();
    a.group(7, Ax, 0);
    a.when(L, failed);
    a.bind(done);
    a.group(0, Sp, 128);
    a.jump(ready);
    a.bind(failed);
    // These syscalls failed before language work started. Do not call that an allocation.
    for (offset, bytes) in b"native_stack_setup\n\0\0\0\0\0\0"
        .chunks_exact(8)
        .enumerate()
    {
        let mut word = [0_u8; 8];
        word.copy_from_slice(bytes);
        a.imm(Ax, u64::from_le_bytes(word));
        a.store(Sp, (offset * 8) as i32, Ax);
    }
    a.imm(Di, 2);
    a.mov(Si, Sp);
    a.imm(Dx, 19);
    a.imm(Ax, 1);
    a.syscall();
    a.imm(Di, 70);
    a.imm(Ax, 60);
    a.syscall();
    a.bytes(&[0x0f, 0x0b]);
    a.bind(ready);
}
