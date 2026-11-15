// Syscall-only RPC endpoint. Acknowledgment follows the actual descriptor operation.
use crate::x86::*;
use super::{elf, syscall};
use super::process::{self, ProcessCode};
use super::directories;

pub(super) const REQUEST: i64 = -128;
const RESPONSE: i64 = -144;
const POLL: i64 = -160;
const AUXILIARY: i64 = -176;
const IMPORTED: i64 = -184;
const SAVED: i64 = -192;
pub(super) const SCRATCH: i64 = -8192;
const EVENTS: i64 = -9216;
const EVENT_OUTPUT: i64 = -11264;
const DRAIN: i64 = -76800;

pub(super) fn empty(a: &mut Assembler, reply: Label) {
    a.immediate(R8, 0);
    a.immediate(R9, 0);
    a.jump(reply);
}
pub(super) fn check_fd(a: &mut Assembler, reg: u8, bad: Label) {
    a.immediate(C, 3);
    a.compare(reg, C);
    a.condition(6, bad);
    a.immediate(C, 2147483647);
    a.compare(reg, C);
    a.condition(7, bad);
    a.compare(reg, R12);
    a.condition(4, bad);
    a.compare(reg, R13);
    a.condition(4, bad);
}
pub(super) fn allocate(a: &mut Assembler, size: u8) {
    a.mov(SI, size);
    a.immediate(DI, 0);
    a.immediate(D, 3);
    a.immediate(R10, 0x22);
    a.immediate(R8, !0u64);
    a.immediate(R9, 0);
    syscall(a, 9);
}
fn exact(a: &mut Assembler, label: Label, number: u64, failure: Label) {
    a.bind(label);
    let more = a.label();
    a.bind(more);
    syscall(a, number);
    a.test(A, true);
    a.condition(14, failure);
    a.rr(0x01, SI, A, true);
    a.rr(0x29, D, A, true);
    a.test(D, true);
    a.condition(5, more);
    a.ret();
}
fn socket(a: &mut Assembler, domain: u64, kind: u64) {
    a.immediate(DI, domain);
    a.immediate(SI, kind | 0x80000);
    a.immediate(D, 0);
    syscall(a, 41);
}
fn nodelay(a: &mut Assembler, file: u8) {
    a.immediate(A, 1);
    a.store_width(BP, SCRATCH, A, 4);
    a.mov(DI, file);
    a.immediate(SI, 6);
    a.immediate(D, 1);
    a.lea(R10, BP, SCRATCH);
    a.immediate(R8, 4);
    syscall(a, 54);
}
fn zero(a: &mut Assembler, start: i64, count: usize) {
    a.immediate(A, 0);
    for index in 0usize..count {
        a.store(BP, start + (index * 8) as i64, A);
    }
}

fn emit_storage(a: &mut Assembler, labels: &[Label], reply: Label, invalid: Label, bad: Label) {
    let opening = a.label();
    a.bind(labels[0]);
    a.immediate(A, 0x800c2); // O_CLOEXEC | O_CREAT | O_EXCL | O_RDWR.
    a.jump(opening);
    a.bind(labels[1]);
    a.immediate(A, 0x80442); // O_CLOEXEC | O_CREAT | O_APPEND | O_RDWR.
    a.bind(opening);
    a.store(BP, SCRATCH, A);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.test(R15, true);
    a.condition(4, invalid);
    a.lea(R8, R15, -1);
    a.mov(R9, R14);
    a.rr(0x01, R9, R8, true);
    a.load_width(A, R9, 0, 1, false);
    a.test(A, false);
    a.condition(5, invalid);
    a.mov(R9, R14);
    let scanning = a.label();
    let valid = a.label();
    a.bind(scanning);
    a.test(R8, true);
    a.condition(4, valid);
    a.load_width(A, R9, 0, 1, false);
    a.test(A, false);
    a.condition(4, invalid);
    a.lea(R9, R9, 1);
    a.lea(R8, R8, -1);
    a.jump(scanning);
    a.bind(valid);
    a.immediate(A, 0o666);
    a.store(BP, SCRATCH + 8, A);
    a.immediate(A, 0x0a);
    a.store(BP, SCRATCH + 16, A);
    a.mov(SI, R14);
    a.lea(D, BP, SCRATCH);
    a.immediate(R10, 24);
    syscall(a, 437);
    empty(a, reply);

    for index in 2usize..5usize {
        a.bind(labels[index]);
        a.load(DI, BP, REQUEST + 8);
        check_fd(a, DI, bad);
        if index == 3 { a.load(SI, BP, REQUEST + 16); }
        if index == 4 { a.immediate(SI, 6); }
        syscall(a, if index == 2 { 74 } else if index == 3 { 77 } else { 73 });
        empty(a, reply);
    }
}

