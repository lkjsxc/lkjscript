//! Checked K lowers to this typed, structured instruction form before encoding.
//! Values are bit patterns or linear-memory addresses. Loads and stores carry
//! their width; calls name ordinary functions, including generated glue and the
//! runtime. No instruction contains target bytecode or source/K type objects.
use crate::k::{
    CheckedProgram, Function as KFunction, Op, Place, Program as KProgram, Projection, TypeKind,
};
use std::collections::{BTreeMap, BTreeSet};

mod ir;
pub use ir::*;

fn aligned(value: u64, alignment: u8) -> Result<u64, String> {
    value
        .checked_add(u64::from(alignment) - 1)
        .map(|v| v & !(u64::from(alignment) - 1))
        .ok_or_else(|| "layout extent exceeds the address representation".to_string())
}
impl Layouts {
    fn new(program: &KProgram) -> Result<Self, String> {
        let mut types = Vec::new();
        for ty in &program.types {
            let storage = match ty.kind {
                TypeKind::Unit | TypeKind::Bool => Storage {
                    bytes: 1,
                    signed: false,
                },
                TypeKind::Int { bits, signed } => Storage {
                    bytes: bits / 8,
                    signed,
                },
                _ => Storage {
                    bytes: 8,
                    signed: false,
                },
            };
            types.push(TypeLayout {
                storage,
                payloads: Vec::new(),
            });
        }
        let mut result = Self { types };
        for (id, ty) in program.types.iter().enumerate() {
            let payloads = match &ty.kind {
                TypeKind::Struct { fields, .. } => vec![result.fields(fields, false)?],
                TypeKind::Enum { variants, .. } => {
                    let mut payloads = Vec::new();
                    for variant in variants {
                        payloads.push(result.fields(&variant.fields, true)?);
                    }
                    payloads
                }
                _ => Vec::new(),
            };
            result.types[id].payloads = payloads;
        }
        Ok(result)
    }
    fn fields(&self, fields: &[crate::k::Field], tag: bool) -> Result<AggregateLayout, String> {
        let mut extent = if tag { 8 } else { 0 };
        let mut alignment = if tag { 8 } else { 1 };
        let mut offsets = Vec::new();
        for field in fields {
            let storage = self.storage(field.ty);
            extent = aligned(extent, storage.bytes)?;
            alignment = alignment.max(storage.bytes);
            offsets.push(extent);
            extent = extent
                .checked_add(u64::from(storage.bytes))
                .ok_or("field extent exceeds the address representation")?;
        }
        Ok(AggregateLayout {
            bytes: aligned(extent.max(1), alignment)?,
            alignment,
            fields: offsets,
        })
    }
}

fn get(out: &mut Code, local: usize) {
    out.push(Instruction::LocalGet(local));
}
fn set(out: &mut Code, local: usize) {
    out.push(Instruction::LocalSet(local));
}
fn tee(out: &mut Code, local: usize) {
    out.push(Instruction::LocalTee(local));
}
fn constant(out: &mut Code, value: i64) {
    out.push(Instruction::I64(value));
}
fn call(out: &mut Code, function: usize) {
    out.push(Instruction::Call(function));
}
fn load(out: &mut Code, offset: u64) {
    out.push(Instruction::Load {
        bytes: 8,
        signed: false,
        offset,
    });
}
fn store(out: &mut Code, offset: u64) {
    out.push(Instruction::Store { bytes: 8, offset });
}
fn load_value(out: &mut Code, storage: Storage, offset: u64) {
    out.push(Instruction::Load {
        bytes: storage.bytes,
        signed: storage.signed,
        offset,
    });
}
fn store_value(out: &mut Code, storage: Storage, offset: u64) {
    out.push(Instruction::Store {
        bytes: storage.bytes,
        offset,
    });
}
fn begin_if(out: &mut Code) {
    out.extend_from_slice(&[Instruction::If(None)]);
}
fn trap(out: &mut Code, code: i64, line: usize) {
    constant(out, code);
    out.extend_from_slice(&[Instruction::GlobalSet(2)]);
    constant(out, line as i64);
    out.extend_from_slice(&[Instruction::GlobalSet(3)]);
    out.push(Instruction::Unreachable);
}
fn trap_if(out: &mut Code, code: i64, line: usize) {
    begin_if(out);
    trap(out, code, line);
    out.push(Instruction::End);
}
const ALLOC: usize = 0;
const FREE: usize = 1;
const DROP_RUN: usize = 2;
const UTF8_VALIDATE: usize = 3;
const FIRST_USER: usize = 4;

