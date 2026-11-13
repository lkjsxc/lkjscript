//! Target-independent instructions after layout and cleanup lowering.
//! Scalar arithmetic is modulo its width; checked source operations contain
//! explicit checks and traps. Calls consume their signature operands in order.
//! Structured branches count enclosing regions; loops branch to their start.
//! Each function ends with End, closing its implicit result region.
//! Addresses are 64-bit offsets into the shared little-endian linear heap.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueType {
    I32,
    I64,
    F64,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Signature {
    pub parameters: Vec<ValueType>,
    pub result: Option<ValueType>,
}
impl Signature {
    pub fn words(parameters: usize) -> Self {
        Self {
            parameters: vec![ValueType::I64; parameters],
            result: Some(ValueType::I64),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScalarOp {
    I32Eqz,
    I32And,
    I32Or,
    I64Eqz,
    I64Eq,
    I64Ne,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,
    I64LeS,
    I64LeU,
    I64GeS,
    I64GeU,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64DivU,
    I64RemS,
    I64RemU,
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrS,
    I64ShrU,
    F64Eq,
    F64Ne,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    I32WrapI64,
    I64ExtendI32U,
    F64ConvertI64S,
    F64ConvertI64U,
    I64ReinterpretF64,
    F64ReinterpretI64,
    I64TruncSatF64S,
    I64TruncSatF64U,
}
impl ScalarOp {
    /// Operand count and types, including the width of a Boolean result.
    pub fn signature(self) -> (usize, ValueType, ValueType) {
        use ValueType::{F64, I32, I64};
        match self {
            Self::I32Eqz => (1, I32, I32),
            Self::I64Eqz => (1, I64, I32),
            Self::I32And | Self::I32Or => (2, I32, I32),
            Self::I64Eq
            | Self::I64Ne
            | Self::I64LtS
            | Self::I64LtU
            | Self::I64GtS
            | Self::I64GtU
            | Self::I64LeS
            | Self::I64LeU
            | Self::I64GeS
            | Self::I64GeU => (2, I64, I32),
            Self::F64Eq | Self::F64Ne | Self::F64Lt | Self::F64Gt | Self::F64Le | Self::F64Ge => {
                (2, F64, I32)
            }
            Self::F64Add | Self::F64Sub | Self::F64Mul | Self::F64Div => (2, F64, F64),
            Self::I32WrapI64 => (1, I64, I32),
            Self::I64ExtendI32U => (1, I32, I64),
            Self::F64ConvertI64S | Self::F64ConvertI64U | Self::F64ReinterpretI64 => (1, I64, F64),
            Self::I64ReinterpretF64 | Self::I64TruncSatF64S | Self::I64TruncSatF64U => {
                (1, F64, I64)
            }
            _ => (2, I64, I64),
        }
    }
}

#[derive(Clone)]
pub enum Instruction {
    I32(i32),
    I64(i64),
    LocalGet(usize),
    LocalSet(usize),
    LocalTee(usize),
    GlobalGet(usize),
    GlobalSet(usize),
    Scalar(ScalarOp),
    /// All linear-memory addresses and loaded values are I64. Narrow signed
    /// loads sign-extend; unsigned loads zero-extend. Stores take the low bits.
    Load {
        bytes: u8,
        signed: bool,
        offset: u64,
    },
    Store {
        bytes: u8,
        offset: u64,
    },
    Drop,
    Block(Option<ValueType>),
    Loop(Option<ValueType>),
    If(Option<ValueType>),
    Else,
    End,
    Branch {
        depth: usize,
        conditional: bool,
    },
    Call(usize),
    /// The table index is I64. An encoder may have a narrower table format and
    /// must diagnose that representation limit before encoding.
    CallIndirect(Signature),
    Return,
    Unreachable,
    /// Logical heap pages are 64 KiB on every route. Growth returns the former
    /// page count, or -1 on host refusal; it introduces no configured maximum.
    MemorySize,
    MemoryGrow,
    /// Consume destination, source and byte count; overlap is permitted.
    MemoryCopy,
}
#[derive(Clone)]
pub struct Code {
    pub locals: Vec<ValueType>,
    pub instructions: Vec<Instruction>,
}
impl Code {
    pub(super) fn words(locals: usize) -> Self {
        Self {
            locals: vec![ValueType::I64; locals],
            instructions: Vec::new(),
        }
    }
    pub(super) fn push(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }
    pub(super) fn extend_from_slice(&mut self, instructions: &[Instruction]) {
        self.instructions.extend_from_slice(instructions);
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RuntimeFunction {
    Allocate,
    Release,
    DropDispatch,
    Utf8Validate,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FunctionRole {
    Source,
    RuntimeSource,
    CopyGlue,
    DropGlue,
    DropStep,
    Runtime(RuntimeFunction),
}
pub struct Function {
    pub name: String,
    pub signature: Signature,
    pub role: FunctionRole,
    pub code: Code,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HostFunction {
    StdoutWrite,
}
pub struct Import {
    pub host: HostFunction,
    pub signature: Signature,
}
pub struct Global {
    pub initial: i64,
    pub mutable: bool,
}
pub struct Export {
    pub name: String,
    pub function: usize,
}
/// Storage of one source value. Aggregates and owned containers use a pointer
/// here; their separate payload layouts are recorded below. Registers retain
/// 64-bit values, so memory width never changes arithmetic or call semantics.
#[derive(Clone, Copy)]
pub struct Storage {
    pub bytes: u8,
    pub signed: bool,
}
pub struct AggregateLayout {
    pub bytes: u64,
    pub alignment: u8,
    pub fields: Vec<u64>,
}
pub struct TypeLayout {
    pub storage: Storage,
    /// A struct has one payload; an enum has one payload for each tag.
    pub payloads: Vec<AggregateLayout>,
}
pub struct Layouts {
    pub types: Vec<TypeLayout>,
}
impl Layouts {
    pub fn storage(&self, ty: usize) -> Storage {
        self.types[ty].storage
    }
    pub fn payload(&self, ty: usize, variant: Option<usize>) -> &AggregateLayout {
        &self.types[ty].payloads[variant.unwrap_or(0)]
    }
}

pub struct Program {
    pub layout: Layouts,
    pub imports: Vec<Import>,
    pub functions: Vec<Function>,
    pub globals: Vec<Global>,
    pub exports: Vec<Export>,
    pub table: Vec<usize>,
    pub data: Vec<u8>,
    pub data_offset: u64,
    pub minimum_memory_pages: u64,
    pub trap_code_global: usize,
    pub trap_line_global: usize,
    pub trap_messages: Vec<String>,
}
