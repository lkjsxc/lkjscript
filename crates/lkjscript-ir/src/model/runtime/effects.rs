use super::*;

impl RuntimeOp {
    pub const fn consumes_affine_arguments(self) -> bool {
        matches!(
            self,
            Self::SysClose | Self::SysSqliteClose | Self::SysSqliteFinalize
        )
    }

    pub const fn effects(self) -> EffectSet {
        match self {
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide => EffectSet::MAY_TRAP,
            Self::CloneBytes
            | Self::CopyBytesSlice
            | Self::ConvertStringToBytes
            | Self::ConvertBytesToString
            | Self::PathFromStr
            | Self::PathFromBytes
            | Self::PathToBytes
            | Self::PathToStr
            | Self::SysSha256 => EffectSet::ALLOCATES
                .union(EffectSet::READS_MEMORY)
                .union(EffectSet::MAY_TRAP),
            Self::Cons
            | Self::StrAppend
            | Self::StrFromByte
            | Self::StrFromI64
            | Self::StrFromF64
            | Self::EmptyStr
            | Self::ByteVectorNew => EffectSet::ALLOCATES.union(EffectSet::MAY_TRAP),
            Self::BytesByteAt
            | Self::Car
            | Self::Cdr
            | Self::ByteSliceByteAt
            | Self::ByteSliceReadU32Le
            | Self::StrRef
            | Self::StrSlice => EffectSet::READS_MEMORY.union(EffectSet::MAY_TRAP),
            Self::FreezeByteVector => EffectSet::READS_MEMORY
                .union(EffectSet::WRITES_MEMORY)
                .union(EffectSet::MAY_TRAP),
            Self::ThawBytes => EffectSet::ALLOCATES
                .union(EffectSet::READS_MEMORY)
                .union(EffectSet::WRITES_MEMORY)
                .union(EffectSet::MAY_TRAP),
            Self::ByteSliceMutSetByte | Self::ByteSliceMutWriteU32Le => {
                EffectSet::WRITES_MEMORY.union(EffectSet::MAY_TRAP)
            }
            Self::BytesLength | Self::ByteSliceLength | Self::StrLen => EffectSet::READS_MEMORY,
            Self::SysReadInto => EffectSet::HOST_IO
                .union(EffectSet::ALLOCATES)
                .union(EffectSet::WRITES_MEMORY)
                .union(EffectSet::MAY_TRAP),
            Self::SysWriteFrom => EffectSet::HOST_IO
                .union(EffectSet::ALLOCATES)
                .union(EffectSet::READS_MEMORY)
                .union(EffectSet::MAY_TRAP),
            Self::SysSqliteOpen
            | Self::SysSqliteClose
            | Self::SysSqliteBusyTimeout
            | Self::SysSqliteExec
            | Self::SysSqlitePrepare
            | Self::SysSqliteFinalize
            | Self::SysSqliteReset
            | Self::SysSqliteClearBindings
            | Self::SysSqliteBindNull
            | Self::SysSqliteBindI64
            | Self::SysSqliteBindF64
            | Self::SysSqliteBindText
            | Self::SysSqliteBindBytes
            | Self::SysSqliteStep
            | Self::SysSqliteColumnCount
            | Self::SysSqliteColumnType
            | Self::SysSqliteColumnI64
            | Self::SysSqliteColumnF64
            | Self::SysSqliteColumnText
            | Self::SysSqliteColumnBytes
            | Self::SysSqliteChanges
            | Self::SysSqliteLastInsertRowid
            | Self::SysSqliteExtendedResultCode
            | Self::SysSqliteBackup
            | Self::Print
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
            | Self::SysOpenAppend
            | Self::SysOpenCreateNew
            | Self::SysOpenDir
            | Self::SysFsync
            | Self::SysTruncate
            | Self::SysRename
            | Self::SysRandomFill
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
