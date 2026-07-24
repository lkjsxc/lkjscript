//! Backend-independent typed SSA, verification, evaluation, and baseline passes.
#![forbid(unsafe_code)]

mod eval;
mod passes;
mod verify;

#[cfg(test)]
mod tests;

use std::fmt;

pub use eval::{evaluate, EvalConfig, EvalOutcome, EvalValue};
pub use passes::{
    canonical_block_order, constant_fold_and_propagate, copy_propagate, direct_call_resolution,
    effect_aware_dce, empty_block_forwarding, normalize_baseline, simplify_branches,
    unreachable_blocks,
};
pub use verify::{verify, VerifiedProgram};

macro_rules! dense_id {
    ($name:ident, $raw:ty) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($raw);

        impl $name {
            pub const fn new(raw: $raw) -> Self {
                Self(raw)
            }

            pub const fn raw(self) -> $raw {
                self.0
            }

            pub fn index(self) -> Option<usize> {
                usize::try_from(self.0).ok()
            }
        }
    };
}

dense_id!(FunctionId, u32);
dense_id!(BlockId, u32);
dense_id!(ValueId, u32);
dense_id!(ProductId, u16);
dense_id!(BindingId, u32);

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
    pub parameters: Vec<SsaType>,
    pub result: Box<SsaType>,
}

