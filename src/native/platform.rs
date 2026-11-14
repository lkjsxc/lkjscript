//! Linux process entry and capability leaves over real machine addresses.
use super::Entry;
use crate::diagnostic::{Diagnostic, Span};
use crate::x86::*;

pub const IMAGE_BASE: u64 = 0x400000;
pub const CODE_OFFSET: usize = 4096;
const INITIAL_STACK: i64 = 0;
const EXIT_STATUS: i64 = 8;
const TRAP_CODE: i64 = 16;
const RESULT: i64 = 24;
const TRAP_LINE: i64 = 32;
const TRAP_FILE: i64 = 40;
const STACK_TOP: i64 = 48;
const GROW_RETURN: i64 = 56;
const GROW_NEEDED: i64 = 64;
const GROW_REQUEST: i64 = 72;
const GROW_ARGUMENTS: i64 = 80;
const MEMORY_ARGUMENTS: i64 = 128;
const TRIM_RETURN: i64 = 152;
const STACK_RESERVATION: u64 = 1u64 << 46;
const PAGE_BYTES: u64 = 4096;

#[derive(Clone, Copy)]
pub struct Platform {
    pub context: Label,
    pub allocator_state: Label,
    pub trap: Label,
    pub heap_exhausted: Label,
    pub stack_exhausted: Label,
    pub grow_stack: Label,
    pub trim_stack: Label,
    pub copy: Label,
    pub map: Label,
    pub unmap: Label,
    pub remap: Label,
    pub complete: Label,
    pub stdout_write: Label,
    pub stderr_write: Label,
    pub stdin_read: Label,
    pub argument_count: Label,
    pub argument_length: Label,
    pub argument_copy: Label,
    pub set_exit_status: Label,
    pub directory_open: Label,
    pub file_open: Label,
    pub file_read: Label,
    pub file_write: Label,
    pub handle_close: Label,
    pub world_file: Label,
    pub clock_read: Label,
    pub process_spawn: Label,
    pub child_wait: Label,
    pub child_terminate: Label,
    pub child_drop: Label,
    pub directory_open_directory: Label,
    pub directory_read: Label,
    pub handle_rewind: Label,
    pub directory_create: Label,
    pub directory_remove: Label,
    pub directory_rename: Label,
    pub file_executable: Label,
    pub file_duplicate: Label,
    pub file_readonly: Label,
    messages: Label,
    signal_action: Label,
    diagnostic_buffer: Label,
    open_how: Label,
}
impl Platform {
    pub fn new(a: &mut Assembler) -> Self {
        Self {
            context: a.label(),
            allocator_state: a.label(),
            trap: a.label(),
            heap_exhausted: a.label(),
            stack_exhausted: a.label(),
            grow_stack: a.label(),
            trim_stack: a.label(),
            copy: a.label(),
            map: a.label(),
            unmap: a.label(),
            remap: a.label(),
            complete: a.label(),
            stdout_write: a.label(),
            stderr_write: a.label(),
            stdin_read: a.label(),
            argument_count: a.label(),
            argument_length: a.label(),
            argument_copy: a.label(),
            set_exit_status: a.label(),
            directory_open: a.label(),
            file_open: a.label(),
            file_read: a.label(),
            file_write: a.label(),
            handle_close: a.label(),
            world_file: a.label(),
            clock_read: a.label(),
            process_spawn: a.label(),
            child_wait: a.label(),
            child_terminate: a.label(),
            child_drop: a.label(),
            directory_open_directory: a.label(),
            directory_read: a.label(),
            handle_rewind: a.label(),
            directory_create: a.label(),
            directory_remove: a.label(),
            directory_rename: a.label(),
            file_executable: a.label(),
            file_duplicate: a.label(),
            file_readonly: a.label(),
            messages: a.label(),
            signal_action: a.label(),
            diagnostic_buffer: a.label(),
            open_how: a.label(),
        }
    }
}

