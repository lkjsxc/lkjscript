//! Named environment access over the process's immutable startup environment.
use super::platform::Platform;
use crate::x86::*;

fn length(a: &mut Assembler) {
    a.mov(R10, SI);
    a.immediate(A, 0);
    let next = a.label();
    let done = a.label();
    a.bind(next);
    a.load_width(C, R10, 0, 1, false);
    a.test(C, false);
    a.condition(4, done);
    a.lea(A, A, 1);
    a.lea(R10, R10, 1);
    a.jump(next);
    a.bind(done);
}

pub(super) fn emit(a: &mut Assembler, p: Platform) {
    // DI names exactly one variable, including its NUL terminator. Return the
    // startup value pointer or zero; keep SI/D/R11 for the copy operation.
    a.bind(p.environment_lookup);
    a.load(R8, CONTEXT, 0); // INITIAL_STACK
    a.load(A, R8, 0);
    a.lea(A, A, 2); // argc + argv's terminator
    a.shift(A, 4, 3);
    a.rr(0x01, R8, A, true);
    let row = a.label();
    let next = a.label();
    let compare = a.label();
    let key_end = a.label();
    let absent = a.label();
    a.bind(row);
    a.load(R9, R8, 0);
    a.test(R9, true);
    a.condition(4, absent);
    a.mov(R10, DI);
    a.bind(compare);
    a.load_width(A, R10, 0, 1, false);
    a.test(A, false);
    a.condition(4, key_end);
    a.load_width(C, R9, 0, 1, false);
    a.compare(A, C);
    a.condition(5, next);
    a.lea(R10, R10, 1);
    a.lea(R9, R9, 1);
    a.jump(compare);
    a.bind(key_end);
    a.load_width(C, R9, 0, 1, false);
    a.immediate(A, b'=' as u64);
    a.compare(A, C);
    a.condition(5, next);
    a.lea(A, R9, 1);
    a.ret();
    a.bind(next);
    a.lea(R8, R8, 8);
    a.jump(row);
    a.bind(absent);
    a.immediate(A, 0);
    a.ret();

    a.bind(p.environment_length);
    a.mov(A, DI); // Entry resolved the granted name; zero represents absence.
    a.test(A, true);
    let missing = a.label();
    a.condition(4, missing);
    a.mov(SI, A);
    length(a);
    a.ret();
    a.bind(missing);
    a.immediate(A, !0u64);
    a.ret();

    a.bind(p.environment_copy);
    a.mov(R11, SI);
    a.mov(A, DI);
    a.test(A, true);
    let copied = a.label();
    a.condition(4, copied);
    a.mov(SI, A);
    length(a);
    a.compare(A, D);
    a.cmov(A, D, 7);
    a.mov(C, A);
    a.mov(DI, R11);
    a.raw(&[0xfc, 0xf3, 0xa4]);
    a.bind(copied);
    a.ret();
}
