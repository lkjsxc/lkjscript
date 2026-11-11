//! WebAssembly 3.0 memory64 encoding. The checked K program is the input.
use crate::k::{CheckedProgram, Function, Op, Place, Program, Projection, TypeKind};
use std::collections::{BTreeMap, BTreeSet};

use crate::encoding::{sleb, uleb};

fn text(out: &mut Vec<u8>, name: &str) {
    uleb(out, name.len() as u64);
    out.extend_from_slice(name.as_bytes());
}
fn section(out: &mut Vec<u8>, id: u8, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > u32::MAX as usize {
        return Err("WebAssembly section exceeds its u32 byte-length representation".to_string());
    }
    out.push(id);
    uleb(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
    Ok(())
}
fn get(out: &mut Vec<u8>, local: usize) {
    out.push(0x20);
    uleb(out, local as u64);
}
fn set(out: &mut Vec<u8>, local: usize) {
    out.push(0x21);
    uleb(out, local as u64);
}
fn tee(out: &mut Vec<u8>, local: usize) {
    out.push(0x22);
    uleb(out, local as u64);
}
fn constant(out: &mut Vec<u8>, value: i64) {
    out.push(0x42);
    sleb(out, value);
}
fn call(out: &mut Vec<u8>, function: usize) {
    out.push(0x10);
    uleb(out, function as u64);
}
fn load(out: &mut Vec<u8>, offset: u64) {
    out.push(0x29);
    uleb(out, 3);
    uleb(out, offset);
}
fn store(out: &mut Vec<u8>, offset: u64) {
    out.push(0x37);
    uleb(out, 3);
    uleb(out, offset);
}
fn begin_if(out: &mut Vec<u8>) {
    out.extend_from_slice(&[0x04, 0x40]);
}
fn trap(out: &mut Vec<u8>, code: i64, line: usize) {
    constant(out, code);
    constant(out, line as i64);
    call(out, 0);
    out.push(0x00);
}
fn trap_if(out: &mut Vec<u8>, code: i64, line: usize) {
    begin_if(out);
    trap(out, code, line);
    out.push(0x0b);
}
const ALLOC: usize = 1;
const FREE: usize = 2;
const DROP_RUN: usize = 3;
const UTF8_VALIDATE: usize = 4;
const FIRST_USER: usize = 5;

// The immutable region precedes the allocator. Each distinct literal has a
// header-pointer cell, a {data, length, capacity} header, then eight-byte u8 slots.
// A text reference uses the same outer cell as an ordinary shared slice reference.
// No owned value points to this region, so neither copying nor cleanup frees it.
struct TextPool {
    offsets: BTreeMap<Vec<u8>, u64>,
    data: Vec<u8>,
}
impl TextPool {
    fn collect(program: &Program) -> Result<Self, String> {
        let mut pool = Self {
            offsets: BTreeMap::new(),
            data: Vec::new(),
        };
        for function in &program.functions {
            for instruction in &function.body {
                if let Op::Text { bytes, .. } = &instruction.op {
                    if pool.offsets.contains_key(bytes) {
                        continue;
                    }
                    let length = u64::try_from(bytes.len())
                        .map_err(|_| "text length exceeds the memory64 representation")?;
                    let extent = length
                        .checked_mul(8)
                        .and_then(|size| size.checked_add(32))
                        .ok_or("static text extent exceeds the memory64 representation")?;
                    let address = pool.heap_start()?;
                    address
                        .checked_add(extent)
                        .ok_or("static text address exceeds the memory64 representation")?;
                    let extent = usize::try_from(extent)
                        .map_err(|_| "static text extent exceeds the host representation")?;
                    let size = pool
                        .data
                        .len()
                        .checked_add(extent)
                        .ok_or("static text size exceeds the host representation")?;
                    if size > u32::MAX as usize {
                        return Err(
                            "WebAssembly data section exceeds its u32 byte-length representation"
                                .to_string(),
                        );
                    }
                    pool.data
                        .try_reserve(extent)
                        .map_err(|_| "allocation failed while encoding static text")?;
                    for value in [address + 8, address + 32, length, length] {
                        pool.data.extend_from_slice(&value.to_le_bytes());
                    }
                    for byte in bytes {
                        pool.data.extend_from_slice(&u64::from(*byte).to_le_bytes());
                    }
                    pool.offsets.insert(bytes.clone(), address);
                }
            }
        }
        Ok(pool)
    }

    fn heap_start(&self) -> Result<u64, String> {
        u64::try_from(self.data.len())
            .ok()
            .and_then(|size| size.checked_add(8))
            .ok_or_else(|| "static text address exceeds the memory64 representation".to_string())
    }

    fn pages(&self) -> Result<u64, String> {
        self.heap_start()?
            .checked_add(65535)
            .map(|extent| extent >> 16)
            .ok_or_else(|| "static text pages exceed the memory64 representation".to_string())
    }
}

struct Body<'a> {
    program: &'a Program,
    function: &'a Function,
    bytes: Vec<u8>,
    slots: Vec<Slot>,
    scratch: usize,
    frame: usize,
    frame_bytes: u64,
    controls: Vec<u8>,
    messages: &'a [String],
    texts: &'a TextPool,
}
impl<'a> Body<'a> {
    fn read(&mut self, local: usize) {
        match self.slots[local] {
            Slot::Frame(offset) => {
                get(&mut self.bytes, self.frame);
                load(&mut self.bytes, offset);
            }
            Slot::Register(register) => get(&mut self.bytes, register),
        }
    }
    fn write(&mut self, local: usize) {
        match self.slots[local] {
            Slot::Frame(offset) => {
                set(&mut self.bytes, self.scratch + 7);
                get(&mut self.bytes, self.frame);
                get(&mut self.bytes, self.scratch + 7);
                store(&mut self.bytes, offset);
            }
            Slot::Register(register) => set(&mut self.bytes, register),
        }
    }
    fn address(&mut self, place: &Place, line: usize) -> usize {
        let mut ty = self.function.locals[place.local].ty;
        if place.projections.is_empty() {
            let offset = match self.slots[place.local] {
                Slot::Frame(offset) => offset,
                Slot::Register(_) => unreachable!("address-taken local has a frame slot"),
            };
            get(&mut self.bytes, self.frame);
            constant(&mut self.bytes, offset as i64);
            self.bytes.push(0x7c);
            return ty;
        }
        self.read(place.local);
        let mut i = 0;
        while i < place.projections.len() {
            match &place.projections[i] {
                Projection::Deref => {
                    ty = match self.program.types[ty].kind {
                        TypeKind::Ref { inner, .. } | TypeKind::Box { inner } => inner,
                        _ => unreachable!("K dereference type"),
                    };
                }
                Projection::Field(field) => {
                    ty = match &self.program.types[ty].kind {
                        TypeKind::Struct { fields, .. } => fields[*field].ty,
                        _ => unreachable!("K field type"),
                    };
                    constant(&mut self.bytes, (*field as i64) * 8);
                    self.bytes.push(0x7c);
                }
                Projection::VariantField(variant, field) => {
                    ty = match &self.program.types[ty].kind {
                        TypeKind::Enum { variants, .. } => variants[*variant].fields[*field].ty,
                        _ => unreachable!("K variant field type"),
                    };
                    constant(&mut self.bytes, ((*field as i64) + 1) * 8);
                    self.bytes.push(0x7c);
                }
                Projection::Index(index) => {
                    ty = match self.program.types[ty].kind {
                        TypeKind::Vec { inner }
                        | TypeKind::Slice { inner, .. }
                        | TypeKind::Array { inner, .. } => inner,
                        _ => unreachable!("K index type"),
                    };
                    tee(&mut self.bytes, self.scratch + 4);
                    load(&mut self.bytes, 8);
                    set(&mut self.bytes, self.scratch + 5);
                    self.read(*index);
                    get(&mut self.bytes, self.scratch + 5);
                    self.bytes.push(0x5a); // i64.ge_u
                    trap_if(&mut self.bytes, 3, line);
                    get(&mut self.bytes, self.scratch + 4);
                    load(&mut self.bytes, 0);
                    self.read(*index);
                    constant(&mut self.bytes, 8);
                    self.bytes.push(0x7e);
                    self.bytes.push(0x7c);
                }
            }
            i += 1;
            if i < place.projections.len() {
                load(&mut self.bytes, 0);
            }
        }
        ty
    }
    fn read_place(&mut self, place: &Place, line: usize) -> usize {
        if place.projections.is_empty() {
            self.read(place.local);
            self.function.locals[place.local].ty
        } else {
            let ty = self.address(place, line);
            load(&mut self.bytes, 0);
            ty
        }
    }
    fn clear_place(&mut self, place: &Place, ty: usize, line: usize) {
        if !heap_type(&self.program.types[ty].kind) {
            return;
        }
        if place.projections.is_empty() {
            constant(&mut self.bytes, 0);
            self.write(place.local);
        } else {
            self.address(place, line);
            constant(&mut self.bytes, 0);
            store(&mut self.bytes, 0);
        }
    }
    fn narrow(&mut self, bits: u8, signed: bool, line: usize) {
        if bits == 64 {
            return;
        }
        tee(&mut self.bytes, self.scratch + 2);
        if signed {
            constant(&mut self.bytes, -(1_i64 << (bits - 1)));
            self.bytes.push(0x53);
            get(&mut self.bytes, self.scratch + 2);
            constant(&mut self.bytes, (1_i64 << (bits - 1)) - 1);
            self.bytes.push(0x55);
            self.bytes.push(0x72);
        } else {
            constant(&mut self.bytes, (1_i64 << bits) - 1);
            self.bytes.push(0x56);
        }
        trap_if(&mut self.bytes, 1, line);
        get(&mut self.bytes, self.scratch + 2);
    }
    fn binary(
        &mut self,
        dst: usize,
        op: &str,
        left: usize,
        right: usize,
        line: usize,
    ) -> Result<(), String> {
        let ty = self.function.locals[left].ty;
        if self.program.types[ty].kind == TypeKind::F64 {
            self.read(left);
            self.bytes.push(0xbf);
            self.read(right);
            self.bytes.push(0xbf);
            let opcode = match op {
                "+" => 0xa0,
                "-" => 0xa1,
                "*" => 0xa2,
                "/" => 0xa3,
                "==" => 0x61,
                "!=" => 0x62,
                "<" => 0x63,
                ">" => 0x64,
                "<=" => 0x65,
                ">=" => 0x66,
                _ => return Err(format!("unsupported f64 operation {op}")),
            };
            self.bytes.push(opcode);
            if opcode >= 0xa0 {
                self.bytes.push(0xbd);
                tee(&mut self.bytes, self.scratch);
                self.bytes.push(0xbf);
                get(&mut self.bytes, self.scratch);
                self.bytes.push(0xbf);
                self.bytes.push(0x62);
                self.bytes.extend_from_slice(&[0x04, 0x7e]);
                constant(&mut self.bytes, 0x7ff8_0000_0000_0000);
                self.bytes.push(0x05);
                get(&mut self.bytes, self.scratch);
                self.bytes.push(0x0b);
            } else {
                self.bytes.push(0xad);
            }
            self.write(dst);
            return Ok(());
        }
        let (bits, signed) = match self.program.types[ty].kind {
            TypeKind::Int { bits, signed } => (bits, signed),
            TypeKind::Bool => (1, false),
            _ => return Err("non-numeric K binary operation".to_string()),
        };
        self.read(left);
        set(&mut self.bytes, self.scratch);
        self.read(right);
        set(&mut self.bytes, self.scratch + 1);
        if op == "/" || op == "%" {
            get(&mut self.bytes, self.scratch + 1);
            self.bytes.push(0x50);
            trap_if(&mut self.bytes, 2, line);
            if signed {
                get(&mut self.bytes, self.scratch);
                constant(
                    &mut self.bytes,
                    if bits == 64 {
                        i64::MIN
                    } else {
                        -(1_i64 << (bits - 1))
                    },
                );
                self.bytes.push(0x51);
                get(&mut self.bytes, self.scratch + 1);
                constant(&mut self.bytes, -1);
                self.bytes.push(0x51);
                self.bytes.push(0x71);
                trap_if(&mut self.bytes, 1, line);
            }
        }
        if op == "<<" || op == ">>" {
            get(&mut self.bytes, self.scratch + 1);
            constant(&mut self.bytes, bits as i64);
            self.bytes.push(0x5a);
            trap_if(&mut self.bytes, 1, line);
        }
        get(&mut self.bytes, self.scratch);
        get(&mut self.bytes, self.scratch + 1);
        let opcode = match op {
            "+" | "wrapping_add" => 0x7c,
            "-" | "wrapping_sub" => 0x7d,
            "*" | "wrapping_mul" => 0x7e,
            "/" => {
                if signed {
                    0x7f
                } else {
                    0x80
                }
            }
            "%" => {
                if signed {
                    0x81
                } else {
                    0x82
                }
            }
            "&" | "&&" => 0x83,
            "|" | "||" => 0x84,
            "^" => 0x85,
            "<<" => 0x86,
            ">>" => {
                if signed {
                    0x87
                } else {
                    0x88
                }
            }
            "==" => 0x51,
            "!=" => 0x52,
            "<" => {
                if signed {
                    0x53
                } else {
                    0x54
                }
            }
            ">" => {
                if signed {
                    0x55
                } else {
                    0x56
                }
            }
            "<=" => {
                if signed {
                    0x57
                } else {
                    0x58
                }
            }
            ">=" => {
                if signed {
                    0x59
                } else {
                    0x5a
                }
            }
            _ => return Err(format!("unsupported binary operation {op}")),
        };
        self.bytes.push(opcode);
        if opcode >= 0x51 && opcode <= 0x5a {
            self.bytes.push(0xad);
            self.write(dst);
            return Ok(());
        }
        set(&mut self.bytes, self.scratch + 2);
        if bits == 64 && (op == "+" || op == "-") {
            if signed {
                get(&mut self.bytes, self.scratch);
                if op == "+" {
                    get(&mut self.bytes, self.scratch + 2);
                } else {
                    get(&mut self.bytes, self.scratch + 1);
                }
                self.bytes.push(0x85);
                get(
                    &mut self.bytes,
                    if op == "+" {
                        self.scratch + 1
                    } else {
                        self.scratch
                    },
                );
                get(&mut self.bytes, self.scratch + 2);
                self.bytes.push(0x85);
                self.bytes.push(0x83);
                constant(&mut self.bytes, 0);
                self.bytes.push(0x53);
            } else if op == "+" {
                get(&mut self.bytes, self.scratch + 2);
                get(&mut self.bytes, self.scratch);
                self.bytes.push(0x54);
            } else {
                get(&mut self.bytes, self.scratch);
                get(&mut self.bytes, self.scratch + 1);
                self.bytes.push(0x54);
            }
            trap_if(&mut self.bytes, 1, line);
        }
        if bits == 64 && op == "*" {
            get(&mut self.bytes, self.scratch + 1);
            self.bytes.push(0x50);
            self.bytes.push(0x45);
            begin_if(&mut self.bytes);
            if signed {
                get(&mut self.bytes, self.scratch);
                constant(&mut self.bytes, i64::MIN);
                self.bytes.push(0x51);
                get(&mut self.bytes, self.scratch + 1);
                constant(&mut self.bytes, -1);
                self.bytes.push(0x51);
                self.bytes.push(0x71);
                trap_if(&mut self.bytes, 1, line);
                get(&mut self.bytes, self.scratch + 2);
                constant(&mut self.bytes, i64::MIN);
                self.bytes.push(0x51);
                get(&mut self.bytes, self.scratch + 1);
                constant(&mut self.bytes, -1);
                self.bytes.push(0x51);
                self.bytes.push(0x71);
                trap_if(&mut self.bytes, 1, line);
            }
            get(&mut self.bytes, self.scratch + 2);
            get(&mut self.bytes, self.scratch + 1);
            self.bytes.push(if signed { 0x7f } else { 0x80 });
            get(&mut self.bytes, self.scratch);
            self.bytes.push(0x52);
            trap_if(&mut self.bytes, 1, line);
            self.bytes.push(0x0b);
        }
        get(&mut self.bytes, self.scratch + 2);
        if op.starts_with("wrapping_") || op == "<<" {
            if bits < 64 {
                if signed {
                    constant(&mut self.bytes, (64 - bits) as i64);
                    self.bytes.push(0x86);
                    constant(&mut self.bytes, (64 - bits) as i64);
                    self.bytes.push(0x87);
                } else {
                    constant(&mut self.bytes, (1_i64 << bits) - 1);
                    self.bytes.push(0x83);
                }
            }
        } else if op == "+" || op == "-" || op == "*" {
            self.narrow(bits, signed, line);
        }
        self.write(dst);
        Ok(())
    }
    fn emit(&mut self, op: &Op, line: usize) -> Result<(), String> {
        match op {
            Op::Int { dst, value } => {
                constant(&mut self.bytes, *value);
                self.write(*dst);
            }
            Op::Float { dst, bits } => {
                constant(&mut self.bytes, *bits as i64);
                self.write(*dst);
            }
            Op::Text { dst, bytes } => {
                constant(&mut self.bytes, self.texts.offsets[bytes] as i64);
                self.write(*dst);
            }
            Op::Unit { dst } => {
                constant(&mut self.bytes, 0);
                self.write(*dst);
            }
            Op::Read { dst, place, moving } => {
                let ty = self.read_place(place, line);
                if !moving {
                    call(&mut self.bytes, helper_index(self.program, ty, true));
                }
                self.write(*dst);
                if *moving {
                    self.clear_place(place, ty, line);
                }
            }
            Op::SliceBorrow { dst, place, .. } => {
                self.address(place, line);
                self.write(*dst);
            }
            Op::Array { dst, elements } => {
                constant(&mut self.bytes, 24);
                call(&mut self.bytes, ALLOC);
                self.write(*dst);
                self.read(*dst);
                constant(&mut self.bytes, elements.len() as i64 * 8);
                call(&mut self.bytes, ALLOC);
                store(&mut self.bytes, 0);
                self.read(*dst);
                constant(&mut self.bytes, elements.len() as i64);
                store(&mut self.bytes, 8);
                self.read(*dst);
                constant(&mut self.bytes, elements.len() as i64);
                store(&mut self.bytes, 16);
                let mut i = 0;
                while i < elements.len() {
                    self.read(*dst);
                    load(&mut self.bytes, 0);
                    self.read(elements[i]);
                    store(&mut self.bytes, i as u64 * 8);
                    i += 1;
                }
            }

            Op::Borrow { dst, place, .. } => {
                self.address(place, line);
                self.write(*dst);
            }
            Op::Assign { place, src } => {
                if place.projections.is_empty() {
                    self.read(*src);
                    self.write(place.local);
                } else {
                    self.address(place, line);
                    self.read(*src);
                    store(&mut self.bytes, 0);
                }
            }
            Op::Drop { place } => {
                if place.projections.is_empty()
                    && !heap_type(&self.program.types[self.function.locals[place.local].ty].kind)
                {
                    return Ok(());
                }
                let ty = self.read_place(place, line);
                call(&mut self.bytes, helper_index(self.program, ty, false));
                self.bytes.push(0x1a);
                self.clear_place(place, ty, line);
            }
            Op::Unary { dst, op, src } => {
                match op.as_str() {
                    "-" => match self.program.types[self.function.locals[*src].ty].kind {
                        TypeKind::F64 => {
                            self.read(*src);
                            constant(&mut self.bytes, i64::MIN);
                            self.bytes.push(0x85);
                            tee(&mut self.bytes, self.scratch);
                            self.bytes.push(0xbf);
                            get(&mut self.bytes, self.scratch);
                            self.bytes.push(0xbf);
                            self.bytes.push(0x62);
                            self.bytes.extend_from_slice(&[0x04, 0x7e]);
                            constant(&mut self.bytes, 0x7ff8_0000_0000_0000);
                            self.bytes.push(0x05);
                            get(&mut self.bytes, self.scratch);
                            self.bytes.push(0x0b);
                        }
                        TypeKind::Int { bits, signed: true } => {
                            self.read(*src);
                            constant(
                                &mut self.bytes,
                                if bits == 64 {
                                    i64::MIN
                                } else {
                                    -(1_i64 << (bits - 1))
                                },
                            );
                            self.bytes.push(0x51);
                            trap_if(&mut self.bytes, 1, line);
                            constant(&mut self.bytes, 0);
                            self.read(*src);
                            self.bytes.push(0x7d);
                        }
                        _ => return Err("invalid negation".to_string()),
                    },
                    "!" => {
                        self.read(*src);
                        if self.program.types[self.function.locals[*src].ty].kind == TypeKind::Bool
                        {
                            self.bytes.extend_from_slice(&[0x50, 0xad]);
                        } else {
                            constant(&mut self.bytes, -1);
                            self.bytes.push(0x85);
                            if let TypeKind::Int {
                                bits,
                                signed: false,
                            } = self.program.types[self.function.locals[*src].ty].kind
                            {
                                if bits < 64 {
                                    constant(&mut self.bytes, (1_i64 << bits) - 1);
                                    self.bytes.push(0x83);
                                }
                            }
                        }
                    }
                    _ => return Err(format!("unsupported unary operation {op}")),
                }
                self.write(*dst);
            }
            Op::Binary {
                dst,
                op,
                left,
                right,
            } => self.binary(*dst, op, *left, *right, line)?,
            Op::Call {
                dst,
                function,
                args,
            } => {
                for arg in args {
                    self.read(*arg);
                }
                call(&mut self.bytes, FIRST_USER + *function);
                self.write(*dst);
            }
            Op::If { condition } => {
                self.read(*condition);
                self.bytes.push(0xa7);
                begin_if(&mut self.bytes);
                self.controls.push(1);
            }
            Op::Else => self.bytes.push(0x05),
            Op::Loop => {
                self.bytes.extend_from_slice(&[0x02, 0x40, 0x03, 0x40]);
                self.controls.push(2);
            }
            Op::End => {
                if self.controls.pop().expect("K control end") == 2 {
                    self.bytes.extend_from_slice(&[0x0c, 0, 0x0b]);
                }
                self.bytes.push(0x0b);
            }
            Op::Break | Op::Continue => {
                let mut depth = 0;
                let mut i = self.controls.len();
                while i > 0 {
                    i -= 1;
                    if self.controls[i] == 2 {
                        if matches!(op, Op::Break) {
                            depth += 1;
                        }
                        break;
                    }
                    depth += 1;
                }
                self.bytes.push(0x0c);
                uleb(&mut self.bytes, depth);
            }
            Op::Return { src } => {
                self.read(*src);
                set(&mut self.bytes, self.scratch + 6);
                if self.frame_bytes != 0 {
                    get(&mut self.bytes, self.frame);
                    call(&mut self.bytes, FREE);
                    self.bytes.push(0x1a);
                }
                get(&mut self.bytes, self.scratch + 6);
                self.bytes.push(0x0f);
            }
            Op::Trap { message } => {
                let mut index = 0;
                while self.messages[index] != *message {
                    index += 1;
                }
                trap(&mut self.bytes, 16 + index as i64, line);
            }
            Op::Struct { dst, fields } | Op::Variant { dst, fields, .. } => {
                let tag = match op {
                    Op::Variant { variant, .. } => Some(*variant),
                    _ => None,
                };
                let words = fields.len() + if tag.is_some() { 1 } else { 0 };
                constant(&mut self.bytes, (words.max(1) as i64) * 8);
                call(&mut self.bytes, ALLOC);
                self.write(*dst);
                if let Some(variant) = tag {
                    self.read(*dst);
                    constant(&mut self.bytes, variant as i64);
                    store(&mut self.bytes, 0);
                }
                let mut i = 0;
                while i < fields.len() {
                    self.read(*dst);
                    self.read(fields[i]);
                    store(
                        &mut self.bytes,
                        ((i + if tag.is_some() { 1 } else { 0 }) as u64) * 8,
                    );
                    i += 1;
                }
            }
            Op::TagTest { dst, src, variant } => {
                self.read(*src);
                if matches!(
                    self.program.types[self.function.locals[*src].ty].kind,
                    TypeKind::Ref { .. }
                ) {
                    load(&mut self.bytes, 0);
                }
                load(&mut self.bytes, 0);
                constant(&mut self.bytes, *variant as i64);
                self.bytes.extend_from_slice(&[0x51, 0xad]);
                self.write(*dst);
            }

            Op::Discriminant { dst, src } => {
                self.read(*src);
                load(&mut self.bytes, 0);
                self.write(*dst);
            }
            Op::Intrinsic { dst, name, args } => self.intrinsic(*dst, name, args, line)?,
        }
        Ok(())
    }
    fn intrinsic(
        &mut self,
        dst: usize,
        name: &str,
        args: &[usize],
        line: usize,
    ) -> Result<(), String> {
        match name {
            "assert" => {
                self.read(args[0]);
                self.bytes.push(0x50);
                trap_if(&mut self.bytes, 5, line);
                constant(&mut self.bytes, 0);
                self.write(dst);
            }
            "cast" => {
                let from = &self.program.types[self.function.locals[args[0]].ty].kind;
                let to = &self.program.types[self.function.locals[dst].ty].kind;
                self.read(args[0]);
                match (from, to) {
                    (TypeKind::F64, TypeKind::F64) => {}
                    (TypeKind::Int { signed, .. }, TypeKind::F64) => {
                        self.bytes.push(if *signed { 0xb9 } else { 0xba });
                        self.bytes.push(0xbd);
                    }
                    (TypeKind::F64, TypeKind::Int { bits, signed }) => {
                        self.bytes.push(0xbf);
                        self.bytes
                            .extend_from_slice(&[0xfc, if *signed { 6 } else { 7 }]);
                        if *bits < 64 {
                            set(&mut self.bytes, self.scratch);
                            let maximum = if *signed {
                                (1_i64 << (*bits - 1)) - 1
                            } else {
                                (1_i64 << *bits) - 1
                            };
                            if *signed {
                                let minimum = -(1_i64 << (*bits - 1));
                                get(&mut self.bytes, self.scratch);
                                constant(&mut self.bytes, minimum);
                                self.bytes.push(0x53);
                                begin_if(&mut self.bytes);
                                constant(&mut self.bytes, minimum);
                                set(&mut self.bytes, self.scratch);
                                self.bytes.push(0x0b);
                            }
                            get(&mut self.bytes, self.scratch);
                            constant(&mut self.bytes, maximum);
                            self.bytes.push(if *signed { 0x55 } else { 0x56 });
                            begin_if(&mut self.bytes);
                            constant(&mut self.bytes, maximum);
                            set(&mut self.bytes, self.scratch);
                            self.bytes.push(0x0b);
                            get(&mut self.bytes, self.scratch);
                        }
                    }
                    (_, TypeKind::Int { bits, signed }) => {
                        if *bits < 64 {
                            if *signed {
                                constant(&mut self.bytes, (64 - *bits) as i64);
                                self.bytes.push(0x86);
                                constant(&mut self.bytes, (64 - *bits) as i64);
                                self.bytes.push(0x87);
                            } else {
                                constant(&mut self.bytes, (1_i64 << *bits) - 1);
                                self.bytes.push(0x83);
                            }
                        }
                    }
                    _ => return Err("unsupported scalar conversion".to_string()),
                }
                self.write(dst);
            }

            "replace" => {
                self.read(args[0]);
                tee(&mut self.bytes, self.scratch + 3);
                load(&mut self.bytes, 0);
                self.write(dst);
                get(&mut self.bytes, self.scratch + 3);
                self.read(args[1]);
                store(&mut self.bytes, 0);
            }

            "box_new" => {
                constant(&mut self.bytes, 8);
                call(&mut self.bytes, ALLOC);
                self.write(dst);
                self.read(dst);
                self.read(args[0]);
                store(&mut self.bytes, 0);
            }
            "box_get" => {
                self.read(args[0]);
                if matches!(
                    self.program.types[self.function.locals[args[0]].ty].kind,
                    TypeKind::Ref { .. }
                ) {
                    load(&mut self.bytes, 0);
                }
                load(&mut self.bytes, 0);
                self.write(dst);
            }
            "vec_new" => {
                constant(&mut self.bytes, 24);
                call(&mut self.bytes, ALLOC);
                self.write(dst);
                for offset in [0, 8, 16] {
                    self.read(dst);
                    constant(&mut self.bytes, 0);
                    store(&mut self.bytes, offset);
                }
            }
            "vec_pop" => {
                self.read(args[0]);
                if matches!(
                    self.program.types[self.function.locals[args[0]].ty].kind,
                    TypeKind::Ref { .. }
                ) {
                    load(&mut self.bytes, 0);
                }
                set(&mut self.bytes, self.scratch + 4);
                get(&mut self.bytes, self.scratch + 4);
                load(&mut self.bytes, 8);
                set(&mut self.bytes, self.scratch + 5);
                constant(&mut self.bytes, 16);
                call(&mut self.bytes, ALLOC);
                self.write(dst);
                get(&mut self.bytes, self.scratch + 5);
                self.bytes.push(0x50);
                begin_if(&mut self.bytes);
                self.read(dst);
                constant(&mut self.bytes, 0);
                store(&mut self.bytes, 0);
                self.bytes.push(0x05);
                get(&mut self.bytes, self.scratch + 5);
                constant(&mut self.bytes, 1);
                self.bytes.push(0x7d);
                set(&mut self.bytes, self.scratch + 5);
                get(&mut self.bytes, self.scratch + 4);
                get(&mut self.bytes, self.scratch + 5);
                store(&mut self.bytes, 8);
                get(&mut self.bytes, self.scratch + 4);
                load(&mut self.bytes, 0);
                get(&mut self.bytes, self.scratch + 5);
                constant(&mut self.bytes, 8);
                self.bytes.push(0x7e);
                self.bytes.push(0x7c);
                set(&mut self.bytes, self.scratch + 3);
                self.read(dst);
                constant(&mut self.bytes, 1);
                store(&mut self.bytes, 0);
                self.read(dst);
                get(&mut self.bytes, self.scratch + 3);
                load(&mut self.bytes, 0);
                store(&mut self.bytes, 8);
                get(&mut self.bytes, self.scratch + 3);
                constant(&mut self.bytes, 0);
                store(&mut self.bytes, 0);
                self.bytes.push(0x0b);
            }

            "vec_push" => {
                self.read(args[0]);
                if matches!(
                    self.program.types[self.function.locals[args[0]].ty].kind,
                    TypeKind::Ref { .. }
                ) {
                    load(&mut self.bytes, 0);
                }
                set(&mut self.bytes, self.scratch + 4);
                get(&mut self.bytes, self.scratch + 4);
                load(&mut self.bytes, 8);
                set(&mut self.bytes, self.scratch + 5);
                get(&mut self.bytes, self.scratch + 4);
                load(&mut self.bytes, 16);
                set(&mut self.bytes, self.scratch + 6);
                get(&mut self.bytes, self.scratch + 5);
                get(&mut self.bytes, self.scratch + 6);
                self.bytes.push(0x51);
                begin_if(&mut self.bytes);
                get(&mut self.bytes, self.scratch + 6);
                self.bytes.push(0x50);
                self.bytes.extend_from_slice(&[0x04, 0x7e]);
                constant(&mut self.bytes, 4);
                self.bytes.push(0x05);
                get(&mut self.bytes, self.scratch + 6);
                constant(&mut self.bytes, 2);
                self.bytes.push(0x7e);
                self.bytes.push(0x0b);
                set(&mut self.bytes, self.scratch + 6);
                get(&mut self.bytes, self.scratch + 6);
                constant(&mut self.bytes, (u64::MAX / 8) as i64);
                self.bytes.push(0x56);
                trap_if(&mut self.bytes, 6, line);
                get(&mut self.bytes, self.scratch + 6);
                constant(&mut self.bytes, 8);
                self.bytes.push(0x7e);
                call(&mut self.bytes, ALLOC);
                set(&mut self.bytes, self.scratch + 3);
                get(&mut self.bytes, self.scratch + 3);
                get(&mut self.bytes, self.scratch + 4);
                load(&mut self.bytes, 0);
                get(&mut self.bytes, self.scratch + 5);
                constant(&mut self.bytes, 8);
                self.bytes.push(0x7e);
                self.bytes.extend_from_slice(&[0xfc, 10, 0, 0]);
                get(&mut self.bytes, self.scratch + 4);
                load(&mut self.bytes, 0);
                call(&mut self.bytes, FREE);
                self.bytes.push(0x1a);
                get(&mut self.bytes, self.scratch + 4);
                get(&mut self.bytes, self.scratch + 3);
                store(&mut self.bytes, 0);
                get(&mut self.bytes, self.scratch + 4);
                get(&mut self.bytes, self.scratch + 6);
                store(&mut self.bytes, 16);
                self.bytes.push(0x0b);
                get(&mut self.bytes, self.scratch + 4);
                load(&mut self.bytes, 0);
                get(&mut self.bytes, self.scratch + 5);
                constant(&mut self.bytes, 8);
                self.bytes.push(0x7e);
                self.bytes.push(0x7c);
                self.read(args[1]);
                store(&mut self.bytes, 0);
                get(&mut self.bytes, self.scratch + 4);
                get(&mut self.bytes, self.scratch + 5);
                constant(&mut self.bytes, 1);
                self.bytes.push(0x7c);
                store(&mut self.bytes, 8);
                constant(&mut self.bytes, 0);
                self.write(dst);
            }
            "clone" => {
                self.read(args[0]);
                let ty = match self.program.types[self.function.locals[args[0]].ty].kind {
                    TypeKind::Ref { inner, .. } => {
                        load(&mut self.bytes, 0);
                        inner
                    }
                    _ => self.function.locals[args[0]].ty,
                };
                call(&mut self.bytes, helper_index(self.program, ty, true));
                self.write(dst);
            }

            "str_as_bytes" => {
                // Both shared views retain the original header cell and origin.
                self.read(args[0]);
                self.write(dst);
            }
            "str_len" => {
                self.read(args[0]);
                load(&mut self.bytes, 0);
                load(&mut self.bytes, 8);
                self.write(dst);
            }
            "str_from_utf8" => {
                self.read(args[0]);
                call(&mut self.bytes, UTF8_VALIDATE);
                set(&mut self.bytes, self.scratch + 4);
                constant(&mut self.bytes, 16);
                call(&mut self.bytes, ALLOC);
                self.write(dst);
                get(&mut self.bytes, self.scratch + 4);
                self.bytes.push(0x50);
                begin_if(&mut self.bytes);
                self.read(dst);
                constant(&mut self.bytes, 1);
                store(&mut self.bytes, 0);
                self.read(dst);
                constant(&mut self.bytes, 8);
                call(&mut self.bytes, ALLOC);
                store(&mut self.bytes, 8);
                self.bytes.push(0x05);
                self.read(dst);
                constant(&mut self.bytes, 0);
                store(&mut self.bytes, 0);
                self.read(dst);
                self.read(args[0]);
                store(&mut self.bytes, 8);
                self.bytes.push(0x0b);
            }
            "vec_len" => {
                self.read(args[0]);
                if matches!(
                    self.program.types[self.function.locals[args[0]].ty].kind,
                    TypeKind::Ref { .. }
                ) {
                    load(&mut self.bytes, 0);
                }
                load(&mut self.bytes, 8);
                self.write(dst);
            }
            "drop" => {
                self.read(args[0]);
                call(
                    &mut self.bytes,
                    helper_index(self.program, self.function.locals[args[0]].ty, false),
                );
                self.bytes.push(0x1a);
                constant(&mut self.bytes, 0);
                self.write(dst);
            }
            _ => return Err(format!("unsupported intrinsic {name}")),
        }
        Ok(())
    }
}
fn helper_index(program: &Program, ty: usize, copying: bool) -> usize {
    FIRST_USER + program.functions.len() + ty * 2 + if copying { 1 } else { 0 }
}
fn heap_type(kind: &TypeKind) -> bool {
    matches!(
        kind,
        TypeKind::Struct { .. }
            | TypeKind::Enum { .. }
            | TypeKind::Box { .. }
            | TypeKind::Vec { .. }
            | TypeKind::Slice { .. }
            | TypeKind::Array { .. }
    )
}
fn allocator() -> Vec<u8> {
    // A block stores its byte extent and the next free block before the payload.
    // First fit reuses released blocks. The memory grows only when none fits.
    let mut b = vec![1, 8, 0x7e];
    get(&mut b, 0);
    constant(&mut b, 7);
    b.push(0x7c);
    tee(&mut b, 1);
    get(&mut b, 0);
    b.push(0x54);
    trap_if(&mut b, 6, 0);
    get(&mut b, 1);
    constant(&mut b, -8);
    b.push(0x83);
    set(&mut b, 0);
    get(&mut b, 0);
    b.push(0x50);
    begin_if(&mut b);
    constant(&mut b, 8);
    set(&mut b, 0);
    b.push(0x0b);
    b.extend_from_slice(&[0x23, 1]);
    set(&mut b, 2);
    constant(&mut b, 0);
    set(&mut b, 3);
    b.extend_from_slice(&[0x02, 0x40, 0x03, 0x40]);
    get(&mut b, 2);
    b.push(0x50);
    b.extend_from_slice(&[0x0d, 1]);
    get(&mut b, 2);
    load(&mut b, 0);
    get(&mut b, 0);
    b.push(0x5a);
    begin_if(&mut b);
    get(&mut b, 2);
    load(&mut b, 8);
    set(&mut b, 4);
    get(&mut b, 3);
    b.push(0x50);
    begin_if(&mut b);
    get(&mut b, 4);
    b.extend_from_slice(&[0x24, 1]);
    b.push(0x05);
    get(&mut b, 3);
    get(&mut b, 4);
    store(&mut b, 8);
    b.push(0x0b);
    get(&mut b, 2);
    constant(&mut b, 16);
    b.push(0x7c);
    b.push(0x0f);
    b.push(0x0b);
    get(&mut b, 2);
    set(&mut b, 3);
    get(&mut b, 2);
    load(&mut b, 8);
    set(&mut b, 2);
    b.extend_from_slice(&[0x0c, 0, 0x0b, 0x0b]);
    b.extend_from_slice(&[0x23, 0]);
    set(&mut b, 1);
    get(&mut b, 0);
    constant(&mut b, 16);
    b.push(0x7c);
    tee(&mut b, 4);
    get(&mut b, 0);
    b.push(0x54);
    trap_if(&mut b, 6, 0);
    get(&mut b, 1);
    get(&mut b, 4);
    b.push(0x7c);
    tee(&mut b, 5);
    get(&mut b, 1);
    b.push(0x54);
    trap_if(&mut b, 6, 0);
    get(&mut b, 5);
    constant(&mut b, 65535);
    b.push(0x7c);
    tee(&mut b, 6);
    get(&mut b, 5);
    b.push(0x54);
    trap_if(&mut b, 6, 0);
    get(&mut b, 6);
    constant(&mut b, 16);
    b.push(0x88);
    set(&mut b, 6);
    get(&mut b, 6);
    b.extend_from_slice(&[0x3f, 0]);
    b.push(0x56);
    begin_if(&mut b);
    get(&mut b, 6);
    b.extend_from_slice(&[0x3f, 0]);
    b.push(0x7d);
    b.extend_from_slice(&[0x40, 0]);
    constant(&mut b, -1);
    b.push(0x51);
    trap_if(&mut b, 6, 0);
    b.push(0x0b);
    get(&mut b, 5);
    b.extend_from_slice(&[0x24, 0]);
    get(&mut b, 1);
    get(&mut b, 0);
    store(&mut b, 0);
    get(&mut b, 1);
    constant(&mut b, 0);
    store(&mut b, 8);
    get(&mut b, 1);
    constant(&mut b, 16);
    b.push(0x7c);
    b.push(0x0b);
    b
}
fn deallocator() -> Vec<u8> {
    let mut b = vec![1, 1, 0x7e];
    get(&mut b, 0);
    b.push(0x50);
    begin_if(&mut b);
    constant(&mut b, 0);
    b.push(0x0f);
    b.push(0x0b);
    get(&mut b, 0);
    constant(&mut b, 16);
    b.push(0x7d);
    set(&mut b, 1);
    get(&mut b, 1);
    b.extend_from_slice(&[0x23, 1]);
    store(&mut b, 8);
    get(&mut b, 1);
    b.extend_from_slice(&[0x24, 1]);
    constant(&mut b, 0);
    b.push(0x0b);
    b
}