/// Platform leaves preserve callee-saved registers except on terminal failures.
pub fn emit(a: &mut Assembler, p: Platform, entry_label: Label, entry: &Entry) {
    a.address(CONTEXT, p.context);
    a.store(CONTEXT, INITIAL_STACK, SP);
    // SIGPIPE is reported through the ordinary output trap.
    a.immediate(A, 13);
    a.immediate(DI, 13);
    a.address(SI, p.signal_action);
    a.immediate(D, 0);
    a.immediate(R10, 8);
    a.syscall();
    let signals_ready = a.label();
    a.test(A, true);
    a.condition(9, signals_ready);
    a.immediate(LINE, 1);
    a.immediate(FILE, 0);
    a.immediate(A, 7);
    a.jump(p.trap);
    a.bind(signals_ready);

    // Place our stack just below the original kernel stack, above the ordinary
    // mmap arena. Releasing unused low pages must not invite the next heap map
    // to occupy the stack's future growth direction.
    let probe = a.label();
    let top_ready = a.label();
    a.load(DI, CONTEXT, INITIAL_STACK);
    a.immediate(C, !(PAGE_BYTES - 1));
    a.rr(0x21, DI, C, true);
    a.bind(probe);
    a.lea(DI, DI, -(PAGE_BYTES as i64));
    a.immediate(SI, PAGE_BYTES);
    a.immediate(D, 3);
    a.immediate(R10, 0x104022); // FIXED_NOREPLACE | NORESERVE | PRIVATE | ANONYMOUS
    a.immediate(R8, !0u64);
    a.immediate(R9, 0);
    a.immediate(A, 9);
    a.syscall();
    a.immediate(C, (-17i64) as u64); // EEXIST: the original stack still occupies it.
    a.compare(A, C);
    a.condition(4, probe);
    a.immediate(C, (-4095i64) as u64);
    a.compare(A, C);
    a.condition(2, top_ready);
    a.jump(p.stack_exhausted);
    a.bind(top_ready);
    a.mov(STACK_FLOOR, A);
    a.lea(A, A, PAGE_BYTES as i64);
    a.store(CONTEXT, STACK_TOP, A);

    // Reserve lazily, halving on refusal. The single top page already suffices
    // to enter the first frame; further growth is requested only when needed.
    let reserve = a.label();
    let smaller = a.label();
    let reserved = a.label();
    a.immediate(B, STACK_RESERVATION);
    a.bind(reserve);
    a.immediate(C, PAGE_BYTES);
    a.compare(B, C);
    a.condition(6, reserved);
    a.mov(SI, B);
    a.rr(0x29, SI, C, true);
    a.mov(DI, STACK_FLOOR);
    a.rr(0x29, DI, SI, true);
    a.condition(2, smaller);
    a.immediate(D, 3);
    a.immediate(R10, 0x104022);
    a.immediate(R8, !0u64);
    a.immediate(R9, 0);
    a.immediate(A, 9);
    a.syscall();
    a.immediate(C, (-4095i64) as u64);
    a.compare(A, C);
    a.condition(3, smaller);
    a.mov(STACK_FLOOR, A);
    // Leave an unmapped guard below the current floor, without charging it
    // against an external address-space allowance.
    a.mov(DI, A);
    a.immediate(SI, PAGE_BYTES);
    a.immediate(A, 11);
    a.syscall();
    a.lea(STACK_FLOOR, STACK_FLOOR, PAGE_BYTES as i64);
    a.jump(reserved);
    a.bind(smaller);
    a.shift(B, 5, 1);
    a.jump(reserve);
    a.bind(reserved);
    a.load(SP, CONTEXT, STACK_TOP);
    a.immediate(BP, 0);

    let registers = [DI, SI, D, C, R8, R9];
    let extra = if entry.arguments.len() > registers.len() {
        entry.arguments.len() - registers.len()
    } else {
        0
    };
    if extra % 2usize == 1usize {
        a.immediate(A, 0);
        a.push(A);
    }
    let mut index = entry.arguments.len();
    while index > registers.len() {
        index -= 1;
        a.immediate(A, entry.arguments[index] as u64);
        a.push(A);
    }
    let mut index = 0usize;
    while index < entry.arguments.len() && index < registers.len() {
        a.immediate(registers[index], entry.arguments[index] as u64);
        index += 1;
    }
    a.call(entry_label);
    a.store(CONTEXT, RESULT, A);
    a.immediate(A, 0);
    a.store(CONTEXT, TRAP_CODE, A);
    a.load(A, CONTEXT, EXIT_STATUS);
    a.store(CONTEXT, TRAP_LINE, A);
    a.jump(p.complete);

    a.bind(p.heap_exhausted);
    a.immediate(LINE, 1);
    a.immediate(FILE, 0);
    a.immediate(A, 6);
    a.jump(p.trap);
    a.bind(p.stack_exhausted);
    a.immediate(LINE, 1);
    a.immediate(FILE, 0);
    a.immediate(A, 8);
    a.jump(p.trap);
    a.bind(p.trap);
    a.store(CONTEXT, TRAP_CODE, A);
    a.store(CONTEXT, TRAP_LINE, LINE);
    a.store(CONTEXT, TRAP_FILE, FILE);
    a.immediate(A, 0);
    a.store(CONTEXT, RESULT, A);
    a.jump(p.complete);

    a.bind(p.complete);
    if let Some(fd) = entry.result_fd {
        let trapped = a.label();
        a.load(A, CONTEXT, TRAP_CODE);
        a.test(A, true);
        a.condition(5, trapped);
        a.lea(SI, CONTEXT, RESULT);
        a.immediate(D, 8);
        let writing = a.label();
        let exit = a.label();
        a.bind(writing);
        a.immediate(DI, fd as u64);
        syscall_retry(a, 1);
        a.test(A, true);
        a.condition(14, exit);
        a.rr(0x01, SI, A, true);
        a.rr(0x29, D, A, true);
        a.test(D, true);
        a.condition(5, writing);
        a.bind(exit);
        a.bind(trapped);
    }
    diagnostic(a, p, entry.result_fd.unwrap_or(2));
    a.load(A, CONTEXT, TRAP_CODE);
    a.test(A, true);
    a.immediate(DI, 1);
    let exiting = a.label();
    a.condition(5, exiting);
    a.load(DI, CONTEXT, EXIT_STATUS);
    a.bind(exiting);
    a.immediate(A, 60);
    a.syscall();
    a.raw(&[0x0f, 0x0b]);

    stack(a, p);
    memory(a, p);
    io(a, p);
    super::process::emit(a, p);
}