// The immutable region precedes the allocator. Each distinct literal has a
// header-pointer cell, a {data, length, capacity} header, then contiguous UTF-8 bytes.
// A text reference uses the same outer cell as an ordinary shared slice reference.
// No owned value points to this region, so neither copying nor cleanup frees it.
struct TextPool {
    offsets: BTreeMap<Vec<u8>, u64>,
    data: Vec<u8>,
}
impl TextPool {
    fn collect(program: &KProgram) -> Result<Self, String> {
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
                        .map_err(|_| "text length exceeds the 64-bit address representation")?;
                    let extent = length
                        .checked_add(32)
                        .ok_or("static text extent exceeds the 64-bit address representation")?;
                    let padded = aligned(pool.data.len() as u64, 8)?;
                    let padded = usize::try_from(padded)
                        .map_err(|_| "static text exceeds the host representation")?;
                    pool.data.resize(padded, 0);
                    let address = pool.heap_start()?;
                    address
                        .checked_add(extent)
                        .ok_or("static text address exceeds the 64-bit address representation")?;
                    let extent = usize::try_from(extent)
                        .map_err(|_| "static text extent exceeds the host representation")?;
                    let _size = pool
                        .data
                        .len()
                        .checked_add(extent)
                        .ok_or("static text size exceeds the host representation")?;
                    pool.data
                        .try_reserve(extent)
                        .map_err(|_| "allocation failed while lowering static text")?;
                    for value in [address + 8, address + 32, length, length] {
                        pool.data.extend_from_slice(&value.to_le_bytes());
                    }
                    pool.data.extend_from_slice(bytes);
                    pool.offsets.insert(bytes.clone(), address);
                }
            }
        }
        Ok(pool)
    }

    fn heap_start(&self) -> Result<u64, String> {
        u64::try_from(self.data.len())
            .ok()
            .and_then(|size| size.checked_add(15))
            .map(|size| size & !7)
            .ok_or_else(|| {
                "static text address exceeds the 64-bit address representation".to_string()
            })
    }

    fn pages(&self) -> Result<u64, String> {
        self.heap_start()?
            .checked_add(65535)
            .map(|extent| extent >> 16)
            .ok_or_else(|| "static text pages exceed the 64-bit address representation".to_string())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Control {
    Conditional,
    Loop,
}

struct Body<'a> {
    program: &'a KProgram,
    layout: &'a Layouts,
    function: &'a KFunction,
    code: Code,
    slots: Vec<Slot>,
    drop_flags: BTreeMap<usize, usize>,
    scratch: usize,
    frame: usize,
    frame_bytes: u64,
    controls: Vec<Control>,
    messages: &'a [String],
    texts: &'a TextPool,
    runtime: usize,
}
impl<'a> Body<'a> {
    fn read(&mut self, local: usize) {
        match self.slots[local] {
            Slot::Frame(offset) => {
                get(&mut self.code, self.frame);
                load_value(
                    &mut self.code,
                    self.layout.storage(self.function.locals[local].ty),
                    offset,
                );
            }
            Slot::Register(register) => get(&mut self.code, register),
        }
    }
    fn write_value(&mut self, local: usize) {
        match self.slots[local] {
            Slot::Frame(offset) => {
                set(&mut self.code, self.scratch + 7);
                get(&mut self.code, self.frame);
                get(&mut self.code, self.scratch + 7);
                store_value(
                    &mut self.code,
                    self.layout.storage(self.function.locals[local].ty),
                    offset,
                );
            }
            Slot::Register(register) => set(&mut self.code, register),
        }
    }
    fn initialized(&mut self, local: usize, present: bool) {
        if let Some(flag) = self.drop_flags.get(&local) {
            self.code.push(Instruction::I32(i32::from(present)));
            set(&mut self.code, *flag);
        }
    }
    fn write(&mut self, local: usize) {
        self.write_value(local);
        self.initialized(local, true);
    }
    // K consumes operands even when their old bits remain in a reusable slot.
    // Conditional destruction reads compiler-owned flags, never those stale bits.
    fn consume_inputs(&mut self, op: &Op) {
        if self.drop_flags.is_empty() {
            return;
        }
        match op {
            Op::Read {
                place,
                moving: true,
                ..
            } if place.projections.is_empty() => {
                self.initialized(place.local, false);
            }
            Op::Assign { src, .. } | Op::Unary { src, .. } | Op::Return { src } => {
                self.initialized(*src, false);
            }
            Op::Binary { left, right, .. } => {
                self.initialized(*left, false);
                self.initialized(*right, false);
            }
            Op::Array { elements, .. }
            | Op::Struct {
                fields: elements, ..
            }
            | Op::Variant {
                fields: elements, ..
            }
            | Op::Call { args: elements, .. }
            | Op::Intrinsic { args: elements, .. } => {
                for local in elements {
                    self.initialized(*local, false);
                }
            }
            Op::If { condition } => self.initialized(*condition, false),
            _ => {}
        }
    }
    fn address(&mut self, place: &Place, line: usize) -> usize {
        let mut ty = self.function.locals[place.local].ty;
        if place.projections.is_empty() {
            let offset = match self.slots[place.local] {
                Slot::Frame(offset) => offset,
                Slot::Register(_) => unreachable!("address-taken local has a frame slot"),
            };
            get(&mut self.code, self.frame);
            constant(&mut self.code, offset as i64);
            self.code.push(Instruction::Scalar(ScalarOp::I64Add));
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
                    let offset = self.layout.payload(ty, None).fields[*field];
                    ty = match &self.program.types[ty].kind {
                        TypeKind::Struct { fields, .. } => fields[*field].ty,
                        _ => unreachable!("K field type"),
                    };
                    constant(&mut self.code, offset as i64);
                    self.code.push(Instruction::Scalar(ScalarOp::I64Add));
                }
                Projection::VariantField(variant, field) => {
                    let offset = self.layout.payload(ty, Some(*variant)).fields[*field];
                    ty = match &self.program.types[ty].kind {
                        TypeKind::Enum { variants, .. } => variants[*variant].fields[*field].ty,
                        _ => unreachable!("K variant field type"),
                    };
                    constant(&mut self.code, offset as i64);
                    self.code.push(Instruction::Scalar(ScalarOp::I64Add));
                }
                Projection::Index(index) => {
                    ty = match self.program.types[ty].kind {
                        TypeKind::Vec { inner }
                        | TypeKind::Slice { inner, .. }
                        | TypeKind::Array { inner, .. } => inner,
                        _ => unreachable!("K index type"),
                    };
                    tee(&mut self.code, self.scratch + 4);
                    load(&mut self.code, 8);
                    set(&mut self.code, self.scratch + 5);
                    self.read(*index);
                    get(&mut self.code, self.scratch + 5);
                    self.code.push(Instruction::Scalar(ScalarOp::I64GeU)); // i64.ge_u
                    trap_if(&mut self.code, 3, line);
                    get(&mut self.code, self.scratch + 4);
                    load(&mut self.code, 0);
                    self.read(*index);
                    constant(&mut self.code, i64::from(self.layout.storage(ty).bytes));
                    self.code.push(Instruction::Scalar(ScalarOp::I64Mul));
                    self.code.push(Instruction::Scalar(ScalarOp::I64Add));
                }
            }
            i += 1;
            if i < place.projections.len() {
                load_value(&mut self.code, self.layout.storage(ty), 0);
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
            load_value(&mut self.code, self.layout.storage(ty), 0);
            ty
        }
    }
    fn clear_place(&mut self, place: &Place, ty: usize, line: usize) {
        if place.projections.is_empty() {
            if heap_type(&self.program.types[ty].kind) {
                constant(&mut self.code, 0);
                self.write_value(place.local);
            }
            self.initialized(place.local, false);
        } else if heap_type(&self.program.types[ty].kind) {
            self.address(place, line);
            constant(&mut self.code, 0);
            store(&mut self.code, 0);
        }
    }
    fn drop_place(&mut self, place: &Place, conditional: bool, line: usize) {
        let root = place.projections.is_empty();
        if root && !heap_type(&self.program.types[self.function.locals[place.local].ty].kind) {
            self.initialized(place.local, false);
            return;
        }
        if conditional && root {
            get(&mut self.code, self.drop_flags[&place.local]);
            begin_if(&mut self.code);
        }
        let ty = self.read_place(place, line);
        call(
            &mut self.code,
            helper_index(self.program, self.runtime, ty, false),
        );
        self.code.push(Instruction::Drop);
        self.clear_place(place, ty, line);
        if conditional && root {
            self.code.push(Instruction::End);
        }
    }
    fn narrow(&mut self, bits: u8, signed: bool, line: usize) {
        if bits == 64 {
            return;
        }
        tee(&mut self.code, self.scratch + 2);
        if signed {
            constant(&mut self.code, -(1_i64 << (bits - 1)));
            self.code.push(Instruction::Scalar(ScalarOp::I64LtS));
            get(&mut self.code, self.scratch + 2);
            constant(&mut self.code, (1_i64 << (bits - 1)) - 1);
            self.code.push(Instruction::Scalar(ScalarOp::I64GtS));
            self.code.push(Instruction::Scalar(ScalarOp::I32Or));
        } else {
            constant(&mut self.code, (1_i64 << bits) - 1);
            self.code.push(Instruction::Scalar(ScalarOp::I64GtU));
        }
        trap_if(&mut self.code, 1, line);
        get(&mut self.code, self.scratch + 2);
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
            self.code
                .push(Instruction::Scalar(ScalarOp::F64ReinterpretI64));
            self.read(right);
            self.code
                .push(Instruction::Scalar(ScalarOp::F64ReinterpretI64));
            let operation = match op {
                "+" => Instruction::Scalar(ScalarOp::F64Add),
                "-" => Instruction::Scalar(ScalarOp::F64Sub),
                "*" => Instruction::Scalar(ScalarOp::F64Mul),
                "/" => Instruction::Scalar(ScalarOp::F64Div),
                "==" => Instruction::Scalar(ScalarOp::F64Eq),
                "!=" => Instruction::Scalar(ScalarOp::F64Ne),
                "<" => Instruction::Scalar(ScalarOp::F64Lt),
                ">" => Instruction::Scalar(ScalarOp::F64Gt),
                "<=" => Instruction::Scalar(ScalarOp::F64Le),
                ">=" => Instruction::Scalar(ScalarOp::F64Ge),
                _ => return Err(format!("unsupported f64 operation {op}")),
            };
            self.code.push(operation);
            if matches!(op, "+" | "-" | "*" | "/") {
                self.code
                    .push(Instruction::Scalar(ScalarOp::I64ReinterpretF64));
                tee(&mut self.code, self.scratch);
                self.code
                    .push(Instruction::Scalar(ScalarOp::F64ReinterpretI64));
                get(&mut self.code, self.scratch);
                self.code
                    .push(Instruction::Scalar(ScalarOp::F64ReinterpretI64));
                self.code.push(Instruction::Scalar(ScalarOp::F64Ne));
                self.code
                    .extend_from_slice(&[Instruction::If(Some(ValueType::I64))]);
                constant(&mut self.code, 0x7ff8_0000_0000_0000);
                self.code.push(Instruction::Else);
                get(&mut self.code, self.scratch);
                self.code.push(Instruction::End);
            } else {
                self.code.push(Instruction::Scalar(ScalarOp::I64ExtendI32U));
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
        set(&mut self.code, self.scratch);
        self.read(right);
        set(&mut self.code, self.scratch + 1);
        if op == "/" || op == "%" {
            get(&mut self.code, self.scratch + 1);
            self.code.push(Instruction::Scalar(ScalarOp::I64Eqz));
            trap_if(&mut self.code, 2, line);
            if signed {
                get(&mut self.code, self.scratch);
                constant(
                    &mut self.code,
                    if bits == 64 {
                        i64::MIN
                    } else {
                        -(1_i64 << (bits - 1))
                    },
                );
                self.code.push(Instruction::Scalar(ScalarOp::I64Eq));
                get(&mut self.code, self.scratch + 1);
                constant(&mut self.code, -1);
                self.code.push(Instruction::Scalar(ScalarOp::I64Eq));
                self.code.push(Instruction::Scalar(ScalarOp::I32And));
                trap_if(&mut self.code, 1, line);
            }
        }
        if op == "<<" || op == ">>" {
            get(&mut self.code, self.scratch + 1);
            constant(&mut self.code, bits as i64);
            self.code.push(Instruction::Scalar(ScalarOp::I64GeU));
            trap_if(&mut self.code, 1, line);
        }
        get(&mut self.code, self.scratch);
        get(&mut self.code, self.scratch + 1);
        let operation = match op {
            "+" | "wrapping_add" => Instruction::Scalar(ScalarOp::I64Add),
            "-" | "wrapping_sub" => Instruction::Scalar(ScalarOp::I64Sub),
            "*" | "wrapping_mul" => Instruction::Scalar(ScalarOp::I64Mul),
            "/" => {
                if signed {
                    Instruction::Scalar(ScalarOp::I64DivS)
                } else {
                    Instruction::Scalar(ScalarOp::I64DivU)
                }
            }
            "%" => {
                if signed {
                    Instruction::Scalar(ScalarOp::I64RemS)
                } else {
                    Instruction::Scalar(ScalarOp::I64RemU)
                }
            }
            "&" | "&&" => Instruction::Scalar(ScalarOp::I64And),
            "|" | "||" => Instruction::Scalar(ScalarOp::I64Or),
            "^" => Instruction::Scalar(ScalarOp::I64Xor),
            "<<" => Instruction::Scalar(ScalarOp::I64Shl),
            ">>" => {
                if signed {
                    Instruction::Scalar(ScalarOp::I64ShrS)
                } else {
                    Instruction::Scalar(ScalarOp::I64ShrU)
                }
            }
            "==" => Instruction::Scalar(ScalarOp::I64Eq),
            "!=" => Instruction::Scalar(ScalarOp::I64Ne),
            "<" => {
                if signed {
                    Instruction::Scalar(ScalarOp::I64LtS)
                } else {
                    Instruction::Scalar(ScalarOp::I64LtU)
                }
            }
            ">" => {
                if signed {
                    Instruction::Scalar(ScalarOp::I64GtS)
                } else {
                    Instruction::Scalar(ScalarOp::I64GtU)
                }
            }
            "<=" => {
                if signed {
                    Instruction::Scalar(ScalarOp::I64LeS)
                } else {
                    Instruction::Scalar(ScalarOp::I64LeU)
                }
            }
            ">=" => {
                if signed {
                    Instruction::Scalar(ScalarOp::I64GeS)
                } else {
                    Instruction::Scalar(ScalarOp::I64GeU)
                }
            }
            _ => return Err(format!("unsupported binary operation {op}")),
        };
        self.code.push(operation);
        if matches!(op, "==" | "!=" | "<" | ">" | "<=" | ">=") {
            self.code.push(Instruction::Scalar(ScalarOp::I64ExtendI32U));
            self.write(dst);
            return Ok(());
        }
        set(&mut self.code, self.scratch + 2);
        if bits == 64 && (op == "+" || op == "-") {
            if signed {
                get(&mut self.code, self.scratch);
                if op == "+" {
                    get(&mut self.code, self.scratch + 2);
                } else {
                    get(&mut self.code, self.scratch + 1);
                }
                self.code.push(Instruction::Scalar(ScalarOp::I64Xor));
                get(
                    &mut self.code,
                    if op == "+" {
                        self.scratch + 1
                    } else {
                        self.scratch
                    },
                );
                get(&mut self.code, self.scratch + 2);
                self.code.push(Instruction::Scalar(ScalarOp::I64Xor));
                self.code.push(Instruction::Scalar(ScalarOp::I64And));
                constant(&mut self.code, 0);
                self.code.push(Instruction::Scalar(ScalarOp::I64LtS));
            } else if op == "+" {
                get(&mut self.code, self.scratch + 2);
                get(&mut self.code, self.scratch);
                self.code.push(Instruction::Scalar(ScalarOp::I64LtU));
            } else {
                get(&mut self.code, self.scratch);
                get(&mut self.code, self.scratch + 1);
                self.code.push(Instruction::Scalar(ScalarOp::I64LtU));
            }
            trap_if(&mut self.code, 1, line);
        }
        if bits == 64 && op == "*" {
            get(&mut self.code, self.scratch + 1);
            self.code.push(Instruction::Scalar(ScalarOp::I64Eqz));
            self.code.push(Instruction::Scalar(ScalarOp::I32Eqz));
            begin_if(&mut self.code);
            if signed {
                get(&mut self.code, self.scratch);
                constant(&mut self.code, i64::MIN);
                self.code.push(Instruction::Scalar(ScalarOp::I64Eq));
                get(&mut self.code, self.scratch + 1);
                constant(&mut self.code, -1);
                self.code.push(Instruction::Scalar(ScalarOp::I64Eq));
                self.code.push(Instruction::Scalar(ScalarOp::I32And));
                trap_if(&mut self.code, 1, line);
                get(&mut self.code, self.scratch + 2);
                constant(&mut self.code, i64::MIN);
                self.code.push(Instruction::Scalar(ScalarOp::I64Eq));
                get(&mut self.code, self.scratch + 1);
                constant(&mut self.code, -1);
                self.code.push(Instruction::Scalar(ScalarOp::I64Eq));
                self.code.push(Instruction::Scalar(ScalarOp::I32And));
                trap_if(&mut self.code, 1, line);
            }
            get(&mut self.code, self.scratch + 2);
            get(&mut self.code, self.scratch + 1);
            self.code.push(if signed {
                Instruction::Scalar(ScalarOp::I64DivS)
            } else {
                Instruction::Scalar(ScalarOp::I64DivU)
            });
            get(&mut self.code, self.scratch);
            self.code.push(Instruction::Scalar(ScalarOp::I64Ne));
            trap_if(&mut self.code, 1, line);
            self.code.push(Instruction::End);
        }
        get(&mut self.code, self.scratch + 2);
        if op.starts_with("wrapping_") || op == "<<" {
            if bits < 64 {
                if signed {
                    constant(&mut self.code, (64 - bits) as i64);
                    self.code.push(Instruction::Scalar(ScalarOp::I64Shl));
                    constant(&mut self.code, (64 - bits) as i64);
                    self.code.push(Instruction::Scalar(ScalarOp::I64ShrS));
                } else {
                    constant(&mut self.code, (1_i64 << bits) - 1);
                    self.code.push(Instruction::Scalar(ScalarOp::I64And));
                }
            }
        } else if op == "+" || op == "-" || op == "*" {
            self.narrow(bits, signed, line);
        }
        self.write(dst);
        Ok(())
    }
    fn emit(&mut self, op: &Op, line: usize) -> Result<(), String> {
        self.consume_inputs(op);
        match op {
            Op::Int { dst, value } => {
                constant(&mut self.code, *value);
                self.write(*dst);
            }
            Op::Float { dst, bits } => {
                constant(&mut self.code, *bits as i64);
                self.write(*dst);
            }
            Op::Text { dst, bytes } => {
                constant(&mut self.code, self.texts.offsets[bytes] as i64);
                self.write(*dst);
            }
            Op::Unit { dst } => {
                constant(&mut self.code, 0);
                self.write(*dst);
            }
            Op::Read { dst, place, moving } => {
                let ty = self.read_place(place, line);
                if !moving && heap_type(&self.program.types[ty].kind) {
                    call(
                        &mut self.code,
                        helper_index(self.program, self.runtime, ty, true),
                    );
                }
                self.write(*dst);
                if *moving && !(place.projections.is_empty() && place.local == *dst) {
                    self.clear_place(place, ty, line);
                }
            }
            Op::SliceBorrow { dst, place, .. } => {
                self.address(place, line);
                self.write(*dst);
            }
            Op::Array { dst, elements } => {
                let TypeKind::Array { inner, .. } =
                    self.program.types[self.function.locals[*dst].ty].kind
                else {
                    unreachable!()
                };
                let storage = self.layout.storage(inner);
                let extent = (elements.len() as u64)
                    .checked_mul(u64::from(storage.bytes))
                    .ok_or("array extent exceeds address representation")?;
                constant(&mut self.code, 24);
                call(&mut self.code, ALLOC + self.runtime);
                self.write(*dst);
                self.read(*dst);
                constant(&mut self.code, extent as i64);
                call(&mut self.code, ALLOC + self.runtime);
                store(&mut self.code, 0);
                self.read(*dst);
                constant(&mut self.code, elements.len() as i64);
                store(&mut self.code, 8);
                self.read(*dst);
                constant(&mut self.code, elements.len() as i64);
                store(&mut self.code, 16);
                let mut i = 0;
                while i < elements.len() {
                    self.read(*dst);
                    load(&mut self.code, 0);
                    self.read(elements[i]);
                    store_value(&mut self.code, storage, i as u64 * u64::from(storage.bytes));
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
                    let ty = self.address(place, line);
                    self.read(*src);
                    store_value(&mut self.code, self.layout.storage(ty), 0);
                }
            }
            Op::Drop { place } => self.drop_place(place, false, line),
            Op::DropIf { place } => self.drop_place(place, true, line),
            Op::Unary { dst, op, src } => {
                match op.as_str() {
                    "-" => match self.program.types[self.function.locals[*src].ty].kind {
                        TypeKind::F64 => {
                            self.read(*src);
                            constant(&mut self.code, i64::MIN);
                            self.code.push(Instruction::Scalar(ScalarOp::I64Xor));
                            tee(&mut self.code, self.scratch);
                            self.code
                                .push(Instruction::Scalar(ScalarOp::F64ReinterpretI64));
                            get(&mut self.code, self.scratch);
                            self.code
                                .push(Instruction::Scalar(ScalarOp::F64ReinterpretI64));
                            self.code.push(Instruction::Scalar(ScalarOp::F64Ne));
                            self.code
                                .extend_from_slice(&[Instruction::If(Some(ValueType::I64))]);
                            constant(&mut self.code, 0x7ff8_0000_0000_0000);
                            self.code.push(Instruction::Else);
                            get(&mut self.code, self.scratch);
                            self.code.push(Instruction::End);
                        }
                        TypeKind::Int { bits, signed: true } => {
                            self.read(*src);
                            constant(
                                &mut self.code,
                                if bits == 64 {
                                    i64::MIN
                                } else {
                                    -(1_i64 << (bits - 1))
                                },
                            );
                            self.code.push(Instruction::Scalar(ScalarOp::I64Eq));
                            trap_if(&mut self.code, 1, line);
                            constant(&mut self.code, 0);
                            self.read(*src);
                            self.code.push(Instruction::Scalar(ScalarOp::I64Sub));
                        }
                        _ => return Err("invalid negation".to_string()),
                    },
                    "!" => {
                        self.read(*src);
                        if self.program.types[self.function.locals[*src].ty].kind == TypeKind::Bool
                        {
                            self.code.extend_from_slice(&[
                                Instruction::Scalar(ScalarOp::I64Eqz),
                                Instruction::Scalar(ScalarOp::I64ExtendI32U),
                            ]);
                        } else {
                            constant(&mut self.code, -1);
                            self.code.push(Instruction::Scalar(ScalarOp::I64Xor));
                            if let TypeKind::Int {
                                bits,
                                signed: false,
                            } = self.program.types[self.function.locals[*src].ty].kind
                            {
                                if bits < 64 {
                                    constant(&mut self.code, (1_i64 << bits) - 1);
                                    self.code.push(Instruction::Scalar(ScalarOp::I64And));
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
                call(&mut self.code, (FIRST_USER + self.runtime) + *function);
                self.write(*dst);
            }
            Op::If { condition } => {
                self.read(*condition);
                self.code.push(Instruction::Scalar(ScalarOp::I32WrapI64));
                begin_if(&mut self.code);
                self.controls.push(Control::Conditional);
            }
            Op::Else => self.code.push(Instruction::Else),
            Op::Loop => {
                self.code
                    .extend_from_slice(&[Instruction::Block(None), Instruction::Loop(None)]);
                self.controls.push(Control::Loop);
            }
            Op::End => {
                if self.controls.pop().expect("K control end") == Control::Loop {
                    self.code.extend_from_slice(&[
                        Instruction::Branch {
                            depth: 0,
                            conditional: false,
                        },
                        Instruction::End,
                    ]);
                }
                self.code.push(Instruction::End);
            }
            Op::Break | Op::Continue | Op::BreakTo { .. } | Op::ContinueTo { .. } => {
                let mut target = match op {
                    Op::BreakTo { depth } | Op::ContinueTo { depth } => *depth,
                    _ => 0,
                };
                let mut depth = 0;
                for control in self.controls.iter().rev() {
                    if *control == Control::Loop {
                        if target == 0 {
                            if matches!(op, Op::Break | Op::BreakTo { .. }) {
                                depth += 1;
                            }
                            break;
                        }
                        target -= 1;
                        // Every source loop lowers to a block around a loop.
                        depth += 2;
                    } else {
                        depth += 1;
                    }
                }
                self.code.push(Instruction::Branch {
                    depth,
                    conditional: false,
                });
            }
            Op::Return { src } => {
                self.read(*src);
                set(&mut self.code, self.scratch + 6);
                if self.frame_bytes != 0 {
                    get(&mut self.code, self.frame);
                    call(&mut self.code, FREE + self.runtime);
                    self.code.push(Instruction::Drop);
                }
                get(&mut self.code, self.scratch + 6);
                self.code.push(Instruction::Return);
            }
            Op::Trap { message } => {
                let mut index = 0;
                while self.messages[index] != *message {
                    index += 1;
                }
                trap(&mut self.code, 16 + index as i64, line);
            }
            Op::Struct { dst, fields } | Op::Variant { dst, fields, .. } => {
                let tag = match op {
                    Op::Variant { variant, .. } => Some(*variant),
                    _ => None,
                };
                let ty = self.function.locals[*dst].ty;
                constant(&mut self.code, self.layout.payload(ty, tag).bytes as i64);
                call(&mut self.code, ALLOC + self.runtime);
                self.write(*dst);
                if let Some(variant) = tag {
                    self.read(*dst);
                    constant(&mut self.code, variant as i64);
                    store(&mut self.code, 0);
                }
                let mut i = 0;
                while i < fields.len() {
                    self.read(*dst);
                    self.read(fields[i]);
                    store_value(
                        &mut self.code,
                        self.layout.storage(self.function.locals[fields[i]].ty),
                        self.layout.payload(ty, tag).fields[i],
                    );
                    i += 1;
                }
            }
            Op::TagTest {
                dst,
                place,
                variant,
            } => {
                let ty = self.read_place(place, line);
                if matches!(self.program.types[ty].kind, TypeKind::Ref { .. }) {
                    load(&mut self.code, 0);
                }
                load(&mut self.code, 0);
                constant(&mut self.code, *variant as i64);
                self.code.extend_from_slice(&[
                    Instruction::Scalar(ScalarOp::I64Eq),
                    Instruction::Scalar(ScalarOp::I64ExtendI32U),
                ]);
                self.write(*dst);
            }

            Op::Discriminant { dst, src } => {
                self.read(*src);
                load(&mut self.code, 0);
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
            "world_stdout" => {
                // Only the checked derivation produces this affine handle.
                constant(&mut self.code, 1);
                self.write(dst);
            }
            "stdout_write" => {
                // Byte vectors, arrays and text now share the host byte ABI.
                self.read(args[1]);
                load(&mut self.code, 0);
                set(&mut self.code, self.scratch);
                get(&mut self.code, self.scratch);
                load(&mut self.code, 0);
                get(&mut self.code, self.scratch);
                load(&mut self.code, 8);
                constant(&mut self.code, line as i64);
                call(&mut self.code, 0);
                self.write(dst);
            }
            "assert" => {
                self.read(args[0]);
                self.code.push(Instruction::Scalar(ScalarOp::I64Eqz));
                trap_if(&mut self.code, 5, line);
                constant(&mut self.code, 0);
                self.write(dst);
            }
            "cast" => {
                let from = &self.program.types[self.function.locals[args[0]].ty].kind;
                let to = &self.program.types[self.function.locals[dst].ty].kind;
                self.read(args[0]);
                match (from, to) {
                    (TypeKind::F64, TypeKind::F64) => {}
                    (TypeKind::Int { signed, .. }, TypeKind::F64) => {
                        self.code.push(if *signed {
                            Instruction::Scalar(ScalarOp::F64ConvertI64S)
                        } else {
                            Instruction::Scalar(ScalarOp::F64ConvertI64U)
                        });
                        self.code
                            .push(Instruction::Scalar(ScalarOp::I64ReinterpretF64));
                    }
                    (TypeKind::F64, TypeKind::Int { bits, signed }) => {
                        self.code
                            .push(Instruction::Scalar(ScalarOp::F64ReinterpretI64));
                        self.code.push(Instruction::Scalar(if *signed {
                            ScalarOp::I64TruncSatF64S
                        } else {
                            ScalarOp::I64TruncSatF64U
                        }));
                        if *bits < 64 {
                            set(&mut self.code, self.scratch);
                            let maximum = if *signed {
                                (1_i64 << (*bits - 1)) - 1
                            } else {
                                (1_i64 << *bits) - 1
                            };
                            if *signed {
                                let minimum = -(1_i64 << (*bits - 1));
                                get(&mut self.code, self.scratch);
                                constant(&mut self.code, minimum);
                                self.code.push(Instruction::Scalar(ScalarOp::I64LtS));
                                begin_if(&mut self.code);
                                constant(&mut self.code, minimum);
                                set(&mut self.code, self.scratch);
                                self.code.push(Instruction::End);
                            }
                            get(&mut self.code, self.scratch);
                            constant(&mut self.code, maximum);
                            self.code.push(if *signed {
                                Instruction::Scalar(ScalarOp::I64GtS)
                            } else {
                                Instruction::Scalar(ScalarOp::I64GtU)
                            });
                            begin_if(&mut self.code);
                            constant(&mut self.code, maximum);
                            set(&mut self.code, self.scratch);
                            self.code.push(Instruction::End);
                            get(&mut self.code, self.scratch);
                        }
                    }
                    (_, TypeKind::Int { bits, signed }) => {
                        if *bits < 64 {
                            if *signed {
                                constant(&mut self.code, (64 - *bits) as i64);
                                self.code.push(Instruction::Scalar(ScalarOp::I64Shl));
                                constant(&mut self.code, (64 - *bits) as i64);
                                self.code.push(Instruction::Scalar(ScalarOp::I64ShrS));
                            } else {
                                constant(&mut self.code, (1_i64 << *bits) - 1);
                                self.code.push(Instruction::Scalar(ScalarOp::I64And));
                            }
                        }
                    }
                    _ => return Err("unsupported scalar conversion".to_string()),
                }
                self.write(dst);
            }

            "replace" => {
                self.read(args[0]);
                tee(&mut self.code, self.scratch + 3);
                load_value(
                    &mut self.code,
                    self.layout.storage(self.function.locals[dst].ty),
                    0,
                );
                self.write(dst);
                get(&mut self.code, self.scratch + 3);
                self.read(args[1]);
                store_value(
                    &mut self.code,
                    self.layout.storage(self.function.locals[args[1]].ty),
                    0,
                );
            }

            "box_new" => {
                constant(
                    &mut self.code,
                    i64::from(self.layout.storage(self.function.locals[args[0]].ty).bytes),
                );
                call(&mut self.code, ALLOC + self.runtime);
                self.write(dst);
                self.read(dst);
                self.read(args[0]);
                store_value(
                    &mut self.code,
                    self.layout.storage(self.function.locals[args[0]].ty),
                    0,
                );
            }
            "box_get" => {
                self.read(args[0]);
                if matches!(
                    self.program.types[self.function.locals[args[0]].ty].kind,
                    TypeKind::Ref { .. }
                ) {
                    load(&mut self.code, 0);
                }
                load_value(
                    &mut self.code,
                    self.layout.storage(self.function.locals[dst].ty),
                    0,
                );
                self.write(dst);
            }
            "vec_new" => {
                constant(&mut self.code, 24);
                call(&mut self.code, ALLOC + self.runtime);
                self.write(dst);
                for offset in [0, 8, 16] {
                    self.read(dst);
                    constant(&mut self.code, 0);
                    store(&mut self.code, offset);
                }
            }
            "vec_pop" => {
                let output = self.function.locals[dst].ty;
                let TypeKind::Enum { variants, .. } = &self.program.types[output].kind else {
                    unreachable!()
                };
                let inner = variants[1].fields[0].ty;
                let storage = self.layout.storage(inner);
                let payload = self.layout.payload(output, Some(1));
                let extent = payload.bytes;
                let field = payload.fields[0];
                self.read(args[0]);
                if matches!(
                    self.program.types[self.function.locals[args[0]].ty].kind,
                    TypeKind::Ref { .. }
                ) {
                    load(&mut self.code, 0);
                }
                set(&mut self.code, self.scratch + 4);
                get(&mut self.code, self.scratch + 4);
                load(&mut self.code, 8);
                set(&mut self.code, self.scratch + 5);
                constant(&mut self.code, extent as i64);
                call(&mut self.code, ALLOC + self.runtime);
                self.write(dst);
                get(&mut self.code, self.scratch + 5);
                self.code.push(Instruction::Scalar(ScalarOp::I64Eqz));
                begin_if(&mut self.code);
                self.read(dst);
                constant(&mut self.code, 0);
                store(&mut self.code, 0);
                self.code.push(Instruction::Else);
                get(&mut self.code, self.scratch + 5);
                constant(&mut self.code, 1);
                self.code.push(Instruction::Scalar(ScalarOp::I64Sub));
                set(&mut self.code, self.scratch + 5);
                get(&mut self.code, self.scratch + 4);
                get(&mut self.code, self.scratch + 5);
                store(&mut self.code, 8);
                get(&mut self.code, self.scratch + 4);
                load(&mut self.code, 0);
                get(&mut self.code, self.scratch + 5);
                constant(&mut self.code, i64::from(storage.bytes));
                self.code.push(Instruction::Scalar(ScalarOp::I64Mul));
                self.code.push(Instruction::Scalar(ScalarOp::I64Add));
                set(&mut self.code, self.scratch + 3);
                self.read(dst);
                constant(&mut self.code, 1);
                store(&mut self.code, 0);
                self.read(dst);
                get(&mut self.code, self.scratch + 3);
                load_value(&mut self.code, storage, 0);
                store_value(&mut self.code, storage, field);
                get(&mut self.code, self.scratch + 3);
                constant(&mut self.code, 0);
                store_value(&mut self.code, storage, 0);
                self.code.push(Instruction::End);
            }

            "vec_push" => {
                let storage = self.layout.storage(self.function.locals[args[1]].ty);
                let stride = u64::from(storage.bytes);
                self.read(args[0]);
                if matches!(
                    self.program.types[self.function.locals[args[0]].ty].kind,
                    TypeKind::Ref { .. }
                ) {
                    load(&mut self.code, 0);
                }
                set(&mut self.code, self.scratch + 4);
                get(&mut self.code, self.scratch + 4);
                load(&mut self.code, 8);
                set(&mut self.code, self.scratch + 5);
                get(&mut self.code, self.scratch + 4);
                load(&mut self.code, 16);
                set(&mut self.code, self.scratch + 6);
                get(&mut self.code, self.scratch + 5);
                get(&mut self.code, self.scratch + 6);
                self.code.push(Instruction::Scalar(ScalarOp::I64Eq));
                begin_if(&mut self.code);
                get(&mut self.code, self.scratch + 6);
                self.code.push(Instruction::Scalar(ScalarOp::I64Eqz));
                self.code
                    .extend_from_slice(&[Instruction::If(Some(ValueType::I64))]);
                constant(&mut self.code, 4);
                self.code.push(Instruction::Else);
                get(&mut self.code, self.scratch + 6);
                constant(&mut self.code, (u64::MAX / 2) as i64);
                self.code.push(Instruction::Scalar(ScalarOp::I64GtU));
                trap_if(&mut self.code, 6, line);
                get(&mut self.code, self.scratch + 6);
                constant(&mut self.code, 2);
                self.code.push(Instruction::Scalar(ScalarOp::I64Mul));
                self.code.push(Instruction::End);
                set(&mut self.code, self.scratch + 6);
                get(&mut self.code, self.scratch + 6);
                constant(&mut self.code, (u64::MAX / stride) as i64);
                self.code.push(Instruction::Scalar(ScalarOp::I64GtU));
                trap_if(&mut self.code, 6, line);
                get(&mut self.code, self.scratch + 6);
                constant(&mut self.code, stride as i64);
                self.code.push(Instruction::Scalar(ScalarOp::I64Mul));
                call(&mut self.code, ALLOC + self.runtime);
                set(&mut self.code, self.scratch + 3);
                get(&mut self.code, self.scratch + 3);
                get(&mut self.code, self.scratch + 4);
                load(&mut self.code, 0);
                get(&mut self.code, self.scratch + 5);
                constant(&mut self.code, stride as i64);
                self.code.push(Instruction::Scalar(ScalarOp::I64Mul));
                self.code.extend_from_slice(&[Instruction::MemoryCopy]);
                get(&mut self.code, self.scratch + 4);
                load(&mut self.code, 0);
                call(&mut self.code, FREE + self.runtime);
                self.code.push(Instruction::Drop);
                get(&mut self.code, self.scratch + 4);
                get(&mut self.code, self.scratch + 3);
                store(&mut self.code, 0);
                get(&mut self.code, self.scratch + 4);
                get(&mut self.code, self.scratch + 6);
                store(&mut self.code, 16);
                self.code.push(Instruction::End);
                get(&mut self.code, self.scratch + 4);
                load(&mut self.code, 0);
                get(&mut self.code, self.scratch + 5);
                constant(&mut self.code, stride as i64);
                self.code.push(Instruction::Scalar(ScalarOp::I64Mul));
                self.code.push(Instruction::Scalar(ScalarOp::I64Add));
                self.read(args[1]);
                store_value(&mut self.code, storage, 0);
                get(&mut self.code, self.scratch + 4);
                get(&mut self.code, self.scratch + 5);
                constant(&mut self.code, 1);
                self.code.push(Instruction::Scalar(ScalarOp::I64Add));
                store(&mut self.code, 8);
                constant(&mut self.code, 0);
                self.write(dst);
            }
            "clone" => {
                self.read(args[0]);
                let ty = match self.program.types[self.function.locals[args[0]].ty].kind {
                    TypeKind::Ref { inner, .. } => {
                        load_value(&mut self.code, self.layout.storage(inner), 0);
                        inner
                    }
                    _ => self.function.locals[args[0]].ty,
                };
                if heap_type(&self.program.types[ty].kind) {
                    call(
                        &mut self.code,
                        helper_index(self.program, self.runtime, ty, true),
                    );
                }
                self.write(dst);
            }

            "str_as_bytes" => {
                // Both shared views retain the original header cell and origin.
                self.read(args[0]);
                self.write(dst);
            }
            "str_len" => {
                self.read(args[0]);
                load(&mut self.code, 0);
                load(&mut self.code, 8);
                self.write(dst);
            }
            "str_from_utf8" => {
                let result_ty = self.function.locals[dst].ty;
                let TypeKind::Enum { variants, .. } = &self.program.types[result_ty].kind else {
                    unreachable!()
                };
                let error_ty = variants[1].fields[0].ty;
                let result_bytes = self
                    .layout
                    .payload(result_ty, Some(0))
                    .bytes
                    .max(self.layout.payload(result_ty, Some(1)).bytes);
                let error_bytes = self.layout.payload(error_ty, None).bytes;
                self.read(args[0]);
                call(&mut self.code, UTF8_VALIDATE + self.runtime);
                set(&mut self.code, self.scratch + 4);
                constant(&mut self.code, result_bytes as i64);
                call(&mut self.code, ALLOC + self.runtime);
                self.write(dst);
                get(&mut self.code, self.scratch + 4);
                self.code.push(Instruction::Scalar(ScalarOp::I64Eqz));
                begin_if(&mut self.code);
                self.read(dst);
                constant(&mut self.code, 1);
                store(&mut self.code, 0);
                self.read(dst);
                constant(&mut self.code, error_bytes as i64);
                call(&mut self.code, ALLOC + self.runtime);
                store_value(
                    &mut self.code,
                    self.layout.storage(error_ty),
                    self.layout.payload(result_ty, Some(1)).fields[0],
                );
                self.code.push(Instruction::Else);
                self.read(dst);
                constant(&mut self.code, 0);
                store(&mut self.code, 0);
                self.read(dst);
                self.read(args[0]);
                store_value(
                    &mut self.code,
                    self.layout.storage(variants[0].fields[0].ty),
                    self.layout.payload(result_ty, Some(0)).fields[0],
                );
                self.code.push(Instruction::End);
            }
            "vec_len" => {
                self.read(args[0]);
                if matches!(
                    self.program.types[self.function.locals[args[0]].ty].kind,
                    TypeKind::Ref { .. }
                ) {
                    load(&mut self.code, 0);
                }
                load(&mut self.code, 8);
                self.write(dst);
            }
            "drop" => {
                self.read(args[0]);
                call(
                    &mut self.code,
                    helper_index(
                        self.program,
                        self.runtime,
                        self.function.locals[args[0]].ty,
                        false,
                    ),
                );
                self.code.push(Instruction::Drop);
                constant(&mut self.code, 0);
                self.write(dst);
            }
            _ => return Err(format!("unsupported intrinsic {name}")),
        }
        Ok(())
    }
}
fn helper_index(program: &KProgram, runtime: usize, ty: usize, copying: bool) -> usize {
    (FIRST_USER + runtime) + program.functions.len() + ty * 2 + if copying { 1 } else { 0 }
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
// Legacy runtime algorithms expressed as ordinary lowered functions. These
// builders are retained for the first backend split; runtime source replaces
// them through a separate, sealed compilation contract. They emit no bytes.
fn copy_fields(
    b: &mut Code,
    program: &KProgram,
    runtime: usize,
    layout: &Layouts,
    fields: &[crate::k::Field],
    offsets: &[u64],
) {
    for (field, offset) in fields.iter().zip(offsets) {
        get(b, 1);
        get(b, 0);
        load_value(b, layout.storage(field.ty), *offset);
        call(b, helper_index(program, runtime, field.ty, true));
        store_value(b, layout.storage(field.ty), *offset);
    }
}

fn copy_helper(program: &KProgram, runtime: usize, layout: &Layouts, ty: usize) -> Code {
    let mut b = Code::words(5);
    let kind = &program.types[ty].kind;
    if !heap_type(kind) {
        get(&mut b, 0);
        b.push(Instruction::End);
        return b;
    }
    get(&mut b, 0);
    b.push(Instruction::Scalar(ScalarOp::I64Eqz));
    begin_if(&mut b);
    constant(&mut b, 0);
    b.push(Instruction::Return);
    b.push(Instruction::End);
    match kind {
        TypeKind::Struct { fields, .. } => {
            constant(&mut b, layout.payload(ty, None).bytes as i64);
            call(&mut b, ALLOC + runtime);
            set(&mut b, 1);
            copy_fields(
                &mut b,
                program,
                runtime,
                layout,
                fields,
                &layout.payload(ty, None).fields,
            );
        }
        TypeKind::Enum { variants, .. } => {
            get(&mut b, 0);
            load(&mut b, 0);
            set(&mut b, 2);
            for (variant, value) in variants.iter().enumerate() {
                get(&mut b, 2);
                constant(&mut b, variant as i64);
                b.push(Instruction::Scalar(ScalarOp::I64Eq));
                begin_if(&mut b);
                constant(&mut b, layout.payload(ty, Some(variant)).bytes as i64);
                call(&mut b, ALLOC + runtime);
                set(&mut b, 1);
                get(&mut b, 1);
                get(&mut b, 2);
                store(&mut b, 0);
                copy_fields(
                    &mut b,
                    program,
                    runtime,
                    layout,
                    &value.fields,
                    &layout.payload(ty, Some(variant)).fields,
                );
                b.push(Instruction::End);
            }
        }
        TypeKind::Box { inner } => {
            constant(&mut b, i64::from(layout.storage(*inner).bytes));
            call(&mut b, ALLOC + runtime);
            tee(&mut b, 1);
            get(&mut b, 0);
            load_value(&mut b, layout.storage(*inner), 0);
            call(&mut b, helper_index(program, runtime, *inner, true));
            store_value(&mut b, layout.storage(*inner), 0);
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
            call(&mut b, ALLOC + runtime);
            set(&mut b, 1);
            get(&mut b, 3);
            constant(&mut b, i64::from(layout.storage(inner).bytes));
            b.push(Instruction::Scalar(ScalarOp::I64Mul));
            call(&mut b, ALLOC + runtime);
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
            b.extend_from_slice(&[Instruction::Block(None), Instruction::Loop(None)]);
            get(&mut b, 4);
            get(&mut b, 3);
            b.push(Instruction::Scalar(ScalarOp::I64GeU));
            b.extend_from_slice(&[Instruction::Branch {
                depth: 1,
                conditional: true,
            }]);
            get(&mut b, 5);
            get(&mut b, 4);
            constant(&mut b, i64::from(layout.storage(inner).bytes));
            b.push(Instruction::Scalar(ScalarOp::I64Mul));
            b.push(Instruction::Scalar(ScalarOp::I64Add));
            get(&mut b, 2);
            get(&mut b, 4);
            constant(&mut b, i64::from(layout.storage(inner).bytes));
            b.push(Instruction::Scalar(ScalarOp::I64Mul));
            b.push(Instruction::Scalar(ScalarOp::I64Add));
            load_value(&mut b, layout.storage(inner), 0);
            call(&mut b, helper_index(program, runtime, inner, true));
            store_value(&mut b, layout.storage(inner), 0);
            get(&mut b, 4);
            constant(&mut b, 1);
            b.push(Instruction::Scalar(ScalarOp::I64Add));
            set(&mut b, 4);
            b.extend_from_slice(&[
                Instruction::Branch {
                    depth: 0,
                    conditional: false,
                },
                Instruction::End,
                Instruction::End,
            ]);
        }
        TypeKind::Slice { .. } => {
            constant(&mut b, 16);
            call(&mut b, ALLOC + runtime);
            set(&mut b, 1);
            get(&mut b, 1);
            get(&mut b, 0);
            constant(&mut b, 16);
            b.extend_from_slice(&[Instruction::MemoryCopy]);
        }
        _ => unreachable!(),
    }
    get(&mut b, 1);
    b.push(Instruction::End);
    b
}

fn drop_start(program: &KProgram, runtime: usize, ty: usize) -> Code {
    let mut b = Code::words(1);
    if !heap_type(&program.types[ty].kind) {
        constant(&mut b, 0);
        b.push(Instruction::End);
        return b;
    }
    get(&mut b, 0);
    b.push(Instruction::Scalar(ScalarOp::I64Eqz));
    begin_if(&mut b);
    constant(&mut b, 0);
    b.push(Instruction::Return);
    b.push(Instruction::End);
    constant(&mut b, 32);
    call(&mut b, ALLOC + runtime);
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
    call(&mut b, DROP_RUN + runtime);
    b.push(Instruction::End);
    b
}

// The child's value is on the operand stack, and the parent continuation was
// already advanced. Zero marks a moved field, so continue to its next sibling.
fn yield_drop_child(b: &mut Code, ty: usize) {
    set(b, 3);
    get(b, 3);
    b.push(Instruction::Scalar(ScalarOp::I64Eqz));
    b.push(Instruction::Scalar(ScalarOp::I32Eqz));
    begin_if(b);
    get(b, 0);
    constant(b, ty as i64);
    store(b, 24);
    get(b, 3);
    b.push(Instruction::Return);
    b.push(Instruction::End);
}

fn drop_fields(b: &mut Code, program: &KProgram, fields: &[crate::k::Field], offsets: &[u64]) {
    let mut next = 1;
    for (i, field) in fields.iter().enumerate() {
        if heap_type(&program.types[field.ty].kind) {
            get(b, 0);
            load(b, 16);
            constant(b, next);
            b.push(Instruction::Scalar(ScalarOp::I64Eq));
            begin_if(b);
            get(b, 0);
            constant(b, next + 1);
            store(b, 16);
            get(b, 1);
            load(b, offsets[i]);
            yield_drop_child(b, field.ty);
            b.push(Instruction::End);
            next += 1;
        }
    }
}

fn drop_step(
    program: &KProgram,
    runtime: usize,
    layout: &Layouts,
    ty: usize,
    destructor: Option<usize>,
) -> Code {
    let mut b = Code::words(4);
    let kind = &program.types[ty].kind;
    get(&mut b, 0);
    load(&mut b, 16);
    b.push(Instruction::Scalar(ScalarOp::I64Eqz));
    begin_if(&mut b);
    if let Some(function) = destructor {
        get(&mut b, 0);
        call(&mut b, (FIRST_USER + runtime) + function);
        b.push(Instruction::Drop);
    }
    get(&mut b, 0);
    constant(&mut b, 1);
    store(&mut b, 16);
    b.push(Instruction::End);
    get(&mut b, 0);
    load(&mut b, 0);
    set(&mut b, 1);
    match kind {
        TypeKind::Struct { fields, .. } => {
            drop_fields(&mut b, program, fields, &layout.payload(ty, None).fields)
        }
        TypeKind::Enum { variants, .. } => {
            get(&mut b, 1);
            load(&mut b, 0);
            set(&mut b, 2);
            for (variant, value) in variants.iter().enumerate() {
                get(&mut b, 2);
                constant(&mut b, variant as i64);
                b.push(Instruction::Scalar(ScalarOp::I64Eq));
                begin_if(&mut b);
                drop_fields(
                    &mut b,
                    program,
                    &value.fields,
                    &layout.payload(ty, Some(variant)).fields,
                );
                b.push(Instruction::End);
            }
        }
        TypeKind::Box { inner } => {
            if heap_type(&program.types[*inner].kind) {
                get(&mut b, 0);
                load(&mut b, 16);
                constant(&mut b, 1);
                b.push(Instruction::Scalar(ScalarOp::I64Eq));
                begin_if(&mut b);
                get(&mut b, 0);
                constant(&mut b, 2);
                store(&mut b, 16);
                get(&mut b, 1);
                load(&mut b, 0);
                yield_drop_child(&mut b, *inner);
                b.push(Instruction::End);
            }
        }
        TypeKind::Vec { inner } | TypeKind::Array { inner, .. } => {
            let inner = *inner;
            if heap_type(&program.types[inner].kind) {
                b.extend_from_slice(&[Instruction::Block(None), Instruction::Loop(None)]);
                get(&mut b, 0);
                load(&mut b, 16);
                set(&mut b, 2);
                get(&mut b, 2);
                constant(&mut b, 1);
                b.push(Instruction::Scalar(ScalarOp::I64Sub));
                set(&mut b, 3);
                get(&mut b, 3);
                get(&mut b, 1);
                load(&mut b, 8);
                b.push(Instruction::Scalar(ScalarOp::I64GeU));
                b.extend_from_slice(&[Instruction::Branch {
                    depth: 1,
                    conditional: true,
                }]);
                get(&mut b, 0);
                get(&mut b, 2);
                constant(&mut b, 1);
                b.push(Instruction::Scalar(ScalarOp::I64Add));
                store(&mut b, 16);
                get(&mut b, 1);
                load(&mut b, 0);
                get(&mut b, 3);
                constant(&mut b, i64::from(layout.storage(inner).bytes));
                b.push(Instruction::Scalar(ScalarOp::I64Mul));
                b.push(Instruction::Scalar(ScalarOp::I64Add));
                load(&mut b, 0);
                yield_drop_child(&mut b, inner);
                b.extend_from_slice(&[
                    Instruction::Branch {
                        depth: 0,
                        conditional: false,
                    },
                    Instruction::End,
                    Instruction::End,
                ]);
            }
            get(&mut b, 1);
            load(&mut b, 0);
            call(&mut b, FREE + runtime);
            b.push(Instruction::Drop);
        }
        _ => {}
    }
    if heap_type(kind) {
        get(&mut b, 1);
        call(&mut b, FREE + runtime);
        b.push(Instruction::Drop);
    }
    constant(&mut b, 0);
    b.push(Instruction::End);
    b
}

fn drop_step_index(program: &KProgram, runtime: usize, ty: usize) -> usize {
    (FIRST_USER + runtime) + program.functions.len() + program.types.len() * 2 + ty
}

// This pool is a lowering/performance choice, never an acceptance limit.
// Further simultaneously live values spill to a growing memory frame.
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
fn local_layout(
    program: &KProgram,
    representations: &Layouts,
    function: &KFunction,
) -> Result<LocalLayout, String> {
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
            Op::Drop { place } | Op::DropIf { place } => {
                if !place.projections.is_empty()
                    || heap_type(&program.types[function.locals[place.local].ty].kind)
                {
                    touch_place(&mut ranges, place, at);
                }
            }
            Op::TagTest { dst, place, .. } => {
                touch_local(&mut ranges, *dst, at);
                touch_place(&mut ranges, place, at);
            }
            Op::Unary { dst, src, .. } | Op::Discriminant { dst, src } => {
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
            Op::Else
            | Op::Break
            | Op::Continue
            | Op::BreakTo { .. }
            | Op::ContinueTo { .. }
            | Op::Trap { .. } => {}
        }
    }
    let mut slots = vec![Slot::Register(0); n];
    let mut frame_bytes = 0_u64;
    let mut starts = Vec::new();
    for local in 0..n {
        if addressed[local] {
            let width = representations.storage(function.locals[local].ty).bytes;
            let offset = aligned(frame_bytes, width)?;
            slots[local] = Slot::Frame(offset);
            frame_bytes = offset
                .checked_add(u64::from(width))
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
    let mut active: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut available_registers = Vec::new();
    let mut available_frames: [Vec<u64>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut registers = function.params;
    for (first, local) in starts {
        while let Some(&(last, previous)) = active.first() {
            if last >= first {
                break;
            }
            active.pop_first();
            match slots[previous] {
                Slot::Register(register) => available_registers.push(register),
                Slot::Frame(offset) => {
                    let width = representations.storage(function.locals[previous].ty).bytes;
                    available_frames[width.trailing_zeros() as usize].push(offset);
                }
            }
        }
        let width = representations.storage(function.locals[local].ty).bytes;
        slots[local] = if let Some(register) = available_registers.pop() {
            Slot::Register(register)
        } else if registers - function.params < REGISTER_POOL {
            let register = registers;
            registers += 1;
            Slot::Register(register)
        } else if let Some(offset) = available_frames[width.trailing_zeros() as usize].pop() {
            Slot::Frame(offset)
        } else {
            let offset = aligned(frame_bytes, width)?;
            frame_bytes = offset
                .checked_add(u64::from(width))
                .ok_or("local frame extent overflow")?;
            Slot::Frame(offset)
        };
        active.insert((ranges[local].1, local));
    }
    let frame_bytes = aligned(frame_bytes, 8)?;
    Ok(LocalLayout {
        slots,
        registers,
        frame_bytes,
    })
}

fn function_body(
    program: &KProgram,
    representations: &Layouts,
    runtime: usize,
    function: &KFunction,
    messages: &[String],
    texts: &TextPool,
) -> Result<Code, String> {
    let layout = local_layout(program, representations, function)?;
    let mut drop_flags = BTreeMap::new();
    for instruction in &function.body {
        if let Op::DropIf { place } = &instruction.op {
            if place.projections.is_empty() && !drop_flags.contains_key(&place.local) {
                let flag = layout
                    .registers
                    .checked_add(9)
                    .and_then(|first| first.checked_add(drop_flags.len()))
                    .ok_or("lowered local index exceeds the host representation")?;
                drop_flags.insert(place.local, flag);
            }
        }
    }
    let flag_count = drop_flags.len();
    let mut body = Body {
        program,
        layout: representations,
        function,
        code: Code::words(0),
        slots: layout.slots,
        drop_flags,
        scratch: layout.registers,
        frame: layout.registers + 8,
        frame_bytes: layout.frame_bytes,
        controls: Vec::new(),
        messages,
        texts,
        runtime,
    };
    body.code.locals = vec![ValueType::I64; layout.registers - function.params + 9];
    body.code.locals.extend(vec![ValueType::I32; flag_count]);
    if flag_count != 0 {
        for local in 0..function.params {
            body.initialized(local, true);
        }
    }
    if layout.frame_bytes != 0 {
        constant(&mut body.code, layout.frame_bytes as i64);
        call(&mut body.code, ALLOC + runtime);
        set(&mut body.code, body.frame);
        for i in 0..function.params {
            if let Slot::Frame(offset) = body.slots[i] {
                get(&mut body.code, body.frame);
                get(&mut body.code, i);
                store_value(
                    &mut body.code,
                    representations.storage(function.locals[i].ty),
                    offset,
                );
            }
        }
    }
    for instruction in &function.body {
        body.emit(&instruction.op, instruction.line)?;
    }
    if layout.frame_bytes != 0 {
        get(&mut body.code, body.frame);
        call(&mut body.code, FREE + runtime);
        body.code.push(Instruction::Drop);
    }
    constant(&mut body.code, 0);
    body.code.push(Instruction::End);
    Ok(body.code)
}
/// Apply representation, cleanup and runtime contracts exactly once. Encoders
/// consume the resulting functions and memory operations without consulting K.
pub fn lower(
    checked: &CheckedProgram,
    source_runtime: &crate::runtime::Runtime,
) -> Result<Program, String> {
    let program = checked.program();
    let runtime = usize::from(program.functions.iter().any(|function| {
        function.body.iter().any(|instruction| {
            matches!(&instruction.op,
            Op::Intrinsic { name, .. } if name == "stdout_write")
        })
    }));
    let texts = TextPool::collect(program)?;
    let layout = Layouts::new(program)?;
    let mut destructors = vec![None; program.types.len()];
    for (owner, function) in &program.destructors {
        destructors[*owner] = Some(*function);
    }
    let mut messages = Vec::new();
    let mut seen_messages = BTreeSet::new();
    for function in &program.functions {
        for instruction in &function.body {
            if let Op::Trap { message } = &instruction.op {
                if seen_messages.insert(message.clone()) {
                    messages.push(message.clone());
                }
            }
        }
    }
    let mut functions = Vec::new();
    for (name, role) in [
        ("memory::allocate", RuntimeFunction::Allocate),
        ("memory::release", RuntimeFunction::Release),
        ("drop::walk", RuntimeFunction::DropDispatch),
        ("str::valid_utf8", RuntimeFunction::Utf8Validate),
    ] {
        functions.push(Function {
            name: name.to_string(),
            signature: Signature::words(1),
            role: FunctionRole::Runtime(role),
            code: Code {
                locals: Vec::new(),
                instructions: vec![Instruction::Unreachable, Instruction::End],
            },
        });
    }
    for function in &program.functions {
        functions.push(Function {
            name: function.name.clone(),
            signature: Signature::words(function.params),
            role: FunctionRole::Source,
            code: function_body(program, &layout, runtime, function, &messages, &texts)?,
        });
    }
    for ty in 0..program.types.len() {
        for copying in [false, true] {
            functions.push(Function {
                name: format!(
                    "{}<{}>",
                    if copying { "copy" } else { "drop" },
                    program.types[ty].name
                ),
                signature: Signature::words(1),
                role: if copying {
                    FunctionRole::CopyGlue
                } else {
                    FunctionRole::DropGlue
                },
                code: if copying {
                    copy_helper(program, runtime, &layout, ty)
                } else {
                    drop_start(program, runtime, ty)
                },
            });
        }
    }
    for (ty, destructor) in destructors.iter().enumerate() {
        functions.push(Function {
            name: format!("drop::step<{}>", program.types[ty].name),
            signature: Signature::words(1),
            role: FunctionRole::DropStep,
            code: drop_step(program, runtime, &layout, ty, *destructor),
        });
    }
    let minimum_memory_pages = texts.pages()?;
    let heap_start = texts.heap_start()?;
    let mut lowered = Program {
        layout,
        imports: if runtime == 0 {
            Vec::new()
        } else {
            vec![Import {
                host: HostFunction::StdoutWrite,
                signature: Signature::words(3),
            }]
        },
        functions,
        globals: vec![
            Global {
                initial: heap_start as i64,
                mutable: true,
            },
            Global {
                initial: 0,
                mutable: true,
            },
            Global {
                initial: 0,
                mutable: true,
            },
            Global {
                initial: 0,
                mutable: true,
            },
        ],
        exports: program
            .functions
            .iter()
            .enumerate()
            .map(|(i, function)| Export {
                name: format!("fn:{}", function.name),
                function: FIRST_USER + runtime + i,
            })
            .collect(),
        table: (0..program.types.len())
            .map(|ty| drop_step_index(program, runtime, ty))
            .collect(),
        data: texts.data,
        data_offset: 8,
        minimum_memory_pages,
        trap_code_global: 2,
        trap_line_global: 3,
        trap_messages: messages,
    };
    source_runtime.link(&mut lowered)?;
    Ok(lowered)
}

/// Lower the checked scalar source of the sealed runtime without recursively
/// building another runtime. Calls retain FIRST_USER + source-function index;
/// the runtime linker remaps those calls when appending these functions.
/// Authority to replace primitive stubs belongs to the runtime preparer, not
/// to an ordinary K function name or this storage-only validation.
pub fn lower_runtime_functions(checked: &CheckedProgram) -> Result<Vec<Function>, String> {
    let program = checked.program();
    let layout = Layouts::new(program)?;
    let texts = TextPool {
        offsets: BTreeMap::new(),
        data: Vec::new(),
    };
    let messages = Vec::new();
    let mut functions = Vec::new();
    for function in &program.functions {
        for ty in function
            .locals
            .iter()
            .map(|local| local.ty)
            .chain(std::iter::once(function.result))
        {
            if !matches!(
                program.types[ty].kind,
                TypeKind::Unit | TypeKind::Bool | TypeKind::Int { .. } | TypeKind::F64
            ) {
                return Err("runtime source functions must use scalar storage".to_string());
            }
        }
        if local_layout(program, &layout, function)?.frame_bytes != 0 {
            return Err("runtime source function would allocate a local frame".to_string());
        }
        let mut trap_messages = messages.clone();
        for instruction in &function.body {
            if let Op::Trap { message } = &instruction.op {
                if !trap_messages.contains(message) {
                    trap_messages.push(message.clone());
                }
            }
        }
        let code = function_body(program, &layout, 0, function, &trap_messages, &texts)?;
        if code.instructions.iter().any(|instruction| matches!(instruction, Instruction::Call(index) if *index < FIRST_USER || *index >= FIRST_USER + program.functions.len())) {
            return Err("runtime source function requires ordinary heap glue".to_string());
        }
        functions.push(Function {
            name: function.name.clone(),
            signature: Signature::words(function.params),
            role: FunctionRole::RuntimeSource,
            code,
        });
    }
    Ok(functions)
}