// Validate a shared byte slice before creating a text view. Continuation state
// encodes the shortest form, excludes surrogates, and caps scalars at U+10FFFF.
// The byte slots and extent are already covered by checked slice construction.
fn utf8_validator() -> Vec<u8> {
    // 0=input, 1=data, 2=length, 3=index, 4=remaining, 5=low, 6=high, 7=byte.
    let mut b = vec![1, 7, 0x7e];
    get(&mut b, 0);
    load(&mut b, 0);
    set(&mut b, 1);
    get(&mut b, 1);
    load(&mut b, 8);
    set(&mut b, 2);
    get(&mut b, 1);
    load(&mut b, 0);
    set(&mut b, 1);
    b.extend_from_slice(&[0x02, 0x40, 0x03, 0x40]);
    get(&mut b, 3);
    get(&mut b, 2);
    b.push(0x5a);
    b.extend_from_slice(&[0x0d, 1]);
    get(&mut b, 1);
    get(&mut b, 3);
    constant(&mut b, 8);
    b.push(0x7e);
    b.push(0x7c);
    load(&mut b, 0);
    set(&mut b, 7);
    get(&mut b, 4);
    constant(&mut b, 0);
    b.push(0x52);
    begin_if(&mut b);
    get(&mut b, 7);
    get(&mut b, 5);
    b.push(0x54);
    get(&mut b, 7);
    get(&mut b, 6);
    b.push(0x56);
    b.push(0x72);
    begin_if(&mut b);
    constant(&mut b, 0);
    b.push(0x0f);
    b.push(0x0b);
    get(&mut b, 4);
    constant(&mut b, 1);
    b.push(0x7d);
    set(&mut b, 4);
    constant(&mut b, 128);
    set(&mut b, 5);
    constant(&mut b, 191);
    set(&mut b, 6);
    b.push(0x05);
    get(&mut b, 7);
    constant(&mut b, 128);
    b.push(0x5a);
    begin_if(&mut b);
    constant(&mut b, 128);
    set(&mut b, 5);
    constant(&mut b, 191);
    set(&mut b, 6);
    for (low, high, remaining) in [(194, 223, 1), (224, 239, 2), (240, 244, 3)] {
        get(&mut b, 7);
        constant(&mut b, low);
        b.push(0x5a);
        get(&mut b, 7);
        constant(&mut b, high);
        b.push(0x58);
        b.push(0x71);
        begin_if(&mut b);
        constant(&mut b, remaining);
        set(&mut b, 4);
        if remaining > 1 {
            for (first, bound, local) in if remaining == 2 {
                [(224, 160, 5), (237, 159, 6)]
            } else {
                [(240, 144, 5), (244, 143, 6)]
            } {
                get(&mut b, 7);
                constant(&mut b, first);
                b.push(0x51);
                begin_if(&mut b);
                constant(&mut b, bound);
                set(&mut b, local);
                b.push(0x0b);
            }
        }
        b.push(0x05);
    }
    constant(&mut b, 0);
    b.push(0x0f);
    b.extend_from_slice(&[0x0b, 0x0b, 0x0b, 0x0b, 0x0b]);
    get(&mut b, 3);
    constant(&mut b, 1);
    b.push(0x7c);
    set(&mut b, 3);
    b.extend_from_slice(&[0x0c, 0, 0x0b, 0x0b]);
    get(&mut b, 4);
    b.extend_from_slice(&[0x50, 0xad, 0x0b]);
    b
}

