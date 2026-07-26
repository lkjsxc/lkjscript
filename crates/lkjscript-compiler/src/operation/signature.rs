use crate::operation::signature_memory::memory_signature;
use crate::operation::signature_system::system_signature;
use crate::operation::signature_values::value_signature;
use crate::operation::signature_variants::variant_signature;
use crate::operation::*;

impl Operation {
    pub fn signature(self) -> Type {
        match self {
            Self::Add
            | Self::Subtract
            | Self::Multiply
            | Self::Divide
            | Self::EqualValue
            | Self::SameObject
            | Self::ListEqual
            | Self::F64BitsEqual
            | Self::F64FromI64Exact
            | Self::F64FromI64Rounded
            | Self::I64FromF64Exact
            | Self::I64FromF64Trunc
            | Self::Less
            | Self::LessEqual
            | Self::Greater
            | Self::GreaterEqual
            | Self::Not
            | Self::Cons
            | Self::Car
            | Self::Cdr
            | Self::IsEmptyList
            | Self::Print
            | Self::Flush
            | Self::ReadByte
            | Self::WriteByte
            | Self::Exit
            | Self::BitAnd
            | Self::BitOr
            | Self::BitXor
            | Self::And
            | Self::Or
            | Self::WriteStr
            | Self::EmptyStr
            | Self::ArgCount
            | Self::Arg => value_signature(self),
            Self::BufNew
            | Self::BufLen
            | Self::BufRef
            | Self::BufSet
            | Self::OwnedBufNew
            | Self::OwnedBufLen
            | Self::OwnedBufRef
            | Self::OwnedBufSet
            | Self::BufClone
            | Self::BufFromStr
            | Self::BufToStr
            | Self::PathFromStr
            | Self::PathFromBuf
            | Self::PathToBuf
            | Self::PathToStr
            | Self::BufSlice
            | Self::BufGetU32
            | Self::BufSetU32
            | Self::StrLen
            | Self::StrRef
            | Self::StrAppend
            | Self::StrSlice
            | Self::StrFromByte
            | Self::StrFromI64
            | Self::StrFromF64 => memory_signature(self),
            Self::StdinHandle
            | Self::SysIsatty
            | Self::DropResource
            | Self::SysReadByte
            | Self::SysWriteByte
            | Self::SysReadInto
            | Self::SysWriteFrom
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
            | Self::SysSha256
            | Self::SysSqliteOpen
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
            | Self::SysTtySet => system_signature(self),
            Self::Ok
            | Self::Err
            | Self::IsOk
            | Self::UnwrapOk
            | Self::UnwrapErr
            | Self::Some
            | Self::IsSome
            | Self::UnwrapSome => variant_signature(self),
        }
    }
}
