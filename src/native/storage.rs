//! Durable filesystem leaves; each effect requires the borrowed owned handle.
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
fn reject_closed(a: &mut Assembler) {
    let open = a.label();
    a.test(DI, true);
    a.condition(5, open);
    a.immediate(A, (-9i64) as u64);
    epilogue(a);
    a.bind(open);
}
fn path(a: &mut Assembler, invalid: Label) {
    a.test(D, true);
    a.condition(4, invalid);
    a.lea(R9, D, -1);
    a.mov(R8, SI);
    a.rr(0x01, R8, R9, true);
    a.load_width(A, R8, 0, 1, false);
    a.test(A, false);
    a.condition(5, invalid);
    a.mov(R8, SI);
    let scanning = a.label();
    let complete = a.label();
    a.bind(scanning);
    a.test(R9, true);
    a.condition(4, complete);
    a.load_width(A, R8, 0, 1, false);
    a.test(A, false);
    a.condition(4, invalid);
    a.lea(R8, R8, 1);
    a.lea(R9, R9, -1);
    a.jump(scanning);
    a.bind(complete);
}
fn promote(a: &mut Assembler) {
    let done = a.label();
    a.immediate(C, 3);
    a.compare(A, C);
    a.condition(3, done); // Negative errno is also above three unsigned.
    a.mov(R8, A);
    a.mov(DI, A);
    a.immediate(SI, 1030);
    a.immediate(D, 3);
    syscall(a, 72);
    a.mov(R9, A);
    a.mov(DI, R8);
    a.immediate(A, 3);
    a.syscall(); // close consumes ownership, including on EINTR.
    a.mov(A, R9);
    a.bind(done);
}
pub(super) fn emit(a: &mut Assembler, p: Platform) {
    let invalid = a.label();
    a.bind(p.file_storage_open);
    prologue(a, p, 32, Span::default());
    reject_closed(a);
    a.store(BP, -24, C);
    a.store(BP, -16, R8); // Caller selects ordinary 0666 or atomic private 0600.
    a.immediate(A, 0x0a); // RESOLVE_BENEATH | RESOLVE_NO_MAGICLINKS.
    a.store(BP, -8, A);
    path(a, invalid);
    a.lea(D, BP, -24);
    a.immediate(R10, 24);
    syscall(a, 437);
    promote(a);
    epilogue(a);
    a.bind(invalid);
    a.immediate(A, (-22i64) as u64);
    epilogue(a);

    a.bind(p.file_sync);
    prologue(a, p, 0, Span::default());
    reject_closed(a);
    syscall(a, 74);
    epilogue(a);

    a.bind(p.file_truncate);
    prologue(a, p, 0, Span::default());
    reject_closed(a);
    syscall(a, 77); // A usize outside signed off_t returns EINVAL.
    epilogue(a);

    a.bind(p.file_lock_exclusive);
    prologue(a, p, 0, Span::default());
    reject_closed(a);
    a.immediate(SI, 6); // LOCK_EX | LOCK_NB; lifetime follows the open description.
    syscall(a, 73);
    epilogue(a);

    let dot = a.label();
    let synced = a.label();
    a.bind(p.directory_sync);
    prologue(a, p, 32, Span::default());
    reject_closed(a);
    a.immediate(A, 0x90000); // O_DIRECTORY | O_CLOEXEC, readable instead of O_PATH.
    a.store(BP, -24, A);
    a.immediate(A, 0);
    a.store(BP, -16, A);
    a.immediate(A, 0x0a);
    a.store(BP, -8, A);
    a.address(SI, dot);
    a.lea(D, BP, -24);
    a.immediate(R10, 24);
    syscall(a, 437);
    a.test(A, true);
    a.condition(8, synced);
    a.store(BP, -32, A);
    a.mov(DI, A);
    syscall(a, 74);
    a.mov(R9, A);
    a.load(DI, BP, -32);
    a.immediate(A, 3);
    a.syscall();
    a.mov(A, R9);
    a.bind(synced);
    epilogue(a);
    a.bind(dot);
    a.raw(b".\0");
}
