//! Small safe byte emitter with checked relative relocations; no assembler process.
use super::{Result, error};
use std::collections::BTreeMap;

pub(super) const MAX_BYTES: usize = 32 * 1024 * 1024;
pub(super) type Label = u32;

#[derive(Clone, Copy)]
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

pub(super) struct Asm {
    bytes: Vec<u8>,
    labels: BTreeMap<Label, usize>,
    patches: Vec<(usize, Label)>,
    next: Label,
    failed: bool,
    absolute: Vec<(usize, Label)>,
}

impl Asm {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            labels: BTreeMap::new(),
            patches: Vec::new(),
            next: 0,
            failed: false,
            absolute: Vec::new(),
        }
    }
    pub fn label(&mut self) -> Label {
        let n = self.next;
        self.next += 1;
        n
    }
    pub fn bind(&mut self, label: Label) {
        if self.labels.insert(label, self.bytes.len()).is_some() {
            self.failed = true;
        }
    }
    pub fn bytes(&mut self, bytes: &[u8]) {
        if self.bytes.len().saturating_add(bytes.len()) > MAX_BYTES
            || self.bytes.try_reserve(bytes.len()).is_err()
        {
            self.failed = true;
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
    fn relocation(&mut self, label: Label) {
        self.patches.push((self.bytes.len(), label));
        self.bytes(&[0; 4]);
    }
    pub fn address(&mut self, dst: R, label: Label) {
        self.rex(true, dst, R::Ax);
        self.bytes(&[0x8d, ((dst as u8 & 7) << 3) | 5]);
        self.relocation(label);
    }
    pub fn jump(&mut self, label: Label) {
        self.bytes(&[0xe9]);
        self.relocation(label);
    }
    pub fn when(&mut self, condition: u8, label: Label) {
        self.bytes(&[0x0f, 0x80 | condition]);
        self.relocation(label);
    }
    pub fn call(&mut self, label: Label) {
        self.call_frame();
        self.bytes(&[0xe8]);
        self.relocation(label);
        self.load(R::Sp, R::Sp, 8);
    }
    pub fn indirect(&mut self, reg: R) {
        self.call_frame();
        self.rex(true, R::Ax, reg);
        self.bytes(&[0xff, 0xd0 | (reg as u8 & 7)]);
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
        self.absolute.push((self.bytes.len(), label));
        self.bytes(&[0; 8]);
    }
    pub fn finish(mut self) -> Result<Vec<u8>> {
        if self.failed {
            return Err(error(
                "native_elf_capacity",
                "native image exceeds its bound or repeats a label",
            ));
        }
        for (offset, label) in self.patches {
            let target = self
                .labels
                .get(&label)
                .ok_or_else(|| error("native_elf_label", "native label has no definition"))?;
            let displacement = i32::try_from(*target as i64 - offset as i64 - 4)
                .map_err(|_| error("native_elf_capacity", "native relocation exceeds rel32"))?;
            self.bytes[offset..offset + 4].copy_from_slice(&displacement.to_le_bytes());
        }
        for (offset, label) in self.absolute {
            let target = self.labels.get(&label).ok_or_else(|| {
                error("native_elf_label", "native data pointer has no definition")
            })?;
            let address = super::TEXT_ADDRESS + *target as u64;
            self.bytes[offset..offset + 8].copy_from_slice(&address.to_le_bytes());
        }
        Ok(self.bytes)
    }
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