fn syscall_retry(a: &mut Assembler, number: u64) {
    let retry = a.label();
    a.bind(retry);
    a.immediate(A, number);
    a.syscall();
    a.immediate(C, (-4i64) as u64);
    a.compare(A, C);
    a.condition(4, retry);
}

// Both cold stack helpers use context scratch and return by an indirect jump.
// They never touch an unprovisioned stack page and never alter allocated saved
// registers. The current platform is single-threaded; future threads need their
// own context just as they need their own machine stacks.
fn stack(a: &mut Assembler, p: Platform) {
    a.bind(p.grow_stack);
    a.store(CONTEXT, GROW_RETURN, A);
    a.store(CONTEXT, GROW_NEEDED, R11);
    let arguments = [DI, SI, D, C, R8, R9];
    for index in 0..arguments.len() {
        a.store(CONTEXT, GROW_ARGUMENTS + index as i64 * 8, arguments[index]);
    }
    a.immediate(C, !(PAGE_BYTES - 1));
    a.rr(0x21, R11, C, true);
    a.store(CONTEXT, GROW_NEEDED, R11);
    a.mov(D, STACK_FLOOR);
    a.rr(0x29, D, R11, true);
    a.load(SI, CONTEXT, STACK_TOP);
    a.rr(0x29, SI, STACK_FLOOR, true);
    a.compare(SI, D);
    a.cmov(SI, D, 2);
    a.store(CONTEXT, GROW_REQUEST, SI);
    let growing = a.label();
    let refused = a.label();
    let failed = a.label();
    let restore = a.label();
    a.bind(growing);
    a.load(SI, CONTEXT, GROW_REQUEST);
    a.mov(DI, STACK_FLOOR);
    a.rr(0x29, DI, SI, true);
    a.condition(2, refused);
    a.immediate(D, 3);
    a.immediate(R10, 0x104022);
    a.immediate(R8, !0u64);
    a.immediate(R9, 0);
    a.immediate(A, 9);
    a.syscall();
    a.immediate(C, (-4095i64) as u64);
    a.compare(A, C);
    a.condition(3, refused);
    a.mov(STACK_FLOOR, A);
    a.immediate(A, 1);
    a.jump(restore);
    a.bind(refused);
    a.load(SI, CONTEXT, GROW_REQUEST);
    a.mov(D, STACK_FLOOR);
    a.load(C, CONTEXT, GROW_NEEDED);
    a.rr(0x29, D, C, true);
    a.compare(SI, D);
    a.condition(6, failed);
    a.shift(SI, 5, 1);
    a.immediate(C, !(PAGE_BYTES - 1));
    a.rr(0x21, SI, C, true);
    a.compare(SI, D);
    a.cmov(SI, D, 2);
    a.store(CONTEXT, GROW_REQUEST, SI);
    a.jump(growing);
    a.bind(failed);
    a.immediate(A, 0);
    a.bind(restore);
    for index in 0..arguments.len() {
        a.load(arguments[index], CONTEXT, GROW_ARGUMENTS + index as i64 * 8);
    }
    a.load(R11, CONTEXT, GROW_RETURN);
    a.jump_register(R11);

    a.bind(p.trim_stack);
    a.store(CONTEXT, TRIM_RETURN, A);
    a.mov(DI, SP);
    a.immediate(C, !(PAGE_BYTES - 1));
    a.rr(0x21, DI, C, true);
    a.compare(DI, STACK_FLOOR);
    let unchanged = a.label();
    let trimmed = a.label();
    a.condition(6, unchanged);
    a.store(CONTEXT, GROW_NEEDED, DI);
    a.mov(SI, DI);
    a.rr(0x29, SI, STACK_FLOOR, true);
    a.mov(DI, STACK_FLOOR);
    a.immediate(A, 11);
    a.syscall();
    a.test(A, true);
    a.condition(8, unchanged);
    a.load(STACK_FLOOR, CONTEXT, GROW_NEEDED);
    a.immediate(A, 1);
    a.jump(trimmed);
    a.bind(unchanged);
    a.immediate(A, 0);
    a.bind(trimmed);
    a.load(R11, CONTEXT, TRIM_RETURN);
    a.jump_register(R11);
}