pub(super) fn image(local: &[u8], address: &[u8], nonce: &[u8]) -> Result<Vec<u8>, i64> {
    let mut a = Assembler::new();
    let p = ProcessCode::new(&mut a);
    let next = a.label();
    let rpc = a.label();
    let unix_request = a.label();
    let reply = a.label();
    let exit = a.label();
    let startup_failed = a.label();
    let read_all = a.label();
    let write_all = a.label();
    let bad = a.label();
    let invalid = a.label();
    let local_data = a.label();
    let unix_data = a.label();
    let secret = a.label();
    let ready = a.label();
    let mut operations = Vec::new();
    for _index in 0usize..16usize { operations.push(a.label()); }
    let mut storage = Vec::new();
    for _index in 0usize..5usize { storage.push(a.label()); }
    let mut directories = Vec::new();
    for _index in 0usize..7usize { directories.push(a.label()); }

    a.mov(BP, SP);
    a.lea(SP, SP, DRAIN);
    a.immediate(A, 0);
    a.store(BP, -16, A); // Process-record head.
    a.immediate(R14, 0);
    a.immediate(R15, 0);
    a.immediate(DI, 1); // PR_SET_PDEATHSIG
    a.immediate(SI, 9);
    a.immediate(D, 0);
    a.immediate(R10, 0);
    a.immediate(R8, 0);
    syscall(&mut a, 157);
    a.test(A, true);
    a.condition(8, startup_failed);
    zero(&mut a, SCRATCH, 4);
    a.immediate(A, 1); // SIG_IGN for SIGPIPE.
    a.store(BP, SCRATCH, A);
    a.immediate(DI, 13);
    a.lea(SI, BP, SCRATCH);
    a.immediate(D, 0);
    a.immediate(R10, 8);
    syscall(&mut a, 13);
    a.test(A, true);
    a.condition(8, startup_failed);
    socket(&mut a, 1, 5); // Private abstract AF_UNIX SOCK_SEQPACKET import endpoint.
    a.test(A, true);
    a.condition(8, startup_failed);
    a.mov(R13, A);
    a.mov(DI, R13);
    a.address(SI, unix_data);
    a.immediate(D, address.len() as u64);
    syscall(&mut a, 49);
    a.test(A, true);
    a.condition(8, startup_failed);
    a.mov(DI, R13);
    a.immediate(SI, 2147483647);
    syscall(&mut a, 50);
    a.test(A, true);
    a.condition(8, startup_failed);
    socket(&mut a, 2, 1);
    a.test(A, true);
    a.condition(8, startup_failed);
    a.mov(R12, A);
    a.mov(DI, R12);
    a.address(SI, local_data);
    a.immediate(D, local.len() as u64);
    syscall(&mut a, 42);
    a.test(A, true);
    a.condition(8, startup_failed);
    nodelay(&mut a, R12);
    a.test(A, true);
    a.condition(8, startup_failed);
    a.immediate(A, 0);
    a.store(BP, RESPONSE, A);
    a.immediate(DI, 1);
    a.lea(SI, BP, RESPONSE);
    a.immediate(D, 8);
    a.call(write_all);
    a.mov(DI, R12);
    a.address(SI, secret);
    a.immediate(D, nonce.len() as u64);
    a.call(write_all);
    a.jump(next);
    a.bind(startup_failed);
    a.negate(A);
    a.store(BP, RESPONSE, A);
    a.immediate(DI, 1);
    a.lea(SI, BP, RESPONSE);
    a.immediate(D, 8);
    a.call(write_all);
    a.jump(exit);

    a.bind(next);
    a.immediate(A, 0);
    a.store(BP, POLL, A);
    a.store(BP, POLL + 8, A);
    a.store_width(BP, POLL, R12, 4);
    a.store_width(BP, POLL + 8, R13, 4);
    a.immediate(A, 1);
    a.store_width(BP, POLL + 4, A, 2);
    a.store_width(BP, POLL + 12, A, 2);
    a.lea(DI, BP, POLL);
    a.immediate(SI, 2);
    a.immediate(D, !0u64);
    syscall(&mut a, 7);
    a.test(A, true);
    a.condition(8, exit);
    a.load_width(A, BP, POLL + 14, 2, false);
    a.test(A, true);
    a.condition(5, unix_request);
    a.jump(rpc);

    a.bind(rpc);
    a.mov(DI, R12);
    a.lea(SI, BP, REQUEST);
    a.immediate(D, 64);
    a.call(read_all);
    a.immediate(R14, 0);
    a.load(R15, BP, REQUEST + 56);
    a.test(R15, true);
    a.condition(4, ready);
    allocate(&mut a, R15);
    a.test(A, true);
    let allocated = a.label();
    a.condition(9, allocated);
    a.store(BP, SAVED, A);
    a.mov(B, R15);
    let discard = a.label();
    a.bind(discard);
    a.mov(D, B);
    a.immediate(C, 65536);
    a.compare(D, C);
    a.cmov(D, C, 7);
    a.store(BP, AUXILIARY, D);
    a.mov(DI, R12);
    a.lea(SI, BP, DRAIN);
    a.call(read_all);
    a.load(D, BP, AUXILIARY);
    a.rr(0x29, B, D, true);
    a.test(B, true);
    a.condition(5, discard);
    a.load(A, BP, SAVED);
    empty(&mut a, reply);
    a.bind(allocated);
    a.mov(R14, A);
    a.mov(DI, R12);
    a.mov(SI, R14);
    a.mov(D, R15);
    a.call(read_all);
    a.bind(ready);
    a.load(A, BP, REQUEST);
    for index in 0usize..operations.len() {
        a.immediate(C, (index + 1) as u64);
        a.compare(A, C);
        a.condition(4, operations[index]);
    }
    for index in 0usize..storage.len() {
        a.immediate(C, (index + 30) as u64);
        a.compare(A, C);
        a.condition(4, storage[index]);
    }
    for index in 0usize..directories.len() {
        a.immediate(C, (index + 35) as u64);
        a.compare(A, C);
        a.condition(4, directories[index]);
    }
    for (number, label) in vec![(20u64, p.spawn), (21u64, p.wait), (22u64, p.terminate), (23u64, p.drop_child)] {
        a.immediate(C, number);
        a.compare(A, C);
        a.condition(4, label);
    }
    a.jump(invalid);

    emit_storage(&mut a, &storage, reply, invalid, bad);
    directories::emit(&mut a, &directories, reply, invalid, bad);
    emit_network(&mut a, &operations, reply, invalid, bad);
    emit_files(&mut a, &operations, reply, bad);
    emit_readiness(&mut a, &operations, reply, invalid, bad);
    process::emit(&mut a, &p, reply);

    a.bind(bad);
    a.immediate(A, (-9i64) as u64);
    empty(&mut a, reply);
    a.bind(invalid);
    a.immediate(A, (-22i64) as u64);
    empty(&mut a, reply);

    a.bind(reply);
    a.store(BP, RESPONSE, A);
    a.store(BP, RESPONSE + 8, R9);
    a.store(BP, -48, R8);
    a.store(BP, -40, R9);
    a.mov(DI, R12);
    a.lea(SI, BP, RESPONSE);
    a.immediate(D, 16);
    a.call(write_all);
    let no_body = a.label();
    a.load(D, BP, -40);
    a.test(D, true);
    a.condition(4, no_body);
    a.mov(DI, R12);
    a.load(SI, BP, -48);
    a.call(write_all);
    a.bind(no_body);
    let no_mapping = a.label();
    a.test(R14, true);
    a.condition(4, no_mapping);
    a.mov(DI, R14);
    a.mov(SI, R15);
    syscall(&mut a, 11);
    a.bind(no_mapping);
    a.immediate(R14, 0);
    a.immediate(R15, 0);
    a.jump(next);

    emit_import(&mut a, unix_request, secret, next);
    exact(&mut a, read_all, 0, exit);
    exact(&mut a, write_all, 1, exit);
    a.bind(exit);
    a.call(p.shutdown);
    a.immediate(DI, 0);
    a.immediate(A, 60);
    a.syscall();
    a.bind(local_data);
    a.raw(local);
    a.bind(unix_data);
    a.raw(address);
    a.bind(secret);
    a.raw(nonce);
    Ok(elf(a.finish().map_err(|_| 8i64)?))
}