fn copy_fields(b: &mut Vec<u8>, program: &Program, fields: &[crate::k::Field], first: u64) {
    for (i, field) in fields.iter().enumerate() {
        let offset = (first + i as u64) * 8;
        get(b, 1);
        get(b, 0);
        load(b, offset);
        call(b, helper_index(program, field.ty, true));
        store(b, offset);
    }
}

fn copy_helper(program: &Program, ty: usize) -> Vec<u8> {
    let mut b = vec![1, 5, 0x7e];
    let kind = &program.types[ty].kind;
    if !heap_type(kind) {
        get(&mut b, 0);
        b.push(0x0b);
        return b;
    }
    get(&mut b, 0);
    b.push(0x50);
    begin_if(&mut b);
    constant(&mut b, 0);
    b.push(0x0f);
    b.push(0x0b);
    match kind {
        TypeKind::Struct { fields, .. } => {
            constant(&mut b, fields.len().max(1) as i64 * 8);
            call(&mut b, ALLOC);
            set(&mut b, 1);
            copy_fields(&mut b, program, fields, 0);
        }
        TypeKind::Enum { variants, .. } => {
            get(&mut b, 0);
            load(&mut b, 0);
            set(&mut b, 2);
            for (variant, value) in variants.iter().enumerate() {
                get(&mut b, 2);
                constant(&mut b, variant as i64);
                b.push(0x51);
                begin_if(&mut b);
                constant(&mut b, (value.fields.len() + 1) as i64 * 8);
                call(&mut b, ALLOC);
                set(&mut b, 1);
                get(&mut b, 1);
                get(&mut b, 2);
                store(&mut b, 0);
                copy_fields(&mut b, program, &value.fields, 1);
                b.push(0x0b);
            }
        }
        TypeKind::Box { inner } => {
            constant(&mut b, 8);
            call(&mut b, ALLOC);
            tee(&mut b, 1);
            get(&mut b, 0);
            load(&mut b, 0);
            call(&mut b, helper_index(program, *inner, true));
            store(&mut b, 0);
        }
        TypeKind::Vec { inner } | TypeKind::Array { inner, .. } => {
            let inner = *inner;
            get(&mut b, 0);
            load(&mut b, 0);
            set(&mut b, 2);
            get(&mut b, 0);
            load(&mut b, 8);
            set(&mut b, 3);
            constant(&mut b, 24);
            call(&mut b, ALLOC);
            set(&mut b, 1);
            get(&mut b, 3);
            constant(&mut b, 8);
            b.push(0x7e);
            call(&mut b, ALLOC);
            set(&mut b, 5);
            get(&mut b, 1);
            get(&mut b, 5);
            store(&mut b, 0);
            get(&mut b, 1);
            get(&mut b, 3);
            store(&mut b, 8);
            get(&mut b, 1);
            get(&mut b, 3);
            store(&mut b, 16);
            constant(&mut b, 0);
            set(&mut b, 4);
            b.extend_from_slice(&[0x02, 0x40, 0x03, 0x40]);
            get(&mut b, 4);
            get(&mut b, 3);
            b.push(0x5a);
            b.extend_from_slice(&[0x0d, 1]);
            get(&mut b, 5);
            get(&mut b, 4);
            constant(&mut b, 8);
            b.push(0x7e);
            b.push(0x7c);
            get(&mut b, 2);
            get(&mut b, 4);
            constant(&mut b, 8);
            b.push(0x7e);
            b.push(0x7c);
            load(&mut b, 0);
            call(&mut b, helper_index(program, inner, true));
            store(&mut b, 0);
            get(&mut b, 4);
            constant(&mut b, 1);
            b.push(0x7c);
            set(&mut b, 4);
            b.extend_from_slice(&[0x0c, 0, 0x0b, 0x0b]);
        }
        TypeKind::Slice { .. } => {
            constant(&mut b, 16);
            call(&mut b, ALLOC);
            set(&mut b, 1);
            get(&mut b, 1);
            get(&mut b, 0);
            constant(&mut b, 16);
            b.extend_from_slice(&[0xfc, 10, 0, 0]);
        }
        _ => unreachable!(),
    }
    get(&mut b, 1);
    b.push(0x0b);
    b
}

