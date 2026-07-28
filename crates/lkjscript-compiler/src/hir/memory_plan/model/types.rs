#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryType {
    Never,
    Unit,
    Bool,
    I64,
    F64,
    String,
    Buffer,
    Path,
    Capability(CapabilityKind),
    ByteVector,
    ByteSlice,
    ByteSliceMut,
    Symbol,
    Resource(ResourceKind),
    Product(String),
    Enum {
        id: [u8; 32],
        name: String,
        arguments: Vec<Self>,
    },
    TypeParameter(String),
    List(Box<Self>),
    Function {
        parameters: Vec<Self>,
        result: Box<Self>,
    },
    ForAll {
        variables: Vec<String>,
        body: Box<Self>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryParameterMode {
    Copy,
    BorrowShared,
    BorrowExclusive,
    Consume,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryResultMode {
    Trivial,
    Owned,
    Borrowed,
    External,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryBorrowKind {
    Shared,
    Exclusive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryBindingStorage {
    Local,
    Function,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryExpressionKind {
    I64Literal(i64),
    F64Literal(u64),
    BoolLiteral(bool),
    UnitLiteral,
    EmptyList,
    StringLiteral,
    Load {
        binding: u32,
        storage: MemoryBindingStorage,
    },
    Move {
        place: u32,
        binding: u32,
    },
    Borrow {
        place: u32,
        loan: u32,
        kind: MemoryBorrowKind,
        binding: u32,
    },
    DirectCall,
    IndirectCall,
    Operation(u16),
    F64FromI64Exact,
    F64FromI64Rounded,
    I64FromF64Exact,
    I64FromF64Trunc,
    Sequence,
    If,
    While,
    Loop,
    Return,
    Break,
    Continue,
    Trap,
    Exit,
    Let,
    MutableLocal,
    SetLocal,
    ProductValue,
    ProductField,
    WithProductField,
    EnumValue,
    EnumIsVariant,
    EnumField,
    EnumUnwrap,
    MatchUnreachable,
    SymbolLiteral,
}
