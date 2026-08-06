use crate::{CapabilityKind, ResourceKind};

pub const MAX_SEMANTIC_TYPE_NODES: usize = 16_384;
pub const MAX_SEMANTIC_DECLARATIONS: usize = 16_384;
pub const MAX_SEMANTIC_EDGES: usize = 65_536;
pub const MAX_SEMANTIC_DESCRIPTOR_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticPrimitiveKind {
    Never,
    Unit,
    Bool,
    I64,
    F64,
    String,
    Bytes,
    Path,
    ByteVector,
    ByteSlice,
    ByteSliceMut,
    Symbol,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticType {
    Primitive(SemanticPrimitiveKind),
    Capability(CapabilityKind),
    Resource(ResourceKind),
    Product([u8; 32]),
    Enum {
        identity: [u8; 32],
        arguments: Vec<Self>,
    },
    Parameter(String),
    List(Box<Self>),
    Function {
        parameters: Vec<Self>,
        result: Box<Self>,
    },
    ForAll {
        parameters: Vec<String>,
        body: Box<Self>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticProductField {
    pub identity: [u8; 32],
    pub source_order: u64,
    pub ty: SemanticType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticProductDeclaration {
    pub identity: [u8; 32],
    pub fields: Vec<SemanticProductField>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEnumVariantField {
    pub identity: [u8; 32],
    pub source_order: u64,
    pub ty: SemanticType,
    pub indirect: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEnumVariant {
    pub identity: [u8; 32],
    pub source_order: u64,
    pub fields: Vec<SemanticEnumVariantField>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEnumDeclaration {
    pub identity: [u8; 32],
    pub type_parameters: Vec<String>,
    pub variants: Vec<SemanticEnumVariant>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticDeclaration {
    Product(SemanticProductDeclaration),
    Enum(SemanticEnumDeclaration),
}

impl SemanticDeclaration {
    pub fn identity(&self) -> [u8; 32] {
        match self {
            Self::Product(value) => value.identity,
            Self::Enum(value) => value.identity,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDescriptor {
    pub root: SemanticType,
    /// Exact reachable closure, sorted by stable declaration identity.
    pub declarations: Vec<SemanticDeclaration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticContractError(pub &'static str);

impl std::fmt::Display for SemanticContractError {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.write_str(self.0)
    }
}

impl std::error::Error for SemanticContractError {}
