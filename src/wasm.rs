//! WebAssembly 3.0 memory64 encoding. The checked K program is the input.
use crate::k::{CheckedProgram, Function, Op, Place, Program, Projection, TypeKind};

fn uleb(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 127) as u8;
        value >>= 7;
        out.push(if value == 0 { byte } else { byte | 128 });
        if value == 0 {
            break;
        }
    }
}
fn sleb(out: &mut Vec<u8>, mut value: i64) {
    loop {
        let byte = (value & 127) as u8;
        value >>= 7;
        let done = (value == 0 && byte & 64 == 0) || (value == -1 && byte & 64 != 0);
        out.push(if done { byte } else { byte | 128 });
        if done {
            break;
        }
    }
}
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
const FIRST_USER: usize = 3;

struct Body<'a> {
    program: &'a Program,
    function: &'a Function,
    bytes: Vec<u8>,
    cells: Vec<Option<usize>>,
    scratch: usize,
    controls: Vec<u8>,
    messages: &'a [String],
}
impl<'a> Body<'a> {
    fn read(&mut self, local: usize) {
        match self.cells[local] {
            Some(cell) => {
                get(&mut self.bytes, cell);
                load(&mut self.bytes, 0);
            }
            None => get(&mut self.bytes, local),
        }
    }
    fn write(&mut self, local: usize) {
        match self.cells[local] {
            Some(cell) => {
                set(&mut self.bytes, self.scratch + 7);
                get(&mut self.bytes, cell);
                get(&mut self.bytes, self.scratch + 7);
                store(&mut self.bytes, 0);
            }
            None => set(&mut self.bytes, local),
        }
    }
    fn address(&mut self, place: &Place, line: usize) -> usize {
        let mut ty = self.function.locals[place.local].ty;
        if place.projections.is_empty() {
            get(
                &mut self.bytes,
                self.cells[place.local].expect("address-taken local has a cell"),
            );
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
                        TypeKind::String => crate::k::U8,
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
                for cell in &self.cells {
                    if let Some(cell) = cell {
                        get(&mut self.bytes, *cell);
                        call(&mut self.bytes, FREE);
                        self.bytes.push(0x1a);
                    }
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
            | TypeKind::String
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
fn field_operations(
    bytes: &mut Vec<u8>,
    program: &Program,
    fields: &[crate::k::Field],
    first: u64,
    copying: bool,
) {
    let mut i = 0;
    while i < fields.len() {
        let offset = (first + i as u64) * 8;
        if copying {
            get(bytes, 1);
        }
        get(bytes, 0);
        load(bytes, offset);
        call(bytes, helper_index(program, fields[i].ty, copying));
        if copying {
            store(bytes, offset);
        } else {
            bytes.push(0x1a);
        }
        i += 1;
    }
}

fn type_helper(program: &Program, ty: usize, copying: bool, destructor: Option<usize>) -> Vec<u8> {
    let mut b = vec![1, 5, 0x7e];
    let kind = &program.types[ty].kind;
    if !heap_type(kind) {
        if copying {
            get(&mut b, 0);
        } else {
            constant(&mut b, 0);
        }
        b.push(0x0b);
        return b;
    }
    get(&mut b, 0);
    b.push(0x50);
    begin_if(&mut b);
    constant(&mut b, 0);
    b.push(0x0f);
    b.push(0x0b);
    if !copying {
        if let Some(function) = destructor {
            constant(&mut b, 8);
            call(&mut b, ALLOC);
            set(&mut b, 5);
            get(&mut b, 5);
            get(&mut b, 0);
            store(&mut b, 0);
            get(&mut b, 5);
            call(&mut b, FIRST_USER + function);
            b.push(0x1a);
            get(&mut b, 5);
            load(&mut b, 0);
            set(&mut b, 0);
            get(&mut b, 5);
            call(&mut b, FREE);
            b.push(0x1a);
        }
    }

    match kind {
        TypeKind::Struct { fields, .. } => {
            if copying {
                constant(&mut b, fields.len().max(1) as i64 * 8);
                call(&mut b, ALLOC);
                set(&mut b, 1);
            }
            field_operations(&mut b, program, fields, 0, copying);
        }
        TypeKind::Enum { variants, .. } => {
            get(&mut b, 0);
            load(&mut b, 0);
            set(&mut b, 2);
            let mut variant = 0;
            while variant < variants.len() {
                get(&mut b, 2);
                constant(&mut b, variant as i64);
                b.push(0x51);
                begin_if(&mut b);
                let fields = &variants[variant].fields;
                if copying {
                    constant(&mut b, (fields.len() + 1) as i64 * 8);
                    call(&mut b, ALLOC);
                    set(&mut b, 1);
                    get(&mut b, 1);
                    get(&mut b, 2);
                    store(&mut b, 0);
                }
                field_operations(&mut b, program, fields, 1, copying);
                b.push(0x0b);
                variant += 1;
            }
        }
        TypeKind::Box { inner } => {
            if copying {
                constant(&mut b, 8);
                call(&mut b, ALLOC);
                tee(&mut b, 1);
            }
            get(&mut b, 0);
            load(&mut b, 0);
            call(&mut b, helper_index(program, *inner, copying));
            if copying {
                store(&mut b, 0);
            } else {
                b.push(0x1a);
            }
        }
        TypeKind::Vec { .. } | TypeKind::Array { .. } | TypeKind::String => {
            let inner = match kind {
                TypeKind::Vec { inner } | TypeKind::Array { inner, .. } => *inner,
                _ => crate::k::U8,
            };
            get(&mut b, 0);
            load(&mut b, 0);
            set(&mut b, 2);
            get(&mut b, 0);
            load(&mut b, 8);
            set(&mut b, 3);
            if copying {
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
            }
            constant(&mut b, 0);
            set(&mut b, 4);
            b.extend_from_slice(&[0x02, 0x40, 0x03, 0x40]);
            get(&mut b, 4);
            get(&mut b, 3);
            b.push(0x5a);
            b.extend_from_slice(&[0x0d, 1]);
            if copying {
                get(&mut b, 5);
                get(&mut b, 4);
                constant(&mut b, 8);
                b.push(0x7e);
                b.push(0x7c);
            }
            get(&mut b, 2);
            get(&mut b, 4);
            constant(&mut b, 8);
            b.push(0x7e);
            b.push(0x7c);
            load(&mut b, 0);
            call(&mut b, helper_index(program, inner, copying));
            if copying {
                store(&mut b, 0);
            } else {
                b.push(0x1a);
            }
            get(&mut b, 4);
            constant(&mut b, 1);
            b.push(0x7c);
            set(&mut b, 4);
            b.extend_from_slice(&[0x0c, 0, 0x0b, 0x0b]);
            if !copying {
                get(&mut b, 2);
                call(&mut b, FREE);
                b.push(0x1a);
            }
        }
        TypeKind::Slice { .. } => {
            if copying {
                constant(&mut b, 16);
                call(&mut b, ALLOC);
                set(&mut b, 1);
                get(&mut b, 1);
                get(&mut b, 0);
                constant(&mut b, 16);
                b.extend_from_slice(&[0xfc, 10, 0, 0]);
            }
        }
        _ => unreachable!(),
    }
    if copying {
        get(&mut b, 1);
    } else {
        get(&mut b, 0);
        call(&mut b, FREE);
        b.push(0x1a);
        constant(&mut b, 0);
    }
    b.push(0x0b);
    b
}

fn function_body(
    program: &Program,
    function: &Function,
    messages: &[String],
) -> Result<Vec<u8>, String> {
    let n = function.locals.len();
    if n > u32::MAX as usize - 8 {
        return Err("WebAssembly local indices exceed the u32 representation".to_string());
    }
    let mut cells = vec![None; n];
    let mut extra = 8;
    for instruction in &function.body {
        if let Op::Borrow { place, .. } | Op::SliceBorrow { place, .. } = &instruction.op {
            if place.projections.is_empty() && cells[place.local].is_none() {
                cells[place.local] = Some(n + extra);
                extra += 1;
            }
        }
    }
    if n + extra > u32::MAX as usize {
        return Err("WebAssembly local indices exceed the u32 representation".to_string());
    }
    let mut body = Body {
        program,
        function,
        bytes: Vec::new(),
        cells,
        scratch: n,
        controls: Vec::new(),
        messages,
    };
    uleb(&mut body.bytes, 1);
    uleb(&mut body.bytes, (n - function.params + extra) as u64);
    body.bytes.push(0x7e);
    let mut i = 0;
    while i < n {
        if let Some(cell) = body.cells[i] {
            constant(&mut body.bytes, 8);
            call(&mut body.bytes, ALLOC);
            set(&mut body.bytes, cell);
            get(&mut body.bytes, cell);
            get(&mut body.bytes, i);
            store(&mut body.bytes, 0);
        }
        i += 1;
    }
    for instruction in &function.body {
        body.emit(&instruction.op, instruction.line)?;
    }
    constant(&mut body.bytes, 0);
    body.bytes.push(0x0b);
    Ok(body.bytes)
}
pub fn emit(checked: &CheckedProgram) -> Result<Vec<u8>, String> {
    let program = checked.program();
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
    if program.functions.len() as u64 + program.types.len() as u64 * 2 + FIRST_USER as u64
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
        (program.functions.len() + 2 + program.types.len() * 2) as u64,
    );
    uleb(&mut functions, 1);
    uleb(&mut functions, 1);
    for f in &program.functions {
        let mut index = 0;
        while arities[index] != f.params {
            index += 1;
        }
        uleb(&mut functions, (index + 1) as u64);
    }
    for _ in 0..program.types.len() * 2 {
        uleb(&mut functions, 1);
    }
    section(&mut out, 3, &functions)?;
    section(&mut out, 5, &[1, 4, 1])?;
    let mut globals = vec![2, 0x7e, 1];
    constant(&mut globals, 8);
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
    let mut code = Vec::new();
    uleb(
        &mut code,
        (program.functions.len() + 2 + program.types.len() * 2) as u64,
    );
    let alloc = allocator();
    uleb(&mut code, alloc.len() as u64);
    code.extend_from_slice(&alloc);
    let free = deallocator();
    uleb(&mut code, free.len() as u64);
    code.extend_from_slice(&free);
    for f in &program.functions {
        let body = function_body(program, f, &messages)?;
        uleb(&mut code, body.len() as u64);
        code.extend_from_slice(&body);
    }
    for ty in 0..program.types.len() {
        for copying in [false, true] {
            let body = type_helper(program, ty, copying, destructors[ty]);
            uleb(&mut code, body.len() as u64);
            code.extend_from_slice(&body);
        }
    }
    section(&mut out, 10, &code)?;
    let mut names = Vec::new();
    uleb(
        &mut names,
        (program.functions.len() + FIRST_USER + program.types.len() * 2) as u64,
    );
    uleb(&mut names, 0);
    text(&mut names, "trap");
    uleb(&mut names, ALLOC as u64);
    text(&mut names, "memory::allocate");
    uleb(&mut names, FREE as u64);
    text(&mut names, "memory::release");
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
