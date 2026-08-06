use super::model::Op;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodedOperand {
    None,
    U16(u16),
    Index(usize),
    PlaceLocal { place: usize, local: usize },
}

impl DecodedOperand {
    pub const fn index(self) -> Option<usize> {
        match self {
            Self::U16(value) => Some(value as usize),
            Self::Index(value) => Some(value),
            Self::None | Self::PlaceLocal { .. } => None,
        }
    }

    pub const fn place_local(self) -> Option<(usize, usize)> {
        match self {
            Self::PlaceLocal { place, local } => Some((place, local)),
            Self::None | Self::U16(_) | Self::Index(_) => None,
        }
    }

    pub const fn is_none(self) -> bool {
        matches!(self, Self::None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInstruction {
    offset: usize,
    next_offset: usize,
    op: Op,
    operand: DecodedOperand,
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

    pub const fn operand(self) -> DecodedOperand {
        self.operand
    }

    pub(crate) const fn new(
        offset: usize,
        next_offset: usize,
        op: Op,
        operand: DecodedOperand,
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
    #[rustfmt::skip]
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
        Self::Pop,
        Self::Dup,
        Self::SysTtyGet,
        Self::SysPoll,
        Self::StdinHandle,
        Self::SysIsatty,
        Self::SysTtyGuardSave,
        Self::SysTtyGuardClear,
        Self::False,
        Self::True,
        Self::MemoryWitnessCompare,
        Self::SysTtySet,
        Self::Unit,
        Self::EmptyList,
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
        Self::StrFromI64,
        Self::StrFromF64,
        Self::MakeProduct,
        Self::LoadProductField,
        Self::WithProductField,
        Self::Trap,
        Self::SysReadInto,
        Self::SysWriteFrom,
        Self::ConvertStringToBytes,
        Self::ConvertBytesToString,
        Self::SysOpenAppend,
        Self::SysOpenCreateNew,
        Self::SysOpenDir,
        Self::SysFsync,
        Self::SysTruncate,
        Self::SysRename,
        Self::SysRandomFill,
        Self::SysSha256,
        Self::SysSqliteOpen,
        Self::SysSqliteClose,
        Self::SysSqliteBusyTimeout,
        Self::SysSqliteExec,
        Self::SysSqlitePrepare,
        Self::SysSqliteFinalize,
        Self::SysSqliteReset,
        Self::SysSqliteClearBindings,
        Self::SysSqliteBindNull,
        Self::SysSqliteBindI64,
        Self::SysSqliteBindF64,
        Self::SysSqliteBindText,
        Self::SysSqliteBindBytes,
        Self::SysSqliteStep,
        Self::SysSqliteColumnCount,
        Self::SysSqliteColumnType,
        Self::SysSqliteColumnI64,
        Self::SysSqliteColumnF64,
        Self::SysSqliteColumnText,
        Self::SysSqliteColumnBytes,
        Self::SysSqliteChanges,
        Self::SysSqliteLastInsertRowid,
        Self::SysSqliteExtendedResultCode,
        Self::SysSqliteBackup,
        Self::MakeEnum,
        Self::IsEnumVariant,
        Self::LoadEnumField,
        Self::F64FromI64Exact,
        Self::F64FromI64Rounded,
        Self::I64FromF64Exact,
        Self::I64FromF64Trunc,
        Self::PathFromStr,
        Self::PathFromBytes,
        Self::PathToBytes,
        Self::PathToStr,
        Self::ByteVectorNew, Self::ByteVectorPlaceInit, Self::ByteVectorMove,
        Self::ByteVectorBorrow, Self::ByteVectorBorrowMut,
        Self::StoreUniqueLocal, Self::StoreViewLocal, Self::TakeUniqueLocal,
        Self::LoadViewLocal, Self::ByteVectorDropPlace,
        Self::ByteSliceLen, Self::ByteSliceRef, Self::ByteSliceMutSet,
        Self::EndBorrowLocal, Self::ByteVectorPlaceEnd,
        Self::BytesLength, Self::BytesByteAt, Self::CopyBytesSlice,
        Self::CloneBytes, Self::FreezeByteVector, Self::ThawBytes,
        Self::BytesDropPlace, Self::BytesPlaceEnd, Self::BytesPlaceInit,
        Self::BytesMove, Self::BytesBorrow,
        Self::ByteSliceReadU32Le, Self::ByteSliceMutWriteU32Le,
        Self::StoreStructuralLocal, Self::TakeStructuralLocal,
        Self::LoadStructuralViewLocal, Self::EndStructuralBorrowLocal,
        Self::LoadStructuralOwnerLocal, Self::StructuralPlaceInit,
        Self::StructuralMove, Self::StructuralDropPlace, Self::StructuralPlaceEnd,
        Self::StructuralBorrow, Self::StructuralBorrowMut, Self::StructuralPublish,
        Self::StructuralDestinationCreate, Self::StructuralDestinationFieldInit,
        Self::StructuralDestinationFinish, Self::StructuralDestinationAbort,
        Self::StructuralAggregateFieldBorrow, Self::StructuralAggregateTag,
        Self::StructuralAggregateConsumePayload, Self::StructuralStringUtf8View,
        Self::StructuralCopy, Self::ResourceDrop, Self::StructuralAggregateFieldCopy,
        Self::MemoryWitnessIndependentOwner, Self::MemoryWitnessDispose,
    ];

    pub fn from_byte(byte: u8) -> Option<Self> {
        Self::ALL.iter().copied().find(|op| *op as u8 == byte)
    }
}
