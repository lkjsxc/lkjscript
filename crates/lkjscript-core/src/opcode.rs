//! Dense bytecode opcodes and their centralized decode/stack metadata.

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    Nop = 0,
    LoadConst = 1,
    LoadLocal = 2,
    StoreLocal = 3,
    LoadGlobal = 4,
    StoreGlobal = 5,
    Add = 10,
    Sub = 11,
    Mul = 12,
    Div = 13,
    EqualValue = 20,
    Lt = 22,
    Le = 23,
    Gt = 24,
    Ge = 25,
    Not = 26,
    BitAnd = 27,
    BitOr = 28,
    BitXor = 29,
    Jump = 30,
    JumpIfFalse = 31,
    Call = 40,
    Return = 41,
    MakeClosure = 42,
    Cons = 50,
    Car = 51,
    Cdr = 52,
    IsEmptyList = 53,
    SameObject = 54,
    ListEqual = 55,
    F64BitsEqual = 56,
    Print = 60,
    Flush = 61,
    ReadByte = 62,
    WriteByte = 63,
    Exit = 64,
    WriteStr = 65,
    BufNew = 66,
    BufLen = 67,
    BufRef = 68,
    BufSet = 69,
    Pop = 70,
    Dup = 71,
    BufGetU32 = 72,
    BufSetU32 = 73,
    SysTtyGet = 74,
    SysPoll = 75,
    StdinHandle = 76,
    SysIsatty = 77,
    SysTtyGuardSave = 78,
    SysTtyGuardClear = 79,
    False = 80,
    True = 81,
    SysTtySet = 83,
    Unit = 84,
    BufClone = 85,
    EmptyList = 86,
    OptionNone = 87,
    StrLen = 90,
    StrRef = 91,
    StrAppend = 92,
    StrSlice = 93,
    StrFromByte = 94,
    SysOpenRead = 100,
    SysOpenWrite = 101,
    SysClose = 102,
    SysReadByte = 103,
    SysWriteByte = 104,
    Arg = 110,
    Argc = 111,
    EmptyStr = 112,
    SysNowMs = 123,
    SysWaitMs = 124,
    SysSocket = 130,
    SysBind = 131,
    SysListen = 132,
    SysAccept = 133,
    SysPathExists = 134,
    SysRecv = 135,
    SysSend = 136,
    OkWrap = 140,
    ErrWrap = 141,
    IsOk = 142,
    UnwrapOk = 143,
    UnwrapErr = 144,
    StrFromI64 = 146,
    StrFromF64 = 147,
    SomeWrap = 148,
    IsSome = 149,
    UnwrapSome = 150,
    MakeProduct = 151,
    LoadProductField = 152,
    WithProductField = 153,
    Trap = 154,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackEffect {
    Fixed {
        required: usize,
        pops: usize,
        pushes: usize,
    },
    Call,
    MakeProduct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    Next,
    Jump,
    Branch,
    Return,
    Exit,
    Trap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpInfo {
    pub operand_width: usize,
    pub stack: StackEffect,
    pub control: ControlFlow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInstruction {
    offset: usize,
    next_offset: usize,
    op: Op,
    operand: Option<u16>,
}

impl DecodedInstruction {
    pub const fn offset(self) -> usize {
        self.offset
    }

    pub const fn next_offset(self) -> usize {
        self.next_offset
    }

    pub const fn op(self) -> Op {
        self.op
    }

    pub const fn operand(self) -> Option<u16> {
        self.operand
    }

    pub(crate) const fn new(
        offset: usize,
        next_offset: usize,
        op: Op,
        operand: Option<u16>,
    ) -> Self {
        Self {
            offset,
            next_offset,
            op,
            operand,
        }
    }
}

impl Op {
    pub const ALL: &'static [Self] = &[
        Self::Nop,
        Self::LoadConst,
        Self::LoadLocal,
        Self::StoreLocal,
        Self::LoadGlobal,
        Self::StoreGlobal,
        Self::Add,
        Self::Sub,
        Self::Mul,
        Self::Div,
        Self::EqualValue,
        Self::Lt,
        Self::Le,
        Self::Gt,
        Self::Ge,
        Self::Not,
        Self::BitAnd,
        Self::BitOr,
        Self::BitXor,
        Self::Jump,
        Self::JumpIfFalse,
        Self::Call,
        Self::Return,
        Self::MakeClosure,
        Self::Cons,
        Self::Car,
        Self::Cdr,
        Self::IsEmptyList,
        Self::SameObject,
        Self::ListEqual,
        Self::F64BitsEqual,
        Self::Print,
        Self::Flush,
        Self::ReadByte,
        Self::WriteByte,
        Self::Exit,
        Self::WriteStr,
        Self::BufNew,
        Self::BufLen,
        Self::BufRef,
        Self::BufSet,
        Self::Pop,
        Self::Dup,
        Self::BufGetU32,
        Self::BufSetU32,
        Self::SysTtyGet,
        Self::SysPoll,
        Self::StdinHandle,
        Self::SysIsatty,
        Self::SysTtyGuardSave,
        Self::SysTtyGuardClear,
        Self::False,
        Self::True,
        Self::SysTtySet,
        Self::Unit,
        Self::BufClone,
        Self::EmptyList,
        Self::OptionNone,
        Self::StrLen,
        Self::StrRef,
        Self::StrAppend,
        Self::StrSlice,
        Self::StrFromByte,
        Self::SysOpenRead,
        Self::SysOpenWrite,
        Self::SysClose,
        Self::SysReadByte,
        Self::SysWriteByte,
        Self::Arg,
        Self::Argc,
        Self::EmptyStr,
        Self::SysNowMs,
        Self::SysWaitMs,
        Self::SysSocket,
        Self::SysBind,
        Self::SysListen,
        Self::SysAccept,
        Self::SysPathExists,
        Self::SysRecv,
        Self::SysSend,
        Self::OkWrap,
        Self::ErrWrap,
        Self::IsOk,
        Self::UnwrapOk,
        Self::UnwrapErr,
        Self::StrFromI64,
        Self::StrFromF64,
        Self::SomeWrap,
        Self::IsSome,
        Self::UnwrapSome,
        Self::MakeProduct,
        Self::LoadProductField,
        Self::WithProductField,
        Self::Trap,
    ];

    pub fn from_byte(byte: u8) -> Option<Self> {
        Self::ALL.iter().copied().find(|op| *op as u8 == byte)
    }

    pub const fn info(self) -> OpInfo {
        use ControlFlow::{Branch, Exit, Jump, Next, Return, Trap};
        use StackEffect::{Call, Fixed, MakeProduct};

        let operand_width = match self {
            Self::LoadConst
            | Self::LoadGlobal
            | Self::StoreGlobal
            | Self::Jump
            | Self::JumpIfFalse
            | Self::MakeClosure
            | Self::MakeProduct
            | Self::LoadProductField
            | Self::WithProductField
            | Self::Trap => 2,
            Self::LoadLocal | Self::StoreLocal | Self::Call => 1,
            _ => 0,
        };
        let control = match self {
            Self::Jump => Jump,
            Self::JumpIfFalse => Branch,
            Self::Return => Return,
            Self::Exit => Exit,
            Self::Trap => Trap,
            _ => Next,
        };
        let stack = match self {
            Self::Nop | Self::Trap => Fixed {
                required: 0,
                pops: 0,
                pushes: 0,
            },
            Self::LoadConst
            | Self::LoadLocal
            | Self::LoadGlobal
            | Self::Flush
            | Self::ReadByte
            | Self::StdinHandle
            | Self::SysTtyGuardClear
            | Self::False
            | Self::True
            | Self::Unit
            | Self::EmptyList
            | Self::OptionNone
            | Self::Argc
            | Self::EmptyStr
            | Self::SysNowMs
            | Self::SysSocket => Fixed {
                required: 0,
                pops: 0,
                pushes: 1,
            },
            Self::StoreLocal | Self::StoreGlobal => Fixed {
                required: 1,
                pops: 0,
                pushes: 0,
            },
            Self::Add
            | Self::Sub
            | Self::Mul
            | Self::Div
            | Self::EqualValue
            | Self::Lt
            | Self::Le
            | Self::Gt
            | Self::Ge
            | Self::BitAnd
            | Self::BitOr
            | Self::BitXor
            | Self::Cons
            | Self::SameObject
            | Self::ListEqual
            | Self::F64BitsEqual
            | Self::StrRef
            | Self::StrAppend
            | Self::SysWriteByte
            | Self::SysPoll
            | Self::SysTtyGet
            | Self::SysTtySet
            | Self::SysBind
            | Self::SysListen
            | Self::SysSend => Fixed {
                required: 2,
                pops: 2,
                pushes: 1,
            },
            Self::BufRef | Self::BufGetU32 => Fixed {
                required: 2,
                pops: 2,
                pushes: 1,
            },
            Self::StrSlice | Self::BufSet => Fixed {
                required: 3,
                pops: 3,
                pushes: 1,
            },
            Self::BufSetU32 => Fixed {
                required: 3,
                pops: 3,
                pushes: 1,
            },
            Self::Not
            | Self::Car
            | Self::Cdr
            | Self::IsEmptyList
            | Self::Print
            | Self::WriteByte
            | Self::WriteStr
            | Self::BufNew
            | Self::BufLen
            | Self::BufClone
            | Self::SysIsatty
            | Self::SysTtyGuardSave
            | Self::StrLen
            | Self::StrFromByte
            | Self::SysOpenRead
            | Self::SysOpenWrite
            | Self::SysClose
            | Self::SysReadByte
            | Self::Arg
            | Self::SysWaitMs
            | Self::SysAccept
            | Self::SysPathExists
            | Self::SysRecv
            | Self::OkWrap
            | Self::ErrWrap
            | Self::IsOk
            | Self::UnwrapOk
            | Self::UnwrapErr
            | Self::StrFromI64
            | Self::StrFromF64
            | Self::SomeWrap
            | Self::IsSome
            | Self::UnwrapSome
            | Self::LoadProductField => Fixed {
                required: 1,
                pops: 1,
                pushes: 1,
            },
            Self::Jump => Fixed {
                required: 0,
                pops: 0,
                pushes: 0,
            },
            Self::JumpIfFalse | Self::Exit | Self::Pop | Self::Return => Fixed {
                required: 1,
                pops: 1,
                pushes: 0,
            },
            Self::MakeClosure => Fixed {
                required: 1,
                pops: 1,
                pushes: 1,
            },
            Self::Call => Call,
            Self::MakeProduct => MakeProduct,
            Self::Dup => Fixed {
                required: 1,
                pops: 0,
                pushes: 1,
            },
            Self::WithProductField => Fixed {
                required: 2,
                pops: 2,
                pushes: 1,
            },
        };
        OpInfo {
            operand_width,
            stack,
            control,
        }
    }

    pub const fn operand_width(self) -> usize {
        self.info().operand_width
    }
}

#[cfg(test)]
mod tests {
    use super::{ControlFlow, Op, StackEffect};

    #[test]
    fn every_known_opcode_has_truthful_metadata_and_round_trips() {
        let mut seen = [false; 256];
        for op in Op::ALL {
            let byte = *op as u8;
            assert!(!seen[usize::from(byte)]);
            seen[usize::from(byte)] = true;
            assert_eq!(Op::from_byte(byte), Some(*op));
            assert!(op.operand_width() <= 2);
        }
        assert_eq!(Op::from_byte(21), None);
        assert_eq!(Op::from_byte(82), None);
        assert_eq!(Op::from_byte(145), None);
        assert_eq!(Op::from_byte(255), None);
        assert_eq!(Op::Jump.info().control, ControlFlow::Jump);
        assert_eq!(Op::Return.info().control, ControlFlow::Return);
        assert_eq!(Op::Call.info().stack, StackEffect::Call);
        assert_eq!(Op::MakeProduct.info().stack, StackEffect::MakeProduct);
    }
}