fn memory(a: &mut Assembler, p: Platform) {
    a.bind(p.map);
    a.store(CONTEXT, MEMORY_ARGUMENTS, DI);
    let mapping = a.label();
    let map_trimmed = a.label();
    let map_failed = a.label();
    let mapped = a.label();
    a.bind(mapping);
    a.load(SI, CONTEXT, MEMORY_ARGUMENTS);
    a.immediate(DI, 0);
    a.immediate(D, 3);
    a.immediate(R10, 0x22);
    a.immediate(R8, !0u64);
    a.immediate(R9, 0);
    a.immediate(A, 9);
    a.syscall();
    a.immediate(C, (-4095i64) as u64);
    a.compare(A, C);
    a.condition(2, mapped);
    a.immediate(C, (-12i64) as u64);
    a.compare(A, C);
    a.condition(5, map_failed);
    a.address(A, map_trimmed);
    a.jump(p.trim_stack);
    a.bind(map_trimmed);
    a.test(A, true);
    a.condition(5, mapping);
    a.bind(map_failed);
    a.immediate(A, 0);
    a.bind(mapped);
    a.ret();

    a.bind(p.remap);
    a.store(CONTEXT, MEMORY_ARGUMENTS, DI);
    a.store(CONTEXT, MEMORY_ARGUMENTS + 8, SI);
    a.store(CONTEXT, MEMORY_ARGUMENTS + 16, D);
    let remapping = a.label();
    let remap_trimmed = a.label();
    let remap_failed = a.label();
    let remapped = a.label();
    a.bind(remapping);
    a.load(DI, CONTEXT, MEMORY_ARGUMENTS);
    a.load(SI, CONTEXT, MEMORY_ARGUMENTS + 8);
    a.load(D, CONTEXT, MEMORY_ARGUMENTS + 16);
    a.immediate(R10, 1); // MREMAP_MAYMOVE
    a.immediate(A, 25);
    a.syscall();
    a.immediate(C, (-4095i64) as u64);
    a.compare(A, C);
    a.condition(2, remapped);
    a.immediate(C, (-12i64) as u64);
    a.compare(A, C);
    a.condition(5, remap_failed);
    a.address(A, remap_trimmed);
    a.jump(p.trim_stack);
    a.bind(remap_trimmed);
    a.test(A, true);
    a.condition(5, remapping);
    a.bind(remap_failed);
    a.immediate(A, 0);
    a.bind(remapped);
    a.ret();

    a.bind(p.unmap);
    a.immediate(A, 11);
    a.syscall();
    a.ret();

    a.bind(p.copy);
    let done = a.label();
    let forward = a.label();
    a.test(D, true);
    a.condition(4, done);
    a.compare(DI, SI);
    a.condition(6, forward);
    a.mov(A, SI);
    a.rr(0x01, A, D, true);
    a.compare(DI, A);
    a.condition(3, forward);
    a.rr(0x01, DI, D, true);
    a.rr(0x01, SI, D, true);
    a.lea(DI, DI, -1);
    a.lea(SI, SI, -1);
    a.mov(C, D);
    a.raw(&[0xfd, 0xf3, 0xa4, 0xfc]);
    a.jump(done);
    a.bind(forward);
    a.mov(C, D);
    a.raw(&[0xfc, 0xf3, 0xa4]);
    a.bind(done);
    a.immediate(A, 0);
    a.ret();
}

