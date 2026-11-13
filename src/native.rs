//! Static Linux x86-64 encoding of the shared lowered program.
//! Heap and stack extents are selected at process startup from Linux controls.
//! Language values use
//! frame slots; only the platform primitives below issue system calls.
use crate::lower::{HostFunction, Instruction, Program, ScalarOp, Signature, ValueType};
use std::collections::BTreeMap;
mod io;

pub struct Entry {
    pub function: usize,
    pub arguments: Vec<i64>,
    pub result_fd: Option<u32>,
}
const IMAGE_BASE: u64 = 0x400000;
const CODE_OFFSET: usize = 4096;
const CONTEXT_GLOBALS: i64 = 80;
const HEAP_RESERVED: i64 = 0;
const STACK_RESERVED: i64 = 8;
const HEAP_USED: i64 = 16;
const INITIAL_STACK: i64 = 24;
const EXIT_STATUS: i64 = 32;
const PACKET: i64 = 40;
const A: u8 = 0;
const C: u8 = 1;
const D: u8 = 2;
const SP: u8 = 4;
const BP: u8 = 5;
const SI: u8 = 6;
const DI: u8 = 7;
const R8: u8 = 8;
const R9: u8 = 9;
const R10: u8 = 10;
const R11: u8 = 11;
const GLOBALS: u8 = 13;
const HEAP: u8 = 14;
const STACK: u8 = 15;
type Label = usize;
type Result<T> = std::result::Result<T, String>;

struct Assembler {
    bytes: Vec<u8>,
    labels: Vec<Option<u64>>,
    references: Vec<(usize, Label)>,
}
impl Assembler {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            labels: Vec::new(),
            references: Vec::new(),
        }
    }
    fn label(&mut self) -> Label {
        let id = self.labels.len();
        self.labels.push(None);
        id
    }
    fn bind(&mut self, label: Label) {
        assert!(self.labels[label].is_none());
        self.labels[label] = Some(IMAGE_BASE + CODE_OFFSET as u64 + self.bytes.len() as u64);
    }
    fn data_label(&mut self, label: Label, offset: usize) {
        assert!(self.labels[label].is_none());
        self.labels[label] = Some(IMAGE_BASE + offset as u64);
    }
    fn raw(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }
    fn rex(&mut self, wide: bool, r: u8, b: u8) {
        let prefix = 0x40 | (u8::from(wide) * 8) | (((r >> 3) & 1) * 4) | ((b >> 3) & 1);
        if prefix != 0x40 {
            self.bytes.push(prefix);
        }
    }
    fn rr(&mut self, opcode: u8, dst: u8, src: u8, wide: bool) {
        self.rex(wide, src, dst);
        self.raw(&[opcode, 0xc0 | ((src & 7) << 3) | (dst & 7)]);
    }
    fn mov(&mut self, dst: u8, src: u8) {
        self.rr(0x89, dst, src, true);
    }
    fn immediate(&mut self, reg: u8, value: u64) -> usize {
        self.rex(true, 0, reg);
        self.bytes.push(0xb8 | (reg & 7));
        let at = self.bytes.len();
        self.raw(&value.to_le_bytes());
        at
    }
    fn address(&mut self, reg: u8, label: Label) {
        let at = self.immediate(reg, 0);
        self.references.push((at, label));
    }
    fn memory(&mut self, opcode: &[u8], reg: u8, base: u8, offset: i64, wide: bool) {
        if let Ok(offset) = i32::try_from(offset) {
            self.rex(wide, reg, base);
            self.raw(opcode);
            self.bytes.push(0x80 | ((reg & 7) << 3) | (base & 7));
            if base & 7 == 4 {
                self.bytes.push(0x24);
            }
            self.raw(&offset.to_le_bytes());
        } else {
            let temporary = [R10, R11, R9]
                .into_iter()
                .find(|r| *r != reg && *r != base)
                .unwrap();
            self.immediate(temporary, offset as u64);
            self.rr(0x01, temporary, base, true);
            self.memory(opcode, reg, temporary, 0, wide);
        }
    }
    fn load(&mut self, reg: u8, base: u8, offset: i64) {
        self.memory(&[0x8b], reg, base, offset, true);
    }
    fn store(&mut self, base: u8, offset: i64, reg: u8) {
        self.memory(&[0x89], reg, base, offset, true);
    }
    fn lea(&mut self, reg: u8, base: u8, offset: i64) {
        self.memory(&[0x8d], reg, base, offset, true);
    }
    fn compare(&mut self, left: u8, right: u8) {
        self.rr(0x39, left, right, true);
    }
    fn test(&mut self, reg: u8, wide: bool) {
        self.rr(0x85, reg, reg, wide);
    }
    fn shift(&mut self, reg: u8, kind: u8, count: u8) {
        self.rex(true, 0, reg);
        self.raw(&[0xc1, 0xc0 | (kind << 3) | (reg & 7), count]);
    }
    fn jump(&mut self, label: Label) {
        self.address(R11, label);
        self.raw(&[0x41, 0xff, 0xe3]);
    }
    fn condition(&mut self, condition: u8, label: Label) {
        self.raw(&[0x70 | (condition ^ 1), 13]);
        self.jump(label);
    }
    fn call(&mut self, label: Label) {
        self.address(R11, label);
        self.raw(&[0x41, 0xff, 0xd3]);
    }
    fn call_register(&mut self, reg: u8) {
        self.rex(false, 0, reg);
        self.raw(&[0xff, 0xd0 | (reg & 7)]);
    }
    fn syscall(&mut self) {
        self.raw(&[0x0f, 0x05]);
    }
    fn ret(&mut self) {
        self.bytes.push(0xc3);
    }
    fn finish(mut self) -> Result<Vec<u8>> {
        for (at, label) in self.references {
            let address = self.labels[label].ok_or("unbound native code label")?;
            self.bytes[at..at + 8].copy_from_slice(&address.to_le_bytes());
        }
        Ok(self.bytes)
    }
}

