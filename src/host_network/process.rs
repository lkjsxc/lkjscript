//! Process effects for the independent interpreter's descriptor broker.
//! This module selects only Linux syscalls; it has no IR or backend dependency.
use crate::x86::*;

pub struct ProcessCode {
    pub spawn: Label,
    pub wait: Label,
    pub terminate: Label,
    pub drop_child: Label,
    pub shutdown: Label,
}
impl ProcessCode {
    pub fn new(a: &mut Assembler) -> Self {
        Self {
            spawn: a.label(), wait: a.label(), terminate: a.label(),
            drop_child: a.label(), shutdown: a.label(),
        }
    }
}
const HEAD: i64 = -16;
const REQUEST: i64 = -128;
const ARGV: i64 = -256;
const ARGV_BYTES: i64 = -264;
const RECORD: i64 = -272;
const PIPE_READ: i64 = -280;
const PIPE_WRITE: i64 = -288;
const EXECUTABLE: i64 = -296;
const CHILD_PID: i64 = -336;
const RETURN_VALUE: i64 = -344;
const WAIT_STATUS: i64 = -352;
const PARENT_PID: i64 = -360;
const LINK: i64 = -368;
const PIPE_PAIR: i64 = -384;
const RECEIVED: i64 = -392;

fn syscall(a: &mut Assembler, number: u64) {
    let again = a.label();
    a.bind(again);
    a.immediate(A, number);
    a.syscall();
    a.immediate(C, (-4i64) as u64);
    a.compare(A, C);
    a.condition(4, again);
}
fn reply_empty(a: &mut Assembler, reply: Label) {
    a.immediate(R8, 0);
    a.immediate(R9, 0);
    a.jump(reply);
}
fn allocate(a: &mut Assembler) {
    // SI is the requested size. Linux rounds the mapping to whole pages.
    a.immediate(DI, 0);
    a.immediate(D, 3);
    a.immediate(R10, 0x22);
    a.immediate(R8, !0u64);
    a.immediate(R9, 0);
    syscall(a, 9);
}
fn locate(a: &mut Assembler, missing: Label) {
    // Retain both the owned record and its incoming link for removal after wait.
    a.lea(DI, BP, HEAD);
    a.load(C, DI, 0);
    a.load(R8, BP, REQUEST + 8);
    let next = a.label();
    let found = a.label();
    a.bind(next);
    a.test(C, true);
    a.condition(4, missing);
    a.load(A, C, 8);
    a.compare(A, R8);
    a.condition(4, found);
    a.mov(DI, C);
    a.load(C, C, 0);
    a.jump(next);
    a.bind(found);
    a.store(BP, RECORD, C);
    a.store(BP, LINK, DI);
}
fn remove_record(a: &mut Assembler) {
    a.load(DI, BP, RECORD);
    a.load(A, DI, 0);
    a.load(SI, BP, LINK);
    a.store(SI, 0, A);
    a.immediate(SI, 16);
    syscall(a, 11);
}
fn kill_and_wait(a: &mut Assembler) {
    a.load(C, BP, RECORD);
    a.load(DI, C, 8);
    a.immediate(SI, 9);
    syscall(a, 62);
    a.load(C, BP, RECORD);
    a.load(DI, C, 8);
    a.immediate(SI, 0);
    a.immediate(D, 0);
    a.immediate(R10, 0);
    syscall(a, 61);
}

