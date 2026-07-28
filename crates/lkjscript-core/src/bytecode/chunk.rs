//! Bytecode chunk and function prototypes.

mod failure;
pub use failure::*;

use crate::opcode::Op;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProductId(u16);

impl ProductId {
    pub const fn new(raw: u16) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u16 {
        self.0
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductMetadata {
    pub id: ProductId,
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProductFieldRef {
    pub product: ProductId,
    pub field: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstId(pub u16);

#[derive(Debug, Clone)]
pub enum Constant {
    I64(i64),
    F64(f64),
    Str(String),
    StaticBytes(Box<[u8]>),
    Symbol(String),
    /// Prototype index for MakeClosure.
    Proto(u32),
}

#[derive(Debug, Clone)]
pub struct FunctionProto {
    pub name: String,
    pub arity: u8,
    pub locals: u8,
    pub parameter_resources: Vec<Option<crate::ResourceKind>>,
    pub return_resource: Option<ResourceReturnKind>,
    pub parameter_uniques: Vec<Option<UniqueValueKind>>,
    pub parameter_unique_places: Vec<Option<u8>>,
    pub return_unique: Option<UniqueValueKind>,
    pub unique_places: u8,
    pub failure_cleanups: Vec<FailureCleanupPlan>,
    pub failure_cleanup_ranges: Vec<FailureCleanupRange>,
    pub code: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub constants: Vec<Constant>,
    pub protos: Vec<FunctionProto>,
    pub main: FunctionProto,
    pub required_capabilities: Vec<crate::CapabilityKind>,
    pub global_names: Vec<String>,
    pub global_prototypes: Vec<Option<u32>>,
    pub products: Vec<ProductMetadata>,
    pub product_fields: Vec<ProductFieldRef>,
    pub enums: Vec<crate::EnumMetadata>,
    pub enum_constructions: Vec<crate::EnumConstructionRef>,
    pub enum_variants: Vec<crate::EnumVariantRef>,
    pub enum_fields: Vec<crate::EnumFieldRef>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            protos: Vec::new(),
            main: FunctionProto {
                name: "<main>".into(),
                arity: 0,
                locals: 0,
                parameter_resources: Vec::new(),
                return_resource: None,
                parameter_uniques: Vec::new(),
                parameter_unique_places: Vec::new(),
                return_unique: None,
                unique_places: 0,
                failure_cleanups: Vec::new(),
                failure_cleanup_ranges: Vec::new(),
                code: Vec::new(),
            },
            required_capabilities: Vec::new(),
            global_names: Vec::new(),
            global_prototypes: Vec::new(),
            products: Vec::new(),
            product_fields: Vec::new(),
            enums: Vec::new(),
            enum_constructions: Vec::new(),
            enum_variants: Vec::new(),
            enum_fields: Vec::new(),
        }
    }

    pub fn add_const(&mut self, c: Constant) -> ConstId {
        let id = self.constants.len() as u16;
        self.constants.push(c);
        ConstId(id)
    }

    pub fn intern_global(&mut self, name: &str) -> u16 {
        if let Some((i, _)) = self
            .global_names
            .iter()
            .enumerate()
            .find(|(_, n)| n.as_str() == name)
        {
            return i as u16;
        }
        let id = self.global_names.len() as u16;
        self.global_names.push(name.to_string());
        self.global_prototypes.push(None);
        id
    }
}

impl FunctionProto {
    pub fn emit(&mut self, op: Op) {
        self.code.push(op as u8);
    }

    pub fn emit_u8(&mut self, b: u8) {
        self.code.push(b);
    }

    pub fn emit_u16(&mut self, n: u16) {
        self.code.extend_from_slice(&n.to_le_bytes());
    }

    pub fn emit_op_u16(&mut self, op: Op, n: u16) {
        self.emit(op);
        self.emit_u16(n);
    }

    pub fn emit_op_u8(&mut self, op: Op, n: u8) {
        self.emit(op);
        self.emit_u8(n);
    }

    pub fn len(&self) -> usize {
        self.code.len()
    }

    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }

    pub fn patch_u16(&mut self, at: usize, n: u16) {
        let bytes = n.to_le_bytes();
        self.code[at] = bytes[0];
        self.code[at + 1] = bytes[1];
    }
}
