//! x86-64 encoding and relative relocation. No IR operation is selected here.
use crate::diagnostic::{Diagnostic, Span};

pub type Label = usize;
pub const A: u8 = 0;
pub const C: u8 = 1;
pub const D: u8 = 2;
pub const B: u8 = 3;
pub const SP: u8 = 4;
pub const BP: u8 = 5;
pub const SI: u8 = 6;
pub const DI: u8 = 7;
pub const R8: u8 = 8;
pub const R9: u8 = 9;
pub const R10: u8 = 10;
pub const R11: u8 = 11;
pub const R12: u8 = 12;
pub const R13: u8 = 13;
pub const R14: u8 = 14;
pub const R15: u8 = 15;
pub const CONTEXT: u8 = R12;
pub const STACK_FLOOR: u8 = R13;
pub const LINE: u8 = R14;
pub const FILE: u8 = R15;

struct Reference {
    at: usize,
    label: Label,
    site: Span,
}

pub struct Assembler {
    pub bytes: Vec<u8>,
    labels: Vec<Option<usize>>,
    references: Vec<Reference>,
    site: Span,
    problem: Option<Diagnostic>,
}
impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}
impl Assembler {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            labels: Vec::new(),
            references: Vec::new(),
            site: Span {
                file: 0,
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
            problem: None,
        }
    }
    pub fn location(&mut self, file: usize, line: usize, column: usize) {
        self.site.file = file;
        self.site.line = line;
        self.site.column = column;
    }
    fn fail(&mut self, message: &str) {
        if self.problem.is_none() {
            self.problem = Some(Diagnostic::new(self.site, "E_EMIT", message));
        }
    }
    pub fn label(&mut self) -> Label {
        let label = self.labels.len();
        self.labels.push(None);
        label
    }
    pub fn bind(&mut self, label: Label) {
        let offset = self.bytes.len();
        self.data_label(label, offset);
    }
    /// Bind a target at an offset relative to the first emitted code byte.
    pub fn data_label(&mut self, label: Label, offset: usize) {
        if label >= self.labels.len() || self.labels[label].is_some() {
            self.fail("invalid or already bound x86-64 label");
            return;
        }
        self.labels[label] = Some(offset);
    }
    pub fn raw(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }
    fn word(&mut self, value: u64, bytes: usize) {
        let mut index = 0usize;
        while index < bytes {
            self.bytes.push((value >> (index * 8usize)) as u8);
            index += 1;
        }
    }
    fn rex_for(&mut self, wide: bool, r: u8, b: u8, force: bool) {
        let prefix = 0x40 | ((wide as u8) << 3) | (((r >> 3) & 1) << 2) | ((b >> 3) & 1);
        if prefix != 0x40 || force {
            self.bytes.push(prefix);
        }
    }
    pub fn rr(&mut self, opcode: u8, dst: u8, src: u8, wide: bool) {
        let byte = opcode == 0x88
            || opcode == 0x00
            || opcode == 0x08
            || opcode == 0x20
            || opcode == 0x28
            || opcode == 0x30
            || opcode == 0x38;
        self.rex_for(wide, src, dst, byte && (src >= 4 || dst >= 4));
        self.raw(&[opcode, 0xc0 | ((src & 7) << 3) | (dst & 7)]);
    }
    pub fn mov(&mut self, dst: u8, src: u8) {
        if dst != src {
            self.rr(0x89, dst, src, true);
        }
    }
    /// Apply a scalar storage width while retaining the value in a register.
    pub fn normalize(&mut self, reg: u8, bytes: u8, signed: bool) {
        match bytes {
            1 | 2 => {
                self.rex_for(signed, reg, reg, bytes == 1 && reg >= 4);
                let opcode = if bytes == 1 {
                    if signed { 0xbe } else { 0xb6 }
                } else if signed {
                    0xbf
                } else {
                    0xb7
                };
                self.raw(&[0x0f, opcode, 0xc0 | ((reg & 7) << 3) | (reg & 7)]);
            }
            4 if signed => {
                self.rex_for(true, reg, reg, false);
                self.raw(&[0x63, 0xc0 | ((reg & 7) << 3) | (reg & 7)]);
            }
            4 => self.rr(0x89, reg, reg, false),
            8 => {}
            _ => self.fail("unsupported x86-64 scalar register width"),
        }
    }
    /// Materialize a bit pattern without changing flags. The payload is 4 or 8 bytes.
    pub fn immediate(&mut self, reg: u8, value: u64) -> usize {
        if value <= 0xffff_ffffu64 {
            self.rex_for(false, 0, reg, false);
            self.bytes.push(0xb8 | (reg & 7));
            let at = self.bytes.len();
            self.word(value, 4);
            at
        } else if value >= 0xffff_ffff_8000_0000u64 {
            self.rex_for(true, 0, reg, false);
            self.raw(&[0xc7, 0xc0 | (reg & 7)]);
            let at = self.bytes.len();
            self.word(value, 4);
            at
        } else {
            self.rex_for(true, 0, reg, false);
            self.bytes.push(0xb8 | (reg & 7));
            let at = self.bytes.len();
            self.word(value, 8);
            at
        }
    }
    fn relative(&mut self, label: Label) {
        let at = self.bytes.len();
        self.word(0, 4);
        self.references.push(Reference {
            at,
            label,
            site: self.site,
        });
    }
    pub fn address(&mut self, reg: u8, label: Label) {
        self.rex_for(true, reg, 0, false);
        self.raw(&[0x8d, 0x05 | ((reg & 7) << 3)]);
        self.relative(label);
    }
    pub fn memory(&mut self, opcode: &[u8], reg: u8, base: u8, offset: i64, wide: bool) {
        if offset < -2147483648i64 || offset > 2147483647i64 {
            self.fail("x86-64 memory displacement exceeds signed 32-bit representation");
        }
        let byte = opcode.len() == 1 && (opcode[0] == 0x88 || opcode[0] == 0x8a);
        self.rex_for(wide, reg, base, byte && reg >= 4);
        self.raw(opcode);
        let mode = if offset == 0 && (base & 7) != 5 {
            0u8
        } else if offset >= -128i64 && offset <= 127i64 {
            1u8
        } else {
            2u8
        };
        self.bytes.push((mode << 6) | ((reg & 7) << 3) | (base & 7));
        if (base & 7) == 4 {
            self.bytes.push(0x24);
        }
        if mode == 1 {
            self.word(offset as u64, 1);
        } else if mode == 2 {
            self.word(offset as u64, 4);
        }
    }
    pub fn load(&mut self, reg: u8, base: u8, offset: i64) {
        self.memory(&[0x8b], reg, base, offset, true);
    }
    pub fn store(&mut self, base: u8, offset: i64, reg: u8) {
        self.memory(&[0x89], reg, base, offset, true);
    }
    pub fn load_width(&mut self, reg: u8, base: u8, offset: i64, bytes: u8, signed: bool) {
        match bytes {
            1 => self.memory(
                &[0x0f, if signed { 0xbe } else { 0xb6 }],
                reg,
                base,
                offset,
                signed,
            ),
            2 => self.memory(
                &[0x0f, if signed { 0xbf } else { 0xb7 }],
                reg,
                base,
                offset,
                signed,
            ),
            4 if signed => self.memory(&[0x63], reg, base, offset, true),
            4 => self.memory(&[0x8b], reg, base, offset, false),
            8 => self.load(reg, base, offset),
            _ => self.fail("unsupported x86-64 scalar load width"),
        }
    }
    pub fn store_width(&mut self, base: u8, offset: i64, reg: u8, bytes: u8) {
        match bytes {
            1 => self.memory(&[0x88], reg, base, offset, false),
            2 => {
                self.bytes.push(0x66);
                self.memory(&[0x89], reg, base, offset, false);
            }
            4 => self.memory(&[0x89], reg, base, offset, false),
            8 => self.store(base, offset, reg),
            _ => self.fail("unsupported x86-64 scalar store width"),
        }
    }
    pub fn lea(&mut self, reg: u8, base: u8, offset: i64) {
        self.memory(&[0x8d], reg, base, offset, true);
    }
    pub fn compare(&mut self, left: u8, right: u8) {
        self.rr(0x39, left, right, true);
    }
    pub fn test(&mut self, reg: u8, wide: bool) {
        self.rr(0x85, reg, reg, wide);
    }
    pub fn shift(&mut self, reg: u8, kind: u8, count: u8) {
        self.rex_for(true, 0, reg, false);
        self.raw(&[0xc1, 0xc0 | ((kind & 7) << 3) | (reg & 7), count]);
    }
    pub fn shift_cl(&mut self, reg: u8, kind: u8) {
        self.rex_for(true, 0, reg, false);
        self.raw(&[0xd3, 0xc0 | ((kind & 7) << 3) | (reg & 7)]);
    }
    pub fn imul(&mut self, dst: u8, src: u8) {
        self.rex_for(true, dst, src, false);
        self.raw(&[0x0f, 0xaf, 0xc0 | ((dst & 7) << 3) | (src & 7)]);
    }
    pub fn mul(&mut self, src: u8) {
        self.rex_for(true, 0, src, false);
        self.raw(&[0xf7, 0xe0 | (src & 7)]);
    }
    /// Divide the caller-prepared RDX:RAX pair. The quotient is RAX, remainder RDX.
    pub fn divide(&mut self, src: u8, signed: bool) {
        self.rex_for(true, 0, src, false);
        self.raw(&[0xf7, (if signed { 0xf8 } else { 0xf0 }) | (src & 7)]);
    }
    pub fn negate(&mut self, reg: u8) {
        self.rex_for(true, 0, reg, false);
        self.raw(&[0xf7, 0xd8 | (reg & 7)]);
    }
    pub fn not(&mut self, reg: u8) {
        self.rex_for(true, 0, reg, false);
        self.raw(&[0xf7, 0xd0 | (reg & 7)]);
    }
    /// Both SETcc and MOVZX preserve flags, allowing multiple predicates.
    pub fn setcc(&mut self, reg: u8, condition: u8) {
        self.rex_for(false, 0, reg, reg >= 4);
        self.raw(&[0x0f, 0x90 | (condition & 15), 0xc0 | (reg & 7)]);
        self.rex_for(false, reg, reg, reg >= 4);
        self.raw(&[0x0f, 0xb6, 0xc0 | ((reg & 7) << 3) | (reg & 7)]);
    }
    pub fn cmov(&mut self, dst: u8, src: u8, condition: u8) {
        self.rex_for(true, dst, src, false);
        self.raw(&[
            0x0f,
            0x40 | (condition & 15),
            0xc0 | ((dst & 7) << 3) | (src & 7),
        ]);
    }
    pub fn push(&mut self, reg: u8) {
        self.rex_for(false, 0, reg, false);
        self.bytes.push(0x50 | (reg & 7));
    }
    pub fn pop(&mut self, reg: u8) {
        self.rex_for(false, 0, reg, false);
        self.bytes.push(0x58 | (reg & 7));
    }
    pub fn jump(&mut self, label: Label) {
        self.bytes.push(0xe9);
        self.relative(label);
    }
    pub fn condition(&mut self, condition: u8, label: Label) {
        self.raw(&[0x0f, 0x80 | (condition & 15)]);
        self.relative(label);
    }
    pub fn call(&mut self, label: Label) {
        self.bytes.push(0xe8);
        self.relative(label);
    }
    pub fn jump_register(&mut self, reg: u8) {
        self.rex_for(false, 0, reg, false);
        self.raw(&[0xff, 0xe0 | (reg & 7)]);
    }
    pub fn call_register(&mut self, reg: u8) {
        self.rex_for(false, 0, reg, false);
        self.raw(&[0xff, 0xd0 | (reg & 7)]);
    }
    pub fn ret(&mut self) {
        self.bytes.push(0xc3);
    }
    pub fn syscall(&mut self) {
        self.raw(&[0x0f, 0x05]);
    }
    pub fn mov_to_xmm(&mut self, xmm: u8, reg: u8) {
        self.bytes.push(0x66);
        self.rex_for(true, xmm, reg, false);
        self.raw(&[0x0f, 0x6e, 0xc0 | ((xmm & 7) << 3) | (reg & 7)]);
    }
    pub fn mov_from_xmm(&mut self, reg: u8, xmm: u8) {
        self.bytes.push(0x66);
        self.rex_for(true, xmm, reg, false);
        self.raw(&[0x0f, 0x7e, 0xc0 | ((xmm & 7) << 3) | (reg & 7)]);
    }
    pub fn f64_op(&mut self, opcode: u8, dst: u8, src: u8) {
        self.bytes.push(0xf2);
        self.rex_for(false, dst, src, false);
        self.raw(&[0x0f, opcode, 0xc0 | ((dst & 7) << 3) | (src & 7)]);
    }
    pub fn f64_compare(&mut self, left: u8, right: u8) {
        self.bytes.push(0x66);
        self.rex_for(false, left, right, false);
        self.raw(&[0x0f, 0x2e, 0xc0 | ((left & 7) << 3) | (right & 7)]);
    }
    pub fn int_to_f64(&mut self, xmm: u8, reg: u8) {
        self.bytes.push(0xf2);
        self.rex_for(true, xmm, reg, false);
        self.raw(&[0x0f, 0x2a, 0xc0 | ((xmm & 7) << 3) | (reg & 7)]);
    }
    pub fn f64_to_int(&mut self, reg: u8, xmm: u8) {
        self.bytes.push(0xf2);
        self.rex_for(true, reg, xmm, false);
        self.raw(&[0x0f, 0x2c, 0xc0 | ((reg & 7) << 3) | (xmm & 7)]);
    }
    pub fn finish(mut self) -> Result<Vec<u8>, Diagnostic> {
        if let Some(problem) = self.problem {
            return Err(problem);
        }
        for reference in &self.references {
            let target = match self.labels.get(reference.label) {
                Some(Some(target)) => *target,
                _ => {
                    return Err(Diagnostic::new(
                        reference.site,
                        "E_EMIT",
                        "unbound x86-64 label",
                    ));
                }
            };
            let base = reference.at + 4usize;
            let displacement = if target >= base {
                let distance = target - base;
                if distance > 2147483647usize {
                    return Err(Diagnostic::new(
                        reference.site,
                        "E_EMIT",
                        "x86-64 relative address exceeds signed 32-bit representation",
                    ));
                }
                distance as i64
            } else {
                let distance = base - target;
                if distance > 2147483648usize {
                    return Err(Diagnostic::new(
                        reference.site,
                        "E_EMIT",
                        "x86-64 relative address exceeds signed 32-bit representation",
                    ));
                }
                -(distance as i64)
            };
            let mut byte = 0usize;
            while byte < 4 {
                self.bytes[reference.at + byte] = ((displacement as u64) >> (byte * 8usize)) as u8;
                byte += 1;
            }
        }
        Ok(self.bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn finish(a: Assembler) -> Vec<u8> {
        match a.finish() {
            Ok(bytes) => bytes,
            Err(problem) => panic!("{}", problem.message),
        }
    }
    #[test]
    fn immediates_preserve_flags_and_choose_the_smallest_mov_form() {
        let mut a = Assembler::new();
        a.immediate(A, 0);
        a.immediate(R8, 0xffff_ffff);
        a.immediate(C, (-1i64) as u64);
        a.immediate(D, 0x0123_4567_89ab_cdef);
        assert_eq!(
            finish(a),
            vec![
                0xb8, 0, 0, 0, 0, 0x41, 0xb8, 0xff, 0xff, 0xff, 0xff, 0x48, 0xc7, 0xc1, 0xff, 0xff,
                0xff, 0xff, 0x48, 0xba, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01
            ]
        );
    }
    #[test]
    fn relative_calls_branches_and_rip_addresses_are_patched() {
        let mut a = Assembler::new();
        let start = a.label();
        let end = a.label();
        a.bind(start);
        a.call(end);
        a.condition(4, start);
        a.address(R8, end);
        a.bind(end);
        a.ret();
        assert_eq!(
            finish(a),
            vec![
                0xe8, 13, 0, 0, 0, 0x0f, 0x84, 0xf5, 0xff, 0xff, 0xff, 0x4c, 0x8d, 0x05, 0, 0, 0,
                0, 0xc3
            ]
        );
    }
    #[test]
    fn memory_widths_handle_low_byte_registers_and_stack_bases() {
        let mut a = Assembler::new();
        a.store_width(SP, 0, SI, 1);
        a.load_width(R9, R13, 0, 1, true);
        a.store_width(R12, -8, R10, 2);
        a.load(A, BP, -256);
        assert_eq!(
            finish(a),
            vec![
                0x40, 0x88, 0x34, 0x24, 0x4d, 0x0f, 0xbe, 0x4d, 0, 0x66, 0x45, 0x89, 0x54, 0x24,
                0xf8, 0x48, 0x8b, 0x85, 0, 0xff, 0xff, 0xff
            ]
        );
    }
    #[test]
    fn encoding_errors_retain_the_ir_location() {
        let mut a = Assembler::new();
        let far = a.label();
        a.location(7, 42, 9);
        a.jump(far);
        a.data_label(far, 2147483653usize);
        let problem = match a.finish() {
            Ok(_) => panic!("far branch accepted"),
            Err(problem) => problem,
        };
        assert_eq!((problem.file, problem.line, problem.column), (7, 42, 9));
        assert_eq!(problem.code, "E_EMIT");
    }
}