fn emit_network(a: &mut Assembler, ops: &[Label], reply: Label, invalid: Label, bad: Label) {
    for (index, connect) in vec![(0usize, true), (1usize, false)] {
        a.bind(ops[index]);
        a.immediate(A, 16);
        a.compare(R15, A);
        a.condition(5, invalid);
        a.load_width(A, R14, 0, 2, false);
        a.immediate(C, 2);
        a.compare(A, C);
        a.condition(5, invalid);
        socket(a, 2, 1);
        a.test(A, true);
        let opened = a.label();
        let failed = a.label();
        a.condition(9, opened);
        empty(a, reply);
        a.bind(opened);
        a.store(BP, AUXILIARY, A);
        a.mov(DI, A);
        a.mov(SI, R14);
        a.immediate(D, 16);
        syscall(a, if connect { 42 } else { 49 });
        if connect {
            let already = a.label();
            a.immediate(C, (-106i64) as u64);
            a.compare(A, C);
            a.condition(4, already);
            a.test(A, true);
            a.condition(8, failed);
            a.bind(already);
        } else {
            a.test(A, true);
            a.condition(8, failed);
            a.load(DI, BP, AUXILIARY);
            a.immediate(SI, 2147483647);
            syscall(a, 50);
            a.test(A, true);
            a.condition(8, failed);
        }
        a.load(A, BP, AUXILIARY);
        empty(a, reply);
        a.bind(failed);
        a.store(BP, SAVED, A);
        a.load(DI, BP, AUXILIARY);
        a.immediate(A, 3);
        a.syscall();
        a.load(A, BP, SAVED);
        empty(a, reply);
    }
    a.bind(ops[2]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.immediate(SI, 0);
    a.immediate(D, 0);
    a.immediate(R10, 0x80000);
    syscall(a, 288);
    let accepted = a.label();
    let accepted_done = a.label();
    a.test(A, true);
    a.condition(9, accepted);
    empty(a, reply);
    a.bind(accepted);
    a.store(BP, AUXILIARY, A);
    a.load(C, BP, REQUEST + 16);
    a.test(C, true);
    a.condition(4, accepted_done);
    a.mov(B, A);
    nodelay(a, B);
    a.test(A, true);
    a.condition(9, accepted_done);
    a.store(BP, SAVED, A);
    a.load(DI, BP, AUXILIARY);
    a.immediate(A, 3);
    a.syscall();
    a.load(A, BP, SAVED);
    empty(a, reply);
    a.bind(accepted_done);
    a.load(A, BP, AUXILIARY);
    empty(a, reply);

    a.bind(ops[3]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.immediate(A, 16);
    a.store_width(BP, SCRATCH + 16, A, 4);
    a.lea(SI, BP, SCRATCH);
    a.lea(D, BP, SCRATCH + 16);
    syscall(a, 51);
    let address_ok = a.label();
    a.test(A, true);
    a.condition(9, address_ok);
    empty(a, reply);
    a.bind(address_ok);
    a.lea(R8, BP, SCRATCH);
    a.immediate(R9, 16);
    a.jump(reply);
}

fn emit_files(a: &mut Assembler, ops: &[Label], reply: Label, bad: Label) {
    a.bind(ops[4]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.load(R15, BP, REQUEST + 16);
    let buffer_ready = a.label();
    a.test(R15, true);
    a.condition(4, buffer_ready);
    allocate(a, R15);
    let buffer_allocated = a.label();
    a.test(A, true);
    a.condition(9, buffer_allocated);
    empty(a, reply);
    a.bind(buffer_allocated);
    a.mov(R14, A);
    a.bind(buffer_ready);
    a.load(DI, BP, REQUEST + 8);
    a.mov(SI, R14);
    a.mov(D, R15);
    syscall(a, 0);
    let read_ok = a.label();
    a.test(A, true);
    a.condition(9, read_ok);
    empty(a, reply);
    a.bind(read_ok);
    a.mov(R8, R14);
    a.mov(R9, A);
    a.jump(reply);

    a.bind(ops[5]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.mov(SI, R14);
    a.mov(D, R15);
    syscall(a, 1);
    empty(a, reply);

    a.bind(ops[6]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.immediate(A, 3);
    a.syscall();
    empty(a, reply);

    a.bind(ops[7]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.immediate(SI, 1030);
    a.immediate(D, 4);
    syscall(a, 72);
    empty(a, reply);

    a.bind(ops[8]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    // A fresh open of /proc/self/fd/N preserves native readonly error behavior,
    // including ENXIO for sockets, rather than silently duplicating write access.
    a.mov(A, DI);
    a.lea(SI, BP, SCRATCH + 96);
    a.immediate(C, 0);
    a.store_width(SI, 0, C, 1);
    a.immediate(B, 10);
    let digit = a.label();
    a.bind(digit);
    a.immediate(D, 0);
    a.divide(B, false);
    a.lea(D, D, b'0' as i64);
    a.lea(SI, SI, -1);
    a.store_width(SI, 0, D, 1);
    a.test(A, true);
    a.condition(5, digit);
    let prefix = b"/proc/self/fd/";
    let mut index = prefix.len();
    while index > 0 {
        index -= 1;
        a.lea(SI, SI, -1);
        a.immediate(A, prefix[index] as u64);
        a.store_width(SI, 0, A, 1);
    }
    a.immediate(DI, (-100i64) as u64);
    a.immediate(D, 0x80000);
    a.immediate(R10, 0);
    syscall(a, 257);
    empty(a, reply);

    a.bind(ops[9]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.immediate(SI, 0);
    a.immediate(D, 0);
    syscall(a, 8);
    empty(a, reply);

    a.bind(ops[10]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.immediate(SI, 0o755);
    syscall(a, 91);
    empty(a, reply);

    a.bind(ops[11]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.immediate(SI, 3);
    a.immediate(D, 0);
    syscall(a, 72);
    let flags = a.label();
    a.test(A, true);
    a.condition(9, flags);
    empty(a, reply);
    a.bind(flags);
    a.immediate(C, (!0x800u64));
    a.rr(0x21, A, C, true);
    a.load(C, BP, REQUEST + 16);
    a.test(C, true);
    let flags_ready = a.label();
    a.condition(4, flags_ready);
    a.immediate(C, 0x800);
    a.rr(0x09, A, C, true);
    a.bind(flags_ready);
    a.mov(D, A);
    a.load(DI, BP, REQUEST + 8);
    a.immediate(SI, 4);
    syscall(a, 72);
    empty(a, reply);
}

fn emit_readiness(a: &mut Assembler, ops: &[Label], reply: Label, invalid: Label, bad: Label) {
    a.bind(ops[12]);
    a.immediate(DI, 0x80000);
    syscall(a, 291);
    empty(a, reply);
    a.bind(ops[13]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.load(D, BP, REQUEST + 16);
    check_fd(a, D, bad);
    a.load(A, BP, REQUEST + 32);
    a.immediate(C, 3);
    a.compare(A, C);
    a.condition(7, invalid);
    a.mov(B, A);
    a.immediate(C, 2);
    a.rr(0x21, B, C, true);
    a.shift(B, 4, 1);
    a.immediate(C, 1);
    a.rr(0x21, A, C, true);
    a.test(A, true);
    let interest_ready = a.label();
    a.condition(4, interest_ready);
    a.immediate(C, 0x2001);
    a.rr(0x09, B, C, true);
    a.bind(interest_ready);
    a.store_width(BP, SCRATCH, B, 4);
    a.load(A, BP, REQUEST + 24);
    a.store(BP, SCRATCH + 4, A);
    a.immediate(SI, 1);
    a.lea(R10, BP, SCRATCH);
    syscall(a, 233);
    a.immediate(C, (-17i64) as u64);
    a.compare(A, C);
    let watched = a.label();
    a.condition(5, watched);
    a.load(DI, BP, REQUEST + 8);
    a.immediate(SI, 3);
    a.load(D, BP, REQUEST + 16);
    a.lea(R10, BP, SCRATCH);
    syscall(a, 233);
    a.bind(watched);
    empty(a, reply);

    a.bind(ops[14]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.load(D, BP, REQUEST + 16);
    check_fd(a, D, bad);
    a.immediate(SI, 2);
    a.immediate(R10, 0);
    syscall(a, 233);
    empty(a, reply);

    a.bind(ops[15]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    let again = a.label();
    let waited = a.label();
    let timeout_ready = a.label();
    a.bind(again);
    a.immediate(DI, 1);
    a.lea(SI, BP, SCRATCH);
    syscall(a, 228);
    a.test(A, true);
    a.condition(8, waited);
    a.load(A, BP, SCRATCH);
    a.immediate(C, 1000000000);
    a.imul(A, C);
    a.load(C, BP, SCRATCH + 8);
    a.rr(0x01, A, C, true);
    a.load(B, BP, REQUEST + 16);
    a.compare(B, A);
    let unexpired = a.label();
    a.condition(7, unexpired);
    a.immediate(A, 0);
    a.store(BP, SCRATCH, A);
    a.store(BP, SCRATCH + 8, A);
    a.jump(timeout_ready);
    a.bind(unexpired);
    a.rr(0x29, B, A, true);
    a.mov(A, B);
    a.immediate(D, 0);
    a.immediate(C, 1000000000);
    a.divide(C, false);
    a.store(BP, SCRATCH, A);
    a.store(BP, SCRATCH + 8, D);
    a.bind(timeout_ready);
    a.load(DI, BP, REQUEST + 8);
    a.lea(SI, BP, EVENTS);
    a.immediate(D, 64);
    a.lea(R10, BP, SCRATCH);
    a.immediate(R8, 0);
    a.immediate(R9, 8);
    a.immediate(A, 441);
    a.syscall();
    a.immediate(C, (-4i64) as u64);
    a.compare(A, C);
    a.condition(4, again);
    a.bind(waited);
    a.test(A, true);
    let events_ready = a.label();
    a.condition(9, events_ready);
    empty(a, reply);
    a.bind(events_ready);
    a.store(BP, SAVED, A);
    a.mov(R10, A);
    a.lea(SI, BP, EVENTS);
    a.lea(DI, BP, EVENT_OUTPUT);
    let mapped = a.label();
    let more = a.label();
    a.bind(more);
    a.test(R10, true);
    a.condition(4, mapped);
    a.load(A, SI, 4);
    a.store(DI, 0, A);
    a.load_width(B, SI, 0, 4, false);
    a.immediate(D, 0);
    for (kernel, public) in vec![(1u64, 1u64), (4u64, 2u64), (0x2010u64, 4u64), (8u64, 8u64)] {
        let absent = a.label();
        a.mov(A, B);
        a.immediate(C, kernel);
        a.rr(0x21, A, C, true);
        a.test(A, true);
        a.condition(4, absent);
        a.immediate(C, public);
        a.rr(0x09, D, C, true);
        a.bind(absent);
    }
    a.store(DI, 8, D);
    a.lea(SI, SI, 12);
    a.lea(DI, DI, 16);
    a.lea(R10, R10, -1);
    a.jump(more);
    a.bind(mapped);
    a.load(A, BP, SAVED);
    a.mov(R9, A);
    a.shift(R9, 4, 4);
    a.lea(R8, BP, EVENT_OUTPUT);
    a.jump(reply);
}

const MESSAGE: i64 = -8400;
const IOV: i64 = -8424;
const CONTROL: i64 = -8464;
const HEADER: i64 = -8544;
fn emit_import(a: &mut Assembler, entry: Label, secret: Label, next: Label) {
    a.bind(entry);
    a.mov(DI, R13);
    a.immediate(SI, 0);
    a.immediate(D, 0);
    a.immediate(R10, 0x80000);
    syscall(a, 288);
    a.test(A, true);
    a.condition(8, next);
    a.store(BP, AUXILIARY, A);
    a.immediate(A, !0u64);
    a.store(BP, IMPORTED, A);
    zero(a, HEADER, 7);
    zero(a, CONTROL, 3);
    a.lea(A, BP, MESSAGE);
    a.store(BP, IOV, A);
    a.immediate(A, 48);
    a.store(BP, IOV + 8, A);
    a.lea(A, BP, IOV);
    a.store(BP, HEADER + 16, A);
    a.immediate(A, 1);
    a.store(BP, HEADER + 24, A);
    a.lea(A, BP, CONTROL);
    a.store(BP, HEADER + 32, A);
    a.immediate(A, 24);
    a.store(BP, HEADER + 40, A);
    a.load(DI, BP, AUXILIARY);
    a.lea(SI, BP, HEADER);
    a.immediate(D, 0x40000000);
    syscall(a, 47);
    a.store(BP, SAVED, A);
    let rejected = a.label();
    let respond = a.label();
    let adopted = a.label();
    a.immediate(C, 48);
    a.compare(A, C);
    a.condition(5, rejected);
    a.load_width(A, BP, HEADER + 48, 4, false);
    a.immediate(C, 0x28); // MSG_TRUNC | MSG_CTRUNC; other reported flags are harmless.
    a.rr(0x21, A, C, true);
    a.test(A, true);
    a.condition(5, rejected);
    a.load(A, BP, CONTROL);
    a.immediate(C, 20);
    a.compare(A, C);
    a.condition(5, rejected);
    a.load_width(A, BP, CONTROL + 8, 4, false);
    a.immediate(C, 1);
    a.compare(A, C);
    a.condition(5, rejected);
    a.load_width(A, BP, CONTROL + 12, 4, false);
    a.compare(A, C);
    a.condition(5, rejected);
    a.load_width(A, BP, CONTROL + 16, 4, true);
    a.store(BP, IMPORTED, A);
    a.load(A, BP, MESSAGE);
    a.compare(A, C);
    a.condition(5, rejected);
    a.load(A, BP, MESSAGE + 8);
    a.test(A, true);
    a.condition(5, rejected);
    a.address(SI, secret);
    for index in 0usize..4usize {
        a.load(A, BP, MESSAGE + 16 + (index * 8) as i64);
        a.load(C, SI, (index * 8) as i64);
        a.compare(A, C);
        a.condition(5, rejected);
    }
    a.load(A, BP, IMPORTED);
    a.jump(adopted);
    a.bind(rejected);
    // Close any received rights even when the authenticated packet is malformed.
    a.load_width(A, BP, CONTROL + 8, 4, false);
    a.immediate(C, 1);
    a.compare(A, C);
    let no_rights = a.label();
    a.condition(5, no_rights);
    a.load_width(A, BP, CONTROL + 12, 4, false);
    a.compare(A, C);
    a.condition(5, no_rights);
    a.load(B, BP, CONTROL);
    a.immediate(C, 20);
    a.compare(B, C);
    a.condition(2, no_rights);
    a.load_width(DI, BP, CONTROL + 16, 4, true);
    a.immediate(A, 3);
    a.syscall();
    a.immediate(C, 24);
    a.compare(B, C);
    a.condition(2, no_rights);
    a.load_width(DI, BP, CONTROL + 20, 4, true);
    a.immediate(A, 3);
    a.syscall();
    a.bind(no_rights);
    a.immediate(A, !0u64);
    a.store(BP, IMPORTED, A);
    a.immediate(A, (-22i64) as u64);
    a.bind(adopted);
    a.store(BP, RESPONSE, A);
    a.load(DI, BP, AUXILIARY);
    a.lea(SI, BP, RESPONSE);
    a.immediate(D, 8);
    syscall(a, 1);
    a.immediate(C, 8);
    a.compare(A, C);
    a.condition(4, respond);
    a.load(DI, BP, IMPORTED);
    a.test(DI, true);
    a.condition(8, respond);
    a.immediate(A, 3);
    a.syscall();
    a.bind(respond);
    a.load(DI, BP, AUXILIARY);
    a.immediate(A, 3);
    a.syscall();
    a.jump(next);
}

pub(super) fn import_image(address: &[u8], nonce: &[u8]) -> Result<Vec<u8>, i64> {
    let mut a = Assembler::new();
    let failed = a.label();
    let result = a.label();
    let closed = a.label();
    let address_data = a.label();
    let request_data = a.label();
    a.mov(BP, SP);
    a.lea(SP, SP, -256);
    // A closed source stream must not turn into the new Unix socket on fd zero.
    a.immediate(DI, 0);
    a.immediate(SI, 1); // F_GETFD
    a.immediate(D, 0);
    syscall(&mut a, 72);
    a.immediate(C, (-9i64) as u64);
    a.compare(A, C);
    a.condition(4, closed);
    a.test(A, true);
    a.condition(8, failed);
    socket(&mut a, 1, 5);
    a.test(A, true);
    a.condition(8, failed);
    a.mov(R12, A);
    a.mov(DI, R12);
    a.address(SI, address_data);
    a.immediate(D, address.len() as u64);
    syscall(&mut a, 42);
    a.test(A, true);
    a.condition(8, failed);
    zero(&mut a, -128, 7);
    a.address(A, request_data);
    a.store(BP, -160, A);
    a.immediate(A, 48);
    a.store(BP, -152, A);
    a.lea(A, BP, -160);
    a.store(BP, -112, A);
    a.immediate(A, 1);
    a.store(BP, -104, A);
    a.lea(A, BP, -192);
    a.store(BP, -96, A);
    a.immediate(A, 24);
    a.store(BP, -88, A);
    a.immediate(A, 20);
    a.store(BP, -192, A);
    a.immediate(A, 1);
    a.store_width(BP, -184, A, 4);
    a.store_width(BP, -180, A, 4);
    a.immediate(A, 0);
    a.store(BP, -176, A); // SCM_RIGHTS passes stdin only; padding is zero.
    a.mov(DI, R12);
    a.lea(SI, BP, -128);
    a.immediate(D, 0x4000);
    syscall(&mut a, 46);
    a.test(A, true);
    a.condition(8, failed);
    a.immediate(C, 48);
    a.compare(A, C);
    let wrong = a.label();
    a.condition(5, wrong);
    a.mov(DI, R12);
    a.lea(SI, BP, -224);
    a.immediate(D, 8);
    syscall(&mut a, 0);
    a.immediate(C, 8);
    a.compare(A, C);
    a.condition(5, wrong);
    a.load(A, BP, -224);
    a.jump(result);
    a.bind(wrong);
    a.immediate(A, (-5i64) as u64);
    a.jump(result);
    a.bind(closed);
    a.immediate(A, 0);
    a.jump(result);
    a.bind(failed);
    // Kernel failures already use negative errno.
    a.bind(result);
    a.store(BP, -224, A);
    a.immediate(DI, 1);
    a.lea(SI, BP, -224);
    a.immediate(D, 8);
    syscall(&mut a, 1);
    a.immediate(DI, 0);
    a.immediate(A, 60);
    a.syscall();
    a.bind(address_data);
    a.raw(address);
    a.bind(request_data);
    a.raw(&[1u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    a.raw(nonce);
    Ok(elf(a.finish().map_err(|_| 8i64)?))
}
