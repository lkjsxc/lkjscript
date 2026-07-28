//! Single whole-chunk bytecode validation boundary.

mod control;
mod decode;
mod entry;
mod entry_capabilities;
mod enum_shape;
mod instruction;
mod merge;
mod prelude_shape;
mod shape;
mod shape_products;

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

    pub fn required_capabilities(&self) -> &[crate::CapabilityKind] {
        &self.chunk.required_capabilities
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
    ByteVector(u32),
    ByteSlice {
        owner: u32,
        mutable: bool,
        used: bool,
    },
    Path,
    Capability(crate::CapabilityKind),
    Resource(crate::ResourceKind),
    ResourceResult(crate::ResourceKind),
    Product(ProductId),
    Enum(EnumId, Option<VariantId>),
}

impl std::fmt::Display for Kind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Any => "any",
            Self::Unit => "unit",
            Self::Bool => "bool",
            Self::I64 => "i64",
            Self::F64 => "f64",
            Self::Str => "string",
            Self::Symbol => "symbol",
            Self::Proto(_) => "function-prototype",
            Self::Closure(_) => "function",
            Self::List => "list",
            Self::Buf => "buf",
            Self::ByteVector(_) => "byte-vector",
            Self::ByteSlice { mutable: false, .. } => "byte-slice",
            Self::ByteSlice { mutable: true, .. } => "byte-slice-mut",
            Self::Path => "path",
            Self::Capability(_) => "capability",
            Self::Resource(_) => "resource",
            Self::ResourceResult(_) => "result resource",
            Self::Product(_) => "product",
            Self::Enum(_, _) => "enum",
        };
        formatter.write_str(name)?;
        match self {
            Self::Proto(id) | Self::Closure(id) => write!(formatter, " {id}"),
            Self::Capability(kind) => write!(formatter, " {}", kind.as_str()),
            Self::Resource(kind) | Self::ResourceResult(kind) => {
                write!(formatter, " {}", kind.as_str())
            }
            Self::ByteVector(owner) | Self::ByteSlice { owner, .. } => {
                write!(formatter, " owner {owner}")
            }
            Self::Product(id) => write!(formatter, " {}", id.raw()),
            Self::Enum(_, Some(_)) => formatter.write_str(" variant"),
            Self::Enum(_, None) => Ok(()),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UniquePlaceState {
    Inactive,
    Active {
        owner: Option<u32>,
        transferred: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct State {
    pub(super) stack: Vec<Kind>,
    pub(super) locals: Vec<Option<Kind>>,
    pub(super) globals: Vec<Option<Kind>>,
    pub(super) unique_places: Vec<UniquePlaceState>,
}

pub use entry::validate_chunk;

#[cfg(test)]
mod tests;
