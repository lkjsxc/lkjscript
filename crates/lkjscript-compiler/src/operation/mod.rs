//! Canonical identities and type schemes for built-in operations.

use crate::types::Type;

/// A built-in operation after name resolution.
///
/// Backends consume this identity rather than comparing source spellings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
    EqualValue,
    SameObject,
    ListEqual,
    F64BitsEqual,
    F64FromI64Exact,
    F64FromI64Rounded,
    I64FromF64Exact,
    I64FromF64Trunc,
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
    Exit,
    BitAnd,
    BitOr,
    BitXor,
    And,
    Or,
    WriteStr,
    EmptyStr,
    ArgCount,
    Arg,
    BufNew,
    BufLen,
    BufRef,
    BufSet,
    OwnedBufNew,
    OwnedBufLen,
    OwnedBufRef,
    OwnedBufSet,
    BufClone,
    BufFromStr,
    BufToStr,
    BufSlice,
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
    SysReadInto,
    SysWriteFrom,
    SysTtyGuardSave,
    SysTtyGuardClear,
    SysOpenRead,
    SysOpenWrite,
    SysOpenAppend,
    SysOpenCreateNew,
    SysOpenDir,
    SysFsync,
    SysTruncate,
    SysRename,
    SysRandomFill,
    SysSha256,
    SysSqliteOpen,
    SysSqliteClose,
    SysSqliteBusyTimeout,
    SysSqliteExec,
    SysSqlitePrepare,
    SysSqliteFinalize,
    SysSqliteReset,
    SysSqliteClearBindings,
    SysSqliteBindNull,
    SysSqliteBindI64,
    SysSqliteBindF64,
    SysSqliteBindText,
    SysSqliteBindBytes,
    SysSqliteStep,
    SysSqliteColumnCount,
    SysSqliteColumnType,
    SysSqliteColumnI64,
    SysSqliteColumnF64,
    SysSqliteColumnText,
    SysSqliteColumnBytes,
    SysSqliteChanges,
    SysSqliteLastInsertRowid,
    SysSqliteExtendedResultCode,
    SysSqliteBackup,
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

impl Operation {
    pub const fn edition2_only(self) -> bool {
        matches!(
            self,
            Self::F64FromI64Exact
                | Self::F64FromI64Rounded
                | Self::I64FromF64Exact
                | Self::I64FromF64Trunc
        )
    }
}

mod catalog;
mod effects;
mod instantiation;
mod names;
mod resolution;
mod signature;
mod signature_memory;
mod signature_system;
mod signature_values;
mod signature_variants;

#[cfg(test)]
mod tests;
