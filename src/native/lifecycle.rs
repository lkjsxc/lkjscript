//! Kernel entropy, wall time and owned graceful-stop effects.
use super::platform::Platform;
use super::{epilogue, prologue};
use crate::diagnostic::Span;
use crate::x86::*;

fn syscall(a: &mut Assembler, number: u64) {
    let again = a.label();
    a.bind(again);
    a.immediate(A, number);
    a.syscall();
    a.immediate(C, (-4i64) as u64);
    a.compare(A, C);
    a.condition(4, again);
}
pub(super) fn entropy(a: &mut Assembler, p: Platform) {
    a.bind(p.entropy_fill);
    prologue(a, p, 0, Span::default());
    let next = a.label();
    let done = a.label();
    let failed = a.label();
    a.bind(next);
    a.test(SI, true);
    a.condition(4, done);
    a.immediate(D, 0);
    syscall(a, 318); // getrandom, without a userspace entropy source.
    a.test(A, true);
    a.condition(8, failed);
    let progressed = a.label();
    a.condition(5, progressed);
    a.immediate(A, (-5i64) as u64);
    a.jump(failed);
    a.bind(progressed);
    a.rr(0x01, DI, A, true);
    a.rr(0x29, SI, A, true);
    a.jump(next);
    a.bind(done);
    a.immediate(A, 0);
    a.bind(failed);
    epilogue(a);
}
pub(super) fn wall_time(a: &mut Assembler, p: Platform) {
    a.bind(p.clock_unix_seconds);
    prologue(a, p, 16, Span::default());
    a.immediate(DI, 0); // CLOCK_REALTIME; seconds retain their signed representation.
    a.lea(SI, BP, -16);
    syscall(a, 228);
    a.load(D, BP, -16);
    epilogue(a);
}
pub(super) fn wall_nanoseconds(a: &mut Assembler, p: Platform) {
    a.bind(p.clock_unix_nanoseconds);
    prologue(a, p, 16, Span::default());
    a.immediate(DI, 0);
    a.lea(SI, BP, -16);
    syscall(a, 228);
    let done = a.label();
    let overflow = a.label();
    a.test(A, true);
    a.condition(8, done);
    a.load(A, BP, -16);
    a.test(A, true);
    a.condition(8, overflow);
    a.immediate(C, 1_000_000_000);
    a.mul(C);
    a.test(D, true);
    a.condition(5, overflow);
    a.load(C, BP, -8);
    a.rr(0x01, A, C, true);
    a.condition(2, overflow);
    a.mov(D, A);
    a.immediate(A, 0);
    a.jump(done);
    a.bind(overflow);
    a.immediate(A, (-75i64) as u64);
    a.bind(done);
    epilogue(a);
}
pub(super) fn private_file(a: &mut Assembler, p: Platform) {
    a.bind(p.file_set_private);
    prologue(a, p, 0, Span::default());
    let live = a.label();
    a.test(DI, true);
    a.condition(5, live);
    a.immediate(A, (-9i64) as u64);
    epilogue(a);
    a.bind(live);
    a.immediate(SI, 0o600);
    syscall(a, 91);
    epilogue(a);
}
pub(super) fn child_stop(a: &mut Assembler, p: Platform) {
    a.bind(p.child_request_stop);
    a.load(DI, DI, 0);
    let missing = a.label();
    a.test(DI, true);
    a.condition(4, missing);
    a.immediate(SI, 15);
    syscall(a, 62);
    a.ret();
    a.bind(missing);
    a.immediate(A, (-10i64) as u64);
    a.ret();
}
pub(super) fn stop(a: &mut Assembler, p: Platform) {
    a.bind(p.world_stop);
    prologue(a, p, 32, Span::default());
    a.immediate(A, 0x4000);
    a.store(BP, -8, A);
    a.immediate(DI, 0); // Block SIGTERM, retaining the previous complete mask.
    a.lea(SI, BP, -8);
    a.lea(D, BP, -16);
    a.immediate(R10, 8);
    syscall(a, 14);
    let failed = a.label();
    let restore = a.label();
    let acquired = a.label();
    a.test(A, true);
    a.condition(8, failed);
    a.immediate(DI, !0u64);
    a.lea(SI, BP, -8);
    a.immediate(D, 8);
    a.immediate(R10, 0x80800); // SFD_NONBLOCK | SFD_CLOEXEC.
    syscall(a, 289);
    a.test(A, true);
    a.condition(8, restore);
    a.immediate(C, 3);
    a.compare(A, C);
    a.condition(3, acquired);
    a.store(BP, -32, A);
    a.mov(DI, A);
    a.immediate(SI, 1030);
    a.immediate(D, 3);
    syscall(a, 72);
    a.store(BP, -24, A);
    a.load(DI, BP, -32);
    a.immediate(A, 3);
    a.syscall();
    a.load(A, BP, -24);
    a.test(A, true);
    a.condition(8, restore);
    a.bind(acquired);
    a.address(R8, p.stop_state);
    a.load(C, R8, 0);
    let claimed = a.label();
    a.test(C, true);
    a.condition(5, claimed);
    a.load(D, BP, -16);
    a.immediate(C, 0x4000);
    a.rr(0x21, D, C, true);
    a.store(R8, 8, D);
    a.immediate(C, 1);
    a.store(R8, 0, C); // Claim only after the signalfd is owned successfully.
    a.bind(claimed);
    epilogue(a);
    a.bind(restore);
    a.store(BP, -24, A);
    a.immediate(DI, 2); // A failed acquisition has no mask effect or claim.
    a.lea(SI, BP, -16);
    a.immediate(D, 0);
    a.immediate(R10, 8);
    syscall(a, 14);
    a.load(A, BP, -24);
    a.bind(failed);
    epilogue(a);

    a.bind(p.stop_requested);
    prologue(a, p, 144, Span::default());
    a.store(BP, -8, DI);
    a.load(A, DI, 8);
    let requested = a.label();
    let returned = a.label();
    let corrupt = a.label();
    a.test(A, true);
    a.condition(5, requested);
    a.load(DI, DI, 0);
    a.lea(SI, BP, -144);
    a.immediate(D, 128);
    syscall(a, 0);
    a.immediate(C, (-11i64) as u64);
    a.compare(A, C);
    let read = a.label();
    a.condition(5, read);
    a.immediate(A, 0);
    a.jump(returned);
    a.bind(read);
    a.test(A, true);
    a.condition(8, returned);
    a.immediate(C, 128);
    a.compare(A, C);
    a.condition(5, corrupt);
    a.load_width(A, BP, -144, 4, false);
    a.immediate(C, 15);
    a.compare(A, C);
    a.condition(5, corrupt);
    a.load(DI, BP, -8);
    a.immediate(A, 1);
    a.store(DI, 8, A);
    a.bind(requested);
    a.immediate(A, 1);
    a.jump(returned);
    a.bind(corrupt);
    a.immediate(A, (-5i64) as u64);
    a.bind(returned);
    epilogue(a);

    a.bind(p.stop_target);
    syscall(a, 39);
    a.ret();
}
pub(super) fn restore_child_mask(a: &mut Assembler, p: Platform, failed: Label) {
    let done = a.label();
    a.address(R8, p.stop_state);
    a.load(A, R8, 0);
    a.test(A, true);
    a.condition(4, done);
    a.load(A, R8, 8);
    a.test(A, true);
    a.setcc(DI, 4); // SIG_UNBLOCK when the original bit was zero; SIG_BLOCK otherwise.
    a.lea(SI, R8, 16);
    a.immediate(D, 0);
    a.immediate(R10, 8);
    syscall(a, 14);
    a.test(A, true);
    a.condition(8, failed);
    a.bind(done);
}
pub(super) fn data(a: &mut Assembler, p: Platform, bytes: &mut Vec<u8>, offset: usize) {
    a.data_label(p.stop_state, offset + bytes.len());
    bytes.extend_from_slice(&[0u8; 16]); // Claimed flag and original SIGTERM bit.
    bytes.extend_from_slice(&[0u8, 64u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8]);
}