pub fn emit(a: &mut Assembler, p: &ProcessCode, reply: Label) {
    emit_spawn(a, p.spawn, reply);
    let missing = a.label();

    a.bind(p.wait);
    locate(a, missing);
    a.load(DI, C, 8);
    a.lea(SI, BP, WAIT_STATUS);
    a.load(D, BP, REQUEST + 16);
    a.test(D, true);
    a.setcc(D, 5);
    a.immediate(R10, 0);
    syscall(a, 61);
    let waiting_done = a.label();
    a.test(A, true);
    a.condition(14, waiting_done);
    a.load_width(A, BP, WAIT_STATUS, 4, false);
    a.lea(A, A, 1);
    a.store(BP, RETURN_VALUE, A);
    remove_record(a);
    a.load(A, BP, RETURN_VALUE);
    a.bind(waiting_done);
    reply_empty(a, reply);

    a.bind(p.terminate);
    locate(a, missing);
    a.load(DI, C, 8);
    a.immediate(SI, 9);
    syscall(a, 62);
    reply_empty(a, reply);

    a.bind(p.drop_child);
    locate(a, missing);
    kill_and_wait(a);
    remove_record(a);
    a.immediate(A, 0);
    reply_empty(a, reply);

    a.bind(missing);
    a.immediate(A, (-10i64) as u64);
    reply_empty(a, reply);

    a.bind(p.shutdown);
    let next = a.label();
    let done = a.label();
    a.bind(next);
    a.load(C, BP, HEAD);
    a.test(C, true);
    a.condition(4, done);
    a.store(BP, RECORD, C);
    a.lea(A, BP, HEAD);
    a.store(BP, LINK, A);
    kill_and_wait(a);
    remove_record(a);
    a.jump(next);
    a.bind(done);
    a.ret();
}

