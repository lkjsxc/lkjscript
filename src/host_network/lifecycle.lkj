// Kernel entropy, wall time and SIGTERM state in the independent host process.
use crate::x86::*;
use super::syscall;
use super::broker::{allocate, check_fd, empty, REQUEST, SCRATCH};

const CLAIMED: i64 = -24;
const ORIGINAL: i64 = -32;
const MASK: i64 = -56; // Kept outside the broker reply's -48/-40 scratch words.

pub(super) fn initialize(a: &mut Assembler) {
    a.immediate(A, 0);
    a.store(BP, CLAIMED, A);
    a.store(BP, ORIGINAL, A);
    a.immediate(A, 0x4000);
    a.store(BP, MASK, A);
}
pub(super) fn restore_child_mask(a: &mut Assembler, failed: Label) {
    let done = a.label();
    a.load(A, BP, CLAIMED);
    a.test(A, true);
    a.condition(4, done);
    a.load(A, BP, ORIGINAL);
    a.test(A, true);
    a.setcc(DI, 4); // Unblock only when the pre-claim SIGTERM bit was clear.
    a.lea(SI, BP, MASK);
    a.immediate(D, 0);
    a.immediate(R10, 8);
    syscall(a, 14);
    a.test(A, true);
    a.condition(8, failed);
    a.bind(done);
}
pub(super) fn emit(a: &mut Assembler, labels: &[Label], reply: Label, invalid: Label, bad: Label) {
    a.bind(labels[0]); // getrandom with exact partial-progress reporting.
    a.test(R15, true);
    a.condition(5, invalid);
    a.load(SI, BP, REQUEST + 8);
    a.immediate(C, 9223372036854775799);
    a.compare(SI, C);
    let extent = a.label();
    a.condition(6, extent);
    a.immediate(A, (-75i64) as u64);
    empty(a, reply);
    a.bind(extent);
    a.lea(SI, SI, 8);
    a.mov(R15, SI);
    allocate(a, SI);
    a.test(A, true);
    let allocated = a.label();
    a.condition(9, allocated);
    a.immediate(R15, 0);
    empty(a, reply);
    a.bind(allocated);
    a.mov(R14, A);
    a.immediate(R8, 0);
    let next = a.label();
    let filled = a.label();
    let failed = a.label();
    let returned = a.label();
    a.bind(next);
    a.mov(SI, R15);
    a.lea(SI, SI, -8);
    a.rr(0x29, SI, R8, true);
    a.test(SI, true);
    a.condition(4, filled);
    a.lea(DI, R14, 8);
    a.rr(0x01, DI, R8, true);
    a.immediate(D, 0);
    syscall(a, 318);
    a.test(A, true);
    a.condition(8, failed);
    let progressed = a.label();
    a.condition(5, progressed);
    a.immediate(A, (-5i64) as u64);
    a.jump(failed);
    a.bind(progressed);
    a.rr(0x01, R8, A, true);
    a.jump(next);
    a.bind(filled);
    a.immediate(A, 0);
    a.bind(failed);
    a.negate(A);
    a.store(R14, 0, A);
    a.mov(A, R8);
    a.lea(R9, R8, 8);
    a.mov(R8, R14);
    a.jump(reply);

    a.bind(labels[1]); // CLOCK_REALTIME signed seconds are reply data, not status.
    a.immediate(DI, 0);
    a.lea(SI, BP, SCRATCH);
    syscall(a, 228);
    a.test(A, true);
    a.condition(8, returned);
    a.lea(R8, BP, SCRATCH);
    a.immediate(R9, 8);
    a.load(C, BP, REQUEST + 8);
    a.test(C, true);
    let time_ready = a.label();
    a.condition(4, time_ready);
    a.immediate(R9, 16);
    a.bind(time_ready);
    a.jump(reply);

    a.bind(labels[2]); // fchmod affects the actual open description's inode.
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.immediate(SI, 0o600);
    syscall(a, 91);
    empty(a, reply);

    a.bind(labels[3]); // Claim the process SIGTERM endpoint only after success.
    a.immediate(DI, 0);
    a.lea(SI, BP, MASK);
    a.lea(D, BP, SCRATCH);
    a.immediate(R10, 8);
    syscall(a, 14);
    a.test(A, true);
    a.condition(8, returned);
    a.immediate(DI, !0u64);
    a.lea(SI, BP, MASK);
    a.immediate(D, 8);
    a.immediate(R10, 0x80800);
    syscall(a, 289);
    let restore = a.label();
    a.test(A, true);
    a.condition(8, restore);
    a.load(C, BP, CLAIMED);
    let claimed = a.label();
    a.test(C, true);
    a.condition(5, claimed);
    a.load(D, BP, SCRATCH);
    a.immediate(C, 0x4000);
    a.rr(0x21, D, C, true);
    a.store(BP, ORIGINAL, D);
    a.immediate(C, 1);
    a.store(BP, CLAIMED, C);
    a.bind(claimed);
    empty(a, reply);
    a.bind(restore);
    a.store(BP, SCRATCH + 8, A);
    a.immediate(DI, 2);
    a.lea(SI, BP, SCRATCH);
    a.immediate(D, 0);
    a.immediate(R10, 8);
    syscall(a, 14);
    a.load(A, BP, SCRATCH + 8);
    empty(a, reply);

    a.bind(labels[4]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    a.lea(SI, BP, SCRATCH);
    a.immediate(D, 128);
    syscall(a, 0);
    a.immediate(C, (-11i64) as u64);
    a.compare(A, C);
    let consumed = a.label();
    a.condition(5, consumed);
    a.immediate(A, 0);
    empty(a, reply);
    a.bind(consumed);
    a.test(A, true);
    a.condition(8, returned);
    let corrupt = a.label();
    a.immediate(C, 128);
    a.compare(A, C);
    a.condition(5, corrupt);
    a.load_width(A, BP, SCRATCH, 4, false);
    a.immediate(C, 15);
    a.compare(A, C);
    a.condition(5, corrupt);
    a.immediate(A, 1);
    empty(a, reply);
    a.bind(corrupt);
    a.immediate(A, (-5i64) as u64);
    empty(a, reply);

    a.bind(labels[5]);
    a.load(DI, BP, REQUEST + 8);
    check_fd(a, DI, bad);
    // Validate that the registry's descriptor remains live before exposing PID.
    a.immediate(SI, 1);
    syscall(a, 72);
    a.test(A, true);
    a.condition(8, returned);
    syscall(a, 39);
    a.bind(returned);
    empty(a, reply);
}
