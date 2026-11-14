//! Linux capability leaves for processes, monotonic time and directory changes.
use super::platform::Platform;
use super::{epilogue, prologue};
use crate::diagnostic::Span;
use crate::x86::*;

fn syscall(a: &mut Assembler, number: u64) {
    let retry = a.label();
    a.bind(retry);
    a.immediate(A, number);
    a.syscall();
    a.immediate(C, (-4i64) as u64);
    a.compare(A, C);
    a.condition(4, retry);
}

// SI/D is a NUL-terminated path slice. Names used by mutations contain no slash.
fn path(a: &mut Assembler, invalid: Label, name: bool) {
    a.test(D, true);
    a.condition(4, invalid);
    a.mov(R8, SI);
    a.mov(R9, D);
    a.lea(R9, R9, -1);
    if name {
        a.test(R9, true);
        a.condition(4, invalid);
    }
    a.rr(0x01, R8, R9, true);
    a.load_width(A, R8, 0, 1, false);
    a.test(A, true);
    a.condition(5, invalid);
    a.mov(R8, SI);
    let next = a.label();
    let done = a.label();
    a.bind(next);
    a.test(R9, true);
    a.condition(4, done);
    a.load_width(A, R8, 0, 1, false);
    a.test(A, true);
    a.condition(4, invalid);
    if name {
        a.immediate(R10, b'/' as u64);
        a.compare(A, R10);
        a.condition(4, invalid);
    }
    a.lea(R8, R8, 1);
    a.lea(R9, R9, -1);
    a.jump(next);
    a.bind(done);
    if name {
        let valid = a.label();
        a.load_width(A, SI, 0, 1, false);
        a.immediate(R10, b'.' as u64);
        a.compare(A, R10);
        a.condition(5, valid);
        a.immediate(A, 2);
        a.compare(D, A);
        a.condition(4, invalid);
        a.immediate(A, 3);
        a.compare(D, A);
        a.condition(5, valid);
        a.load_width(A, SI, 1, 1, false);
        a.compare(A, R10);
        a.condition(4, invalid);
        a.bind(valid);
    }
}

fn files(a: &mut Assembler, p: Platform) {
    let invalid = a.label();
    let promote = a.label();
    let temporary = a.label();
    let executable = a.label();
    let name = a.label();
    let self_path = a.label();
    a.bind(p.world_file);
    a.test(DI, true);
    a.condition(4, temporary);
    a.immediate(A, 4);
    a.compare(DI, A);
    a.condition(4, executable);
    a.condition(7, invalid);
    a.lea(DI, DI, -1);
    a.immediate(SI, 1030);
    a.immediate(D, 3);
    syscall(a, 72);
    let standard = a.label();
    a.immediate(C, (-9i64) as u64);
    a.compare(A, C);
    a.condition(5, standard);
    a.immediate(A, 0); // An owned closed stream, not descriptor zero.
    a.bind(standard);
    a.ret();
    a.bind(temporary);
    a.address(DI, name);
    a.immediate(SI, 1); // MFD_CLOEXEC
    syscall(a, 319);
    a.jump(promote);
    a.bind(executable);
    a.immediate(DI, (-100i64) as u64);
    a.address(SI, self_path);
    a.immediate(D, 0x80000);
    a.immediate(R10, 0);
    syscall(a, 257);
    a.bind(promote);
    let ready = a.label();
    a.immediate(C, 3);
    a.compare(A, C);
    a.condition(3, ready);
    a.mov(R8, A);
    a.mov(DI, A);
    a.immediate(SI, 1030);
    a.immediate(D, 3);
    syscall(a, 72);
    a.mov(R9, A);
    a.mov(DI, R8);
    a.immediate(A, 3);
    a.syscall();
    a.mov(A, R9);
    a.bind(ready);
    a.ret();
    a.bind(p.handle_rewind);
    a.immediate(SI, 0);
    a.immediate(D, 0);
    syscall(a, 8);
    a.ret();
    a.bind(p.file_duplicate);
    let duplicate_closed = a.label();
    a.test(DI, true);
    a.condition(4, duplicate_closed);
    a.immediate(SI, 1030);
    a.immediate(D, 3);
    syscall(a, 72);
    a.ret();
    a.bind(duplicate_closed);
    a.immediate(A, 0);
    a.ret();
    a.bind(p.file_readonly);
    a.test(DI, true);
    a.condition(4, duplicate_closed);
    // A descriptor is at most one machine word: twenty decimal digits plus
    // the sealed prefix and terminator fit in this stack buffer.
    prologue(a, p, 64, Span::default());
    a.lea(R8, BP, -1);
    a.immediate(A, 0);
    a.store_width(R8, 0, A, 1);
    a.mov(A, DI);
    a.immediate(C, 10);
    let digit = a.label();
    a.bind(digit);
    a.immediate(D, 0);
    a.divide(C, false);
    a.lea(R8, R8, -1);
    a.lea(D, D, b'0' as i64);
    a.store_width(R8, 0, D, 1);
    a.test(A, true);
    a.condition(5, digit);
    let prefix = b"/proc/self/fd/";
    let mut index = prefix.len();
    while index > 0 {
        index -= 1;
        a.lea(R8, R8, -1);
        a.immediate(A, prefix[index] as u64);
        a.store_width(R8, 0, A, 1);
    }
    a.mov(SI, R8);
    a.immediate(DI, (-100i64) as u64);
    a.immediate(D, 0x80000); // O_RDONLY | O_CLOEXEC, with a fresh open offset.
    a.immediate(R10, 0);
    syscall(a, 257);
    a.mov(SP, BP);
    a.pop(BP);
    a.jump(promote);
    a.bind(p.file_executable);
    a.immediate(SI, 0o755);
    syscall(a, 91);
    a.ret();
    a.bind(p.clock_read);
    a.immediate(A, 16);
    a.compare(SI, A);
    a.condition(2, invalid);
    a.mov(SI, DI);
    a.immediate(DI, 1); // CLOCK_MONOTONIC
    syscall(a, 228);
    a.ret();
    a.bind(invalid);
    a.immediate(A, (-22i64) as u64);
    a.ret();
    a.bind(name);
    a.raw(b"lkjscript\0");
    a.bind(self_path);
    a.raw(b"/proc/self/exe\0");
}

