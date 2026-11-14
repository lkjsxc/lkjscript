//! Opt-in Linux CPU sampling. The shared samples file owns its address map.
//! No DWARF, external profiler, allocator calls or bounded stack traces.
use super::platform::{Platform, CODE_OFFSET, IMAGE_BASE};
use crate::x86::*;
use std::collections::BTreeMap;

const HEADER: usize = 48;
const ROW: usize = 64;
const PERIOD_US: u64 = 1000;

#[derive(Clone, Copy)]
pub struct Labels {
    pub start: Label,
    handler: Label,
    restore: Label,
    lookup: Label,
    include: Label,
    state: Label,
    table: Label,
    end: Label,
    path: Label,
    action: Label,
    timer: Label,
    altstack: Label,
    stack: Label,
}
impl Labels {
    pub fn new(a: &mut Assembler) -> Self {
        Self {
            start: a.label(), handler: a.label(), restore: a.label(),
            lookup: a.label(), include: a.label(), state: a.label(),
            table: a.label(), end: a.label(), path: a.label(),
            action: a.label(), timer: a.label(), altstack: a.label(), stack: a.label(),
        }
    }
}
fn add(a: &mut Assembler, register: u8, value: u64) {
    a.immediate(C, value);
    a.rr(0x01, register, C, true);
}
fn failure(a: &mut Assembler, p: Platform) {
    let ready = a.label();
    a.test(A, true);
    a.condition(9, ready);
    a.immediate(LINE, 1);
    a.immediate(FILE, 0);
    a.immediate(A, 7);
    a.jump(p.trap);
    a.bind(ready);
}
pub fn emit(a: &mut Assembler, p: Platform, l: Labels, path_len: usize) {
    a.bind(l.start);
    // Each process gets a separate inode. Self-spawned workers cannot truncate
    // their parent's live sampling map. A u64 PID uses twenty decimal digits.
    a.immediate(A, 39);
    a.syscall();
    a.address(B, l.path);
    a.lea(B, B, path_len as i64 + 20);
    a.immediate(R14, 20);
    let digit = a.label();
    a.bind(digit);
    a.immediate(D, 0);
    a.immediate(C, 10);
    a.divide(C, false);
    a.immediate(SI, b'0' as u64);
    a.rr(0x01, D, SI, true);
    a.store_width(B, 0, D, 1);
    a.lea(B, B, -1);
    a.lea(R14, R14, -1);
    a.test(R14, true);
    a.condition(5, digit);
    // Only this explicit build option grants the instrumentation output path.
    a.immediate(A, 257);
    a.immediate(DI, (-100i64) as u64);
    a.address(SI, l.path);
    a.immediate(D, 0xa0242); // RDWR | CREAT | TRUNC | NOFOLLOW | CLOEXEC
    a.immediate(R10, 0o600);
    a.syscall();
    failure(a, p);
    a.mov(B, A);
    a.address(SI, l.table);
    a.address(D, l.end);
    a.rr(0x29, D, SI, true);
    a.mov(R14, D);
    let writing = a.label();
    a.bind(writing);
    a.mov(DI, B);
    a.immediate(A, 1);
    a.syscall();
    failure(a, p);
    let progress = a.label();
    a.test(A, true);
    a.condition(5, progress);
    a.immediate(A, 7);
    a.jump(p.trap);
    a.bind(progress);
    a.rr(0x01, SI, A, true);
    a.rr(0x29, D, A, true);
    a.test(D, true);
    a.condition(5, writing);
    a.immediate(A, 9);
    a.immediate(DI, 0);
    a.mov(SI, R14);
    a.immediate(D, 3);
    a.immediate(R10, 1); // MAP_SHARED: counts survive ordinary exit and signals.
    a.mov(R8, B);
    a.immediate(R9, 0);
    a.syscall();
    failure(a, p);
    a.address(R11, l.state);
    a.store(R11, 0, A);
    a.mov(DI, B);
    a.immediate(A, 3);
    a.syscall();
    a.address(DI, l.altstack);
    a.address(A, l.stack);
    a.store(DI, 0, A);
    a.immediate(A, 131);
    a.immediate(SI, 0);
    a.syscall();
    failure(a, p);
    a.address(SI, l.action);
    a.address(A, l.handler);
    a.store(SI, 0, A);
    a.address(A, l.restore);
    a.store(SI, 16, A);
    a.immediate(A, 13);
    a.immediate(DI, 27); // SIGPROF
    a.immediate(D, 0);
    a.immediate(R10, 8);
    a.syscall();
    failure(a, p);
    a.immediate(A, 38);
    a.immediate(DI, 2); // ITIMER_PROF: process user + kernel CPU, not wall time.
    a.address(SI, l.timer);
    a.immediate(D, 0);
    a.syscall();
    failure(a, p);
    a.ret();

    a.bind(l.handler);
    // Linux x86-64 ucontext: gregs begin at 40; RBP=10, RSP=15, RIP=16.
    a.mov(R10, D);
    a.address(A, l.state);
    a.load(R12, A, 0);
    a.load(A, R12, 24);
    add(a, A, 1);
    a.store(R12, 24, A);
    a.load(B, R10, 120);
    a.load(R14, R10, 160);
    a.address(A, p.context);
    a.load(R15, A, 48); // stack top
    a.load(A, R10, 168);
    a.mov(R13, A);
    a.call(l.lookup);
    let walking = a.label();
    a.test(A, true);
    a.condition(4, walking);
    a.load(D, A, 16);
    add(a, D, 1);
    a.store(A, 16, D);
    a.call(l.include);
    // Leaf helpers have no frame; sample their return address from interrupted SP.
    // The pre-prologue and final ret of a framed body have the same layout.
    a.load(D, A, 56);
    a.test(D, true);
    let leaf = a.label();
    a.condition(4, leaf);
    a.compare(R13, D);
    a.condition(2, leaf);
    a.load_width(D, R13, 0, 1, false);
    a.immediate(C, 0xc3);
    a.compare(D, C);
    a.condition(5, walking);
    a.bind(leaf);
    a.mov(SI, R14);
    // Between push rbp and mov rbp,rsp the saved BP occupies interrupted SP.
    // The return address is one word above it until the frame becomes active.
    let return_slot = a.label();
    a.test(D, true);
    a.condition(4, return_slot);
    a.lea(C, D, -3);
    a.compare(R13, C);
    a.condition(5, return_slot);
    a.lea(SI, SI, 8);
    a.bind(return_slot);
    a.compare(SI, R15);
    a.condition(3, walking);
    a.load(A, SI, 0);
    a.lea(A, A, -1);
    a.call(l.lookup);
    a.call(l.include);
    a.bind(walking);
    let done = a.label();
    a.test(B, true);
    a.condition(4, done);
    a.compare(B, R14);
    a.condition(2, done);
    a.lea(D, R15, -16);
    a.compare(B, D);
    a.condition(7, done);
    a.mov(D, B);
    a.immediate(C, 7);
    a.rr(0x21, D, C, true);
    a.test(D, true);
    a.condition(5, done);
    a.load(A, B, 8);
    a.lea(A, A, -1); // A return belongs to the call's function, not the next one.
    a.call(l.lookup);
    a.call(l.include);
    a.lea(R14, B, 16);
    a.load(B, B, 0);
    a.jump(walking);
    a.bind(done);
    a.ret();

    a.bind(l.restore);
    a.immediate(A, 15); // rt_sigreturn restores every interrupted register/flag.
    a.syscall();
    a.raw(&[0x0f, 0x0b]);

    // Lookup returns the table row or null, preserving the unwind registers.
    a.bind(l.lookup);
    a.immediate(SI, 0);
    a.load(DI, R12, 8);
    let search = a.label();
    let found = a.label();
    let lower = a.label();
    let absent = a.label();
    a.bind(search);
    a.compare(SI, DI);
    a.condition(3, absent);
    a.mov(C, SI);
    a.rr(0x01, C, DI, true);
    a.shift(C, 5, 1);
    a.mov(R8, C);
    a.shift(R8, 4, 6);
    a.rr(0x01, R8, R12, true);
    a.lea(R8, R8, HEADER as i64);
    a.load(D, R8, 0);
    a.compare(A, D);
    a.condition(2, lower);
    a.load(D, R8, 8);
    a.compare(A, D);
    a.condition(2, found);
    a.lea(SI, C, 1);
    a.jump(search);
    a.bind(lower);
    a.mov(DI, C);
    a.jump(search);
    a.bind(found);
    a.mov(A, R8);
    a.ret();
    a.bind(absent);
    a.immediate(A, 0);
    a.ret();

    a.bind(l.include);
    let included = a.label();
    a.test(A, true);
    a.condition(4, included);
    a.load(D, R12, 24);
    a.load(C, A, 32);
    a.compare(D, C);
    a.condition(4, included);
    a.store(A, 32, D);
    a.load(D, A, 24);
    add(a, D, 1);
    a.store(A, 24, D);
    a.bind(included);
    a.ret();
}

