use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoreTrait {
    Copy,
    Clone,
    Drop,
    Send,
    Sync,
}

impl CoreTrait {
    pub const ALL: [Self; 5] = [Self::Copy, Self::Clone, Self::Drop, Self::Send, Self::Sync];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Copy => "Copy",
            Self::Clone => "Clone",
            Self::Drop => "Drop",
            Self::Send => "Send",
            Self::Sync => "Sync",
        }
    }

    pub const fn is_auto(self) -> bool {
        matches!(self, Self::Copy | Self::Send | Self::Sync)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitDefinition {
    pub id: TraitId,
    pub name: String,
    pub origin: Origin,
    pub core: Option<CoreTrait>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplDefinition {
    pub id: ImplId,
    pub trait_id: TraitId,
    pub product: ProductId,
    pub origin: SourceId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitBound {
    pub parameter: String,
    pub trait_id: TraitId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSubstitution {
    pub parameter: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraitWitnessKind {
    AutoTrait,
    Explicit(ImplId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitWitness {
    pub trait_id: TraitId,
    pub ty: Type,
    pub kind: TraitWitnessKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericInstantiation {
    pub substitutions: Vec<TypeSubstitution>,
    pub witnesses: Vec<TraitWitness>,
}
