// The broker owns directory descriptors from their initial World acquisition.
// Path effects keep the same inode and never reopen an authority by ambient path.
use crate::x86::*;
use super::syscall;
use super::broker::{allocate, check_fd, empty, REQUEST, SCRATCH};

fn path(a: &mut Assembler, invalid: Label, name: bool) {
    // SI/D is an exact NUL-terminated byte slice. Mutation names are basenames.
    a.test(D, true);
    a.condition(4, invalid);
    a.lea(R8, D, -1);
    if name {
        a.test(R8, true);
        a.condition(4, invalid);
    }
    a.mov(R9, SI);
    a.rr(0x01, R9, R8, true);
    a.load_width(A, R9, 0, 1, false);
    a.test(A, false);
    a.condition(5, invalid);
    a.mov(R9, SI);
    let next = a.label();
    let done = a.label();
    a.bind(next);
    a.test(R8, true);
    a.condition(4, done);
    a.load_width(A, R9, 0, 1, false);
    a.test(A, false);
    a.condition(4, invalid);
    if name {
        a.immediate(C, b'/' as u64);
        a.compare(A, C);
        a.condition(4, invalid);
    }
    a.lea(R9, R9, 1);
    a.lea(R8, R8, -1);
    a.jump(next);
    a.bind(done);
    if name {
        let valid = a.label();
        a.load_width(A, SI, 0, 1, false);
        a.immediate(C, b'.' as u64);
        a.compare(A, C);
        a.condition(5, valid);
        a.immediate(A, 2);
        a.compare(D, A);
        a.condition(4, invalid);
        a.immediate(A, 3);
        a.compare(D, A);
        a.condition(5, valid);
        a.load_width(A, SI, 1, 1, false);
        a.compare(A, C);
        a.condition(4, invalid);
        a.bind(valid);
    }
}

pub(super) fn emit(a: &mut Assembler, labels: &[Label], reply: Label, invalid: Label, bad: Label) {
    for index in 0usize..3usize {
        a.bind(labels[index]);
        if index != 0 {
            a.load(DI, BP, REQUEST + 8);
            check_fd(a, DI, bad);
        }
        a.mov(SI, R14);
        a.mov(D, R15);
        path(a, invalid, false);
        if index == 0 {
            a.immediate(DI, (-100i64) as u64);
            a.immediate(D, 0x290000); // O_PATH | O_DIRECTORY | O_CLOEXEC.
            a.immediate(R10, 0);
            syscall(a, 257);
        } else {
            a.immediate(A, if index == 1 { 0x80000 } else { 0x90000 });
            a.store(BP, SCRATCH, A);
            a.immediate(A, 0);
            a.store(BP, SCRATCH + 8, A);
            if index == 1 {
                let read_only = a.label();
                a.load(C, BP, REQUEST + 16);
                a.immediate(A, 1);
                a.compare(C, A);
                a.condition(7, invalid);
                a.test(C, true);
                a.condition(4, read_only);
                a.immediate(A, 0x80241); // Existing create: O_WRONLY | O_CREAT | O_TRUNC.
                a.store(BP, SCRATCH, A);
                a.immediate(A, 0o666);
                a.store(BP, SCRATCH + 8, A);
                a.bind(read_only);
            }
            a.immediate(A, 0x0a); // RESOLVE_BENEATH | RESOLVE_NO_MAGICLINKS.
            a.store(BP, SCRATCH + 16, A);
            a.lea(D, BP, SCRATCH);
            a.immediate(R10, 24);
            syscall(a, 437);
        }
        empty(a, reply);
    }

    a.bind(labels[3]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.test(R15, true); // This operation has no input body.
    a.condition(5, invalid);
    a.load(R15, BP, REQUEST + 16);
    let buffer_ready = a.label();
    a.test(R15, true);
    a.condition(4, buffer_ready);
    allocate(a, R15);
    let allocated = a.label();
    a.test(A, true);
    a.condition(9, allocated);
    empty(a, reply);
    a.bind(allocated);
    a.mov(R14, A);
    a.bind(buffer_ready);
    a.load(DI, BP, REQUEST + 8);
    a.mov(SI, R14);
    a.mov(D, R15);
    syscall(a, 217);
    let read = a.label();
    a.test(A, true);
    a.condition(9, read);
    empty(a, reply);
    a.bind(read);
    a.mov(R8, R14);
    a.mov(R9, A);
    a.jump(reply);

    for index in 4usize..6usize {
        a.bind(labels[index]);
        a.load(DI, BP, REQUEST + 8);
        check_fd(a, DI, bad);
        a.mov(SI, R14);
        a.mov(D, R15);
        path(a, invalid, true);
        if index == 4 {
            a.immediate(D, 0o777);
            syscall(a, 258);
        } else {
            a.load(D, BP, REQUEST + 16);
            a.immediate(A, 1);
            a.compare(D, A);
            a.condition(7, invalid);
            a.shift(D, 4, 9); // AT_REMOVEDIR.
            syscall(a, 263);
        }
        empty(a, reply);
    }

    a.bind(labels[6]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.load(DI, BP, REQUEST + 16);
    check_fd(a, DI, bad);
    a.load(D, BP, REQUEST + 24);
    a.compare(D, R15);
    a.condition(7, invalid);
    a.mov(SI, R14);
    path(a, invalid, true);
    a.mov(SI, R14);
    a.rr(0x01, SI, D, true);
    a.store(BP, SCRATCH, SI);
    a.mov(A, R15);
    a.rr(0x29, A, D, true);
    a.mov(D, A);
    path(a, invalid, true);
    a.load(DI, BP, REQUEST + 8);
    a.mov(SI, R14);
    a.load(D, BP, REQUEST + 16);
    a.load(R10, BP, SCRATCH);
    syscall(a, 264);
    empty(a, reply);
}
