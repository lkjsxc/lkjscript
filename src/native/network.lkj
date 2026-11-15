//! Linux IPv4 listener leaves. A returned descriptor is owned and at least three.
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

pub(super) fn emit(a: &mut Assembler, p: Platform) {
    let promote = a.label();
    let listen_invalid = a.label();
    let listen_failed = a.label();
    let listen_done = a.label();
    a.bind(p.world_listen);
    prologue(a, p, if p.listener_reuse { 32 } else { 16 }, Span::default());
    a.immediate(A, 16);
    a.compare(SI, A);
    a.condition(5, listen_invalid);
    a.load_width(A, DI, 0, 2, false);
    a.immediate(C, 2); // AF_INET
    a.compare(A, C);
    a.condition(5, listen_invalid);
    a.store(BP, -8, DI);
    a.immediate(DI, 2);
    a.immediate(SI, 0x80001); // SOCK_STREAM | SOCK_CLOEXEC
    a.immediate(D, 0);
    syscall(a, 41); // socket
    a.call(promote);
    a.test(A, true);
    a.condition(8, listen_done);
    a.store(BP, -16, A);
    if p.listener_reuse {
        // Rebinding a stopped service must tolerate its own accepted TIME_WAIT
        // sockets. SO_REUSEADDR does not permit two active listeners on the port.
        a.immediate(A, 1);
        a.store_width(BP, -24, A, 4);
        a.load(DI, BP, -16);
        a.immediate(SI, 1); // SOL_SOCKET
        a.immediate(D, 2); // SO_REUSEADDR
        a.lea(R10, BP, -24);
        a.immediate(R8, 4);
        syscall(a, 54);
        a.test(A, true);
        a.condition(8, listen_failed);
        a.load(DI, BP, -16);
    } else { a.mov(DI, A); }
    a.load(SI, BP, -8);
    a.immediate(D, 16);
    syscall(a, 49); // bind
    a.test(A, true);
    a.condition(8, listen_failed);
    a.load(DI, BP, -16);
    a.immediate(SI, 2147483647); // The kernel clamps its own backlog limit.
    syscall(a, 50); // listen
    a.test(A, true);
    a.condition(8, listen_failed);
    a.load(A, BP, -16);
    a.jump(listen_done);
    a.bind(listen_failed);
    a.mov(R9, A);
    a.load(DI, BP, -16);
    a.immediate(A, 3);
    a.syscall(); // close consumes the descriptor even when interrupted.
    a.mov(A, R9);
    a.jump(listen_done);
    a.bind(listen_invalid);
    a.immediate(A, (-22i64) as u64);
    a.bind(listen_done);
    epilogue(a);

    a.bind(p.listener_accept);
    prologue(a, p, 0, Span::default());
    a.immediate(SI, 0);
    a.immediate(D, 0);
    a.immediate(R10, 0x80000); // SOCK_CLOEXEC
    syscall(a, 288); // accept4
    a.call(promote);
    epilogue(a);

    let address_invalid = a.label();
    let address_done = a.label();
    a.bind(p.listener_address);
    prologue(a, p, 16, Span::default());
    a.immediate(A, 16);
    a.compare(D, A);
    a.condition(5, address_invalid);
    a.store_width(BP, -8, A, 4);
    a.lea(D, BP, -8);
    syscall(a, 51); // getsockname
    a.test(A, true);
    a.condition(8, address_done);
    a.load_width(A, BP, -8, 4, false);
    a.immediate(C, 16);
    a.compare(A, C);
    a.condition(5, address_invalid);
    a.immediate(A, 0);
    a.jump(address_done);
    a.bind(address_invalid);
    a.immediate(A, (-22i64) as u64);
    a.bind(address_done);
    epilogue(a);

    // Promotion preserves closed stdio and also passes negative errno unchanged.
    a.bind(promote);
    let promoted = a.label();
    a.immediate(C, 3);
    a.compare(A, C);
    a.condition(3, promoted);
    a.mov(R8, A);
    a.mov(DI, A);
    a.immediate(SI, 1030); // F_DUPFD_CLOEXEC
    a.immediate(D, 3);
    syscall(a, 72);
    a.mov(R9, A);
    a.mov(DI, R8);
    a.immediate(A, 3);
    a.syscall();
    a.mov(A, R9);
    a.bind(promoted);
    a.ret();
}