// DI is the zero-based user argument index; SI receives argv[index + 1].
fn argument_address(a: &mut Assembler, invalid: Label) {
    a.load(R9, CONTEXT, INITIAL_STACK);
    a.load(C, R9, 0);
    a.test(C, true);
    a.condition(4, invalid);
    a.immediate(A, 1);
    a.rr(0x29, C, A, true);
    a.compare(DI, C);
    a.condition(3, invalid);
    a.mov(A, DI);
    a.shift(A, 4, 3);
    a.lea(R8, R9, 16);
    a.rr(0x01, R8, A, true);
    a.load(SI, R8, 0);
}

fn argument_length(a: &mut Assembler) {
    a.mov(R10, SI);
    a.immediate(A, 0);
    a.immediate(R8, 1);
    let scanning = a.label();
    let end = a.label();
    a.bind(scanning);
    a.load_width(C, R10, 0, 1, false);
    a.test(C, false);
    a.condition(4, end);
    a.rr(0x01, A, R8, true);
    a.rr(0x01, R10, R8, true);
    a.jump(scanning);
    a.bind(end);
}

// SI/D is a caller-owned path including one trailing NUL. Preserve DI and C.
fn path(a: &mut Assembler, invalid: Label) {
    a.test(D, true);
    a.condition(4, invalid);
    a.mov(R9, D);
    a.immediate(A, 1);
    a.rr(0x29, R9, A, true);
    a.mov(R8, SI);
    a.rr(0x01, R8, R9, true);
    a.load_width(R10, R8, 0, 1, false);
    a.test(R10, false);
    a.condition(5, invalid);
    a.mov(R8, SI);
    let scanning = a.label();
    let complete = a.label();
    a.bind(scanning);
    a.test(R9, true);
    a.condition(4, complete);
    a.load_width(R10, R8, 0, 1, false);
    a.test(R10, false);
    a.condition(4, invalid);
    a.rr(0x01, R8, A, true);
    a.rr(0x29, R9, A, true);
    a.jump(scanning);
    a.bind(complete);
}