fn directories(a: &mut Assembler, p: Platform) {
    let invalid = a.label();
    let how = a.label();
    a.bind(p.directory_open_directory);
    path(a, invalid, false);
    a.address(D, how);
    a.immediate(R10, 24);
    syscall(a, 437);
    let directory_ready = a.label();
    a.immediate(C, 3);
    a.compare(A, C);
    a.condition(3, directory_ready);
    a.mov(R8, A);
    a.mov(DI, A);
    a.immediate(SI, 1030);
    a.immediate(D, 3);
    syscall(a, 72);
    a.mov(R9, A);
    a.mov(DI, R8);
    a.immediate(A, 3);
    a.syscall();
    a.mov(A, R9);
    a.bind(directory_ready);
    a.ret();
    a.bind(p.directory_read);
    syscall(a, 217);
    a.ret();
    a.bind(p.directory_create);
    path(a, invalid, true);
    a.immediate(D, 0o777);
    syscall(a, 258);
    a.ret();
    a.bind(p.directory_remove);
    a.mov(R11, C);
    path(a, invalid, true);
    a.mov(D, R11);
    a.shift(D, 4, 9); // AT_REMOVEDIR
    syscall(a, 263);
    a.ret();
    a.bind(invalid);
    a.immediate(A, (-22i64) as u64);
    a.ret();

    a.bind(p.directory_rename);
    prologue(a, p, 48, Span::default());
    let registers = vec![DI, SI, D, C, R8, R9];
    for index in 0..registers.len() {
        a.store(BP, -8 - index as i64 * 8, registers[index]);
    }
    let invalid_rename = a.label();
    let finish_rename = a.label();
    path(a, invalid_rename, true);
    a.load(SI, BP, -40);
    a.load(D, BP, -48);
    path(a, invalid_rename, true);
    a.load(DI, BP, -8);
    a.load(SI, BP, -16);
    a.load(D, BP, -32);
    a.load(R10, BP, -40);
    syscall(a, 264);
    a.jump(finish_rename);
    a.bind(invalid_rename);
    a.immediate(A, (-22i64) as u64);
    a.bind(finish_rename);
    epilogue(a);
    a.bind(how);
    a.raw(&[0, 0, 9, 0, 0, 0, 0, 0]);
    a.raw(&[0, 0, 0, 0, 0, 0, 0, 0]);
    a.raw(&[10, 0, 0, 0, 0, 0, 0, 0]);
}

