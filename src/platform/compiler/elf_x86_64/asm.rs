//! Safe byte emitter with checked absolute relocations; no assembler process.
//! R11 is reserved scratch for wide addresses, immediates, and internal transfers.
use super::{Result, error};

pub(super) type Label = usize;

#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum R {
    Ax,
    Cx,
    Dx,
    Bx,
    Sp,
    Bp,
    Si,
    Di,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

#[derive(Clone, Copy)]
enum Failure {
    Allocation,
    Label,
    Encoding,
}

pub(super) struct Asm {
    bytes: Vec<u8>,
    labels: Vec<Option<usize>>,
    failed: Option<Failure>,
    absolute: Vec<(usize, Label)>,
}

impl Asm {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            labels: Vec::new(),
            failed: None,
            absolute: Vec::new(),
        }
    }
    pub fn check(&self) -> Result<()> {
        match self.failed {
            None => Ok(()),
            Some(Failure::Allocation) => Err(super::Diagnostic::new(
                super::DiagnosticClass::Resource,
                "native_elf_allocation",
                "cannot allocate native code or relocation storage",
            )),
            Some(Failure::Label) => Err(error(
                "native_elf_label", "native label is absent or has multiple definitions",
            )),
            Some(Failure::Encoding) => Err(error(
                "native_elf_encoding", "invalid native encoding or reserved-register use",
            )),
        }
    }
    fn fail(&mut self, failure: Failure) {
        self.failed.get_or_insert(failure);
    }
    pub fn label(&mut self) -> Label {
        let label = self.labels.len();
        if self.failed.is_some() {
            return label;
        }
        if self.labels.try_reserve(1).is_err() {
            self.fail(Failure::Allocation);
            return label;
        }
        self.labels.push(None);
        label
    }
    pub fn bind(&mut self, label: Label) {
        if self.failed.is_some() {
            return;
        }
        match self.labels.get_mut(label) {
            Some(slot @ None) => *slot = Some(self.bytes.len()),
            _ => self.fail(Failure::Label),
        }
    }
    pub fn bytes(&mut self, bytes: &[u8]) {
        if self.failed.is_some() {
            return;
        }
        if self.bytes.try_reserve(bytes.len()).is_err() {
            self.fail(Failure::Allocation);
            return;
        }
        self.bytes.extend_from_slice(bytes);
    }
    fn rex(&mut self, wide: bool, reg: R, base: R) {
        self.bytes(&[0x40
            | if wide { 8 } else { 0 }
            | (((reg as u8) >> 3) << 2)
            | ((base as u8) >> 3)]);
    }
    pub fn imm(&mut self, dst: R, value: u64) {
        self.rex(true, R::Ax, dst);
        self.bytes(&[0xb8 + (dst as u8 & 7)]);
        self.bytes(&value.to_le_bytes());
    }
    pub fn mov(&mut self, dst: R, src: R) {
        self.bin(0x89, dst, src);
    }
    pub fn bin(&mut self, opcode: u8, dst: R, src: R) {
        self.rex(true, src, dst);
        self.bytes(&[opcode, 0xc0 | ((src as u8 & 7) << 3) | (dst as u8 & 7)]);
    }
    pub fn bin32(&mut self, opcode: u8, dst: R, src: R) {
        self.rex(false, src, dst);
        self.bytes(&[opcode, 0xc0 | ((src as u8 & 7) << 3) | (dst as u8 & 7)]);
    }
    pub fn rotate32(&mut self, dst: R, amount: u8) {
        self.rex(false, R::Ax, dst);
        self.bytes(&[0xc1, 0xc8 | (dst as u8 & 7), amount]);
    }
    pub fn imul(&mut self, dst: R, src: R) {
        self.rex(true, dst, src);
        self.bytes(&[0x0f, 0xaf, 0xc0 | ((dst as u8 & 7) << 3) | (src as u8 & 7)]);
    }
    pub fn group(&mut self, group: u8, dst: R, immediate: i32) {
        self.rex(true, R::Ax, dst);
        self.bytes(&[0x81, 0xc0 | (group << 3) | (dst as u8 & 7)]);
        self.bytes(&immediate.to_le_bytes());
    }
    /// Counts and extents use their full width. R11 is volatile scratch.
    pub fn group_wide(&mut self, group: u8, dst: R, immediate: u64) {
        if let Ok(immediate) = i32::try_from(immediate) {
            self.group(group, dst, immediate);
            return;
        }
        let opcode = match group {
            0 => 0x01,
            5 => 0x29,
            7 => 0x39,
            _ => { self.fail(Failure::Encoding); return; }
        };
        if dst == R::R11 {
            self.fail(Failure::Encoding);
            return;
        }
        self.imm(R::R11, immediate);
        self.bin(opcode, dst, R::R11);
    }
    pub fn shift(&mut self, group: u8, dst: R, amount: u8) {
        self.rex(true, R::Ax, dst);
        self.bytes(&[0xc1, 0xc0 | (group << 3) | (dst as u8 & 7), amount]);
    }
    pub fn unary(&mut self, group: u8, dst: R) {
        self.rex(true, R::Ax, dst);
        self.bytes(&[0xf7, 0xc0 | (group << 3) | (dst as u8 & 7)]);
    }
    fn memory(&mut self, opcode: u8, reg: R, base: R, displacement: i32) {
        self.memory_width(true, opcode, reg, base, displacement);
    }
    fn memory_width(&mut self, wide: bool, opcode: u8, reg: R, base: R, displacement: i32) {
        self.rex(wide, reg, base);
        self.bytes(&[opcode, 0x80 | ((reg as u8 & 7) << 3) | (base as u8 & 7)]);
        if base as u8 & 7 == 4 {
            self.bytes(&[0x24]);
        }
        self.bytes(&displacement.to_le_bytes());
    }
    fn indexed_address(&mut self, dst: R, base: R, index: R) {
        self.bytes(&[
            0x48 | (((dst as u8) >> 3) << 2)
                | (((index as u8) >> 3) << 1) | ((base as u8) >> 3),
            0x8d,
            0x84 | ((dst as u8 & 7) << 3),
            ((index as u8 & 7) << 3) | (base as u8 & 7),
            0, 0, 0, 0,
        ]);
    }
    fn memory_offset(&mut self, opcode: u8, reg: R, base: R, offset: i64) {
        if let Ok(offset) = i32::try_from(offset) {
            self.memory(opcode, reg, base, offset);
            return;
        }
        if base == R::R11 || (opcode == 0x89 && reg == R::R11) {
            self.fail(Failure::Encoding);
            return;
        }
        self.imm(R::R11, offset as u64);
        // LEA preserves flags, exactly as a short memory displacement does.
        self.indexed_address(R::R11, base, R::R11);
        self.memory(opcode, reg, R::R11, 0);
    }
    pub fn load_offset(&mut self, dst: R, base: R, offset: i64) {
        self.memory_offset(0x8b, dst, base, offset);
    }
    pub fn store_offset(&mut self, base: R, offset: i64, src: R) {
        self.memory_offset(0x89, src, base, offset);
    }
    pub fn load(&mut self, dst: R, base: R, offset: i32) {
        self.memory(0x8b, dst, base, offset);
    }
    pub fn store(&mut self, base: R, offset: i32, src: R) {
        self.memory(0x89, src, base, offset);
    }
    pub fn load32(&mut self, dst: R, base: R, offset: i32) {
        self.memory_width(false, 0x8b, dst, base, offset);
    }
    pub fn store32(&mut self, base: R, offset: i32, src: R) {
        self.memory_width(false, 0x89, src, base, offset);
    }
    pub fn lea(&mut self, dst: R, base: R, offset: i32) {
        self.memory(0x8d, dst, base, offset);
    }
    pub fn load_byte(&mut self, dst: R, base: R) {
        self.rex(true, dst, base);
        self.bytes(&[0x0f, 0xb6, 0x80 | ((dst as u8 & 7) << 3) | (base as u8 & 7)]);
        if base as u8 & 7 == 4 {
            self.bytes(&[0x24]);
        }
        self.bytes(&0_i32.to_le_bytes());
    }
    pub fn store_byte(&mut self, base: R, src: R) {
        self.rex(false, src, base);
        self.bytes(&[0x88, 0x80 | ((src as u8 & 7) << 3) | (base as u8 & 7)]);
        if base as u8 & 7 == 4 {
            self.bytes(&[0x24]);
        }
        self.bytes(&0_i32.to_le_bytes());
    }
    pub fn push(&mut self, reg: R) {
        if reg as u8 > 7 {
            self.bytes(&[0x41]);
        }
        self.bytes(&[0x50 + (reg as u8 & 7)]);
    }
    pub fn pop(&mut self, reg: R) {
        if reg as u8 > 7 {
            self.bytes(&[0x41]);
        }
        self.bytes(&[0x58 + (reg as u8 & 7)]);
    }
    pub fn address(&mut self, dst: R, label: Label) {
        self.rex(true, R::Ax, dst);
        self.bytes(&[0xb8 + (dst as u8 & 7)]);
        self.absolute(label);
    }
    fn jump_register(&mut self, reg: R) {
        self.rex(true, R::Ax, reg);
        self.bytes(&[0xff, 0xe0 | (reg as u8 & 7)]);
    }
    fn call_register(&mut self, reg: R) {
        self.rex(true, R::Ax, reg);
        self.bytes(&[0xff, 0xd0 | (reg as u8 & 7)]);
    }
    pub fn jump(&mut self, label: Label) {
        self.address(R::R11, label);
        self.jump_register(R::R11);
    }
    pub fn when(&mut self, condition: u8, label: Label) {
        if condition > 15 {
            self.fail(Failure::Encoding);
            return;
        }
        // The inverted short branch skips one fixed 13-byte absolute jump.
        self.bytes(&[0x70 | (condition ^ 1), 13]);
        self.jump(label);
    }
    pub fn call(&mut self, label: Label) {
        self.call_frame();
        self.address(R::R11, label);
        self.call_register(R::R11);
        self.load(R::Sp, R::Sp, 8);
    }
    pub fn indirect(&mut self, reg: R) {
        if reg == R::R11 {
            self.push(reg);
        }
        self.call_frame();
        if reg == R::R11 {
            // The saved target is outside the call record. Restore the original
            // argument base before loading it, so the callee sees the same ABI.
            self.load(R::R11, R::Sp, 8);
            self.group(0, R::R11, 8);
            self.store(R::Sp, 8, R::R11);
            self.load(R::R11, R::R11, -8);
        }
        self.call_register(reg);
        self.load(R::Sp, R::Sp, 8);
    }
    // Keep the stack 16-byte aligned at every native call, independently of
    // argument/evaluation-stack depth. The saved original stack is also the
    // argument base, at [entry rsp + 16] or [frame rbp + 24]. R11 is volatile.
    fn call_frame(&mut self) {
        self.mov(R::R11, R::Sp);
        self.group(4, R::Sp, -16);
        self.group(5, R::Sp, 16);
        self.store(R::Sp, 8, R::R11);
    }
    pub fn set(&mut self, condition: u8) {
        self.bytes(&[0x0f, 0x90 | condition, 0xc0, 0x48, 0x0f, 0xb6, 0xc0]);
    }
    pub fn syscall(&mut self) {
        self.bytes(&[0x0f, 0x05]);
    }
    pub fn ret(&mut self) {
        self.bytes(&[0xc3]);
    }
    pub fn align(&mut self) {
        let padding = (8 - self.bytes.len() % 8) % 8;
        self.bytes(&[0; 8][..padding]);
    }
    pub fn absolute(&mut self, label: Label) {
        if self.failed.is_some() {
            return;
        }
        if self.absolute.try_reserve(1).is_err() {
            self.fail(Failure::Allocation);
            return;
        }
        self.absolute.push((self.bytes.len(), label));
        self.bytes(&[0; 8]);
    }
    pub fn finish(mut self) -> Result<Vec<u8>> {
        self.check()?;
        for (offset, label) in self.absolute {
            let target = self.labels.get(label).copied().flatten().ok_or_else(|| {
                error("native_elf_label", "native label has no definition")
            })?;
            let address = absolute_address(target)?;
            let end = offset.checked_add(8).ok_or_else(|| {
                error("native_elf_layout", "native relocation extent overflowed")
            })?;
            self.bytes.get_mut(offset..end).ok_or_else(|| {
                error("native_elf_layout", "native relocation is outside the image")
            })?.copy_from_slice(&address.to_le_bytes());
        }
        Ok(self.bytes)
    }
}

pub(super) fn absolute_address(offset: usize) -> Result<u64> {
    u64::try_from(offset).ok()
        .and_then(|offset| super::TEXT_ADDRESS.checked_add(offset))
        .ok_or_else(|| error("native_elf_layout", "native absolute address overflowed"))
}

// x86-64 condition codes used by the emitter.
pub(super) const O: u8 = 0;
pub(super) const B: u8 = 2;
pub(super) const AE: u8 = 3;
pub(super) const E: u8 = 4;
pub(super) const NE: u8 = 5;
pub(super) const L: u8 = 12;
pub(super) const GE: u8 = 13;
pub(super) const LE: u8 = 14;