// A cleanup frame is { value, type, next_child, child_type }, each one i64.
// One growable stack holds the active depth-first path. The dispatcher calls a
// resumable step; zero means complete, otherwise the result is the next child.
// A custom destructor is an ordinary call before the first child. Its &mut self
// points at the value slot, which remains stable until that call returns.
// Reentrant cleanup gets its own stack.
fn drop_dispatcher() -> Vec<u8> {
    // 0 base, 1 top byte offset, 2 capacity bytes, 3 child, 4 child type,
    // 5 replacement base, 6 replacement capacity, 7 current frame.
    let mut b = vec![1, 7, 0x7e];
    constant(&mut b, 32);
    set(&mut b, 2);
    b.extend_from_slice(&[0x03, 0x40]);
    get(&mut b, 0);
    get(&mut b, 1);
    b.push(0x7c);
    tee(&mut b, 7);
    get(&mut b, 7);
    load(&mut b, 8);
    b.push(0xa7);
    b.extend_from_slice(&[0x11, 1, 0]); // call_indirect (i64) -> i64, table 0
    set(&mut b, 3);
    get(&mut b, 3);
    b.push(0x50);
    begin_if(&mut b);
    get(&mut b, 1);
    b.push(0x50);
    begin_if(&mut b);
    get(&mut b, 0);
    call(&mut b, FREE);
    b.push(0x0f);
    b.push(0x0b);
    get(&mut b, 1);
    constant(&mut b, 32);
    b.push(0x7d);
    set(&mut b, 1);
    b.push(0x05);
    get(&mut b, 7);
    load(&mut b, 24);
    set(&mut b, 4);
    get(&mut b, 1);
    constant(&mut b, 32);
    b.push(0x7c);
    set(&mut b, 1);
    get(&mut b, 1);
    get(&mut b, 2);
    b.push(0x51);
    begin_if(&mut b);
    get(&mut b, 2);
    constant(&mut b, 2);
    b.push(0x7e);
    tee(&mut b, 6);
    get(&mut b, 2);
    b.push(0x54);
    trap_if(&mut b, 6, 0);
    get(&mut b, 6);
    call(&mut b, ALLOC);
    set(&mut b, 5);
    get(&mut b, 5);
    get(&mut b, 0);
    get(&mut b, 2);
    b.extend_from_slice(&[0xfc, 10, 0, 0]);
    get(&mut b, 0);
    call(&mut b, FREE);
    b.push(0x1a);
    get(&mut b, 5);
    set(&mut b, 0);
    get(&mut b, 6);
    set(&mut b, 2);
    b.push(0x0b);
    get(&mut b, 0);
    get(&mut b, 1);
    b.push(0x7c);
    set(&mut b, 7);
    get(&mut b, 7);
    get(&mut b, 3);
    store(&mut b, 0);
    get(&mut b, 7);
    get(&mut b, 4);
    store(&mut b, 8);
    get(&mut b, 7);
    constant(&mut b, 0);
    store(&mut b, 16);
    b.push(0x0b);
    b.extend_from_slice(&[0x0c, 0, 0x0b]);
    constant(&mut b, 0);
    b.push(0x0b);
    b
}