fn emit_spawn(a: &mut Assembler, entry: Label, reply: Label) {
    let invalid = a.label();
    let bad_executable = a.label();
    let overflow = a.label();
    let failed = a.label();
    let cleanup = a.label();
    let child = a.label();
    let child_failed = a.label();
    let reap_failed_child = a.label();
    let launched = a.label();
    let empty = a.label();
    let default_signal = a.label();
    a.bind(entry);
    a.immediate(A, 0);
    for offset in vec![ARGV, RECORD, ARGV_BYTES, RECEIVED] {
        a.store(BP, offset, A);
    }
    a.immediate(A, !0u64);
    for offset in vec![PIPE_READ, PIPE_WRITE, EXECUTABLE, -304, -312, -320, -328] {
        a.store(BP, offset, A);
    }
    a.test(R15, true);
    a.condition(4, invalid);
    a.mov(SI, R14);
    a.rr(0x01, SI, R15, true);
    a.load_width(A, SI, -1, 1, false);
    a.test(A, true);
    a.condition(5, invalid);
    // Count exactly the supplied NUL-separated arguments, including argv[0].
    a.mov(SI, R14);
    a.mov(D, R15);
    a.immediate(R8, 0);
    let count = a.label();
    let counted = a.label();
    a.bind(count);
    a.test(D, true);
    a.condition(4, counted);
    a.load_width(A, SI, 0, 1, false);
    a.test(A, true);
    let nonzero = a.label();
    a.condition(5, nonzero);
    a.lea(R8, R8, 1);
    a.bind(nonzero);
    a.lea(SI, SI, 1);
    a.lea(D, D, -1);
    a.jump(count);
    a.bind(counted);
    a.immediate(A, (!0u64) / 8 - 1);
    a.compare(R8, A);
    a.condition(7, overflow);
    a.lea(SI, R8, 1);
    a.shift(SI, 4, 3);
    a.store(BP, ARGV_BYTES, SI);
    allocate(a);
    a.test(A, true);
    a.condition(8, failed);
    a.store(BP, ARGV, A);
    a.mov(R8, A);
    a.mov(SI, R14);
    a.mov(D, R15);
    a.store(R8, 0, SI);
    a.lea(R8, R8, 8);
    let pointer = a.label();
    let pointed = a.label();
    a.bind(pointer);
    a.load_width(A, SI, 0, 1, false);
    a.lea(SI, SI, 1);
    a.lea(D, D, -1);
    a.test(D, true);
    a.condition(4, pointed);
    a.test(A, true);
    a.condition(5, pointer);
    a.store(R8, 0, SI);
    a.lea(R8, R8, 8);
    a.jump(pointer);
    a.bind(pointed);
    a.immediate(A, 0);
    a.store(R8, 0, A);

    // Allocate ownership before fork, so refusal cannot create an unowned child.
    a.immediate(SI, 16);
    allocate(a);
    a.test(A, true);
    a.condition(8, failed);
    a.store(BP, RECORD, A);
    a.lea(DI, BP, PIPE_PAIR);
    a.immediate(SI, 0x80000);
    syscall(a, 293);
    a.test(A, true);
    a.condition(8, failed);
    a.load_width(A, BP, PIPE_PAIR, 4, false);
    a.store(BP, PIPE_READ, A);
    a.load_width(A, BP, PIPE_PAIR + 4, 4, false);
    a.store(BP, PIPE_WRITE, A);
    // All broker standard streams are open; pipe ends cannot occupy 0..3.
    // Duplicate application descriptors before dup2, including repeated aliases.
    for index in 0usize..5usize {
        let duplicated = a.label();
        a.load(DI, BP, REQUEST + 8 + index as i64 * 8);
        a.test(DI, true);
        if index == 0 { a.condition(4, bad_executable); }
        else { a.condition(4, duplicated); }
        a.immediate(SI, 1030);
        a.immediate(D, 4);
        syscall(a, 72);
        a.test(A, true);
        a.condition(8, failed);
        a.store(BP, EXECUTABLE - index as i64 * 8, A);
        a.bind(duplicated);
    }
    syscall(a, 39);
    a.store(BP, PARENT_PID, A);
    syscall(a, 57);
    a.test(A, true);
    a.condition(8, failed);
    a.condition(4, child);
    a.store(BP, CHILD_PID, A);
    a.load(DI, BP, PIPE_WRITE);
    a.immediate(A, 3);
    a.syscall();
    a.immediate(A, !0u64);
    a.store(BP, PIPE_WRITE, A);
    let status_read = a.label();
    let status_eof = a.label();
    a.bind(status_read);
    a.load(R8, BP, RECEIVED);
    a.load(DI, BP, PIPE_READ);
    a.lea(SI, BP, WAIT_STATUS);
    a.rr(0x01, SI, R8, true);
    a.immediate(D, 8);
    a.rr(0x29, D, R8, true);
    syscall(a, 0);
    a.test(A, true);
    a.condition(8, reap_failed_child);
    a.condition(4, status_eof);
    a.load(R8, BP, RECEIVED);
    a.rr(0x01, R8, A, true);
    a.store(BP, RECEIVED, R8);
    a.immediate(A, 8);
    a.compare(R8, A);
    a.condition(5, status_read);
    a.load(A, BP, WAIT_STATUS);
    a.jump(reap_failed_child);
    a.bind(status_eof);
    a.load(A, BP, RECEIVED);
    a.test(A, true);
    a.condition(4, launched);
    a.immediate(A, (-5i64) as u64);
    a.bind(reap_failed_child);
    a.store(BP, RETURN_VALUE, A);
    a.load(DI, BP, CHILD_PID);
    a.immediate(SI, 9);
    syscall(a, 62);
    a.load(DI, BP, CHILD_PID);
    a.immediate(SI, 0);
    a.immediate(D, 0);
    a.immediate(R10, 0);
    syscall(a, 61);
    a.jump(cleanup);

    a.bind(launched);
    a.load(C, BP, RECORD);
    a.load(A, BP, HEAD);
    a.store(C, 0, A);
    a.load(A, BP, CHILD_PID);
    a.store(C, 8, A);
    a.store(BP, HEAD, C);
    a.store(BP, RETURN_VALUE, A);
    a.immediate(A, 0);
    a.store(BP, RECORD, A);
    a.jump(cleanup);

    a.bind(child);
    a.immediate(DI, 1); // PR_SET_PDEATHSIG; never orphan a program on host death.
    a.immediate(SI, 9);
    a.immediate(D, 0);
    a.immediate(R10, 0);
    a.immediate(R8, 0);
    syscall(a, 157);
    a.test(A, true);
    a.condition(8, child_failed);
    syscall(a, 110);
    a.load(C, BP, PARENT_PID);
    a.compare(A, C);
    let parent_alive = a.label();
    a.condition(4, parent_alive);
    a.immediate(A, (-10i64) as u64);
    a.jump(child_failed);
    a.bind(parent_alive);
    a.load(DI, BP, PIPE_READ);
    a.immediate(A, 3);
    a.syscall();
    for index in 0usize..4usize {
        let closed = a.label();
        let assigned = a.label();
        a.load(DI, BP, EXECUTABLE - 8 - index as i64 * 8);
        a.test(DI, true);
        a.condition(8, closed);
        a.immediate(SI, index as u64);
        syscall(a, 33);
        a.test(A, true);
        a.condition(8, child_failed);
        a.jump(assigned);
        a.bind(closed);
        a.immediate(DI, index as u64);
        a.immediate(A, 3);
        a.syscall();
        a.bind(assigned);
    }
    // This closes every broker/imported capability at successful exec, leaving
    // only the four explicit stdio/result grants after dup2 cleared CLOEXEC.
    a.immediate(DI, 4);
    a.immediate(SI, 0xffffffff);
    a.immediate(D, 4); // CLOSE_RANGE_CLOEXEC.
    syscall(a, 436);
    a.test(A, true);
    a.condition(8, child_failed);
    a.immediate(DI, 13); // Do not inherit the broker's ignored SIGPIPE.
    a.address(SI, default_signal);
    a.immediate(D, 0);
    a.immediate(R10, 8);
    syscall(a, 13);
    a.test(A, true);
    a.condition(8, child_failed);
    a.load(DI, BP, EXECUTABLE);
    a.address(SI, empty);
    a.load(D, BP, ARGV);
    a.immediate(R10, 0);
    a.immediate(R8, 0x1000);
    syscall(a, 322);
    a.bind(child_failed);
    a.store(BP, WAIT_STATUS, A);
    a.load(DI, BP, PIPE_WRITE);
    a.lea(SI, BP, WAIT_STATUS);
    a.immediate(D, 8);
    let write_status = a.label();
    let exit_child = a.label();
    a.bind(write_status);
    syscall(a, 1);
    a.test(A, true);
    a.condition(14, exit_child);
    a.rr(0x01, SI, A, true);
    a.rr(0x29, D, A, true);
    a.test(D, true);
    a.condition(5, write_status);
    a.bind(exit_child);
    a.immediate(DI, 127);
    a.immediate(A, 60);
    a.syscall();

    a.bind(invalid);
    a.immediate(A, (-22i64) as u64);
    a.jump(failed);
    a.bind(bad_executable);
    a.immediate(A, (-9i64) as u64);
    a.jump(failed);
    a.bind(overflow);
    a.immediate(A, (-75i64) as u64);
    a.bind(failed);
    a.store(BP, RETURN_VALUE, A);
    a.bind(cleanup);
    for offset in vec![PIPE_READ, PIPE_WRITE, EXECUTABLE, -304, -312, -320, -328] {
        a.load(DI, BP, offset);
        a.immediate(A, 3);
        a.syscall();
    }
    let argv_done = a.label();
    a.load(DI, BP, ARGV);
    a.test(DI, true);
    a.condition(4, argv_done);
    a.load(SI, BP, ARGV_BYTES);
    syscall(a, 11);
    a.bind(argv_done);
    let record_done = a.label();
    a.load(DI, BP, RECORD);
    a.test(DI, true);
    a.condition(4, record_done);
    a.immediate(SI, 16);
    syscall(a, 11);
    a.bind(record_done);
    a.load(A, BP, RETURN_VALUE);
    reply_empty(a, reply);
    a.bind(empty);
    a.raw(&[0u8]);
    a.bind(default_signal);
    a.raw(&[0u8; 32]);
}