fn stream_write(a: &mut Assembler, label: Label, fd: u64) {
    a.bind(label);
    a.mov(R8, DI);
    a.mov(R9, SI);
    let writing = a.label();
    let done = a.label();
    let failed = a.label();
    a.bind(writing);
    a.test(R9, true);
    a.condition(4, done);
    a.immediate(DI, fd);
    a.mov(SI, R8);
    a.mov(D, R9);
    syscall_retry(a, 1);
    a.test(A, true);
    a.condition(14, failed);
    a.rr(0x01, R8, A, true);
    a.rr(0x29, R9, A, true);
    a.jump(writing);
    a.bind(failed);
    a.immediate(A, !0u64);
    a.ret();
    a.bind(done);
    a.immediate(A, 0);
    a.ret();
}

fn io(a: &mut Assembler, p: Platform) {
    let invalid = a.label();
    let open_result = a.label();
    a.bind(p.argument_count);
    a.load(R8, CONTEXT, INITIAL_STACK);
    a.load(A, R8, 0);
    a.test(A, true);
    let count_ready = a.label();
    a.condition(4, count_ready);
    a.lea(A, A, -1);
    a.bind(count_ready);
    a.ret();

    a.bind(p.argument_length);
    argument_address(a, invalid);
    argument_length(a);
    a.ret();

    a.bind(p.argument_copy);
    a.mov(R11, SI);
    argument_address(a, invalid);
    argument_length(a);
    a.compare(A, D);
    a.cmov(A, D, 7);
    a.mov(C, A);
    a.mov(DI, R11);
    a.raw(&[0xfc, 0xf3, 0xa4]);
    a.ret();

    stream_write(a, p.stdout_write, 1);
    stream_write(a, p.stderr_write, 2);
    a.bind(p.stdin_read);
    a.mov(D, SI);
    a.mov(SI, DI);
    a.immediate(DI, 0);
    syscall_retry(a, 0);
    a.ret();
    a.bind(p.file_read);
    syscall_retry(a, 0);
    a.ret();
    a.bind(p.file_write);
    syscall_retry(a, 1);
    a.ret();

    a.bind(p.set_exit_status);
    a.immediate(A, 255);
    a.rr(0x21, DI, A, true);
    a.store(CONTEXT, EXIT_STATUS, DI);
    a.immediate(A, 0);
    a.ret();

    a.bind(p.directory_open);
    a.mov(D, SI);
    a.mov(SI, DI);
    path(a, invalid);
    a.immediate(DI, (-100i64) as u64);
    a.immediate(D, 0x290000);
    a.immediate(R10, 0);
    syscall_retry(a, 257);
    a.jump(open_result);

    a.bind(p.file_open);
    path(a, invalid);
    a.address(R8, p.open_how);
    let read_only = a.label();
    let opening = a.label();
    a.test(C, true);
    a.condition(4, read_only);
    a.immediate(A, 0x80241);
    a.store(R8, 0, A);
    a.immediate(A, 0o666);
    a.store(R8, 8, A);
    a.jump(opening);
    a.bind(read_only);
    a.immediate(A, 0x80000);
    a.store(R8, 0, A);
    a.immediate(A, 0);
    a.store(R8, 8, A);
    a.bind(opening);
    a.immediate(A, 0x0a); // RESOLVE_BENEATH | RESOLVE_NO_MAGICLINKS
    a.store(R8, 16, A);
    a.mov(D, R8);
    a.immediate(R10, 24);
    syscall_retry(a, 437);
    a.jump(open_result);

    a.bind(p.handle_close);
    let already_closed = a.label();
    a.test(DI, true);
    a.condition(4, already_closed);
    a.immediate(A, 3);
    a.syscall();
    // close consumes ownership even on EINTR; retry could close a reused handle.
    a.bind(already_closed);
    a.immediate(A, 0);
    a.ret();

    a.bind(open_result);
    let ready = a.label();
    a.immediate(C, 3);
    a.compare(A, C);
    a.condition(3, ready);
    a.mov(R8, A);
    a.mov(DI, A);
    a.immediate(SI, 1030); // F_DUPFD_CLOEXEC
    a.immediate(D, 3);
    syscall_retry(a, 72);
    a.mov(R9, A);
    a.mov(DI, R8);
    a.immediate(A, 3);
    a.syscall();
    a.mov(A, R9);
    a.bind(ready);
    a.ret();

    a.bind(invalid);
    a.immediate(A, (-22i64) as u64);
    a.ret();
}

