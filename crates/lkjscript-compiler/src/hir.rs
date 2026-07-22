//! Owned, resolved, and typed high-level intermediate representation.

use std::path::PathBuf;

pub use lkjscript_core::ProductId;

pub use crate::operation::Operation;
pub use crate::types::Type;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    Function,
    MutableGlobalValue,
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
    pub forms: Vec<TopLevel>,
    /// Runtime global slots in deterministic bytecode layout order.
    pub global_layout: Vec<BindingId>,
    pub main_locals: u8,
}

impl Program {
    pub fn binding(&self, id: BindingId) -> Option<&Binding> {
        id.index().and_then(|index| self.bindings.get(index))
    }
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
pub enum TopLevel {
    Function(Function),
    Value(ValueDefinition),
    Do { origin: SourceId, expression: Expr },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub binding: BindingId,
    pub origin: SourceId,
    pub params: Vec<BindingId>,
    pub arity: u8,
    pub local_count: u8,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueDefinition {
    pub binding: BindingId,
    pub origin: SourceId,
    pub value: Expr,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectSet(u16);

impl EffectSet {
    pub const PURE: Self = Self(0);
    pub const ALLOCATES: Self = Self(1 << 0);
    pub const READS_MEMORY: Self = Self(1 << 1);
    pub const WRITES_MEMORY: Self = Self(1 << 2);
    pub const HOST_IO: Self = Self(1 << 3);
    pub const MAY_TRAP: Self = Self(1 << 4);
    pub const MAY_EXIT: Self = Self(1 << 5);
    pub const MAY_DIVERGE: Self = Self(1 << 6);
    pub const CONSERVATIVE_CALL: Self = Self::ALLOCATES
        .union(Self::READS_MEMORY)
        .union(Self::WRITES_MEMORY)
        .union(Self::HOST_IO)
        .union(Self::MAY_TRAP)
        .union(Self::MAY_EXIT)
        .union(Self::MAY_DIVERGE);

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub ty: Type,
    pub effects: EffectSet,
    pub origin: SourceId,
    pub kind: ExprKind,
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
    Load(BindingId),
    Call {
        callee: BindingId,
        args: Vec<Expr>,
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
    SetGlobal {
        target: BindingId,
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
    pub slot: u8,
    pub value: Expr,
}
