//! Direct instruction selection from checked lkjscript IR to native x86-64.
use crate::diagnostic::{Diagnostic, Span};
use crate::ir::{self, CheckedProgram, Function, Op, Place, Program, Projection, TypeKind};
use crate::runtime::{self, Primitive, Runtime};
use crate::x86::*;
use crate::x86::{Assembler, Label};
use std::collections::{BTreeMap, BTreeSet};

mod intrinsics;
mod layout;
mod ownership;
pub(crate) mod platform;
mod scalar;

use layout::Layouts;
use ownership::Ownership;

pub struct Entry {
    pub function: usize,
    pub arguments: Vec<i64>,
    pub result_fd: Option<u32>,
}
pub struct Image {
    pub bytes: Vec<u8>,
    pub trap_messages: Vec<String>,
}
#[derive(Clone, Copy)]
struct RuntimeLabels {
    allocate: Label,
    release: Label,
    reallocate: Label,
    utf8_validate: Label,
}
impl RuntimeLabels {
    fn new(runtime: &Runtime, labels: &[Label]) -> Self {
        Self {
            allocate: labels[runtime.entry(runtime::Entry::Allocate)],
            release: labels[runtime.entry(runtime::Entry::Release)],
            reallocate: labels[runtime.entry(runtime::Entry::Reallocate)],
            utf8_validate: labels[runtime.entry(runtime::Entry::Utf8Validate)],
        }
    }
}
fn parameter_register(index: usize) -> u8 {
    match index {
        0 => DI,
        1 => SI,
        2 => D,
        3 => C,
        4 => R8,
        _ => R9,
    }
}
fn first_site(function: &Function) -> Span {
    Span {
        file: function.file,
        start: 0,
        end: 0,
        line: function
            .body
            .first()
            .map_or(1, |instruction| instruction.line),
        column: function
            .body
            .first()
            .map_or(1, |instruction| instruction.column),
    }
}
fn allocate_slot(
    frame: &mut u64,
    bytes: u64,
    alignment: u8,
    site: Span,
) -> Result<i64, Diagnostic> {
    let mask = alignment as u64 - 1;
    *frame = match frame.checked_add(mask) {
        Some(value) => value & !mask,
        None => {
            return Err(Diagnostic::new(
                site,
                "E_EMIT",
                "native frame extent overflow",
            ));
        }
    };
    *frame = match frame.checked_add(bytes.max(alignment as u64)) {
        Some(value) => value,
        None => {
            return Err(Diagnostic::new(
                site,
                "E_EMIT",
                "native frame extent overflow",
            ));
        }
    };
    if *frame > 2147483647 {
        return Err(Diagnostic::new(
            site,
            "E_EMIT",
            "native frame exceeds signed 32-bit displacement representation",
        ));
    }
    Ok(-(*frame as i64))
}
fn frame_extent(bytes: u64, site: Span) -> Result<u64, Diagnostic> {
    let aligned = match bytes.checked_add(15) {
        Some(value) => value & !15,
        None => {
            return Err(Diagnostic::new(
                site,
                "E_EMIT",
                "native frame extent overflow",
            ));
        }
    };
    if aligned > 2147483647 {
        return Err(Diagnostic::new(
            site,
            "E_EMIT",
            "native frame exceeds signed 32-bit displacement representation",
        ));
    }
    Ok(aligned)
}
fn prologue(a: &mut Assembler, p: platform::Platform, bytes: u64) {
    a.mov(R11, SP);
    a.immediate(A, bytes + 16);
    a.rr(0x29, R11, A, true);
    a.condition(2, p.stack_exhausted);
    a.compare(R11, STACK_FLOOR);
    a.condition(2, p.stack_exhausted);
    a.push(BP);
    a.mov(BP, SP);
    if bytes != 0 {
        a.immediate(A, bytes);
        a.rr(0x29, SP, A, true);
    }
}
fn copy_bytes(a: &mut Assembler, p: platform::Platform, bytes: u64) {
    if bytes == 0 {
        return;
    }
    if bytes == 1 || bytes == 2 || bytes == 4 || bytes == 8 {
        a.load_width(A, SI, 0, bytes as u8, false);
        a.store_width(DI, 0, A, bytes as u8);
    } else if bytes == 16 || bytes == 24 || bytes == 32 {
        for word in 0..bytes / 8 {
            a.load(R8 + word as u8, SI, word as i64 * 8);
        }
        for word in 0..bytes / 8 {
            a.store(DI, word as i64 * 8, R8 + word as u8);
        }
    } else {
        a.immediate(D, bytes);
        a.call(p.copy);
    }
}
fn epilogue(a: &mut Assembler) {
    a.mov(SP, BP);
    a.pop(BP);
    a.ret();
}
enum Control {
    Conditional {
        otherwise: Label,
        end: Label,
        has_else: bool,
    },
    Loop {
        start: Label,
        end: Label,
    },
}
struct Body<'a> {
    a: &'a mut Assembler,
    program: &'a Program,
    function: &'a Function,
    layout: &'a Layouts,
    platform: platform::Platform,
    runtime: RuntimeLabels,
    slots: Vec<i64>,
    scratch: Vec<i64>,
    partial_scratch: Vec<i64>,
    ownership: Ownership,
    site: Span,
    functions: &'a [Label],
    needed: &'a mut Vec<bool>,
    drops: &'a [Label],
    needed_drops: &'a mut Vec<bool>,
    literals: &'a mut BTreeMap<Vec<u8>, Label>,
    messages: &'a [String],
    controls: Vec<Control>,
    result_slot: Option<i64>,
    return_label: Label,
    source_lines: bool,
}
impl<'a> Body<'a> {
    fn problem(&self, message: &str) -> Diagnostic {
        Diagnostic::new(self.site, "E_EMIT", message)
    }
    fn ty(&self, local: usize) -> usize {
        self.function.locals[local].ty
    }
    // A local read changes only the requested register.
    fn read(&mut self, local: usize, register: u8) {
        let storage = self.layout.storage(self.ty(local));
        self.a.load_width(
            register,
            BP,
            self.slots[local],
            storage.bytes as u8,
            storage.signed,
        );
    }
    fn read_word(&mut self, local: usize, word: usize, register: u8) {
        self.a
            .load(register, BP, self.slots[local] + word as i64 * 8);
    }
    fn write(&mut self, local: usize, register: u8) {
        let bytes = self.layout.storage(self.ty(local)).bytes as u8;
        self.a.store_width(BP, self.slots[local], register, bytes);
        self.mark(local);
    }
    fn write_word(&mut self, local: usize, word: usize, register: u8) {
        self.a
            .store(BP, self.slots[local] + word as i64 * 8, register);
    }
    fn mark(&mut self, local: usize) {
        self.ownership.change(self.a, &Place::local(local), true);
    }
    fn local_address(&mut self, local: usize, register: u8) {
        self.a.lea(register, BP, self.slots[local]);
    }
    fn trap(&mut self, code: u64) {
        self.a.immediate(A, code);
        self.a.jump(self.platform.trap);
    }
    fn trap_if(&mut self, condition: u8, code: u64) {
        let valid = self.a.label();
        self.a.condition(condition ^ 1, valid);
        self.trap(code);
        self.a.bind(valid);
    }
    // Small inline copies read every source word before writing any destination;
    // larger copies use the overlap-safe platform leaf. Registers are caller-saved.
    fn copy_value(&mut self, destination: u8, source: u8, ty: usize) {
        let bytes = self.layout.storage(ty).bytes;
        if destination == SI && source == DI {
            self.a.mov(R8, DI);
            self.a.mov(DI, SI);
            self.a.mov(SI, R8);
        } else if source == DI {
            self.a.mov(SI, DI);
            self.a.mov(DI, destination);
        } else {
            self.a.mov(DI, destination);
            self.a.mov(SI, source);
        }
        copy_bytes(self.a, self.platform, bytes);
    }
    fn copy_local(&mut self, destination: usize, source: usize) {
        let ty = self.ty(destination);
        let scalar = self.layout.storage(ty).scalar;
        if scalar {
            self.read(source, A);
            self.write(destination, A);
        } else {
            self.local_address(destination, DI);
            self.local_address(source, SI);
            self.copy_value(DI, SI, ty);
            self.mark(destination);
        }
    }
    fn drop_at(&mut self, ty: usize, address: u8) {
        if self.layout.storage(ty).cleanup {
            self.needed_drops[ty] = true;
            if address != DI {
                self.a.mov(DI, address);
            }
            self.a.call(self.drops[ty]);
        }
    }
    // Address selection uses RAX, RCX, R10 and R11. For an unsized place the
    // accompanying count is retained in scratch[7].
    fn place_address(&mut self, place: &Place, register: u8) -> Result<usize, Diagnostic> {
        let mut ty = self.ty(place.local);
        self.local_address(place.local, R10);
        for projection in &place.projections {
            match projection {
                Projection::Deref => {
                    let inner = match self.program.types[ty].kind {
                        TypeKind::Ref { inner, .. } | TypeKind::Box { inner } => inner,
                        _ => return Err(self.problem("checked dereference has no address")),
                    };
                    if matches!(
                        self.program.types[inner].kind,
                        TypeKind::Slice { .. } | TypeKind::Str
                    ) {
                        self.a.load(R11, R10, 8);
                    }
                    self.a.load(R10, R10, 0);
                    ty = inner;
                }
                Projection::Field(field) => {
                    let offset = self.layout.field_offset(ty, None, *field);
                    let inner = match &self.program.types[ty].kind {
                        TypeKind::Struct { fields, .. } => fields[*field].ty,
                        _ => return Err(self.problem("checked field has no aggregate")),
                    };
                    self.a.lea(R10, R10, offset as i64);
                    ty = inner;
                }
                Projection::VariantField(variant, field) => {
                    let offset = self.layout.field_offset(ty, Some(*variant), *field);
                    let inner = match &self.program.types[ty].kind {
                        TypeKind::Enum { variants, .. } => variants[*variant].fields[*field].ty,
                        _ => return Err(self.problem("checked variant field has no enum")),
                    };
                    self.a.lea(R10, R10, offset as i64);
                    ty = inner;
                }
                Projection::Index(index) => {
                    let inner = match self.program.types[ty].kind {
                        TypeKind::Array { inner, len } => {
                            self.a.immediate(R11, len as u64);
                            inner
                        }
                        TypeKind::Vec { inner } => {
                            self.a.load(R11, R10, 8);
                            self.a.load(R10, R10, 0);
                            inner
                        }
                        TypeKind::Slice { inner, .. } => inner,
                        _ => return Err(self.problem("checked index has no collection")),
                    };
                    self.read(*index, A);
                    self.a.compare(A, R11);
                    self.trap_if(3, 3);
                    self.a.immediate(C, self.layout.stride(inner));
                    self.a.imul(A, C);
                    self.a.rr(0x01, R10, A, true);
                    ty = inner;
                }
            }
        }
        if matches!(
            self.program.types[ty].kind,
            TypeKind::Slice { .. } | TypeKind::Str
        ) {
            self.a.store(BP, self.scratch[7], R11);
        }
        if register != R10 {
            self.a.mov(register, R10);
        }
        Ok(ty)
    }
    fn consume_inputs(&mut self, op: &Op) {
        match op {
            Op::Read {
                place,
                moving: true,
                ..
            } => self.ownership.change(self.a, place, false),
            Op::Assign { src, .. } | Op::Unary { src, .. } | Op::Return { src } => {
                self.ownership.change(self.a, &Place::local(*src), false);
            }
            Op::Binary { left, right, .. } => {
                self.ownership.change(self.a, &Place::local(*left), false);
                self.ownership.change(self.a, &Place::local(*right), false);
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
                    self.ownership.change(self.a, &Place::local(*local), false);
                }
            }
            Op::If { condition } => self
                .ownership
                .change(self.a, &Place::local(*condition), false),
            Op::ViewRange { start, end, .. } => {
                self.ownership.change(self.a, &Place::local(*start), false);
                self.ownership.change(self.a, &Place::local(*end), false);
            }
            _ => {}
        }
    }
    fn call(&mut self, dst: usize, called: usize, args: &[usize]) -> Result<(), Diagnostic> {
        let aggregate_result = !self.layout.storage(self.ty(dst)).scalar;
        let hidden = if aggregate_result { 1usize } else { 0 };
        let words = args.len() + hidden;
        let stacked = if words > 6 { words - 6 } else { 0 };
        let padded = (stacked + 1) & !1;
        let stack_bytes = padded as u64 * 8;
        if stack_bytes != 0 {
            self.a.mov(R11, SP);
            self.a.immediate(A, stack_bytes + 8);
            self.a.rr(0x29, R11, A, true);
            self.a.condition(2, self.platform.stack_exhausted);
            self.a.compare(R11, STACK_FLOOR);
            self.a.condition(2, self.platform.stack_exhausted);
        }
        if padded != stacked {
            self.a.immediate(A, 8);
            self.a.rr(0x29, SP, A, true);
        }
        let mut word = words;
        while word > 6 {
            word -= 1;
            let arg = args[word - hidden];
            let scalar = self.layout.storage(self.ty(arg)).scalar;
            if scalar {
                self.read(arg, A);
            } else {
                self.local_address(arg, A);
            }
            self.a.push(A);
        }
        if aggregate_result {
            self.local_address(dst, DI);
        }
        for index in 0..args.len() {
            let abi = index + hidden;
            if abi < 6 {
                let register = parameter_register(abi);
                let local = args[index];
                let scalar = self.layout.storage(self.ty(local)).scalar;
                if scalar {
                    self.read(local, register);
                } else {
                    self.local_address(local, register);
                }
            }
        }
        self.needed[called] = true;
        self.a.call(self.functions[called]);
        if stack_bytes != 0 {
            self.a.immediate(R11, stack_bytes);
            self.a.rr(0x01, SP, R11, true);
        }
        if aggregate_result {
            self.mark(dst);
        } else {
            self.write(dst, A);
        }
        Ok(())
    }
    fn collection(&mut self, place: &Place) -> Result<(usize, u64), Diagnostic> {
        let ty = self.place_address(place, R10)?;
        let stride = match self.program.types[ty].kind {
            TypeKind::Array { inner, len } => {
                self.a.immediate(R11, len as u64);
                self.layout.stride(inner)
            }
            TypeKind::Vec { inner } => {
                self.a.load(R11, R10, 8);
                self.a.load(R10, R10, 0);
                self.layout.stride(inner)
            }
            TypeKind::Slice { inner, .. } => {
                self.a.load(R11, BP, self.scratch[7]);
                self.layout.stride(inner)
            }
            TypeKind::Str => {
                self.a.load(R11, BP, self.scratch[7]);
                1
            }
            _ => return Err(self.problem("checked view has no collection")),
        };
        self.a.store(BP, self.scratch[0], R10);
        self.a.store(BP, self.scratch[1], R11);
        Ok((ty, stride))
    }
    fn view_range(
        &mut self,
        dst: usize,
        place: &Place,
        start: usize,
        end: usize,
        inclusive: bool,
    ) -> Result<(), Diagnostic> {
        let (source, stride) = self.collection(place)?;
        self.read(end, A);
        self.a.load(C, BP, self.scratch[1]);
        self.a.compare(A, C);
        self.trap_if(if inclusive { 3 } else { 7 }, 3);
        if inclusive {
            self.a.immediate(C, 1);
            self.a.rr(0x01, A, C, true);
        }
        self.a.store(BP, self.scratch[2], A);
        self.read(start, C);
        self.a.compare(C, A);
        self.trap_if(7, 3);
        if matches!(self.program.types[source].kind, TypeKind::Str) {
            for endpoint in 0..2 {
                if endpoint == 0 {
                    self.read(start, A);
                } else {
                    self.a.load(A, BP, self.scratch[2]);
                }
                self.a.load(C, BP, self.scratch[1]);
                self.a.compare(A, C);
                let boundary = self.a.label();
                self.a.condition(4, boundary);
                self.a.load(C, BP, self.scratch[0]);
                self.a.rr(0x01, C, A, true);
                self.a.load_width(A, C, 0, 1, false);
                self.a.immediate(C, 0xc0);
                self.a.rr(0x21, A, C, true);
                self.a.immediate(C, 0x80);
                self.a.compare(A, C);
                self.trap_if(4, 3);
                self.a.bind(boundary);
            }
        }
        self.read(start, A);
        self.a.immediate(C, stride);
        self.a.imul(A, C);
        self.a.load(C, BP, self.scratch[0]);
        self.a.rr(0x01, A, C, true);
        self.write_word(dst, 0, A);
        self.a.load(A, BP, self.scratch[2]);
        self.read(start, C);
        self.a.rr(0x29, A, C, true);
        self.write_word(dst, 1, A);
        self.mark(dst);
        Ok(())
    }
    fn partial_drop(&mut self, ty: usize, place: &Place, depth: usize) -> Result<(), Diagnostic> {
        let absent = self.a.label();
        if let Some(flag) = self.ownership.flag(place) {
            self.a.load_width(A, BP, flag, 1, false);
            self.a.test(A, true);
            self.a.condition(4, absent);
        }
        if !self.ownership.descendants(place) {
            self.drop_at(ty, DI);
            self.a.bind(absent);
            return Ok(());
        }
        self.a.store(BP, self.partial_scratch[depth], DI);
        let destructor = self
            .program
            .destructors
            .iter()
            .find(|entry| entry.0 == ty)
            .map(|entry| entry.1);
        if let Some(destructor) = destructor {
            self.needed[destructor] = true;
            self.a.call(self.functions[destructor]);
        }
        let kind = self.program.types[ty].kind.clone();
        match kind {
            TypeKind::Struct { fields, .. } => {
                for index in 0..fields.len() {
                    let mut child = place.clone();
                    child.projections.push(Projection::Field(index));
                    self.a.load(DI, BP, self.partial_scratch[depth]);
                    self.a
                        .lea(DI, DI, self.layout.field_offset(ty, None, index) as i64);
                    self.partial_drop(fields[index].ty, &child, depth + 1)?;
                }
            }
            TypeKind::Enum { variants, .. } => {
                let done = self.a.label();
                for variant in 0..variants.len() {
                    let next = self.a.label();
                    self.a.load(DI, BP, self.partial_scratch[depth]);
                    self.a.load(A, DI, 0);
                    self.a.immediate(C, variant as u64);
                    self.a.compare(A, C);
                    self.a.condition(5, next);
                    for index in 0..variants[variant].fields.len() {
                        let mut child = place.clone();
                        child
                            .projections
                            .push(Projection::VariantField(variant, index));
                        self.a.load(DI, BP, self.partial_scratch[depth]);
                        self.a.lea(
                            DI,
                            DI,
                            self.layout.field_offset(ty, Some(variant), index) as i64,
                        );
                        self.partial_drop(variants[variant].fields[index].ty, &child, depth + 1)?;
                    }
                    self.a.jump(done);
                    self.a.bind(next);
                }
                self.a.bind(done);
            }
            TypeKind::Box { inner } => {
                let mut child = place.clone();
                child.projections.push(Projection::Deref);
                self.a.load(DI, BP, self.partial_scratch[depth]);
                self.a.load(DI, DI, 0);
                self.partial_drop(inner, &child, depth + 1)?;
                self.a.load(DI, BP, self.partial_scratch[depth]);
                self.a.load(DI, DI, 0);
                self.a.call(self.runtime.release);
            }
            _ => {
                self.a.load(DI, BP, self.partial_scratch[depth]);
                self.drop_at(ty, DI);
            }
        }
        self.a.bind(absent);
        Ok(())
    }
    fn drop_place(&mut self, place: &Place) -> Result<(), Diagnostic> {
        let ty = self.place_address(place, DI)?;
        self.partial_drop(ty, place, 0)?;
        self.ownership.change(self.a, place, false);
        Ok(())
    }
    fn emit(&mut self, instruction: &ir::Instruction) -> Result<(), Diagnostic> {
        self.site = Span {
            file: self.function.file,
            start: 0,
            end: 0,
            line: instruction.line,
            column: instruction.column,
        };
        self.a
            .location(self.site.file, self.site.line, self.site.column);
        if self.source_lines
            && !matches!(
                instruction.op,
                Op::Else | Op::End | Op::Loop | Op::Unit { .. } | Op::Int { .. } | Op::Float { .. }
            )
        {
            self.a.immediate(LINE, instruction.line as u64);
            self.a.immediate(FILE, self.function.file as u64);
        }
        self.consume_inputs(&instruction.op);
        match &instruction.op {
            Op::Int { dst, value } => {
                self.a.immediate(A, *value as u64);
                self.write(*dst, A);
            }
            Op::Float { dst, bits } => {
                self.a.immediate(A, *bits);
                self.write(*dst, A);
            }
            Op::Unit { dst } => {
                self.a.immediate(A, 0);
                self.write(*dst, A);
            }
            Op::Text { dst, bytes } => {
                let label = match self.literals.get(bytes) {
                    Some(label) => *label,
                    None => {
                        let label = self.a.label();
                        self.literals.insert(bytes.clone(), label);
                        label
                    }
                };
                self.a.address(A, label);
                let scalar = self.layout.storage(self.ty(*dst)).scalar;
                if scalar {
                    self.write(*dst, A);
                } else {
                    self.write_word(*dst, 0, A);
                    self.a.immediate(A, bytes.len() as u64);
                    self.write_word(*dst, 1, A);
                    self.mark(*dst);
                }
            }
            Op::Read { dst, place, .. } => {
                let ty = self.place_address(place, SI)?;
                let scalar = self.layout.storage(ty).scalar;
                if scalar {
                    let bytes = self.layout.storage(ty).bytes as u8;
                    let signed = self.layout.storage(ty).signed;
                    self.a.load_width(A, SI, 0, bytes, signed);
                    self.write(*dst, A);
                } else {
                    self.local_address(*dst, DI);
                    self.copy_value(DI, SI, ty);
                    self.mark(*dst);
                }
            }
            Op::Borrow { dst, place, .. } => {
                let ty = self.place_address(place, A)?;
                if matches!(
                    self.program.types[ty].kind,
                    TypeKind::Slice { .. } | TypeKind::Str
                ) {
                    self.write_word(*dst, 0, A);
                    self.a.load(A, BP, self.scratch[7]);
                    self.write_word(*dst, 1, A);
                    self.mark(*dst);
                } else {
                    self.write(*dst, A);
                }
            }
            Op::SliceBorrow { dst, place, .. } => {
                self.collection(place)?;
                self.a.load(A, BP, self.scratch[0]);
                self.write_word(*dst, 0, A);
                self.a.load(A, BP, self.scratch[1]);
                self.write_word(*dst, 1, A);
                self.mark(*dst);
            }
            Op::ViewRange {
                dst,
                place,
                start,
                end,
                inclusive,
                ..
            } => {
                self.view_range(*dst, place, *start, *end, *inclusive)?;
            }
            Op::Assign { place, src } => {
                let ty = self.place_address(place, DI)?;
                let scalar = self.layout.storage(ty).scalar;
                if scalar {
                    self.read(*src, A);
                    self.a
                        .store_width(DI, 0, A, self.layout.storage(ty).bytes as u8);
                } else {
                    self.local_address(*src, SI);
                    self.copy_value(DI, SI, ty);
                }
                self.ownership.change(self.a, place, true);
            }
            Op::Drop { place } | Op::DropIf { place } => self.drop_place(place)?,
            Op::Unary { dst, op, src } => self.scalar_unary(*dst, op, *src)?,
            Op::Binary {
                dst,
                op,
                left,
                right,
            } => self.scalar_binary(*dst, op, *left, *right)?,
            Op::Call {
                dst,
                function,
                args,
            } => self.call(*dst, *function, args)?,
            Op::Array { dst, elements } => {
                let inner = match self.program.types[self.ty(*dst)].kind {
                    TypeKind::Array { inner, .. } => inner,
                    _ => return Err(self.problem("checked array constructor has no array")),
                };
                for index in 0..elements.len() {
                    self.local_address(*dst, DI);
                    self.a
                        .lea(DI, DI, (index as u64 * self.layout.stride(inner)) as i64);
                    self.local_address(elements[index], SI);
                    self.copy_value(DI, SI, inner);
                }
                self.mark(*dst);
            }
            Op::Struct { dst, fields } | Op::Variant { dst, fields, .. } => {
                let variant = match instruction.op {
                    Op::Variant { variant, .. } => Some(variant),
                    _ => None,
                };
                let ty = self.ty(*dst);
                if let Some(variant) = variant {
                    self.a.immediate(A, variant as u64);
                    self.write_word(*dst, 0, A);
                }
                for index in 0..fields.len() {
                    self.local_address(*dst, DI);
                    self.a
                        .lea(DI, DI, self.layout.field_offset(ty, variant, index) as i64);
                    self.local_address(fields[index], SI);
                    let field_ty = self.ty(fields[index]);
                    self.copy_value(DI, SI, field_ty);
                }
                self.mark(*dst);
            }
            Op::Discriminant { dst, src } => {
                self.read_word(*src, 0, A);
                self.write(*dst, A);
            }
            Op::TagTest {
                dst,
                place,
                variant,
            } => {
                let ty = self.place_address(place, A)?;
                if matches!(self.program.types[ty].kind, TypeKind::Ref { .. }) {
                    self.a.load(A, A, 0);
                }
                self.a.load(A, A, 0);
                self.a.immediate(C, *variant as u64);
                self.a.compare(A, C);
                self.a.setcc(A, 4);
                self.write(*dst, A);
            }
            Op::Intrinsic { dst, name, args } => self.intrinsic(*dst, name, args)?,
            Op::If { condition } => {
                let otherwise = self.a.label();
                let end = self.a.label();
                self.read(*condition, A);
                self.a.test(A, true);
                self.a.condition(4, otherwise);
                self.controls.push(Control::Conditional {
                    otherwise,
                    end,
                    has_else: false,
                });
            }
            Op::Else => {
                let control = self
                    .controls
                    .pop()
                    .ok_or_else(|| self.problem("checked else has no region"))?;
                match control {
                    Control::Conditional { otherwise, end, .. } => {
                        self.a.jump(end);
                        self.a.bind(otherwise);
                        self.controls.push(Control::Conditional {
                            otherwise,
                            end,
                            has_else: true,
                        });
                    }
                    _ => return Err(self.problem("checked else belongs to a loop")),
                }
            }
            Op::Loop => {
                let start = self.a.label();
                let end = self.a.label();
                self.a.bind(start);
                self.controls.push(Control::Loop { start, end });
            }
            Op::End => {
                match self
                    .controls
                    .pop()
                    .ok_or_else(|| self.problem("checked end has no region"))?
                {
                    Control::Conditional {
                        otherwise,
                        end,
                        has_else,
                    } => {
                        if !has_else {
                            self.a.bind(otherwise);
                        }
                        self.a.bind(end);
                    }
                    Control::Loop { start, end } => {
                        self.a.jump(start);
                        self.a.bind(end);
                    }
                }
            }
            Op::Break | Op::Continue | Op::BreakTo { .. } | Op::ContinueTo { .. } => {
                let mut depth = match instruction.op {
                    Op::BreakTo { depth } | Op::ContinueTo { depth } => depth,
                    _ => 0,
                };
                let continuation = matches!(instruction.op, Op::Continue | Op::ContinueTo { .. });
                let mut index = self.controls.len();
                let mut target = None;
                while index != 0 {
                    index -= 1;
                    if let Control::Loop { start, end } = self.controls[index] {
                        if depth == 0 {
                            target = Some(if continuation { start } else { end });
                            break;
                        }
                        depth -= 1;
                    }
                }
                let target =
                    target.ok_or_else(|| self.problem("checked loop transfer has no target"))?;
                self.a.jump(target);
            }
            Op::Return { src } => {
                if let Some(slot) = self.result_slot {
                    self.a.load(DI, BP, slot);
                    self.local_address(*src, SI);
                    let result_ty = self.function.result;
                    self.copy_value(DI, SI, result_ty);
                    self.a.immediate(A, 0);
                } else {
                    self.read(*src, A);
                }
                self.a.jump(self.return_label);
            }
            Op::Trap { message } => {
                let mut index = 0;
                while index < self.messages.len() && self.messages[index] != *message {
                    index += 1;
                }
                self.trap(16 + index as u64);
            }
        }
        Ok(())
    }
}
struct Selection<'a> {
    program: &'a Program,
    layout: &'a Layouts,
    functions: &'a [Label],
    drops: &'a [Label],
    runtime: RuntimeLabels,
    messages: &'a [String],
    source_lines: bool,
}
fn emit_function(
    a: &mut Assembler,
    p: platform::Platform,
    selection: &Selection<'_>,
    function_index: usize,
    needed: &mut Vec<bool>,
    needed_drops: &mut Vec<bool>,
    literals: &mut BTreeMap<Vec<u8>, Label>,
) -> Result<(), Diagnostic> {
    let program = selection.program;
    let layout = selection.layout;
    let functions = selection.functions;
    let drops = selection.drops;
    let runtime = selection.runtime;
    let messages = selection.messages;
    let source_lines = selection.source_lines;
    let function = &program.functions[function_index];
    let site = first_site(function);
    let mut frame = 0;
    let mut slots = Vec::new();
    for local in &function.locals {
        let storage = layout.storage(local.ty);
        slots.push(allocate_slot(
            &mut frame,
            storage.bytes,
            storage.alignment,
            site,
        )?);
    }
    let mut scratch = Vec::new();
    for _ in 0..8 {
        scratch.push(allocate_slot(&mut frame, 8, 8, site)?);
    }
    let mut ownership = Ownership::new(function);
    ownership.allocate(&mut frame, site)?;
    let mut partial_scratch = Vec::new();
    for _ in 0..ownership.depth + 2 {
        partial_scratch.push(allocate_slot(&mut frame, 8, 8, site)?);
    }
    let mut incoming = Vec::new();
    for _ in 0..function.params {
        incoming.push(allocate_slot(&mut frame, 8, 8, site)?);
    }
    let result_slot = if layout.storage(function.result).scalar {
        None
    } else {
        Some(allocate_slot(&mut frame, 8, 8, site)?)
    };
    let frame = frame_extent(frame, site)?;
    let return_label = a.label();
    a.location(site.file, site.line, site.column);
    a.bind(functions[function_index]);
    prologue(a, p, frame);
    if let Some(slot) = result_slot {
        a.store(BP, slot, DI);
    }
    let hidden = if result_slot.is_some() { 1 } else { 0 };
    // Spill incoming words before an aggregate copy can clobber argument regs.
    for index in 0..function.params {
        let abi = index + hidden;
        let register = if abi < 6 {
            parameter_register(abi)
        } else {
            a.load(A, BP, 16 + (abi - 6) as i64 * 8);
            A
        };
        a.store(BP, incoming[index], register);
    }
    for index in 0..function.params {
        let storage = layout.storage(function.locals[index].ty);
        if storage.scalar {
            a.load(A, BP, incoming[index]);
            a.store_width(BP, slots[index], A, storage.bytes as u8);
        } else {
            a.lea(DI, BP, slots[index]);
            a.load(SI, BP, incoming[index]);
            copy_bytes(a, p, storage.bytes);
        }
    }
    ownership.initialize(a, function.params);
    let mut body = Body {
        a,
        program,
        function,
        layout,
        platform: p,
        runtime,
        slots,
        scratch,
        partial_scratch,
        ownership,
        site,
        functions,
        needed,
        drops,
        needed_drops,
        literals,
        messages,
        controls: Vec::new(),
        result_slot,
        return_label,
        source_lines,
    };
    for instruction in &function.body {
        body.emit(instruction)?;
    }
    body.a.bind(return_label);
    epilogue(body.a);
    Ok(())
}
fn emit_primitive(a: &mut Assembler, p: platform::Platform, label: Label, primitive: Primitive) {
    a.bind(label);
    match primitive {
        Primitive::Load8 => {
            a.load_width(A, DI, 0, 1, false);
            a.ret();
        }
        Primitive::Load64 => {
            a.load(A, DI, 0);
            a.ret();
        }
        Primitive::Store8 | Primitive::Store64 => {
            a.store_width(
                DI,
                0,
                SI,
                if primitive == Primitive::Store8 { 1 } else { 8 },
            );
            a.immediate(A, 0);
            a.ret();
        }
        Primitive::MemoryCopy => {
            prologue(a, p, 0);
            a.call(p.copy);
            a.immediate(A, 0);
            epilogue(a);
        }
        Primitive::Map => a.jump(p.map),
        Primitive::Unmap => a.jump(p.unmap),
        Primitive::AllocatorState => {
            a.address(A, p.allocator_state);
            a.ret();
        }
        Primitive::Fail => a.jump(p.heap_exhausted),
    }
}
fn drop_child(
    a: &mut Assembler,
    layout: &Layouts,
    drops: &[Label],
    needed_drops: &mut [bool],
    ty: usize,
) {
    if layout.storage(ty).cleanup {
        needed_drops[ty] = true;
        a.call(drops[ty]);
    }
}
fn emit_drop(
    a: &mut Assembler,
    p: platform::Platform,
    selection: &Selection<'_>,
    ty: usize,
    needed_drops: &mut Vec<bool>,
    needed: &mut Vec<bool>,
) -> Result<(), Diagnostic> {
    let program = selection.program;
    let layout = selection.layout;
    let functions = selection.functions;
    let drops = selection.drops;
    let runtime = selection.runtime;
    a.bind(drops[ty]);
    prologue(a, p, 48);
    a.store(BP, -8, DI);
    if let Some((_, function)) = program.destructors.iter().find(|entry| entry.0 == ty) {
        needed[*function] = true;
        a.call(functions[*function]);
    }
    match &program.types[ty].kind {
        TypeKind::Struct { fields, .. } => {
            for (index, field) in fields.iter().enumerate() {
                if layout.storage(field.ty).cleanup {
                    a.load(DI, BP, -8);
                    a.lea(DI, DI, layout.field_offset(ty, None, index) as i64);
                    drop_child(a, layout, drops, needed_drops, field.ty);
                }
            }
        }
        TypeKind::Enum { variants, .. } => {
            let done = a.label();
            for (variant, value) in variants.iter().enumerate() {
                let next = a.label();
                a.load(DI, BP, -8);
                a.load(A, DI, 0);
                a.immediate(C, variant as u64);
                a.compare(A, C);
                a.condition(5, next);
                for (index, field) in value.fields.iter().enumerate() {
                    if layout.storage(field.ty).cleanup {
                        a.load(DI, BP, -8);
                        a.lea(DI, DI, layout.field_offset(ty, Some(variant), index) as i64);
                        drop_child(a, layout, drops, needed_drops, field.ty);
                    }
                }
                a.jump(done);
                a.bind(next);
            }
            a.bind(done);
        }
        TypeKind::Box { inner } => {
            a.load(DI, BP, -8);
            a.load(DI, DI, 0);
            a.store(BP, -16, DI);
            drop_child(a, layout, drops, needed_drops, *inner);
            a.load(DI, BP, -16);
            a.call(runtime.release);
        }
        TypeKind::Vec { inner } | TypeKind::Array { inner, .. } => {
            a.load(DI, BP, -8);
            match program.types[ty].kind {
                TypeKind::Vec { .. } => {
                    a.load(A, DI, 0);
                    a.store(BP, -16, A);
                    a.load(A, DI, 8);
                    a.store(BP, -24, A);
                }
                TypeKind::Array { len, .. } => {
                    a.store(BP, -16, DI);
                    a.immediate(A, len as u64);
                    a.store(BP, -24, A);
                }
                _ => {}
            }
            if layout.storage(*inner).cleanup {
                a.immediate(A, 0);
                a.store(BP, -32, A);
                let next = a.label();
                let done = a.label();
                a.bind(next);
                a.load(A, BP, -32);
                a.load(C, BP, -24);
                a.compare(A, C);
                a.condition(3, done);
                a.immediate(C, layout.stride(*inner));
                a.imul(A, C);
                a.load(DI, BP, -16);
                a.rr(0x01, DI, A, true);
                drop_child(a, layout, drops, needed_drops, *inner);
                a.load(A, BP, -32);
                a.immediate(C, 1);
                a.rr(0x01, A, C, true);
                a.store(BP, -32, A);
                a.jump(next);
                a.bind(done);
            }
            if matches!(program.types[ty].kind, TypeKind::Vec { .. }) {
                a.load(DI, BP, -16);
                a.call(runtime.release);
            }
        }
        TypeKind::File | TypeKind::Directory => {
            a.load(DI, BP, -8);
            a.load(DI, DI, 0);
            a.call(p.handle_close);
        }
        _ => {}
    }
    a.immediate(A, 0);
    epilogue(a);
    Ok(())
}
fn labels(a: &mut Assembler, count: usize) -> Vec<Label> {
    let mut result = Vec::new();
    for _ in 0..count {
        result.push(a.label());
    }
    result
}
fn messages(program: &Program) -> Vec<String> {
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
    messages
}
pub fn encode(
    checked: &CheckedProgram,
    runtime: &Runtime,
    entry: Entry,
) -> Result<Image, Diagnostic> {
    encode_program(checked.program(), runtime, entry, false)
}
pub fn encode_runtime(runtime: &Runtime, entry: Entry) -> Result<Image, Diagnostic> {
    encode_program(runtime.checked_program(), runtime, entry, true)
}
fn scalar_entry(kind: &TypeKind) -> bool {
    matches!(
        kind,
        TypeKind::Unit | TypeKind::Bool | TypeKind::Char | TypeKind::Int { .. } | TypeKind::F64
    )
}
fn argument_fits(kind: &TypeKind, value: u64) -> bool {
    match kind {
        TypeKind::Unit => value == 0,
        TypeKind::Bool => value <= 1,
        TypeKind::Char => value <= 0x10ffff && !(value >= 0xd800 && value <= 0xdfff),
        TypeKind::Int { bits, signed } => {
            if *bits == 64 {
                true
            } else if *signed {
                ((((value << (64 - *bits)) as i64) >> (64 - *bits)) as u64) == value
            } else {
                value < (1u64 << *bits)
            }
        }
        TypeKind::F64 => {
            value & 0x7ff0000000000000 != 0x7ff0000000000000
                || value & 0x000fffffffffffff == 0
                || value == 0x7ff8000000000000
        }
        _ => false,
    }
}
fn encode_program(
    program: &Program,
    runtime: &Runtime,
    entry: Entry,
    runtime_model: bool,
) -> Result<Image, Diagnostic> {
    let selected = match program.functions.get(entry.function) {
        Some(function) => function,
        None => {
            return Err(Diagnostic::new(
                Span::default(),
                "E_ENTRY",
                "native entry is not a function",
            ));
        }
    };
    let layout = Layouts::new(program)?;
    if !scalar_entry(&program.types[selected.result].kind) {
        return Err(Diagnostic::new(
            first_site(selected),
            "E_ENTRY",
            "native entry result must be a scalar",
        ));
    }
    if selected.params != entry.arguments.len() {
        return Err(Diagnostic::new(
            first_site(selected),
            "E_ENTRY",
            "native entry argument count differs",
        ));
    }
    for index in 0..selected.params {
        let kind = &program.types[selected.locals[index].ty].kind;
        let value = entry.arguments[index] as u64;
        let world =
            !runtime_model && selected.params == 1 && matches!(kind, TypeKind::World) && value == 1;
        if !world && !argument_fits(kind, value) {
            return Err(Diagnostic::new(
                first_site(selected),
                "E_ENTRY",
                "native entry arguments must be valid scalars or one granted World; addresses and derived capabilities cannot be supplied as bits",
            ));
        }
    }
    let mut a = Assembler::new();
    let p = platform::Platform::new(&mut a);
    let functions = labels(&mut a, program.functions.len());
    let drops = labels(&mut a, program.types.len());
    let runtime_functions = if runtime_model {
        functions.clone()
    } else {
        labels(&mut a, runtime.checked_program().functions.len())
    };
    let runtime_labels = RuntimeLabels::new(runtime, &runtime_functions);
    let trap_messages = messages(program);
    let mut literals = BTreeMap::new();
    let mut needed = vec![false; program.functions.len()];
    let mut emitted = vec![false; program.functions.len()];
    let mut needed_drops = vec![false; program.types.len()];
    let mut emitted_drops = vec![false; program.types.len()];
    needed[entry.function] = true;
    if runtime_model {
        for index in 0..needed.len() {
            needed[index] = true;
        }
    }
    platform::emit(&mut a, p, functions[entry.function], &entry);
    {
        let selection = Selection {
            program,
            layout: &layout,
            functions: &functions,
            drops: &drops,
            runtime: runtime_labels,
            messages: &trap_messages,
            source_lines: !runtime_model,
        };
        loop {
            let mut progress = false;
            for index in 0..program.functions.len() {
                if needed[index] && !emitted[index] {
                    emitted[index] = true;
                    progress = true;
                    let primitive = if runtime_model {
                        runtime.primitive(index)
                    } else {
                        None
                    };
                    if let Some(primitive) = primitive {
                        emit_primitive(&mut a, p, functions[index], primitive);
                    } else {
                        emit_function(
                            &mut a,
                            p,
                            &selection,
                            index,
                            &mut needed,
                            &mut needed_drops,
                            &mut literals,
                        )?;
                    }
                }
            }
            for ty in 0..program.types.len() {
                if needed_drops[ty] && !emitted_drops[ty] {
                    emitted_drops[ty] = true;
                    progress = true;
                    emit_drop(&mut a, p, &selection, ty, &mut needed_drops, &mut needed)?;
                }
            }
            if !progress {
                break;
            }
        }
    }
    if !runtime_model {
        let meaning = runtime.checked_program();
        let runtime_layout = Layouts::new(meaning)?;
        let runtime_drops = labels(&mut a, meaning.types.len());
        let mut needed_runtime = vec![true; meaning.functions.len()];
        let mut unused_drops = vec![false; meaning.types.len()];
        let runtime_messages = messages(meaning);
        let selection = Selection {
            program: meaning,
            layout: &runtime_layout,
            functions: &runtime_functions,
            drops: &runtime_drops,
            runtime: runtime_labels,
            messages: &runtime_messages,
            source_lines: false,
        };
        for index in 0..meaning.functions.len() {
            if let Some(primitive) = runtime.primitive(index) {
                emit_primitive(&mut a, p, runtime_functions[index], primitive);
            } else {
                emit_function(
                    &mut a,
                    p,
                    &selection,
                    index,
                    &mut needed_runtime,
                    &mut unused_drops,
                    &mut literals,
                )?;
            }
        }
    }
    let data_offset = (a.bytes.len() + 4095) & !4095;
    let mut data = Vec::new();
    platform::data(&mut a, p, &mut data, data_offset);
    for (bytes, label) in &literals {
        a.data_label(*label, data_offset + data.len());
        data.extend_from_slice(bytes);
    }
    while a.bytes.len() < data_offset {
        a.bytes.push(0);
    }
    a.bytes.extend_from_slice(&data);
    let image = a.finish()?;
    let bytes = platform::elf(image, data_offset)?;
    Ok(Image {
        bytes,
        trap_messages,
    })
}