fn diagnostic_text(a: &mut Assembler, text: &str) {
    let bytes = text.as_bytes();
    let mut index = bytes.len();
    while index > 0 {
        index -= 1;
        a.lea(R8, R8, -1);
        a.immediate(A, bytes[index] as u64);
        a.store_width(R8, 0, A, 1);
    }
}

fn diagnostic_decimal(a: &mut Assembler, minimum: u64) {
    a.immediate(C, 10);
    a.immediate(R9, 0);
    a.immediate(R10, 1);
    let digit = a.label();
    a.bind(digit);
    a.immediate(D, 0);
    a.divide(C, false);
    a.lea(R8, R8, -1);
    a.rr(0x01, R9, R10, true);
    a.immediate(SI, b'0' as u64);
    a.rr(0x01, D, SI, true);
    a.store_width(R8, 0, D, 1);
    a.test(A, true);
    a.condition(5, digit);
    a.immediate(SI, minimum);
    a.compare(R9, SI);
    a.condition(2, digit);
}

fn diagnostic_write(a: &mut Assembler, fd: u32) {
    let writing = a.label();
    let complete = a.label();
    a.bind(writing);
    a.immediate(DI, fd as u64);
    syscall_retry(a, 1);
    a.test(A, true);
    a.condition(14, complete);
    a.rr(0x01, SI, A, true);
    a.rr(0x29, D, A, true);
    a.test(D, true);
    a.condition(5, writing);
    a.bind(complete);
}

fn diagnostic(a: &mut Assembler, p: Platform, fd: u32) {
    let complete = a.label();
    a.load(A, CONTEXT, TRAP_CODE);
    a.test(A, true);
    a.condition(4, complete);
    a.address(R8, p.diagnostic_buffer);
    a.lea(R8, R8, 128);
    diagnostic_text(a, ": ");
    a.load(A, CONTEXT, TRAP_FILE);
    diagnostic_decimal(a, 1);
    diagnostic_text(a, " file ");
    a.load(A, CONTEXT, TRAP_LINE);
    diagnostic_decimal(a, 1);
    diagnostic_text(a, " at line ");
    a.load(A, CONTEXT, TRAP_CODE);
    diagnostic_decimal(a, 4);
    diagnostic_text(a, "R");
    a.mov(SI, R8);
    a.address(D, p.diagnostic_buffer);
    a.lea(D, D, 128);
    a.rr(0x29, D, SI, true);
    diagnostic_write(a, fd);
    a.address(R8, p.messages);
    a.load(A, CONTEXT, TRAP_CODE);
    a.load(C, R8, 0);
    a.compare(A, C);
    let known = a.label();
    a.condition(2, known);
    a.immediate(A, 4);
    a.bind(known);
    a.shift(A, 4, 4);
    a.rr(0x01, R8, A, true);
    a.load(SI, R8, 8);
    a.load(D, R8, 16);
    diagnostic_write(a, fd);
    a.bind(complete);
}