fn put(bytes: &mut [u8], at: usize, value: u64) {
    for i in 0..8 {
        bytes[at + i] = (value >> (i * 8)) as u8;
    }
}
fn word(bytes: &mut Vec<u8>, value: u64) {
    for i in 0..8 {
        bytes.push((value >> (i * 8)) as u8);
    }
}
pub fn data(
    a: &mut Assembler, l: Labels, bytes: &mut Vec<u8>, offset: usize,
    path: &str, symbols: &[(Label, String)], code_end: usize,
) {
    let mut sorted: BTreeMap<usize, String> = BTreeMap::new();
    for (label, name) in symbols {
        if let Some(position) = a.position(*label) {
            sorted.insert(position, name.clone());
        }
    }
    let mut starts: Vec<usize> = Vec::new();
    for (start, _) in &sorted { starts.push(*start); }
    let mut table = vec![0u8; HEADER + starts.len() * ROW];
    table[..8].copy_from_slice(b"LKJPROF1");
    put(&mut table, 8, starts.len() as u64);
    put(&mut table, 16, PERIOD_US * 1000);
    for (index, (start, name)) in sorted.iter().enumerate() {
        let end = if index + 1 < starts.len() { starts[index + 1] } else { code_end };
        let base = IMAGE_BASE + CODE_OFFSET as u64;
        let at = HEADER + index * ROW;
        put(&mut table, at, base + *start as u64);
        put(&mut table, at + 8, base + end as u64);
        let mut frame = 0;
        let mut cursor = *start;
        while cursor + 4 <= end && cursor < *start + 256 {
            if a.bytes[cursor] == 0x55 && a.bytes[cursor + 1] == 0x48 && a.bytes[cursor + 2] == 0x89 && a.bytes[cursor + 3] == 0xe5 {
                frame = base + cursor as u64 + 4;
                break;
            }
            cursor += 1;
        }
        put(&mut table, at + 56, frame);
        let name_at = table.len();
        put(&mut table, at + 40, name_at as u64);
        put(&mut table, at + 48, name.len() as u64);
        table.extend_from_slice(name.as_bytes());
    }
    a.data_label(l.state, offset + bytes.len());
    word(bytes, 0);
    a.data_label(l.action, offset + bytes.len());
    word(bytes, 0);
    word(bytes, 0x0c000004); // SA_ONSTACK | SA_RESTORER | SA_SIGINFO
    word(bytes, 0);
    word(bytes, 0);
    a.data_label(l.timer, offset + bytes.len());
    word(bytes, 0); word(bytes, PERIOD_US);
    word(bytes, 0); word(bytes, PERIOD_US);
    a.data_label(l.altstack, offset + bytes.len());
    word(bytes, 0); word(bytes, 0); word(bytes, 65536);
    a.data_label(l.stack, offset + bytes.len());
    bytes.extend_from_slice(&[0u8; 65536]);
    a.data_label(l.path, offset + bytes.len());
    bytes.extend_from_slice(path.as_bytes());
    bytes.push(b'.');
    bytes.extend_from_slice(&[b'0'; 20]);
    bytes.push(0);
    a.data_label(l.table, offset + bytes.len());
    bytes.extend_from_slice(&table);
    a.data_label(l.end, offset + bytes.len());
}
fn get(bytes: &[u8], at: usize) -> u64 {
    let mut value = 0u64;
    for i in 0..8 {
        value |= (bytes[at + i] as u64) << (i * 8);
    }
    value
}
fn percent(value: u64, total: u64) -> String {
    let tenths = if total == 0 { 0 } else { (value.min(total) as f64 * 1000.0 / total as f64) as u64 };
    format!("{}.{:01}", tenths / 10, tenths % 10)
}
fn display_name(name: &str) -> String {
    let mut result = String::new();
    for character in name.chars() {
        let code = character as u32;
        if code < 32 || code == 127 || code == 0x2028 || code == 0x2029 {
            result.push_str(&format!("\\u{{{:x}}}", code));
        } else {
            result.push(character);
        }
    }
    result
}
pub fn report(bytes: &[u8]) -> Result<String, String> {
    let invalid = "E_PROFILE invalid or incomplete samples file";
    if bytes.len() < HEADER || &bytes[..8] != b"LKJPROF1" {
        return Err(invalid.to_string());
    }
    let count = get(bytes, 8) as usize;
    if count > (bytes.len() - HEADER) / ROW {
        return Err(invalid.to_string());
    }
    let total = get(bytes, 24);
    let mut names: Vec<String> = Vec::new();
    let mut self_counts: Vec<u64> = Vec::new();
    let mut inclusive: Vec<u64> = Vec::new();
    for i in 0..count {
        let row = HEADER + i * ROW;
        let start = get(bytes, row + 40) as usize;
        let len = get(bytes, row + 48) as usize;
        if start > bytes.len() || len > bytes.len() - start {
            return Err(invalid.to_string());
        }
        let name = match String::from_utf8(bytes[start..start + len].to_vec()) {
            Ok(name) => name,
            Err(_) => return Err(invalid.to_string()),
        };
        names.push(display_name(&name));
        self_counts.push(get(bytes, row + 16));
        inclusive.push(get(bytes, row + 24));
    }
    let mut out = format!("CPU samples: {total}; requested period {} us; inclusive counts each function once per sample\n", get(bytes, 16) / 1000);
    for mode in 0..2 {
        out.push_str(if mode == 0 { "Top self time\n" } else { "Top inclusive time\n" });
        out.push_str("self% incl% samples function\n");
        let mut used = vec![false; count];
        for _ in 0..14 {
            let mut best = None;
            let mut largest = 0u64;
            for i in 0..count {
                let value = if mode == 0 { self_counts[i] } else { inclusive[i] };
                if !used[i] && value > largest { best = Some(i); largest = value; }
            }
            if let Some(i) = best {
                used[i] = true;
                out.push_str(&format!("{} {} {} {}\n",
                    percent(self_counts[i], total), percent(inclusive[i], total),
                    largest, names[i]));
            }
        }
    }
    Ok(out)
}