fn children(a: &mut Assembler, p: Platform) {
    a.bind(p.child_wait);
    prologue(a, p, 32, Span::default());
    a.store(BP, -8, DI);
    a.mov(D, SI);
    a.load(DI, DI, 0);
    let missing = a.label();
    let finish = a.label();
    a.test(DI, true);
    a.condition(4, missing);
    a.lea(SI, BP, -16);
    a.immediate(R10, 0);
    syscall(a, 61);
    a.test(A, true);
    a.condition(14, finish);
    a.load(DI, BP, -8);
    a.immediate(A, 0);
    a.store(DI, 0, A);
    a.load_width(A, BP, -16, 4, false);
    a.lea(A, A, 1);
    a.jump(finish);
    a.bind(missing);
    a.immediate(A, (-10i64) as u64);
    a.bind(finish);
    epilogue(a);

    a.bind(p.child_terminate);
    a.load(DI, DI, 0);
    let absent = a.label();
    a.test(DI, true);
    a.condition(4, absent);
    a.immediate(SI, 9);
    syscall(a, 62);
    a.ret();
    a.bind(absent);
    a.immediate(A, (-10i64) as u64);
    a.ret();

    a.bind(p.child_drop);
    prologue(a, p, 16, Span::default());
    a.store(BP, -8, DI);
    let dropped = a.label();
    a.load(A, DI, 0);
    a.test(A, true);
    a.condition(4, dropped);
    a.call(p.child_terminate);
    a.load(DI, BP, -8);
    a.immediate(SI, 0);
    a.call(p.child_wait);
    a.bind(dropped);
    a.immediate(A, 0);
    epilogue(a);
}