fn drop_start(program: &Program, ty: usize) -> Vec<u8> {
    let mut b = vec![1, 1, 0x7e];
    if !heap_type(&program.types[ty].kind) {
        constant(&mut b, 0);
        b.push(0x0b);
        return b;
    }
    get(&mut b, 0);
    b.push(0x50);
    begin_if(&mut b);
    constant(&mut b, 0);
    b.push(0x0f);
    b.push(0x0b);
    constant(&mut b, 32);
    call(&mut b, ALLOC);
    set(&mut b, 1);
    get(&mut b, 1);
    get(&mut b, 0);
    store(&mut b, 0);
    get(&mut b, 1);
    constant(&mut b, ty as i64);
    store(&mut b, 8);
    get(&mut b, 1);
    constant(&mut b, 0);
    store(&mut b, 16);
    get(&mut b, 1);
    constant(&mut b, 0);
    store(&mut b, 24);
    get(&mut b, 1);
    call(&mut b, DROP_RUN);
    b.push(0x0b);
    b
}

// The child's value is on the operand stack, and the parent continuation was
// already advanced. Zero marks a moved field, so continue to its next sibling.
fn yield_drop_child(b: &mut Vec<u8>, ty: usize) {
    set(b, 3);
    get(b, 3);
    b.push(0x50);
    b.push(0x45);
    begin_if(b);
    get(b, 0);
    constant(b, ty as i64);
    store(b, 24);
    get(b, 3);
    b.push(0x0f);
    b.push(0x0b);
}

