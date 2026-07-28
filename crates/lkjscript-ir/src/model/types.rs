use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Origin {
    pub source: u32,
    pub node: u32,
}

impl Origin {
    pub const SYNTHETIC: Self = Self {
        source: u32::MAX,
        node: u32::MAX,
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Signature {
    pub type_parameters: Vec<String>,
    pub bounds: Vec<TraitBound>,
    pub parameters: Vec<SsaType>,
    pub result: Box<SsaType>,
}

impl Signature {
    pub fn monomorphic(parameters: Vec<SsaType>, result: SsaType) -> Self {
        Self {
            type_parameters: Vec::new(),
            bounds: Vec::new(),
            parameters,
            result: Box::new(result),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SsaType {
    Unit,
    Bool,
    I64,
    F64,
    Str,
    Symbol,
    /// Transitional traced mutable buffer.
    Buf,
    /// Exact affine deterministic byte-vector owner.
    ByteVector,
    /// Exact shared bounded byte-vector view.
    ByteSlice,
    /// Exact exclusive bounded byte-vector view.
    ByteSliceMut,
    Path,
    Capability(lkjscript_contracts::CapabilityKind),
    Resource(lkjscript_contracts::ResourceKind),
    Product(ProductId),
    Enum {
        id: EnumId,
        arguments: Vec<SsaType>,
    },
    List(Box<SsaType>),
    Function(Box<Signature>),
    TypeParameter(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitBound {
    pub parameter: String,
    pub trait_id: TraitId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraitRole {
    Copy,
    Clone,
    Drop,
    Send,
    Sync,
    User,
}

impl TraitRole {
    pub const fn is_auto(self) -> bool {
        matches!(self, Self::Copy | Self::Send | Self::Sync)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitMetadata {
    pub id: TraitId,
    pub name: String,
    pub role: TraitRole,
    pub source: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplMetadata {
    pub id: ImplId,
    pub trait_id: TraitId,
    pub product: ProductId,
    pub source: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeSubstitution {
    pub parameter: String,
    pub ty: SsaType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TraitWitnessKind {
    AutoTrait,
    Explicit(ImplId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitWitness {
    pub trait_id: TraitId,
    pub ty: SsaType,
    pub kind: TraitWitnessKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericInstantiation {
    pub substitutions: Vec<TypeSubstitution>,
    pub witnesses: Vec<TraitWitness>,
}
