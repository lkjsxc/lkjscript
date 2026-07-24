//! Owned, resolved, and typed high-level intermediate representation.

use std::path::PathBuf;

pub use lkjscript_core::ProductId;

pub use crate::operation::Operation;
pub use crate::types::Type;

macro_rules! dense_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u32);

        impl $name {
            pub(crate) const fn new(raw: u32) -> Self {
                Self(raw)
            }

            pub const fn raw(self) -> u32 {
                self.0
            }

            #[allow(dead_code)]
            pub(crate) fn index(self) -> Option<usize> {
                usize::try_from(self.0).ok()
            }
        }
    };
}

dense_id!(TraitId);
dense_id!(ImplId);
dense_id!(PlaceId);
dense_id!(LoanId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(u32);

impl SourceId {
    pub(crate) const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub(crate) fn index(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindingId(u32);

impl BindingId {
    pub(crate) const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub(crate) fn index(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Source(SourceId),
    Builtin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingKind {
    Parameter,
    ImmutableLocal,
    MutableLocal,
    Function,
    BuiltinOperation(Operation),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub id: BindingId,
    pub name: String,
    pub kind: BindingKind,
    pub ty: Type,
    pub origin: Origin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub id: SourceId,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub sources: Vec<Source>,
    pub bindings: Vec<Binding>,
    pub products: Vec<ProductDefinition>,
    pub traits: Vec<TraitDefinition>,
    pub implementations: Vec<ImplDefinition>,
    pub functions: Vec<Function>,
    pub main: Main,
    /// Internal function-closure slots in deterministic bytecode layout order.
    pub global_layout: Vec<BindingId>,
}

impl Program {
    pub fn binding(&self, id: BindingId) -> Option<&Binding> {
        id.index().and_then(|index| self.bindings.get(index))
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductDefinition {
    pub id: ProductId,
    pub name: String,
    pub origin: SourceId,
    pub fields: Vec<ProductField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductField {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Main {
    pub origin: SourceId,
    pub return_type: Type,
    pub local_count: u8,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub binding: BindingId,
    pub origin: SourceId,
    pub params: Vec<BindingId>,
    pub param_places: Vec<PlaceId>,
    pub bounds: Vec<TraitBound>,
    pub arity: u8,
    pub local_count: u8,
    pub summary: EffectSet,
    pub body: Expr,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectSet(u16);

impl EffectSet {
    pub const PURE: Self = Self(0);
    pub const ALLOCATES: Self = Self(1 << 0);
    pub const READS_MEMORY: Self = Self(1 << 1);
    pub const WRITES_MEMORY: Self = Self(1 << 2);
    pub const MUTATES_LOCAL: Self = Self(1 << 3);
    pub const HOST_IO: Self = Self(1 << 4);
    pub const MAY_TRAP: Self = Self(1 << 5);
    pub const MAY_EXIT: Self = Self(1 << 6);
    pub const MAY_DIVERGE: Self = Self(1 << 7);
    pub const CONSERVATIVE_CALL: Self = Self::ALLOCATES
        .union(Self::READS_MEMORY)
        .union(Self::WRITES_MEMORY)
        .union(Self::MUTATES_LOCAL)
        .union(Self::HOST_IO)
        .union(Self::MAY_TRAP)
        .union(Self::MAY_EXIT)
        .union(Self::MAY_DIVERGE);

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn contains(self, effects: Self) -> bool {
        self.0 & effects.0 == effects.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub ty: Type,
    pub effects: EffectSet,
    pub origin: SourceId,
    pub kind: ExprKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Shared,
    Mutable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    LitI64(i64),
    LitF64(f64),
    LitBool(bool),
    LitUnit,
    EmptyList,
    LitNone,
    LitStr(String),
    Load(BindingRef),
    Move {
        place: PlaceId,
        binding: BindingRef,
    },
    Borrow {
        place: PlaceId,
        loan: LoanId,
        kind: BorrowKind,
        binding: BindingRef,
    },
    Call {
        callee: BindingRef,
        args: Vec<Expr>,
        instantiation: Option<GenericInstantiation>,
    },
    Operation {
        binding: BindingId,
        operation: Operation,
        resolved_signature: Type,
        args: Vec<Expr>,
    },
    Do(Vec<Expr>),
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    While {
        condition: Box<Expr>,
        body: Vec<Expr>,
    },
    Let {
        bindings: Vec<LocalDefinition>,
        body: Box<Expr>,
    },
    MutableLocal {
        binding: BindingId,
        place: PlaceId,
        slot: u8,
        initial: Box<Expr>,
        body: Box<Expr>,
    },
    SetLocal {
        target: BindingId,
        slot: u8,
        value: Box<Expr>,
    },
    ProductValue {
        product: ProductId,
        fields: Vec<Expr>,
    },
    ProductField {
        product: ProductId,
        field: u8,
        value: Box<Expr>,
    },
    WithProductField {
        product: ProductId,
        field: u8,
        value: Box<Expr>,
        replacement: Box<Expr>,
    },
    QuoteSymbol(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalDefinition {
    pub binding: BindingId,
    pub place: PlaceId,
    pub slot: u8,
    pub value: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingRef {
    pub binding: BindingId,
    pub storage: BindingStorage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingStorage {
    Local(u8),
    Function,
}