// These leaves are appended only for programs containing the Connector primitive.
// Compiler source itself uses the seed-compatible independent host relay, so old
// seed emission remains byte-identical for the compiler fixed point.
pub(super) fn emit_connector(a: &mut Assembler, p: Platform) {
    let invalid = a.label();
    let packed = a.label();
    a.bind(p.world_connector);
    prologue(a, p, 0, Span::default());
    a.immediate(A, 16);
    a.compare(SI, A);
    a.condition(5, invalid);
    a.load_width(A, DI, 0, 2, false);
    a.immediate(C, 2);
    a.compare(A, C);
    a.condition(5, invalid);
    a.load(A, DI, 8);
    a.test(A, true);
    a.condition(5, invalid);
    a.load(A, DI, 0);
    a.shift(A, 5, 16); // The endpoint occupies 48 bits; never a signed errno.
    a.jump(packed);
    a.bind(invalid);
    a.immediate(A, (-22i64) as u64);
    a.bind(packed);
    epilogue(a);

    let failed = a.label();
    let done = a.label();
    let connected = a.label();
    let promoted = a.label();
    a.bind(p.connector_connect);
    prologue(a, p, 32, Span::default());
    a.mov(A, DI);
    a.shift(A, 4, 16);
    a.immediate(C, 2);
    a.rr(0x09, A, C, true);
    a.store(BP, -32, A);
    a.immediate(A, 0);
    a.store(BP, -24, A);
    a.immediate(DI, 2);
    a.immediate(SI, 0x80001);
    a.immediate(D, 0);
    syscall(a, 41);
    a.test(A, true);
    a.condition(8, done);
    a.immediate(C, 3);
    a.compare(A, C);
    a.condition(3, promoted);
    a.store(BP, -8, A);
    a.mov(DI, A);
    a.immediate(SI, 1030);
    a.immediate(D, 3);
    syscall(a, 72);
    a.store(BP, -16, A);
    a.load(DI, BP, -8);
    a.immediate(A, 3);
    a.syscall();
    a.load(A, BP, -16);
    a.test(A, true);
    a.condition(8, done);
    a.bind(promoted);
    a.store(BP, -8, A);
    a.mov(DI, A);
    a.lea(SI, BP, -32);
    a.immediate(D, 16);
    syscall(a, 42);
    a.immediate(C, (-106i64) as u64); // EISCONN after an interrupted connect.
    a.compare(A, C);
    a.condition(4, connected);
    a.test(A, true);
    a.condition(8, failed);
    a.bind(connected);
    a.load(A, BP, -8);
    a.jump(done);
    a.bind(failed);
    a.store(BP, -16, A);
    a.load(DI, BP, -8);
    a.immediate(A, 3);
    a.syscall();
    a.load(A, BP, -16);
    a.bind(done);
    epilogue(a);
}

// The ordinary listener leaf stays unchanged for stored-seed reproduction.
pub(super) fn emit_accept_nodelay(a: &mut Assembler, p: Platform) {
    let done = a.label();
    let failed = a.label();
    a.bind(p.listener_accept_nodelay);
    prologue(a, p, 32, Span::default());
    a.call(p.listener_accept);
    a.test(A, true);
    a.condition(8, done);
    a.store(BP, -8, A);
    a.mov(DI, A);
    a.immediate(A, 1);
    a.store_width(BP, -16, A, 4);
    a.immediate(SI, 6); // IPPROTO_TCP
    a.immediate(D, 1); // TCP_NODELAY
    a.lea(R10, BP, -16);
    a.immediate(R8, 4);
    syscall(a, 54); // setsockopt
    a.test(A, true);
    a.condition(8, failed);
    a.load(A, BP, -8);
    a.jump(done);
    a.bind(failed);
    a.store(BP, -24, A);
    a.load(DI, BP, -8);
    a.immediate(A, 3);
    a.syscall(); // close consumes the newly accepted descriptor.
    a.load(A, BP, -24);
    a.bind(done);
    epilogue(a);
}