fn drop_fields(b: &mut Vec<u8>, program: &Program, fields: &[crate::k::Field], first: u64) {
    let mut next = 1;
    for (i, field) in fields.iter().enumerate() {
        if heap_type(&program.types[field.ty].kind) {
            get(b, 0);
            load(b, 16);
            constant(b, next);
            b.push(0x51);
            begin_if(b);
            get(b, 0);
            constant(b, next + 1);
            store(b, 16);
            get(b, 1);
            load(b, (first + i as u64) * 8);
            yield_drop_child(b, field.ty);
            b.push(0x0b);
            next += 1;
        }
    }
}

fn drop_step(program: &Program, ty: usize, destructor: Option<usize>) -> Vec<u8> {
    let mut b = vec![1, 4, 0x7e];
    let kind = &program.types[ty].kind;
    get(&mut b, 0);
    load(&mut b, 16);
    b.push(0x50);
    begin_if(&mut b);
    if let Some(function) = destructor {
        get(&mut b, 0);
        call(&mut b, FIRST_USER + function);
        b.push(0x1a);
    }
    get(&mut b, 0);
    constant(&mut b, 1);
    store(&mut b, 16);
    b.push(0x0b);
    get(&mut b, 0);
    load(&mut b, 0);
    set(&mut b, 1);
    match kind {
        TypeKind::Struct { fields, .. } => drop_fields(&mut b, program, fields, 0),
        TypeKind::Enum { variants, .. } => {
            get(&mut b, 1);
            load(&mut b, 0);
            set(&mut b, 2);
            for (variant, value) in variants.iter().enumerate() {
                get(&mut b, 2);
                constant(&mut b, variant as i64);
                b.push(0x51);
                begin_if(&mut b);
                drop_fields(&mut b, program, &value.fields, 1);
                b.push(0x0b);
            }
        }
        TypeKind::Box { inner } => {
            if heap_type(&program.types[*inner].kind) {
                get(&mut b, 0);
                load(&mut b, 16);
                constant(&mut b, 1);
                b.push(0x51);
                begin_if(&mut b);
                get(&mut b, 0);
                constant(&mut b, 2);
                store(&mut b, 16);
                get(&mut b, 1);
                load(&mut b, 0);
                yield_drop_child(&mut b, *inner);
                b.push(0x0b);
            }
        }
        TypeKind::Vec { inner } | TypeKind::Array { inner, .. } => {
            let inner = *inner;
            if heap_type(&program.types[inner].kind) {
                b.extend_from_slice(&[0x02, 0x40, 0x03, 0x40]);
                get(&mut b, 0);
                load(&mut b, 16);
                set(&mut b, 2);
                get(&mut b, 2);
                constant(&mut b, 1);
                b.push(0x7d);
                set(&mut b, 3);
                get(&mut b, 3);
                get(&mut b, 1);
                load(&mut b, 8);
                b.push(0x5a);
                b.extend_from_slice(&[0x0d, 1]);
                get(&mut b, 0);
                get(&mut b, 2);
                constant(&mut b, 1);
                b.push(0x7c);
                store(&mut b, 16);
                get(&mut b, 1);
                load(&mut b, 0);
                get(&mut b, 3);
                constant(&mut b, 8);
                b.push(0x7e);
                b.push(0x7c);
                load(&mut b, 0);
                yield_drop_child(&mut b, inner);
                b.extend_from_slice(&[0x0c, 0, 0x0b, 0x0b]);
            }
            get(&mut b, 1);
            load(&mut b, 0);
            call(&mut b, FREE);
            b.push(0x1a);
        }
        _ => {}
    }
    if heap_type(kind) {
        get(&mut b, 1);
        call(&mut b, FREE);
        b.push(0x1a);
    }
    constant(&mut b, 0);
    b.push(0x0b);
    b
}