impl Signature {
    pub fn monomorphic(parameters: Vec<SsaType>, result: SsaType) -> Self {
        Self {
            type_parameters: Vec::new(),
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
    Buf,
    Handle,
    Product(ProductId),
    List(Box<SsaType>),
    Option(Box<SsaType>),
    Result(Box<SsaType>, Box<SsaType>),
    Function(Box<Signature>),
    TypeParameter(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMetadata {
    pub id: u32,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductMetadata {
    pub id: ProductId,
    pub name: String,
    pub fields: Vec<ProductField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductField {
    pub name: String,
    pub ty: SsaType,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
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

    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, effects: Self) -> bool {
        self.0 & effects.0 == effects.0
    }

    pub const fn is_pure(self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Unit,
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    Symbol(String),
    EmptyList,
    None,
}

impl Constant {
    pub fn ty(&self, declared: &SsaType) -> bool {
        matches!(
            (self, declared),
            (Self::Unit, SsaType::Unit)
                | (Self::Bool(_), SsaType::Bool)
                | (Self::I64(_), SsaType::I64)
                | (Self::F64(_), SsaType::F64)
                | (Self::Str(_), SsaType::Str)
                | (Self::Symbol(_), SsaType::Symbol)
                | (Self::EmptyList, SsaType::List(_))
                | (Self::None, SsaType::Option(_))
        )
    }
}

/// Canonical runtime identities. Names are compiler/source concerns, not SSA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    EqualValue,
    SameObject,
    ListEqual,
    F64BitsEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Not,
    Cons,
    Car,
    Cdr,
    IsEmptyList,
    Print,
    Flush,
    ReadByte,
    WriteByte,
    BitAnd,
    BitOr,
    BitXor,
    WriteStr,
    EmptyStr,
    ArgCount,
    Arg,
    BufNew,
    BufLen,
    BufRef,
    BufSet,
    BufClone,
    BufGetU32,
    BufSetU32,
    StrLen,
    StrRef,
    StrAppend,
    StrSlice,
    StrFromByte,
    StrFromI64,
    StrFromF64,
    StdinHandle,
    SysIsatty,
    SysClose,
    SysReadByte,
    SysWriteByte,
    SysTtyGuardSave,
    SysTtyGuardClear,
    SysOpenRead,
    SysOpenWrite,
    SysPathExists,
    SysWaitMs,
    SysNowMs,
    SysSocket,
    SysBind,
    SysListen,
    SysAccept,
    SysRecv,
    SysSend,
    SysPoll,
    SysTtyGet,
    SysTtySet,
    Ok,
    Err,
    IsOk,
    UnwrapOk,
    UnwrapErr,
    Some,
    IsSome,
    UnwrapSome,
}

impl RuntimeOp {
    pub const fn effects(self) -> EffectSet {
        match self {
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide => EffectSet::MAY_TRAP,
            Self::Cons
            | Self::StrAppend
            | Self::StrFromByte
            | Self::StrFromI64
            | Self::StrFromF64
            | Self::EmptyStr
            | Self::BufNew
            | Self::BufClone
            | Self::Ok
            | Self::Err
            | Self::Some => EffectSet::ALLOCATES.union(EffectSet::MAY_TRAP),
            Self::Car
            | Self::Cdr
            | Self::BufRef
            | Self::BufGetU32
            | Self::StrRef
            | Self::StrSlice
            | Self::UnwrapOk
            | Self::UnwrapErr
            | Self::UnwrapSome => EffectSet::READS_MEMORY.union(EffectSet::MAY_TRAP),
            Self::BufSet | Self::BufSetU32 => EffectSet::WRITES_MEMORY.union(EffectSet::MAY_TRAP),
            Self::BufLen | Self::StrLen | Self::IsOk | Self::IsSome => EffectSet::READS_MEMORY,
            Self::Print
            | Self::Flush
            | Self::ReadByte
            | Self::WriteByte
            | Self::WriteStr
            | Self::ArgCount
            | Self::Arg
            | Self::StdinHandle
            | Self::SysIsatty
            | Self::SysClose
            | Self::SysReadByte
            | Self::SysWriteByte
            | Self::SysTtyGuardSave
            | Self::SysTtyGuardClear
            | Self::SysOpenRead
            | Self::SysOpenWrite
            | Self::SysPathExists
            | Self::SysWaitMs
            | Self::SysNowMs
            | Self::SysSocket
            | Self::SysBind
            | Self::SysListen
            | Self::SysAccept
            | Self::SysRecv
            | Self::SysSend
            | Self::SysPoll
            | Self::SysTtyGet
            | Self::SysTtySet => EffectSet::HOST_IO
                .union(EffectSet::ALLOCATES)
                .union(EffectSet::MAY_TRAP),
            Self::Less
            | Self::LessEqual
            | Self::Greater
            | Self::GreaterEqual
            | Self::Not
            | Self::BitAnd
            | Self::BitOr
            | Self::BitXor
            | Self::IsEmptyList => EffectSet::PURE,
            Self::EqualValue | Self::SameObject | Self::F64BitsEqual => EffectSet::READS_MEMORY,
            Self::ListEqual => EffectSet::READS_MEMORY.union(EffectSet::MAY_TRAP),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Safepoint {
    None,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureBehavior {
    None,
    Trap,
    StructuredOutcome,
    TrapOrOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameLocal {
    pub binding: BindingId,
    pub slot: u16,
    pub value: ValueId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameState {
    /// Stable semantic position linked to an exact bytecode offset after emission.
    pub bytecode_position: u32,
    pub locals: Vec<FrameLocal>,
    pub operand_stack: Vec<ValueId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionMetadata {
    pub origin: Origin,
    pub effects: EffectSet,
    pub safepoint: Safepoint,
    pub failure: FailureBehavior,
    pub frame_state: Option<FrameState>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallTarget {
    Direct(FunctionId),
    Indirect(ValueId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionKind {
    Constant(Constant),
    Copy(ValueId),
    FunctionRef(FunctionId),
    Runtime {
        operation: RuntimeOp,
        arguments: Vec<ValueId>,
        signature: Signature,
    },
    Call {
        target: CallTarget,
        arguments: Vec<ValueId>,
        signature: Signature,
    },
    ProductValue {
        product: ProductId,
        fields: Vec<ValueId>,
    },
    ProductField {
        product: ProductId,
        field: u8,
        value: ValueId,
    },
    WithProductField {
        product: ProductId,
        field: u8,
        value: ValueId,
        replacement: ValueId,
    },
}

impl InstructionKind {
    pub fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::Constant(_) | Self::FunctionRef(_) => Vec::new(),
            Self::Copy(value) => vec![*value],
            Self::Runtime { arguments, .. } | Self::Call { arguments, .. } => arguments.clone(),
            Self::ProductValue { fields, .. } => fields.clone(),
            Self::ProductField { value, .. } => vec![*value],
            Self::WithProductField {
                value, replacement, ..
            } => vec![*value, *replacement],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub id: ValueId,
    pub ty: SsaType,
    pub kind: InstructionKind,
    pub metadata: InstructionMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockParameter {
    pub id: ValueId,
    pub ty: SsaType,
    pub origin: Origin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockMetadata {
    pub loop_header: bool,
    pub origin: Origin,
    pub frame_state: Option<FrameState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuredOutcome {
    DeadlineExceeded,
    ResourceLimitExceeded,
    HostFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Terminator {
    Branch {
        target: BlockId,
        arguments: Vec<ValueId>,
    },
    ConditionalBranch {
        condition: ValueId,
        true_target: BlockId,
        true_arguments: Vec<ValueId>,
        false_target: BlockId,
        false_arguments: Vec<ValueId>,
    },
    Return(ValueId),
    Trap {
        message: String,
    },
    Exit {
        code: ValueId,
    },
    Outcome {
        outcome: StructuredOutcome,
        detail: Option<ValueId>,
    },
}

impl Terminator {
    pub fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::Branch { arguments, .. } => arguments.clone(),
            Self::ConditionalBranch {
                condition,
                true_arguments,
                false_arguments,
                ..
            } => {
                let mut values = Vec::with_capacity(
                    1usize
                        .saturating_add(true_arguments.len())
                        .saturating_add(false_arguments.len()),
                );
                values.push(*condition);
                values.extend(true_arguments);
                values.extend(false_arguments);
                values
            }
            Self::Return(value) | Self::Exit { code: value } => vec![*value],
            Self::Trap { .. } => Vec::new(),
            Self::Outcome { detail, .. } => detail.iter().copied().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub id: BlockId,
    pub parameters: Vec<BlockParameter>,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
    pub metadata: BlockMetadata,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub id: FunctionId,
    pub name: String,
    pub signature: Signature,
    pub effects: EffectSet,
    pub entry: BlockId,
    pub blocks: Vec<Block>,
    pub origin: Origin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub sources: Vec<SourceMetadata>,
    pub products: Vec<ProductMetadata>,
    pub functions: Vec<Function>,
    pub main: FunctionId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeInstructionLink {
    pub value: ValueId,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeBlockLink {
    pub block: BlockId,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionBytecodeLink {
    pub function: FunctionId,
    pub prototype: Option<u32>,
    pub is_main: bool,
    pub blocks: Vec<BytecodeBlockLink>,
    pub instructions: Vec<BytecodeInstructionLink>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeLinkMetadata {
    pub main: FunctionId,
    pub functions: Vec<FunctionBytecodeLink>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrError {
    message: String,
}

impl IrError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for IrError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for IrError {}

pub type Result<T> = std::result::Result<T, IrError>;