#[derive(Clone, Copy)]
struct Platform {
    io: io::Io,
    trap: Label,
    exhausted: Label,
    stack_exhausted: Label,
    memory_fault: Label,
    grow: Label,
    copy: Label,
    complete: Label,
    context: Label,
    literals: Label,
    table: Label,
    arguments: Label,
    stdout: Label,
    signal_action: Label,
    system_info: Label,
    resource_limit: Label,
    diagnostic_buffer: Label,
    trap_code: usize,
    trap_line: usize,
}
impl Platform {
    fn new(a: &mut Assembler, program: &Program) -> Self {
        Self {
            io: io::Io::new(a),
            trap: a.label(),
            exhausted: a.label(),
            stack_exhausted: a.label(),
            memory_fault: a.label(),
            grow: a.label(),
            copy: a.label(),
            complete: a.label(),
            context: a.label(),
            literals: a.label(),
            table: a.label(),
            arguments: a.label(),
            stdout: a.label(),
            signal_action: a.label(),
            system_info: a.label(),
            resource_limit: a.label(),
            diagnostic_buffer: a.label(),
            trap_code: program.trap_code_global,
            trap_line: program.trap_line_global,
        }
    }
    fn global(index: usize) -> i64 {
        CONTEXT_GLOBALS + (index as i64) * 8
    }
    fn failure(&self, a: &mut Assembler, code: u64, line: u64) {
        a.immediate(A, code);
        a.store(GLOBALS, Self::global(self.trap_code), A);
        a.immediate(A, line);
        a.store(GLOBALS, Self::global(self.trap_line), A);
        a.jump(self.trap);
    }
    fn range(&self, a: &mut Assembler, address: u8, length: u8) {
        self.range_to(a, address, length, self.memory_fault);
    }
    fn range_to(&self, a: &mut Assembler, address: u8, length: u8, failure: Label) {
        a.load(R10, GLOBALS, HEAP_USED);
        a.compare(address, R10);
        a.condition(7, failure);
        a.rr(0x29, R10, address, true);
        a.compare(length, R10);
        a.condition(7, failure);
    }
}

/// Linux x86-64 sysinfo and prlimit64 have fixed ABI layouts. Resource readings
/// occur in the artifact process, before its own heap and stack are mapped. Each
/// gets a quarter of the tighter physical/address-space extent; the other half
/// remains available for the image, initial process stack and host mappings.
fn startup_extents(a: &mut Assembler, p: Platform, initial: u64) {
    a.immediate(A, 99); // sysinfo
    a.address(DI, p.system_info);
    a.syscall();
    a.test(A, true);
    a.condition(8, p.exhausted);
    a.load(A, DI, 32); // totalram, excluding swap
    a.memory(&[0x8b], C, DI, 104, false); // mem_unit: u32
    a.raw(&[0x48, 0x0f, 0xaf, 0xc1]); // imul rax, rcx
    a.condition(0, p.exhausted);
    a.mov(R9, A);
    a.immediate(A, 302); // prlimit64(0, RLIMIT_AS, NULL, &old)
    a.immediate(DI, 0);
    a.immediate(SI, 9);
    a.immediate(D, 0);
    a.address(R10, p.resource_limit);
    a.syscall();
    a.test(A, true);
    a.condition(8, p.exhausted);
    a.load(A, R10, 0);
    a.compare(A, R9);
    let physical = a.label();
    a.condition(3, physical);
    a.mov(R9, A);
    a.bind(physical);
    a.shift(R9, 5, 2);
    a.immediate(A, !65535_u64);
    a.rr(0x21, R9, A, true);
    a.immediate(A, initial.max(65536));
    a.compare(R9, A);
    a.condition(2, p.exhausted);
    a.store(GLOBALS, HEAP_RESERVED, R9);
    a.store(GLOBALS, STACK_RESERVED, R9);
}

// Format traps without allocating or using the exhausted language stack. R8
// moves backward through a 128-byte buffer; the maximum record is 81 bytes.
fn diagnostic_text(a: &mut Assembler, text: &str) {
    for byte in text.bytes().rev() {
        a.immediate(A, 1);
        a.rr(0x29, R8, A, true);
        a.immediate(A, u64::from(byte));
        a.memory(&[0x88], A, R8, 0, false);
    }
}
fn diagnostic_decimal(a: &mut Assembler, minimum: u64) {
    a.immediate(C, 10);
    a.immediate(R9, 0);
    a.immediate(R10, 1);
    let digit = a.label();
    a.bind(digit);
    a.immediate(D, 0);
    a.raw(&[0x48, 0xf7, 0xf1]); // unsigned rdx:rax / rcx
    a.rr(0x29, R8, R10, true);
    a.rr(0x01, R9, R10, true);
    a.immediate(SI, u64::from(b'0'));
    a.rr(0x01, D, SI, true);
    a.memory(&[0x88], D, R8, 0, false);
    a.test(A, true);
    a.condition(5, digit);
    a.immediate(SI, minimum);
    a.compare(R9, SI);
    a.condition(2, digit);
}
fn standalone_diagnostic(a: &mut Assembler, p: Platform) {
    let complete = a.label();
    a.load(A, GLOBALS, PACKET);
    a.test(A, true);
    a.condition(4, complete);
    a.address(R8, p.diagnostic_buffer);
    a.immediate(C, 128);
    a.rr(0x01, R8, C, true);
    diagnostic_text(a, "\n");
    a.load(A, GLOBALS, PACKET);
    a.immediate(C, 8);
    a.compare(A, C);
    let heap = a.label();
    let line = a.label();
    a.condition(5, heap);
    diagnostic_text(a, " (stack extent exhausted)");
    a.jump(line);
    a.bind(heap);
    a.immediate(C, 6);
    a.compare(A, C);
    a.condition(5, line);
    diagnostic_text(a, " (heap allocation failed)");
    a.bind(line);
    a.load(A, GLOBALS, PACKET + 16);
    diagnostic_decimal(a, 1);
    diagnostic_text(a, " at line ");
    a.load(A, GLOBALS, PACKET);
    diagnostic_decimal(a, 4);
    diagnostic_text(a, "R");
    a.mov(SI, R8);
    a.address(D, p.diagnostic_buffer);
    a.immediate(A, 128);
    a.rr(0x01, D, A, true);
    a.rr(0x29, D, SI, true);
    let writing = a.label();
    a.bind(writing);
    a.immediate(A, 1);
    a.immediate(DI, 2);
    a.syscall();
    a.immediate(C, (-4_i64) as u64);
    a.compare(A, C);
    a.condition(4, writing);
    a.test(A, true);
    a.condition(14, complete);
    a.rr(0x01, SI, A, true);
    a.rr(0x29, D, A, true);
    a.test(D, true);
    a.condition(5, writing);
    a.bind(complete);
}