fn drop_step_index(program: &Program, ty: usize) -> usize {
    FIRST_USER + program.functions.len() + program.types.len() * 2 + ty
}

// This pool is a lowering/performance choice, never an acceptance limit.
// Further simultaneously live values spill to a growing memory64 frame.
const REGISTER_POOL: usize = 64;

#[derive(Clone, Copy)]
enum Slot {
    Register(usize),
    Frame(u64),
}

struct LocalLayout {
    slots: Vec<Slot>,
    registers: usize,
    frame_bytes: u64,
}

struct LiveLoop {
    start: usize,
    end: usize,
    parent: Option<usize>,
}

fn touch_local(ranges: &mut [(usize, usize)], local: usize, at: usize) {
    ranges[local].0 = ranges[local].0.min(at);
    ranges[local].1 = at;
}

fn touch_place(ranges: &mut [(usize, usize)], place: &Place, at: usize) {
    touch_local(ranges, place.local, at);
    for projection in &place.projections {
        if let Projection::Index(local) = projection {
            touch_local(ranges, *local, at);
        }
    }
}

// Conservative intervals include definitions and uses, and extend values entering
// a loop through its back edge. Address-taken slots remain stable for the entire
// invocation, including uses through aliases not spelled as a local access.
fn local_layout(program: &Program, function: &Function) -> Result<LocalLayout, String> {
    let n = function.locals.len();
    let mut addressed = vec![false; n];
    let mut ranges = vec![(usize::MAX, 0); n];
    let mut loops: Vec<LiveLoop> = Vec::new();
    let mut loop_at = vec![None; function.body.len()];
    let mut controls = Vec::new();
    let mut current_loop = None;
    for (at, instruction) in function.body.iter().enumerate() {
        loop_at[at] = current_loop;
        match &instruction.op {
            Op::Int { dst, .. }
            | Op::Float { dst, .. }
            | Op::Text { dst, .. }
            | Op::Unit { dst } => {
                touch_local(&mut ranges, *dst, at);
            }
            Op::Read { dst, place, .. }
            | Op::Borrow { dst, place, .. }
            | Op::SliceBorrow { dst, place, .. } => {
                touch_local(&mut ranges, *dst, at);
                touch_place(&mut ranges, place, at);
                if matches!(instruction.op, Op::Borrow { .. } | Op::SliceBorrow { .. })
                    && place.projections.is_empty()
                {
                    addressed[place.local] = true;
                }
            }
            Op::Array { dst, elements }
            | Op::Struct {
                dst,
                fields: elements,
            }
            | Op::Variant {
                dst,
                fields: elements,
                ..
            }
            | Op::Call {
                dst,
                args: elements,
                ..
            }
            | Op::Intrinsic {
                dst,
                args: elements,
                ..
            } => {
                touch_local(&mut ranges, *dst, at);
                for local in elements {
                    touch_local(&mut ranges, *local, at);
                }
            }
            Op::Assign { place, src } => {
                touch_place(&mut ranges, place, at);
                touch_local(&mut ranges, *src, at);
            }
            Op::Drop { place } => {
                if !place.projections.is_empty()
                    || heap_type(&program.types[function.locals[place.local].ty].kind)
                {
                    touch_place(&mut ranges, place, at);
                }
            }
            Op::Unary { dst, src, .. }
            | Op::Discriminant { dst, src }
            | Op::TagTest { dst, src, .. } => {
                touch_local(&mut ranges, *dst, at);
                touch_local(&mut ranges, *src, at);
            }
            Op::Binary {
                dst, left, right, ..
            } => {
                touch_local(&mut ranges, *dst, at);
                touch_local(&mut ranges, *left, at);
                touch_local(&mut ranges, *right, at);
            }
            Op::If { condition } => {
                touch_local(&mut ranges, *condition, at);
                controls.push(None);
            }
            Op::Loop => {
                let id = loops.len();
                loops.push(LiveLoop {
                    start: at,
                    end: at,
                    parent: current_loop,
                });
                controls.push(Some(id));
                current_loop = Some(id);
            }
            Op::End => {
                if let Some(id) = controls.pop().expect("checked structured control") {
                    loops[id].end = at;
                    current_loop = loops[id].parent;
                }
            }
            Op::Return { src } => touch_local(&mut ranges, *src, at),
            Op::Else | Op::Break | Op::Continue | Op::Trap { .. } => {}
        }
    }
    let mut slots = vec![Slot::Register(0); n];
    let mut frame_words = 0_u64;
    let mut starts = Vec::new();
    for local in 0..n {
        if addressed[local] {
            slots[local] = Slot::Frame(
                frame_words
                    .checked_mul(8)
                    .ok_or("local frame extent overflow")?,
            );
            frame_words = frame_words
                .checked_add(1)
                .ok_or("local frame extent overflow")?;
        } else if local < function.params {
            slots[local] = Slot::Register(local);
        } else if ranges[local].0 != usize::MAX {
            let first = ranges[local].0;
            let mut enclosing = loop_at[ranges[local].1];
            while let Some(id) = enclosing {
                if first >= loops[id].start {
                    break;
                }
                ranges[local].1 = ranges[local].1.max(loops[id].end);
                enclosing = loops[id].parent;
            }
            starts.push((first, local));
        }
    }
    starts.sort_unstable();
    let mut active = BTreeSet::new();
    let mut available_registers = Vec::new();
    let mut available_frames = Vec::new();
    let mut registers = function.params;
    for (first, local) in starts {
        while let Some(&(last, previous)) = active.first() {
            if last >= first {
                break;
            }
            active.pop_first();
            match slots[previous] {
                Slot::Register(register) => available_registers.push(register),
                Slot::Frame(offset) => available_frames.push(offset),
            }
        }
        slots[local] = if let Some(register) = available_registers.pop() {
            Slot::Register(register)
        } else if registers - function.params < REGISTER_POOL {
            let register = registers;
            registers += 1;
            Slot::Register(register)
        } else if let Some(offset) = available_frames.pop() {
            Slot::Frame(offset)
        } else {
            let offset = frame_words
                .checked_mul(8)
                .ok_or("local frame extent overflow")?;
            frame_words = frame_words
                .checked_add(1)
                .ok_or("local frame extent overflow")?;
            Slot::Frame(offset)
        };
        active.insert((ranges[local].1, local));
    }
    let frame_bytes = frame_words
        .checked_mul(8)
        .ok_or("local frame extent overflow")?;
    if registers > u32::MAX as usize - 9 {
        return Err(
            "WebAssembly parameter/local indices exceed the u32 representation".to_string(),
        );
    }
    Ok(LocalLayout {
        slots,
        registers,
        frame_bytes,
    })
}

