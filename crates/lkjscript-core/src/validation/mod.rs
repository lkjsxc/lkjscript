//! Single whole-chunk bytecode validation boundary.

mod control;
mod decode;
mod entry;
mod enum_shape;
mod instruction;
mod merge;
mod shape;

use crate::{Chunk, Constant, DecodedInstruction, EnumId, FunctionProto, ProductId, VariantId};

#[derive(Debug, Clone)]
pub struct ValidatedChunk {
    chunk: Chunk,
    main_instructions: Vec<DecodedInstruction>,
    proto_instructions: Vec<Vec<DecodedInstruction>>,
}

impl ValidatedChunk {
    pub fn constants(&self) -> &[Constant] {
        &self.chunk.constants
    }

    pub fn protos(&self) -> &[FunctionProto] {
        &self.chunk.protos
    }

    pub fn main(&self) -> &FunctionProto {
        &self.chunk.main
    }

    pub fn global_names(&self) -> &[String] {
        &self.chunk.global_names
    }

    pub fn products(&self) -> &[crate::ProductMetadata] {
        &self.chunk.products
    }

    pub fn product_fields(&self) -> &[crate::ProductFieldRef] {
        &self.chunk.product_fields
    }

    pub fn enums(&self) -> &[crate::EnumMetadata] {
        &self.chunk.enums
    }

    pub fn enum_constructions(&self) -> &[crate::EnumConstructionRef] {
        &self.chunk.enum_constructions
    }

    pub fn enum_variants(&self) -> &[crate::EnumVariantRef] {
        &self.chunk.enum_variants
    }

    pub fn enum_fields(&self) -> &[crate::EnumFieldRef] {
        &self.chunk.enum_fields
    }

    pub fn main_instructions(&self) -> &[DecodedInstruction] {
        &self.main_instructions
    }

    pub fn proto_instructions(&self, index: usize) -> Option<&[DecodedInstruction]> {
        self.proto_instructions.get(index).map(Vec::as_slice)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Any,
    Unit,
    Bool,
    I64,
    F64,
    Str,
    Symbol,
    Proto(u32),
    Closure(u32),
    List,
    Buf,
    Handle,
    Result,
    Option,
    Product(ProductId),
    Enum(EnumId, Option<VariantId>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct State {
    pub(super) stack: Vec<Kind>,
    pub(super) locals: Vec<Option<Kind>>,
    pub(super) globals: Vec<Option<Kind>>,
}

pub use entry::validate_chunk;

#[cfg(test)]
mod tests;
