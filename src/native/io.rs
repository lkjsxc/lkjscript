//! Linux leaves for World-derived authority. Buffer ownership and syscall result
//! interpretation stay in IR and the source library; these leaves do not allocate.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct Io {
    argument_count: Label,
    argument_length: Label,
    argument_copy: Label,
    stdin_read: Label,
    stderr_write: Label,
    set_exit_status: Label,
    directory_open: Label,
    file_open: Label,
    file_read: Label,
    file_write: Label,
    handle_close: Label,
    open_how: Label,
    open_result: Label,
}
impl Io {
    pub(super) fn new(a: &mut Assembler) -> Self {
        Self {
            argument_count: a.label(),
            argument_length: a.label(),
            argument_copy: a.label(),
            stdin_read: a.label(),
            stderr_write: a.label(),
            set_exit_status: a.label(),
            directory_open: a.label(),
            file_open: a.label(),
            file_read: a.label(),
            file_write: a.label(),
            handle_close: a.label(),
            open_how: a.label(),
            open_result: a.label(),
        }
    }
    pub(super) fn import(self, host: HostFunction, stdout: Label) -> (Label, usize) {
        match host {
            HostFunction::StdoutWrite => (stdout, 3),
            HostFunction::ArgumentCount => (self.argument_count, 0),
            HostFunction::ArgumentLength => (self.argument_length, 1),
            HostFunction::ArgumentCopy => (self.argument_copy, 3),
            HostFunction::StdinRead => (self.stdin_read, 2),
            HostFunction::StderrWrite => (self.stderr_write, 3),
            HostFunction::SetExitStatus => (self.set_exit_status, 1),
            HostFunction::DirectoryOpen => (self.directory_open, 2),
            HostFunction::FileOpen => (self.file_open, 4),
            HostFunction::FileRead => (self.file_read, 3),
            HostFunction::FileWrite => (self.file_write, 3),
            HostFunction::HandleClose => (self.handle_close, 1),
        }
    }
    pub(super) fn data(self, a: &mut Assembler, data: &mut Vec<u8>, offset: usize) {
        // openat2 is synchronous. No language callback can observe this private
        // ABI scratch or overlap its use with another syscall.
        a.data_label(self.open_how, offset + data.len());
        data.extend_from_slice(&[0u8; 24]);
    }
}

// Resolve argument index zero to argv[1], retaining the ABI argument pointer DI.
fn argument_address(a: &mut Assembler, invalid: Label) {
    a.load(R8, DI, 0);
    a.load(R9, GLOBALS, INITIAL_STACK);
    a.load(C, R9, 0);
    a.test(C, true);
    a.condition(4, invalid);
    a.immediate(A, 1);
    a.rr(0x29, C, A, true);
    a.compare(R8, C);
    a.condition(3, invalid);
    a.shift(R8, 4, 3);
    a.lea(A, R9, 16);
    a.rr(0x01, A, R8, true);
    a.load(SI, A, 0);
}

// Linux supplies NUL-terminated argument storage. Preserve SI and DI; return its
// byte length in A. Source code never receives an address into the initial stack.
fn argument_length(a: &mut Assembler) {
    a.mov(R10, SI);
    a.immediate(A, 0);
    a.immediate(R8, 1);
    let scanning = a.label();
    let end = a.label();
    a.bind(scanning);
    a.memory(&[0x0f, 0xb6], C, R10, 0, false);
    a.test(C, false);
    a.condition(4, end);
    a.rr(0x01, A, R8, true);
    a.rr(0x01, R10, R8, true);
    a.jump(scanning);
    a.bind(end);
}

// R8/R9 contain the heap offset and terminated length. Preserve DI (arguments),
// return the native path pointer in SI; every byte belongs to the caller's buffer.
fn path(a: &mut Assembler, p: Platform, invalid: Label) {
    p.range(a, R8, R9);
    a.test(R9, true);
    a.condition(4, invalid);
    a.rr(0x01, R8, HEAP, true);
    a.mov(SI, R8);
    a.mov(C, R9);
    a.immediate(A, 1);
    a.rr(0x29, C, A, true);
    a.mov(A, R8);
    a.rr(0x01, A, C, true);
    a.memory(&[0x0f, 0xb6], D, A, 0, false);
    a.test(D, false);
    a.condition(5, invalid);
    a.mov(R10, R8);
    let scanning = a.label();
    let end = a.label();
    a.bind(scanning);
    a.test(C, true);
    a.condition(4, end);
    a.memory(&[0x0f, 0xb6], D, R10, 0, false);
    a.test(D, false);
    a.condition(4, invalid);
    a.immediate(A, 1);
    a.rr(0x01, R10, A, true);
    a.rr(0x29, C, A, true);
    a.jump(scanning);
    a.bind(end);
}

fn syscall_retry(a: &mut Assembler, number: u64) {
    let retry = a.label();
    a.bind(retry);
    a.immediate(A, number);
    a.syscall();
    a.immediate(C, (-4_i64) as u64); // EINTR has not completed the operation.
    a.compare(A, C);
    a.condition(4, retry);
}

// File I/O leaves return a short count or the kernel's negative errno. The source
// library decides whether another read/write is needed.
fn file_io(a: &mut Assembler, p: Platform, label: Label, number: u64, fd: Option<u64>) {
    a.bind(label);
    if let Some(fd) = fd {
        a.immediate(R8, fd);
        a.load(R9, DI, 0);
        a.load(D, DI, -8);
    } else {
        a.load(R8, DI, 0);
        a.load(R9, DI, -8);
        a.load(D, DI, -16);
    }
    p.range(a, R9, D);
    a.rr(0x01, R9, HEAP, true);
    a.mov(DI, R8);
    a.mov(SI, R9);
    syscall_retry(a, number);
    a.ret();
}