fn platform(
    a: &mut Assembler,
    p: Platform,
    program: &Program,
    entry: &Entry,
    functions: &[Label],
    initial: u64,
) {
    // Startup performs only resource observation and capability setup. The ELF
    // contains no observation of the machine that compiled it.
    a.address(GLOBALS, p.context);
    a.store(GLOBALS, INITIAL_STACK, SP);
    a.immediate(A, 1);
    a.store(GLOBALS, Platform::global(p.trap_line), A);
    startup_extents(a, p, initial);
    {
        a.immediate(A, 13);
        a.immediate(DI, 13);
        a.address(SI, p.signal_action);
        a.immediate(D, 0);
        a.immediate(R10, 8);
        a.syscall();
        a.test(A, true);
        a.condition(8, p.exhausted);
    }
    a.immediate(A, 9);
    a.immediate(DI, 0);
    a.load(SI, GLOBALS, HEAP_RESERVED);
    a.immediate(D, 0);
    a.immediate(R10, 0x4022);
    a.immediate(R8, u64::MAX);
    a.immediate(R9, 0);
    a.syscall();
    a.immediate(C, (-4095i64) as u64);
    a.compare(A, C);
    a.condition(3, p.exhausted);
    a.mov(HEAP, A);
    a.immediate(A, 10);
    a.mov(DI, HEAP);
    a.immediate(SI, initial);
    a.immediate(D, 3);
    a.syscall();
    a.test(A, true);
    a.condition(8, p.exhausted);
    a.immediate(A, 9);
    a.immediate(DI, 0);
    a.load(SI, GLOBALS, STACK_RESERVED);
    a.immediate(D, 3);
    a.immediate(R10, 0x4022);
    a.immediate(R8, u64::MAX);
    a.immediate(R9, 0);
    a.syscall();
    a.immediate(C, (-4095i64) as u64);
    a.compare(A, C);
    a.condition(3, p.stack_exhausted);
    a.mov(STACK, A);
    a.load(C, GLOBALS, STACK_RESERVED);
    a.rr(0x01, A, C, true);
    a.mov(SP, A);
    a.immediate(BP, 0);
    a.address(SI, p.literals);
    a.mov(DI, HEAP);
    a.immediate(A, program.data_offset);
    a.rr(0x01, DI, A, true);
    a.immediate(C, program.data.len() as u64);
    a.raw(&[0xfc, 0xf3, 0xa4]);
    a.address(DI, p.arguments);
    a.call(functions[entry.function]);
    a.store(GLOBALS, PACKET + 8, A);
    a.immediate(A, 0);
    a.store(GLOBALS, PACKET, A);
    a.load(A, GLOBALS, EXIT_STATUS);
    a.store(GLOBALS, PACKET + 16, A);
    a.jump(p.complete);

    a.bind(p.exhausted);
    a.immediate(A, 6);
    a.store(GLOBALS, Platform::global(p.trap_code), A);
    a.jump(p.trap);
    a.bind(p.stack_exhausted);
    a.immediate(A, 8);
    a.store(GLOBALS, Platform::global(p.trap_code), A);
    a.jump(p.trap);
    a.bind(p.memory_fault);
    p.failure(a, 3, 0);
    a.bind(p.trap);
    a.load(A, GLOBALS, Platform::global(p.trap_code));
    a.test(A, true);
    let known = a.label();
    a.condition(5, known);
    a.immediate(A, 4);
    a.bind(known);
    a.store(GLOBALS, PACKET, A);
    a.load(A, GLOBALS, Platform::global(p.trap_line));
    a.store(GLOBALS, PACKET + 16, A);
    a.immediate(A, 0);
    a.store(GLOBALS, PACKET + 8, A);
    a.jump(p.complete);
    a.bind(p.complete);
    if let Some(fd) = entry.result_fd {
        a.lea(SI, GLOBALS, PACKET);
        a.immediate(D, 24);
        let writing = a.label();
        let exit = a.label();
        a.bind(writing);
        a.immediate(A, 1);
        a.immediate(DI, u64::from(fd));
        a.syscall();
        a.test(A, true);
        a.condition(14, exit);
        a.rr(0x01, SI, A, true);
        a.rr(0x29, D, A, true);
        a.test(D, true);
        a.condition(5, writing);
        a.bind(exit);
    } else {
        standalone_diagnostic(a, p);
    }
    a.load(A, GLOBALS, PACKET);
    a.test(A, true);
    a.immediate(DI, 1);
    let exiting = a.label();
    a.condition(5, exiting);
    a.load(DI, GLOBALS, EXIT_STATUS);
    a.bind(exiting);
    a.immediate(A, 60);
    a.syscall();
    a.raw(&[0x0f, 0x0b]);

    a.bind(p.grow); // rdi: extra logical pages. Return former pages or -1.
    a.mov(R9, DI);
    a.immediate(C, u64::MAX >> 16);
    a.compare(R9, C);
    let failed = a.label();
    let grown = a.label();
    a.condition(7, failed);
    a.shift(R9, 4, 16);
    a.load(R8, GLOBALS, HEAP_USED);
    a.mov(C, R8);
    a.rr(0x01, C, R9, true);
    a.condition(2, failed);
    a.load(A, GLOBALS, HEAP_RESERVED);
    a.compare(C, A);
    a.condition(7, failed);
    a.test(R9, true);
    a.condition(4, grown);
    a.mov(DI, HEAP);
    a.rr(0x01, DI, R8, true);
    a.mov(SI, R9);
    a.immediate(D, 3);
    a.immediate(A, 10);
    a.syscall();
    a.test(A, true);
    a.condition(8, failed);
    a.rr(0x01, R9, R8, true);
    a.store(GLOBALS, HEAP_USED, R9);
    a.bind(grown);
    a.mov(A, R8);
    a.shift(A, 5, 16);
    a.ret();
    a.bind(failed);
    a.immediate(A, u64::MAX);
    a.ret();

    a.bind(p.copy); // rdi, rsi, rdx: destination/source/count heap offsets.
    p.range(a, DI, D);
    p.range(a, SI, D);
    a.test(D, true);
    let copy_done = a.label();
    let forward = a.label();
    a.condition(4, copy_done);
    a.compare(DI, SI);
    a.condition(6, forward);
    a.mov(A, SI);
    a.rr(0x01, A, D, true);
    a.compare(DI, A);
    a.condition(3, forward);
    a.rr(0x01, DI, HEAP, true);
    a.rr(0x01, SI, HEAP, true);
    a.rr(0x01, DI, D, true);
    a.rr(0x01, SI, D, true);
    a.immediate(A, 1);
    a.rr(0x29, DI, A, true);
    a.rr(0x29, SI, A, true);
    a.mov(C, D);
    a.raw(&[0xfd, 0xf3, 0xa4, 0xfc]);
    a.jump(copy_done);
    a.bind(forward);
    a.rr(0x01, DI, HEAP, true);
    a.rr(0x01, SI, HEAP, true);
    a.mov(C, D);
    a.raw(&[0xfc, 0xf3, 0xa4]);
    a.bind(copy_done);
    a.ret();

    io::emit(a, p);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RegionKind {
    Function,
    Block,
    Loop,
    If,
}
struct Region {
    kind: RegionKind,
    result: Option<ValueType>,
    height: usize,
    start: Label,
    end: Label,
    otherwise: Label,
    entered: bool,
    end_reachable: bool,
    then_reachable: bool,
    has_else: bool,
}
struct Body<'a> {
    a: &'a mut Assembler,
    p: Platform,
    functions: &'a [Label],
    signatures: &'a [Signature],
    signature_ids: &'a BTreeMap<Signature, u64>,
    table_length: usize,
    globals: &'a [crate::lower::Global],
    locals: usize,
    height: usize,
    maximum: usize,
    reachable: bool,
    regions: Vec<Region>,
}
impl Body<'_> {
    fn offset(&self, index: usize) -> i64 {
        -((index as i64 + 1) * 8)
    }
    fn operand(&self, index: usize) -> i64 {
        self.offset(self.locals + index)
    }
    fn pop(&mut self, reg: u8) -> Result<()> {
        self.height = self
            .height
            .checked_sub(1)
            .ok_or("invalid lowered operand stack")?;
        self.a.load(reg, BP, self.operand(self.height));
        Ok(())
    }
    fn push(&mut self, reg: u8) {
        self.a.store(BP, self.operand(self.height), reg);
        self.height += 1;
        self.maximum = self.maximum.max(self.height);
    }
    fn leave(&mut self) {
        self.a.raw(&[0xc9, 0xc3]);
    }
    fn result(&mut self, result: Option<ValueType>) -> Result<()> {
        if result.is_some() {
            self.pop(A)?;
        } else {
            self.a.immediate(A, 0);
        }
        self.leave();
        self.reachable = false;
        Ok(())
    }
    fn begin(&mut self, kind: RegionKind, result: Option<ValueType>) -> Result<()> {
        let start = self.a.label();
        let end = self.a.label();
        let otherwise = self.a.label();
        if kind == RegionKind::If && self.reachable {
            self.pop(A)?;
            self.a.test(A, false);
            self.a.condition(4, otherwise);
        }
        self.a.bind(start);
        self.regions.push(Region {
            kind,
            result,
            height: self.height,
            start,
            end,
            otherwise,
            entered: self.reachable,
            end_reachable: false,
            then_reachable: false,
            has_else: false,
        });
        Ok(())
    }
    fn floating(&mut self, op: ScalarOp) -> Result<()> {
        use ScalarOp::*;
        // xmm0 carries the left/input value and xmm1 the right/threshold.
        let to_xmm0: &[u8] = &[0x66, 0x48, 0x0f, 0x6e, 0xc0];
        let to_xmm1: &[u8] = &[0x66, 0x48, 0x0f, 0x6e, 0xc9];
        let from_xmm0: &[u8] = &[0x66, 0x48, 0x0f, 0x7e, 0xc0];
        let compare: &[u8] = &[0x66, 0x0f, 0x2e, 0xc1];
        match op {
            F64Add | F64Sub | F64Mul | F64Div => {
                self.a.raw(to_xmm0);
                self.a.raw(to_xmm1);
                self.a.raw(&[
                    0xf2,
                    0x0f,
                    match op {
                        F64Add => 0x58,
                        F64Sub => 0x5c,
                        F64Mul => 0x59,
                        _ => 0x5e,
                    },
                    0xc1,
                ]);
                self.a.raw(from_xmm0);
            }
            F64Eq | F64Ne | F64Lt | F64Gt | F64Le | F64Ge => {
                self.a.raw(to_xmm0);
                self.a.raw(to_xmm1);
                self.a.raw(compare);
                let condition = match op {
                    F64Eq => 4,
                    F64Ne => 5,
                    F64Lt => 2,
                    F64Gt => 7,
                    F64Le => 6,
                    _ => 3,
                };
                self.a.raw(&[0x0f, 0x90 | condition, 0xc0]);
                if op == F64Ne {
                    self.a.raw(&[0x0f, 0x9a, 0xc2, 0x08, 0xd0]);
                } else if matches!(op, F64Eq | F64Lt | F64Le) {
                    self.a.raw(&[0x0f, 0x9b, 0xc2, 0x20, 0xd0]);
                }
                self.a.raw(&[0x0f, 0xb6, 0xc0]);
            }
            F64ConvertI64S => {
                self.a.raw(&[0xf2, 0x48, 0x0f, 0x2a, 0xc0]);
                self.a.raw(from_xmm0);
            }
            F64ConvertI64U => {
                let small = self.a.label();
                let end = self.a.label();
                self.a.test(A, true);
                self.a.condition(9, small);
                self.a.mov(C, A);
                self.a.immediate(D, 1);
                self.a.rr(0x21, C, D, true);
                self.a.shift(A, 5, 1);
                self.a.rr(0x09, A, C, true);
                self.a
                    .raw(&[0xf2, 0x48, 0x0f, 0x2a, 0xc0, 0xf2, 0x0f, 0x58, 0xc0]);
                self.a.jump(end);
                self.a.bind(small);
                self.a.raw(&[0xf2, 0x48, 0x0f, 0x2a, 0xc0]);
                self.a.bind(end);
                self.a.raw(from_xmm0);
            }
            I64TruncSatF64S | I64TruncSatF64U => {
                let zero = self.a.label();
                let minimum = self.a.label();
                let maximum = self.a.label();
                let end = self.a.label();
                self.a.raw(to_xmm0);
                self.a.raw(&[0x66, 0x0f, 0x2e, 0xc0]);
                self.a.condition(10, zero);
                if op == I64TruncSatF64S {
                    self.a.immediate(C, (-9223372036854775808.0f64).to_bits());
                    self.a.raw(to_xmm1);
                    self.a.raw(compare);
                    self.a.condition(6, minimum);
                    self.a.immediate(C, 9223372036854775808.0f64.to_bits());
                    self.a.raw(to_xmm1);
                    self.a.raw(compare);
                    self.a.condition(3, maximum);
                    self.a.raw(&[0xf2, 0x48, 0x0f, 0x2c, 0xc0]);
                    self.a.jump(end);
                } else {
                    self.a.raw(&[0x0f, 0x57, 0xc9]);
                    self.a.raw(compare);
                    self.a.condition(6, zero);
                    self.a.immediate(C, 18446744073709551616.0f64.to_bits());
                    self.a.raw(to_xmm1);
                    self.a.raw(compare);
                    self.a.condition(3, maximum);
                    self.a.immediate(C, 9223372036854775808.0f64.to_bits());
                    self.a.raw(to_xmm1);
                    self.a.raw(compare);
                    let small = self.a.label();
                    self.a.condition(2, small);
                    self.a
                        .raw(&[0xf2, 0x0f, 0x5c, 0xc1, 0xf2, 0x48, 0x0f, 0x2c, 0xc0]);
                    self.a.immediate(C, 1u64 << 63);
                    self.a.rr(0x09, A, C, true);
                    self.a.jump(end);
                    self.a.bind(small);
                    self.a.raw(&[0xf2, 0x48, 0x0f, 0x2c, 0xc0]);
                    self.a.jump(end);
                }
                self.a.bind(zero);
                self.a.immediate(A, 0);
                self.a.jump(end);
                self.a.bind(minimum);
                self.a.immediate(A, i64::MIN as u64);
                self.a.jump(end);
                self.a.bind(maximum);
                self.a.immediate(
                    A,
                    if op == I64TruncSatF64S {
                        i64::MAX as u64
                    } else {
                        u64::MAX
                    },
                );
                self.a.bind(end);
            }
            _ => return Err("invalid native scalar instruction".to_string()),
        }
        Ok(())
    }
    fn scalar(&mut self, op: ScalarOp) -> Result<()> {
        use ScalarOp::*;
        let (count, _, _) = op.signature();
        if count == 2 {
            self.pop(C)?;
        }
        self.pop(A)?;
        match op {
            I32Eqz | I64Eqz => {
                self.a.test(A, op == I64Eqz);
                self.a.raw(&[0x0f, 0x94, 0xc0, 0x0f, 0xb6, 0xc0]);
            }
            I32And => self.a.rr(0x21, A, C, false),
            I32Or => self.a.rr(0x09, A, C, false),
            I64Add => self.a.rr(0x01, A, C, true),
            I64Sub => self.a.rr(0x29, A, C, true),
            I64And => self.a.rr(0x21, A, C, true),
            I64Or => self.a.rr(0x09, A, C, true),
            I64Xor => self.a.rr(0x31, A, C, true),
            I64Mul => self.a.raw(&[0x48, 0x0f, 0xaf, 0xc1]),
            I64DivS | I64RemS => {
                self.a.raw(&[0x48, 0x99, 0x48, 0xf7, 0xf9]);
                if op == I64RemS {
                    self.a.mov(A, D);
                }
            }
            I64DivU | I64RemU => {
                self.a.rr(0x31, D, D, false);
                self.a.raw(&[0x48, 0xf7, 0xf1]);
                if op == I64RemU {
                    self.a.mov(A, D);
                }
            }
            I64Shl | I64ShrS | I64ShrU => self.a.raw(&[
                0x48,
                0xd3,
                match op {
                    I64Shl => 0xe0,
                    I64ShrS => 0xf8,
                    _ => 0xe8,
                },
            ]),
            I64Eq | I64Ne | I64LtS | I64LtU | I64GtS | I64GtU | I64LeS | I64LeU | I64GeS
            | I64GeU => {
                self.a.compare(A, C);
                let cc = match op {
                    I64Eq => 4,
                    I64Ne => 5,
                    I64LtS => 12,
                    I64LtU => 2,
                    I64GtS => 15,
                    I64GtU => 7,
                    I64LeS => 14,
                    I64LeU => 6,
                    I64GeS => 13,
                    _ => 3,
                };
                self.a.raw(&[0x0f, 0x90 | cc, 0xc0, 0x0f, 0xb6, 0xc0]);
            }
            I32WrapI64 => self.a.rr(0x89, A, A, false),
            I64ExtendI32U => self.a.rr(0x89, A, A, false),
            I64ReinterpretF64 | F64ReinterpretI64 => {}
            _ => self.floating(op)?,
        }
        self.push(A);
        Ok(())
    }
    fn instruction(&mut self, instruction: &Instruction) -> Result<()> {
        match instruction {
            Instruction::Block(result) => return self.begin(RegionKind::Block, *result),
            Instruction::Loop(result) => return self.begin(RegionKind::Loop, *result),
            Instruction::If(result) => return self.begin(RegionKind::If, *result),
            Instruction::Else => {
                let region = self.regions.last_mut().ok_or("lowered else without if")?;
                if region.kind != RegionKind::If || region.has_else {
                    return Err("invalid lowered else".to_string());
                }
                region.then_reachable = self.reachable;
                region.has_else = true;
                if self.reachable {
                    self.a.jump(region.end);
                }
                self.a.bind(region.otherwise);
                self.height = region.height;
                self.reachable = region.entered;
                return Ok(());
            }
            Instruction::End => {
                let region = self.regions.pop().ok_or("unbalanced lowered end")?;
                if region.kind == RegionKind::If && !region.has_else {
                    self.a.bind(region.otherwise);
                }
                self.a.bind(region.end);
                self.reachable = self.reachable
                    || region.end_reachable
                    || (region.kind == RegionKind::If
                        && if region.has_else {
                            region.then_reachable
                        } else {
                            region.entered
                        });
                self.height = region.height + usize::from(region.result.is_some());
                if region.kind == RegionKind::Function && self.reachable {
                    self.result(region.result)?;
                }
                return Ok(());
            }
            _ => {}
        }
        if !self.reachable {
            return Ok(());
        }
        match instruction {
            Instruction::I32(value) => {
                self.a.immediate(A, u64::from(*value as u32));
                self.push(A);
            }
            Instruction::I64(value) => {
                self.a.immediate(A, *value as u64);
                self.push(A);
            }
            Instruction::LocalGet(local) => {
                if *local >= self.locals {
                    return Err("invalid lowered local".to_string());
                }
                self.a.load(A, BP, self.offset(*local));
                self.push(A);
            }
            Instruction::LocalSet(local) | Instruction::LocalTee(local) => {
                if *local >= self.locals {
                    return Err("invalid lowered local".to_string());
                }
                self.pop(A)?;
                self.a.store(BP, self.offset(*local), A);
                if matches!(instruction, Instruction::LocalTee(_)) {
                    self.push(A);
                }
            }
            Instruction::GlobalGet(index) => {
                if *index >= self.globals.len() {
                    return Err("invalid lowered global".to_string());
                }
                self.a.load(A, GLOBALS, Platform::global(*index));
                self.push(A);
            }
            Instruction::GlobalSet(index) => {
                if !self
                    .globals
                    .get(*index)
                    .is_some_and(|global| global.mutable)
                {
                    return Err("invalid or immutable lowered global".to_string());
                }
                self.pop(A)?;
                self.a.store(GLOBALS, Platform::global(*index), A);
            }
            Instruction::Scalar(op) => self.scalar(*op)?,
            Instruction::Load {
                bytes,
                signed,
                offset,
            } => {
                self.pop(A)?;
                self.a.immediate(C, *offset);
                self.a.rr(0x01, A, C, true);
                self.a.condition(2, self.p.memory_fault);
                self.a.immediate(C, u64::from(*bytes));
                self.p.range(self.a, A, C);
                self.a.rr(0x01, A, HEAP, true);
                match (*bytes, *signed) {
                    (1, false) => self.a.memory(&[0x0f, 0xb6], A, A, 0, true),
                    (1, true) => self.a.memory(&[0x0f, 0xbe], A, A, 0, true),
                    (2, false) => self.a.memory(&[0x0f, 0xb7], A, A, 0, true),
                    (2, true) => self.a.memory(&[0x0f, 0xbf], A, A, 0, true),
                    (4, false) => self.a.memory(&[0x8b], A, A, 0, false),
                    (4, true) => self.a.memory(&[0x63], A, A, 0, true),
                    (8, _) => self.a.load(A, A, 0),
                    _ => return Err("invalid lowered load width".to_string()),
                }
                self.push(A);
            }
            Instruction::Store { bytes, offset } => {
                self.pop(C)?;
                self.pop(A)?;
                self.a.immediate(D, *offset);
                self.a.rr(0x01, A, D, true);
                self.a.condition(2, self.p.memory_fault);
                self.a.immediate(D, u64::from(*bytes));
                self.p.range(self.a, A, D);
                self.a.rr(0x01, A, HEAP, true);
                match bytes {
                    1 => self.a.memory(&[0x88], C, A, 0, false),
                    2 => {
                        self.a.raw(&[0x66]);
                        self.a.memory(&[0x89], C, A, 0, false);
                    }
                    4 => self.a.memory(&[0x89], C, A, 0, false),
                    8 => self.a.store(A, 0, C),
                    _ => return Err("invalid lowered store width".to_string()),
                }
            }
            Instruction::Drop => {
                self.height = self.height.checked_sub(1).ok_or("lowered drop underflow")?;
            }
            Instruction::Branch { depth, conditional } => {
                if *conditional {
                    self.pop(A)?;
                }
                let distance = depth.checked_add(1).ok_or("invalid lowered branch depth")?;
                let target = self
                    .regions
                    .len()
                    .checked_sub(distance)
                    .ok_or("invalid lowered branch depth")?;
                let region = &mut self.regions[target];
                let loop_target = region.kind == RegionKind::Loop;
                let destination = if loop_target {
                    region.start
                } else {
                    region.end
                };
                let result_slot =
                    (!loop_target && region.result.is_some()).then_some(region.height);
                if !loop_target {
                    region.end_reachable = true;
                }
                // A conditional branch carries its result only on its taken edge.
                // The not-taken edge retains all values beneath the condition.
                let skipped = (*conditional && result_slot.is_some()).then(|| self.a.label());
                if *conditional {
                    self.a.test(A, false);
                    if let Some(label) = skipped {
                        self.a.condition(4, label);
                    } else {
                        self.a.condition(5, destination);
                    }
                }
                if let Some(slot) = result_slot {
                    let from = self
                        .height
                        .checked_sub(1)
                        .ok_or("missing lowered branch result")?;
                    self.a.load(C, BP, self.operand(from));
                    self.a.store(BP, self.operand(slot), C);
                }
                if !conditional || skipped.is_some() {
                    self.a.jump(destination);
                }
                if let Some(label) = skipped {
                    self.a.bind(label);
                }
                if !conditional {
                    self.reachable = false;
                }
            }
            Instruction::Call(index) => {
                let signature = self.signatures.get(*index).ok_or("invalid lowered call")?;
                self.height = self
                    .height
                    .checked_sub(signature.parameters.len())
                    .ok_or("lowered call operands missing")?;
                self.a.lea(DI, BP, self.operand(self.height));
                self.a.call(self.functions[*index]);
                if signature.result.is_some() {
                    self.push(A);
                }
            }
            Instruction::CallIndirect(signature) => {
                self.pop(A)?;
                self.a.immediate(C, self.table_length as u64);
                self.a.compare(A, C);
                self.a.condition(3, self.p.memory_fault);
                self.a.shift(A, 4, 4);
                self.a.address(C, self.p.table);
                self.a.rr(0x01, A, C, true);
                self.a.load(C, A, 8);
                self.a.immediate(D, self.signature_ids[signature]);
                self.a.compare(C, D);
                self.a.condition(5, self.p.memory_fault);
                self.a.load(R11, A, 0);
                self.height = self
                    .height
                    .checked_sub(signature.parameters.len())
                    .ok_or("lowered indirect operands missing")?;
                self.a.lea(DI, BP, self.operand(self.height));
                self.a.call_register(R11);
                if signature.result.is_some() {
                    self.push(A);
                }
            }
            Instruction::Return => {
                let result = self
                    .regions
                    .first()
                    .ok_or("return outside function")?
                    .result;
                self.result(result)?;
            }
            Instruction::Unreachable => {
                self.a.jump(self.p.trap);
                self.reachable = false;
            }
            Instruction::MemorySize => {
                self.a.load(A, GLOBALS, HEAP_USED);
                self.a.shift(A, 5, 16);
                self.push(A);
            }
            Instruction::MemoryGrow => {
                self.pop(DI)?;
                self.a.call(self.p.grow);
                self.push(A);
            }
            Instruction::MemoryCopy => {
                self.pop(D)?;
                self.pop(SI)?;
                self.pop(DI)?;
                self.a.call(self.p.copy);
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}

fn function(
    a: &mut Assembler,
    p: Platform,
    program: &Program,
    index: usize,
    functions: &[Label],
    signatures: &[Signature],
    ids: &BTreeMap<Signature, u64>,
) -> Result<()> {
    let function = &program.functions[index];
    let locals = function
        .signature
        .parameters
        .len()
        .checked_add(function.code.locals.len())
        .ok_or("native local count overflow")?;
    locals
        .checked_add(function.code.instructions.len())
        .and_then(|n| n.checked_add(1))
        .and_then(|n| n.checked_mul(8))
        .filter(|n| *n < i64::MAX as usize)
        .ok_or("native frame exceeds signed offset representation")?;
    a.bind(functions[program.imports.len() + index]);
    let required = a.immediate(A, 0);
    a.mov(C, SP);
    a.rr(0x29, C, A, true);
    a.condition(2, p.stack_exhausted);
    a.compare(C, STACK);
    a.condition(2, p.stack_exhausted);
    a.raw(&[0x55]);
    a.mov(BP, SP);
    let allocated = a.immediate(A, 0);
    a.rr(0x29, SP, A, true);
    for (param, ty) in function.signature.parameters.iter().enumerate() {
        a.load(A, DI, -(param as i64) * 8);
        if *ty == ValueType::I32 {
            a.rr(0x89, A, A, false);
        }
        a.store(BP, -((param as i64 + 1) * 8), A);
    }
    for local in function.signature.parameters.len()..locals {
        a.immediate(A, 0);
        a.store(BP, -((local as i64 + 1) * 8), A);
    }
    let mut body = Body {
        a,
        p,
        functions,
        signatures,
        signature_ids: ids,
        table_length: program.table.len(),
        globals: &program.globals,
        locals,
        height: 0,
        maximum: 0,
        reachable: true,
        regions: Vec::new(),
    };
    body.begin(RegionKind::Function, function.signature.result)?;
    for instruction in &function.code.instructions {
        body.instruction(instruction)?;
    }
    if !body.regions.is_empty() {
        return Err("unfinished lowered control region".to_string());
    }
    let frame = ((locals + body.maximum) as u64 * 8 + 15) & !15;
    body.a.bytes[allocated..allocated + 8].copy_from_slice(&frame.to_le_bytes());
    body.a.bytes[required..required + 8].copy_from_slice(&(frame + 24).to_le_bytes());
    Ok(())
}

fn put16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}
fn put32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn put64(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}
fn segment(bytes: &mut [u8], at: usize, flags: u32, offset: u64, size: u64) {
    put32(bytes, at, 1);
    put32(bytes, at + 4, flags);
    put64(bytes, at + 8, offset);
    put64(bytes, at + 16, IMAGE_BASE + offset);
    put64(bytes, at + 24, IMAGE_BASE + offset);
    put64(bytes, at + 32, size);
    put64(bytes, at + 40, size);
    put64(bytes, at + 48, 4096);
}
pub fn encode(program: &Program, entry: Entry) -> Result<Vec<u8>> {
    let mut signatures = program
        .imports
        .iter()
        .map(|i| i.signature.clone())
        .collect::<Vec<_>>();
    signatures.extend(program.functions.iter().map(|f| f.signature.clone()));
    let selected = signatures
        .get(entry.function)
        .ok_or("native entry is not a function")?;
    if selected.parameters.len() != entry.arguments.len() {
        return Err("native entry argument count differs".to_string());
    }
    let initial = program
        .minimum_memory_pages
        .checked_mul(65536)
        .ok_or("native initial heap extent overflow")?;
    if program
        .data_offset
        .checked_add(program.data.len() as u64)
        .is_none_or(|end| end > initial)
    {
        return Err("lowered data exceeds initial memory".to_string());
    }
    if program.trap_code_global >= program.globals.len()
        || program.trap_line_global >= program.globals.len()
    {
        return Err("missing lowered trap globals".to_string());
    }
    let mut ids = BTreeMap::new();
    for signature in &signatures {
        let next = ids.len() as u64;
        ids.entry(signature.clone()).or_insert(next);
    }
    for f in &program.functions {
        for instruction in &f.code.instructions {
            if let Instruction::CallIndirect(signature) = instruction {
                let next = ids.len() as u64;
                ids.entry(signature.clone()).or_insert(next);
            }
        }
    }
    let mut a = Assembler::new();
    let p = Platform::new(&mut a, program);
    let mut functions = Vec::new();
    for _ in &signatures {
        functions.push(a.label());
    }
    for (index, import) in program.imports.iter().enumerate() {
        let (label, parameters) = p.io.import(import.host, p.stdout);
        if import.signature != Signature::words(parameters) {
            return Err("invalid host import signature".to_string());
        }
        functions[index] = label;
    }
    platform(&mut a, p, program, &entry, &functions, initial);
    for index in 0..program.functions.len() {
        function(&mut a, p, program, index, &functions, &signatures, &ids)?;
    }
    let data_offset = (CODE_OFFSET + a.bytes.len() + 4095) & !4095;
    let mut data = vec![0u8; CONTEXT_GLOBALS as usize + program.globals.len() * 8];
    put64(&mut data, HEAP_USED as usize, initial);
    for (i, global) in program.globals.iter().enumerate() {
        put64(
            &mut data,
            CONTEXT_GLOBALS as usize + i * 8,
            global.initial as u64,
        );
    }
    a.data_label(p.context, data_offset);
    a.data_label(p.table, data_offset + data.len());
    let mut table_references = Vec::new();
    for index in &program.table {
        let signature = signatures
            .get(*index)
            .ok_or("invalid lowered table function")?;
        table_references.push((data.len(), functions[*index]));
        data.extend_from_slice(&[0u8; 8]);
        data.extend_from_slice(&ids[signature].to_le_bytes());
    }
    a.data_label(p.signal_action, data_offset + data.len());
    data.extend_from_slice(&1u64.to_le_bytes());
    data.extend_from_slice(&[0u8; 24]);
    a.data_label(p.system_info, data_offset + data.len());
    data.extend_from_slice(&[0u8; 112]);
    a.data_label(p.resource_limit, data_offset + data.len());
    data.extend_from_slice(&[0u8; 16]);
    a.data_label(p.diagnostic_buffer, data_offset + data.len());
    data.extend_from_slice(&[0u8; 128]);
    p.io.data(&mut a, &mut data, data_offset);
    for argument in entry.arguments.iter().rev() {
        data.extend_from_slice(&argument.to_le_bytes());
    }
    a.data_label(
        p.arguments,
        data_offset + data.len() - if entry.arguments.is_empty() { 0 } else { 8 },
    );
    a.data_label(p.literals, data_offset + data.len());
    data.extend_from_slice(&program.data);
    for (at, label) in table_references {
        put64(
            &mut data,
            at,
            a.labels[label].ok_or("unbound native table function")?,
        );
    }
    let code = a.finish()?;
    let mut output = vec![0u8; data_offset + data.len()];
    output[..16].copy_from_slice(&[0x7f, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    put16(&mut output, 16, 2);
    put16(&mut output, 18, 62);
    put32(&mut output, 20, 1);
    put64(&mut output, 24, IMAGE_BASE + CODE_OFFSET as u64);
    put64(&mut output, 32, 64);
    put16(&mut output, 52, 64);
    put16(&mut output, 54, 56);
    put16(&mut output, 56, 2);
    segment(&mut output, 64, 5, 0, data_offset as u64);
    segment(&mut output, 120, 6, data_offset as u64, data.len() as u64);
    output[CODE_OFFSET..CODE_OFFSET + code.len()].copy_from_slice(&code);
    output[data_offset..].copy_from_slice(&data);
    Ok(output)
}