/// Bind RW data labels. offset and all labels are relative to the code start.
pub fn data(
    a: &mut Assembler,
    p: Platform,
    bytes: &mut Vec<u8>,
    offset: usize,
    messages: &[String],
) {
    a.data_label(p.context, offset + bytes.len());
    bytes.extend_from_slice(&[0u8; 160]);
    a.data_label(p.signal_action, offset + bytes.len());
    bytes.extend_from_slice(&[1u8, 0, 0, 0, 0, 0, 0, 0]);
    bytes.extend_from_slice(&[0u8; 24]);
    a.data_label(p.diagnostic_buffer, offset + bytes.len());
    bytes.extend_from_slice(&[0u8; 128]);
    a.data_label(p.open_how, offset + bytes.len());
    bytes.extend_from_slice(&[0u8; 24]);
    a.data_label(p.allocator_state, offset + bytes.len());
    for _ in 0..crate::runtime::ALLOCATOR_STATE_BYTES {
        bytes.push(0);
    }
    a.data_label(p.messages, offset + bytes.len());
    let mut text = vec![
        "completed".to_string(),
        "integer overflow".to_string(),
        "division by zero".to_string(),
        "index out of bounds".to_string(),
        "explicit trap".to_string(),
        "assertion failed".to_string(),
        "heap allocation failed".to_string(),
        "host output failure".to_string(),
        "stack exhausted".to_string(),
    ];
    while text.len() < 16 {
        text.push("trap".to_string());
    }
    for message in messages {
        text.push(message.clone());
    }
    let table = bytes.len();
    let table_bytes = 8 + text.len() * 16;
    for _ in 0..table_bytes {
        bytes.push(0);
    }
    put(bytes, table, text.len() as u64, 8);
    for index in 0..text.len() {
        let address = IMAGE_BASE + CODE_OFFSET as u64 + offset as u64 + bytes.len() as u64;
        put(bytes, table + 8 + index * 16, address, 8);
        put(
            bytes,
            table + 16 + index * 16,
            text[index].len() as u64 + 1,
            8,
        );
        bytes.extend_from_slice(text[index].as_bytes());
        bytes.push(b'\n');
    }
}

fn put(bytes: &mut [u8], at: usize, value: u64, count: usize) {
    let mut index = 0usize;
    while index < count {
        bytes[at + index] = (value >> (index * 8usize)) as u8;
        index += 1;
    }
}
fn segment(bytes: &mut [u8], at: usize, flags: u64, offset: u64, size: u64) {
    put(bytes, at, 1, 4);
    put(bytes, at + 4, flags, 4);
    put(bytes, at + 8, offset, 8);
    put(bytes, at + 16, IMAGE_BASE + offset, 8);
    put(bytes, at + 24, IMAGE_BASE + offset, 8);
    put(bytes, at + 32, size, 8);
    put(bytes, at + 40, size, 8);
    put(bytes, at + 48, PAGE_BYTES, 8);
}

/// image starts at the code entry and already contains aligned code and RW data.
pub fn elf(image: Vec<u8>, data_offset: usize) -> Result<Vec<u8>, Diagnostic> {
    let failure = "ELF image exceeds its address representation";
    if data_offset > image.len() || data_offset % PAGE_BYTES as usize != 0 {
        return Err(Diagnostic::new(
            Span::default(),
            "E_EMIT",
            "invalid ELF data boundary",
        ));
    }
    let total = match CODE_OFFSET.checked_add(image.len()) {
        Some(total) => total,
        None => return Err(Diagnostic::new(Span::default(), "E_EMIT", failure)),
    };
    let absolute_data = CODE_OFFSET + data_offset;
    let mut output = vec![0u8; total];
    let magic = [0x7f, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut index = 0usize;
    while index < magic.len() {
        output[index] = magic[index];
        index += 1;
    }
    put(&mut output, 16, 2, 2);
    put(&mut output, 18, 62, 2);
    put(&mut output, 20, 1, 4);
    put(&mut output, 24, IMAGE_BASE + CODE_OFFSET as u64, 8);
    put(&mut output, 32, 64, 8);
    put(&mut output, 52, 64, 2);
    put(&mut output, 54, 56, 2);
    put(&mut output, 56, 3, 2);
    segment(&mut output, 64, 5, 0, absolute_data as u64);
    segment(
        &mut output,
        120,
        6,
        absolute_data as u64,
        (image.len() - data_offset) as u64,
    );
    put(&mut output, 176, 0x6474e551, 4); // PT_GNU_STACK, writable and nonexecutable.
    put(&mut output, 180, 6, 4);
    put(&mut output, 224, 16, 8);
    let mut index = 0usize;
    while index < image.len() {
        output[CODE_OFFSET + index] = image[index];
        index += 1;
    }
    Ok(output)
}