// DI points at seven descending words: executable, argv bytes, byte count,
// stdin, stdout, stderr and the raw scalar-result stream.
fn spawn(a: &mut Assembler, p: Platform) {
    a.bind(p.process_spawn);
    prologue(a, p, 144, Span::default());
    a.store(BP, -8, DI);
    a.immediate(A, 0);
    a.store(BP, -24, A);
    for offset in vec![-40i64, -48, -56, -64, -72, -80, -88] {
        a.immediate(A, !0u64);
        a.store(BP, offset, A);
    }
    let invalid = a.label();
    let overflow = a.label();
    let allocation_failed = a.label();
    let bad_executable = a.label();
    let failed = a.label();
    let cleanup = a.label();
    let child = a.label();
    let child_failed = a.label();
    let empty = a.label();
    let signal_default = a.label();
    a.load(SI, DI, -8);
    a.load(D, DI, -16);
    a.test(D, true);
    a.condition(4, invalid);
    a.mov(R8, SI);
    a.rr(0x01, R8, D, true);
    a.load_width(A, R8, -1, 1, false);
    a.test(A, true);
    a.condition(5, invalid);
    a.immediate(R9, 0);
    a.mov(R8, SI);
    let count = a.label();
    let counted = a.label();
    a.bind(count);
    a.test(D, true);
    a.condition(4, counted);
    a.load_width(A, R8, 0, 1, false);
    let nonzero = a.label();
    a.test(A, true);
    a.condition(5, nonzero);
    a.lea(R9, R9, 1);
    a.bind(nonzero);
    a.lea(R8, R8, 1);
    a.lea(D, D, -1);
    a.jump(count);
    a.bind(counted);
    a.immediate(A, (!0u64) / 8 - 1);
    a.compare(R9, A);
    a.condition(7, overflow);
    a.store(BP, -16, R9);
    a.lea(DI, R9, 1);
    a.shift(DI, 4, 3);
    a.store(BP, -32, DI);
    a.call(p.map);
    a.test(A, true);
    a.condition(4, allocation_failed);
    a.store(BP, -24, A);
    a.mov(R8, A);
    a.load(DI, BP, -8);
    a.load(SI, DI, -8);
    a.load(D, DI, -16);
    a.store(R8, 0, SI);
    a.lea(R8, R8, 8);
    let pointers = a.label();
    let pointed = a.label();
    a.bind(pointers);
    a.load_width(A, SI, 0, 1, false);
    a.lea(SI, SI, 1);
    a.lea(D, D, -1);
    a.test(D, true);
    a.condition(4, pointed);
    a.test(A, true);
    a.condition(5, pointers);
    a.store(R8, 0, SI);
    a.lea(R8, R8, 8);
    a.jump(pointers);
    a.bind(pointed);
    a.immediate(A, 0);
    a.store(R8, 0, A);

    a.lea(DI, BP, -120);
    a.immediate(SI, 0x80000); // O_CLOEXEC
    syscall(a, 293);
    a.test(A, true);
    a.condition(8, failed);
    for index in 0..2 {
        a.load_width(DI, BP, -120 + index * 4, 4, false);
        a.immediate(SI, 1030);
        a.immediate(D, 4);
        syscall(a, 72);
        a.store(BP, -40 - index * 8, A);
    }
    for index in 0..2 {
        a.load_width(DI, BP, -120 + index * 4, 4, false);
        a.immediate(A, 3);
        a.syscall();
    }
    for index in 0..2 {
        a.load(A, BP, -40 - index * 8);
        a.test(A, true);
        a.condition(8, failed);
    }
    let offsets = vec![0i64, -24, -32, -40, -48];
    for index in 0..5 {
        a.load(DI, BP, -8);
        a.load(DI, DI, offsets[index]);
        let closed = a.label();
        let duplicated = a.label();
        a.test(DI, true);
        if index == 0 {
            a.condition(4, bad_executable);
        } else {
            a.condition(4, closed);
        }
        a.immediate(SI, 1030);
        a.immediate(D, 4);
        syscall(a, 72);
        a.test(A, true);
        a.condition(8, failed);
        a.store(BP, -56 - index as i64 * 8, A);
        a.jump(duplicated);
        a.bind(closed);
        a.immediate(A, !0u64);
        a.store(BP, -56 - index as i64 * 8, A);
        a.bind(duplicated);
    }
    syscall(a, 57);
    a.test(A, true);
    a.condition(8, failed);
    a.condition(4, child);
    a.store(BP, -104, A);
    a.load(DI, BP, -48);
    a.immediate(A, 3);
    a.syscall();
    a.immediate(A, !0u64);
    a.store(BP, -48, A);
    a.load(DI, BP, -40);
    a.lea(SI, BP, -128);
    a.immediate(D, 8);
    syscall(a, 0);
    a.test(A, true);
    let launched = a.label();
    a.condition(4, launched);
    a.load(A, BP, -128);
    a.store(BP, -112, A);
    a.load(DI, BP, -104);
    a.immediate(SI, 0);
    a.immediate(D, 0);
    a.immediate(R10, 0);
    syscall(a, 61);
    a.jump(cleanup);
    a.bind(launched);
    a.load(A, BP, -104);
    a.store(BP, -112, A);
    a.jump(cleanup);

    a.bind(child);
    a.immediate(DI, 1);
    a.immediate(SI, 9);
    a.immediate(D, 0);
    a.immediate(R10, 0);
    a.immediate(R8, 0);
    syscall(a, 157); // PR_SET_PDEATHSIG: owner death stops the child.
    a.test(A, true);
    a.condition(8, child_failed);
    a.immediate(DI, 13);
    a.address(SI, signal_default);
    a.immediate(D, 0);
    a.immediate(R10, 8);
    syscall(a, 13);
    a.test(A, true);
    a.condition(8, child_failed);
    a.load(DI, BP, -40);
    a.immediate(A, 3);
    a.syscall();
    for descriptor in 0..4 {
        a.load(DI, BP, -64 - descriptor * 8);
        let close = a.label();
        let assigned = a.label();
        a.test(DI, true);
        a.condition(8, close);
        a.immediate(SI, descriptor as u64);
        syscall(a, 33);
        a.test(A, true);
        a.condition(8, child_failed);
        a.jump(assigned);
        a.bind(close);
        a.immediate(DI, descriptor as u64);
        a.immediate(A, 3);
        a.syscall();
        a.bind(assigned);
    }
    a.load(DI, BP, -56);
    a.address(SI, empty);
    a.load(D, BP, -24);
    a.immediate(R10, 0); // Explicit empty environment.
    a.immediate(R8, 0x1000); // AT_EMPTY_PATH
    syscall(a, 322);
    a.bind(child_failed);
    a.store(BP, -128, A);
    a.load(DI, BP, -48);
    a.lea(SI, BP, -128);
    a.immediate(D, 8);
    syscall(a, 1);
    a.immediate(DI, 127);
    a.immediate(A, 60);
    a.syscall();

    a.bind(invalid);
    a.immediate(A, (-22i64) as u64);
    a.jump(failed);
    a.bind(overflow);
    a.immediate(A, (-75i64) as u64);
    a.jump(failed);
    a.bind(bad_executable);
    a.immediate(A, (-9i64) as u64);
    a.jump(failed);
    a.bind(allocation_failed);
    a.immediate(A, (-12i64) as u64);
    a.bind(failed);
    a.store(BP, -112, A);
    a.bind(cleanup);
    for offset in vec![-40i64, -48, -56, -64, -72, -80, -88] {
        a.load(DI, BP, offset);
        a.immediate(A, 3);
        a.syscall();
    }
    a.load(DI, BP, -24);
    let unmapped = a.label();
    a.test(DI, true);
    a.condition(4, unmapped);
    a.load(SI, BP, -32);
    a.call(p.unmap);
    a.bind(unmapped);
    a.load(A, BP, -112);
    epilogue(a);
    a.bind(empty);
    a.raw(&[0]);
    a.bind(signal_default);
    a.raw(&[0u8; 32]);
}

pub(super) fn emit(a: &mut Assembler, p: Platform) {
    files(a, p);
    directories(a, p);
    children(a, p);
    spawn(a, p);
}