// Stream write matches the existing Stdout contract: write every byte or trap at
// the source call. A full write, including zero bytes, returns the unit word zero.
fn stream_write(a: &mut Assembler, p: Platform, label: Label, fd: u64) {
    a.bind(label);
    let failed = a.label();
    a.load(R8, DI, 0);
    a.load(R9, DI, -8);
    a.load(A, DI, -16);
    a.store(GLOBALS, Platform::global(p.trap_line), A);
    p.range_to(a, R8, R9, failed);
    a.rr(0x01, R8, HEAP, true);
    let writing = a.label();
    let complete = a.label();
    a.bind(writing);
    a.test(R9, true);
    a.condition(4, complete);
    a.immediate(DI, fd);
    a.mov(SI, R8);
    a.mov(D, R9);
    syscall_retry(a, 1);
    a.test(A, true);
    a.condition(14, failed);
    a.compare(A, R9);
    a.condition(7, failed);
    a.rr(0x01, R8, A, true);
    a.rr(0x29, R9, A, true);
    a.jump(writing);
    a.bind(failed);
    a.immediate(A, 7);
    a.store(GLOBALS, Platform::global(p.trap_code), A);
    a.jump(p.trap);
    a.bind(complete);
    a.immediate(A, 0);
    a.ret();
}

pub(super) fn emit(a: &mut Assembler, p: Platform) {
    let io = p.io;
    let invalid = a.label();

    a.bind(io.argument_count);
    a.load(R8, GLOBALS, INITIAL_STACK);
    a.load(A, R8, 0);
    a.test(A, true);
    let count_ready = a.label();
    a.condition(4, count_ready);
    a.immediate(C, 1);
    a.rr(0x29, A, C, true);
    a.bind(count_ready);
    a.ret();

    a.bind(io.argument_length);
    argument_address(a, invalid);
    argument_length(a);
    a.ret();

    a.bind(io.argument_copy);
    argument_address(a, invalid);
    argument_length(a);
    a.load(D, DI, -16);
    a.compare(A, D);
    let count_fits = a.label();
    a.condition(6, count_fits);
    a.mov(A, D);
    a.bind(count_fits);
    a.mov(D, A);
    a.load(DI, DI, -8);
    p.range(a, DI, D);
    a.rr(0x01, DI, HEAP, true);
    a.mov(R9, D);
    a.mov(C, D);
    a.raw(&[0xfc, 0xf3, 0xa4]);
    a.mov(A, R9);
    a.ret();

    file_io(a, p, io.stdin_read, 0, Some(0));
    stream_write(a, p, p.stdout, 1);
    stream_write(a, p, io.stderr_write, 2);

    a.bind(io.set_exit_status);
    a.load(A, DI, 0);
    a.immediate(C, 255);
    a.rr(0x21, A, C, true);
    a.store(GLOBALS, EXIT_STATUS, A);
    a.immediate(A, 0);
    a.ret();

    a.bind(io.directory_open);
    a.load(R8, DI, 0);
    a.load(R9, DI, -8);
    path(a, p, invalid);
    a.immediate(DI, (-100_i64) as u64); // AT_FDCWD: World names the root explicitly.
    a.immediate(D, 0x290000); // O_PATH | O_DIRECTORY | O_CLOEXEC
    a.immediate(R10, 0);
    syscall_retry(a, 257); // openat
    a.jump(io.open_result);

    a.bind(io.file_open);
    a.load(R8, DI, -8);
    a.load(R9, DI, -16);
    path(a, p, invalid);
    a.address(R8, io.open_how);
    a.load(A, DI, -24);
    a.test(A, true);
    let read_only = a.label();
    let open = a.label();
    a.condition(4, read_only);
    a.immediate(A, 0x80241); // O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC
    a.store(R8, 0, A);
    a.immediate(A, 0o666);
    a.store(R8, 8, A);
    a.jump(open);
    a.bind(read_only);
    a.immediate(A, 0x80000); // O_RDONLY | O_CLOEXEC
    a.store(R8, 0, A);
    a.immediate(A, 0);
    a.store(R8, 8, A);
    a.bind(open);
    a.immediate(A, 0x0a); // RESOLVE_BENEATH | RESOLVE_NO_MAGICLINKS
    a.store(R8, 16, A);
    a.load(DI, DI, 0);
    a.mov(D, R8);
    a.immediate(R10, 24);
    syscall_retry(a, 437); // openat2; ENOSYS remains an ordinary reported error.
    a.jump(io.open_result);

    file_io(a, p, io.file_read, 0, None);
    file_io(a, p, io.file_write, 1, None);
    a.bind(io.handle_close);
    a.load(DI, DI, 0);
    a.immediate(A, 3);
    a.syscall();
    // close releases ownership even when interrupted; retry could close a reused
    // descriptor. Destruction has no additional source-visible result.
    a.immediate(A, 0);
    a.ret();

    a.bind(io.open_result);
    // Opening a file must not revive or retarget a closed standard stream.
    // Only a successful descriptor below three needs promotion. Negative errno
    // values compare above three as unsigned words and return unchanged.
    let descriptor_ready = a.label();
    a.immediate(C, 3);
    a.compare(A, C);
    a.condition(3, descriptor_ready);
    a.mov(R8, A);
    a.mov(DI, A);
    a.immediate(SI, 1030); // F_DUPFD_CLOEXEC
    a.immediate(D, 3);
    syscall_retry(a, 72); // fcntl
    a.mov(R9, A);
    a.mov(DI, R8);
    a.immediate(A, 3);
    a.syscall();
    a.mov(A, R9);
    a.bind(descriptor_ready);
    a.ret();

    a.bind(invalid);
    a.immediate(A, (-22_i64) as u64); // EINVAL
    a.ret();
}