fn function_body(
    program: &Program,
    function: &Function,
    messages: &[String],
    texts: &TextPool,
) -> Result<Vec<u8>, String> {
    let layout = local_layout(program, function)?;
    let mut body = Body {
        program,
        function,
        bytes: Vec::new(),
        slots: layout.slots,
        scratch: layout.registers,
        frame: layout.registers + 8,
        frame_bytes: layout.frame_bytes,
        controls: Vec::new(),
        messages,
        texts,
    };
    uleb(&mut body.bytes, 1);
    uleb(
        &mut body.bytes,
        (layout.registers - function.params + 9) as u64,
    );
    body.bytes.push(0x7e);
    if layout.frame_bytes != 0 {
        constant(&mut body.bytes, layout.frame_bytes as i64);
        call(&mut body.bytes, ALLOC);
        set(&mut body.bytes, body.frame);
        for i in 0..function.params {
            if let Slot::Frame(offset) = body.slots[i] {
                get(&mut body.bytes, body.frame);
                get(&mut body.bytes, i);
                store(&mut body.bytes, offset);
            }
        }
    }
    for instruction in &function.body {
        body.emit(&instruction.op, instruction.line)?;
    }
    if layout.frame_bytes != 0 {
        get(&mut body.bytes, body.frame);
        call(&mut body.bytes, FREE);
        body.bytes.push(0x1a);
    }
    constant(&mut body.bytes, 0);
    body.bytes.push(0x0b);
    Ok(body.bytes)
}
pub fn emit(checked: &CheckedProgram) -> Result<Vec<u8>, String> {
    let program = checked.program();
    let texts = TextPool::collect(program)?;
    let mut destructors = vec![None; program.types.len()];
    for (owner, function) in &program.destructors {
        destructors[*owner] = Some(*function);
    }
    let mut messages = Vec::new();
    for function in &program.functions {
        for instruction in &function.body {
            if let Op::Trap { message } = &instruction.op {
                if !messages.contains(message) {
                    messages.push(message.clone());
                }
            }
        }
    }
    if program.functions.len() as u64 + program.types.len() as u64 * 3 + FIRST_USER as u64
        > u32::MAX as u64
    {
        return Err("WebAssembly function indices exceed the u32 representation".to_string());
    }
    let mut arities = vec![1_usize];
    for f in &program.functions {
        if !arities.contains(&f.params) {
            arities.push(f.params);
        }
    }
    let mut out = vec![0, 97, 115, 109, 1, 0, 0, 0];
    let mut types = Vec::new();
    uleb(&mut types, (arities.len() + 1) as u64);
    types.extend_from_slice(&[0x60, 2, 0x7e, 0x7e, 0]);
    for arity in &arities {
        types.push(0x60);
        uleb(&mut types, *arity as u64);
        for _ in 0..*arity {
            types.push(0x7e);
        }
        types.extend_from_slice(&[1, 0x7e]);
    }
    section(&mut out, 1, &types)?;
    let mut imports = vec![1];
    text(&mut imports, "lkjscript");
    text(&mut imports, "trap");
    imports.extend_from_slice(&[0, 0]);
    section(&mut out, 2, &imports)?;
    let mut functions = Vec::new();
    uleb(
        &mut functions,
        (program.functions.len() + 4 + program.types.len() * 3) as u64,
    );
    uleb(&mut functions, 1);
    uleb(&mut functions, 1);
    uleb(&mut functions, 1);
    uleb(&mut functions, 1);
    for f in &program.functions {
        let mut index = 0;
        while arities[index] != f.params {
            index += 1;
        }
        uleb(&mut functions, (index + 1) as u64);
    }
    for _ in 0..program.types.len() * 3 {
        uleb(&mut functions, 1);
    }
    section(&mut out, 3, &functions)?;
    let mut tables = vec![1, 0x70, 0];
    uleb(&mut tables, program.types.len() as u64);
    section(&mut out, 4, &tables)?;
    let mut memory = vec![1, 4];
    uleb(&mut memory, texts.pages()?);
    section(&mut out, 5, &memory)?;
    let mut globals = vec![2, 0x7e, 1];
    constant(&mut globals, texts.heap_start()? as i64);
    globals.push(0x0b);
    globals.extend_from_slice(&[0x7e, 1]);
    constant(&mut globals, 0);
    globals.push(0x0b);
    section(&mut out, 6, &globals)?;
    let mut exports = Vec::new();
    uleb(&mut exports, (program.functions.len() + 1) as u64);
    text(&mut exports, "memory");
    exports.extend_from_slice(&[2, 0]);
    let mut i = 0;
    while i < program.functions.len() {
        text(&mut exports, &format!("fn:{}", program.functions[i].name));
        exports.push(0);
        uleb(&mut exports, (FIRST_USER + i) as u64);
        i += 1;
    }
    section(&mut out, 7, &exports)?;
    let mut elements = vec![1, 0, 0x41, 0, 0x0b];
    uleb(&mut elements, program.types.len() as u64);
    for ty in 0..program.types.len() {
        uleb(&mut elements, drop_step_index(program, ty) as u64);
    }
    section(&mut out, 9, &elements)?;
    let mut code = Vec::new();
    uleb(
        &mut code,
        (program.functions.len() + 4 + program.types.len() * 3) as u64,
    );
    let alloc = allocator();
    uleb(&mut code, alloc.len() as u64);
    code.extend_from_slice(&alloc);
    let free = deallocator();
    uleb(&mut code, free.len() as u64);
    code.extend_from_slice(&free);
    let dispatcher = drop_dispatcher();
    uleb(&mut code, dispatcher.len() as u64);
    code.extend_from_slice(&dispatcher);
    let validator = utf8_validator();
    uleb(&mut code, validator.len() as u64);
    code.extend_from_slice(&validator);
    for f in &program.functions {
        let body = function_body(program, f, &messages, &texts)?;
        uleb(&mut code, body.len() as u64);
        code.extend_from_slice(&body);
    }
    for ty in 0..program.types.len() {
        for copying in [false, true] {
            let body = if copying {
                copy_helper(program, ty)
            } else {
                drop_start(program, ty)
            };
            uleb(&mut code, body.len() as u64);
            code.extend_from_slice(&body);
        }
    }
    for (ty, destructor) in destructors.iter().enumerate() {
        let body = drop_step(program, ty, *destructor);
        uleb(&mut code, body.len() as u64);
        code.extend_from_slice(&body);
    }
    section(&mut out, 10, &code)?;
    if !texts.data.is_empty() {
        let mut data = vec![1, 0];
        constant(&mut data, 8);
        data.push(0x0b);
        uleb(&mut data, texts.data.len() as u64);
        data.extend_from_slice(&texts.data);
        section(&mut out, 11, &data)?;
    }
    let mut names = Vec::new();
    uleb(
        &mut names,
        (program.functions.len() + FIRST_USER + program.types.len() * 3) as u64,
    );
    uleb(&mut names, 0);
    text(&mut names, "trap");
    uleb(&mut names, ALLOC as u64);
    text(&mut names, "memory::allocate");
    uleb(&mut names, FREE as u64);
    text(&mut names, "memory::release");
    uleb(&mut names, DROP_RUN as u64);
    text(&mut names, "drop::walk");
    uleb(&mut names, UTF8_VALIDATE as u64);
    text(&mut names, "str::valid_utf8");
    let mut i = 0;
    while i < program.functions.len() {
        uleb(&mut names, (FIRST_USER + i) as u64);
        text(&mut names, &program.functions[i].name);
        i += 1;
    }
    for ty in 0..program.types.len() {
        for copying in [false, true] {
            uleb(&mut names, helper_index(program, ty, copying) as u64);
            text(
                &mut names,
                &format!(
                    "{}<{}>",
                    if copying { "copy" } else { "drop" },
                    program.types[ty].name
                ),
            );
        }
    }
    for ty in 0..program.types.len() {
        uleb(&mut names, drop_step_index(program, ty) as u64);
        text(
            &mut names,
            &format!("drop::step<{}>", program.types[ty].name),
        );
    }
    let mut custom = Vec::new();
    text(&mut custom, "name");
    custom.push(1);
    uleb(&mut custom, names.len() as u64);
    custom.extend_from_slice(&names);
    section(&mut out, 0, &custom)?;
    if !messages.is_empty() {
        let mut metadata = Vec::new();
        text(&mut metadata, "lkjscript.traps");
        metadata.push(b'[');
        let mut i = 0;
        while i < messages.len() {
            if i > 0 {
                metadata.push(b',');
            }
            metadata.push(b'"');
            for byte in messages[i].as_bytes() {
                match *byte {
                    b'"' => metadata.extend_from_slice(b"\\\""),
                    b'\\' => metadata.extend_from_slice(b"\\\\"),
                    0..=31 => metadata.extend_from_slice(format!("\\u{:04x}", byte).as_bytes()),
                    _ => metadata.push(*byte),
                }
            }
            metadata.push(b'"');
            i += 1;
        }
        metadata.push(b']');
        section(&mut out, 0, &metadata)?;
    }

    Ok(out)
}
