use crate::ssa::*;

pub(in crate::ssa) fn runtime_operation(operation: Operation) -> Result<RuntimeOp> {
    Ok(match operation {
        Operation::Add => RuntimeOp::Add,
        Operation::Subtract => RuntimeOp::Subtract,
        Operation::Multiply => RuntimeOp::Multiply,
        Operation::Divide => RuntimeOp::Divide,
        Operation::EqualValue => RuntimeOp::EqualValue,
        Operation::SameObject => RuntimeOp::SameObject,
        Operation::ListEqual => RuntimeOp::ListEqual,
        Operation::F64BitsEqual => RuntimeOp::F64BitsEqual,
        Operation::Less => RuntimeOp::Less,
        Operation::LessEqual => RuntimeOp::LessEqual,
        Operation::Greater => RuntimeOp::Greater,
        Operation::GreaterEqual => RuntimeOp::GreaterEqual,
        Operation::Not => RuntimeOp::Not,
        Operation::Cons => RuntimeOp::Cons,
        Operation::Car => RuntimeOp::Car,
        Operation::Cdr => RuntimeOp::Cdr,
        Operation::IsEmptyList => RuntimeOp::IsEmptyList,
        Operation::Print => RuntimeOp::Print,
        Operation::Flush => RuntimeOp::Flush,
        Operation::ReadByte => RuntimeOp::ReadByte,
        Operation::WriteByte => RuntimeOp::WriteByte,
        Operation::BitAnd => RuntimeOp::BitAnd,
        Operation::BitOr => RuntimeOp::BitOr,
        Operation::BitXor => RuntimeOp::BitXor,
        Operation::WriteStr => RuntimeOp::WriteStr,
        Operation::EmptyStr => RuntimeOp::EmptyStr,
        Operation::ArgCount => RuntimeOp::ArgCount,
        Operation::Arg => RuntimeOp::Arg,
        Operation::BufNew => RuntimeOp::BufNew,
        Operation::BufLen => RuntimeOp::BufLen,
        Operation::BufRef => RuntimeOp::BufRef,
        Operation::BufSet => RuntimeOp::BufSet,
        Operation::OwnedBufNew => RuntimeOp::OwnedBufNew,
        Operation::OwnedBufLen => RuntimeOp::OwnedBufLen,
        Operation::OwnedBufRef => RuntimeOp::OwnedBufRef,
        Operation::OwnedBufSet => RuntimeOp::OwnedBufSet,
        Operation::BufClone => RuntimeOp::BufClone,
        Operation::BufFromStr => RuntimeOp::BufFromStr,
        Operation::BufToStr => RuntimeOp::BufToStr,
        Operation::BufSlice => RuntimeOp::BufSlice,
        Operation::BufGetU32 => RuntimeOp::BufGetU32,
        Operation::BufSetU32 => RuntimeOp::BufSetU32,
        Operation::StrLen => RuntimeOp::StrLen,
        Operation::StrRef => RuntimeOp::StrRef,
        Operation::StrAppend => RuntimeOp::StrAppend,
        Operation::StrSlice => RuntimeOp::StrSlice,
        Operation::StrFromByte => RuntimeOp::StrFromByte,
        Operation::StrFromI64 => RuntimeOp::StrFromI64,
        Operation::StrFromF64 => RuntimeOp::StrFromF64,
        Operation::StdinHandle => RuntimeOp::StdinHandle,
        Operation::SysIsatty => RuntimeOp::SysIsatty,
        Operation::DropResource => RuntimeOp::SysClose,
        Operation::SysReadByte => RuntimeOp::SysReadByte,
        Operation::SysWriteByte => RuntimeOp::SysWriteByte,
        Operation::SysReadInto => RuntimeOp::SysReadInto,
        Operation::SysWriteFrom => RuntimeOp::SysWriteFrom,
        Operation::SysTtyGuardSave => RuntimeOp::SysTtyGuardSave,
        Operation::SysTtyGuardClear => RuntimeOp::SysTtyGuardClear,
        Operation::SysOpenRead => RuntimeOp::SysOpenRead,
        Operation::SysOpenWrite => RuntimeOp::SysOpenWrite,
        Operation::SysOpenAppend => RuntimeOp::SysOpenAppend,
        Operation::SysOpenCreateNew => RuntimeOp::SysOpenCreateNew,
        Operation::SysOpenDir => RuntimeOp::SysOpenDir,
        Operation::SysFsync => RuntimeOp::SysFsync,
        Operation::SysTruncate => RuntimeOp::SysTruncate,
        Operation::SysRename => RuntimeOp::SysRename,
        Operation::SysRandomFill => RuntimeOp::SysRandomFill,
        Operation::SysSha256 => RuntimeOp::SysSha256,
        Operation::SysSqliteOpen => RuntimeOp::SysSqliteOpen,
        Operation::SysSqliteClose => RuntimeOp::SysSqliteClose,
        Operation::SysSqliteBusyTimeout => RuntimeOp::SysSqliteBusyTimeout,
        Operation::SysSqliteExec => RuntimeOp::SysSqliteExec,
        Operation::SysSqlitePrepare => RuntimeOp::SysSqlitePrepare,
        Operation::SysSqliteFinalize => RuntimeOp::SysSqliteFinalize,
        Operation::SysSqliteReset => RuntimeOp::SysSqliteReset,
        Operation::SysSqliteClearBindings => RuntimeOp::SysSqliteClearBindings,
        Operation::SysSqliteBindNull => RuntimeOp::SysSqliteBindNull,
        Operation::SysSqliteBindI64 => RuntimeOp::SysSqliteBindI64,
        Operation::SysSqliteBindF64 => RuntimeOp::SysSqliteBindF64,
        Operation::SysSqliteBindText => RuntimeOp::SysSqliteBindText,
        Operation::SysSqliteBindBytes => RuntimeOp::SysSqliteBindBytes,
        Operation::SysSqliteStep => RuntimeOp::SysSqliteStep,
        Operation::SysSqliteColumnCount => RuntimeOp::SysSqliteColumnCount,
        Operation::SysSqliteColumnType => RuntimeOp::SysSqliteColumnType,
        Operation::SysSqliteColumnI64 => RuntimeOp::SysSqliteColumnI64,
        Operation::SysSqliteColumnF64 => RuntimeOp::SysSqliteColumnF64,
        Operation::SysSqliteColumnText => RuntimeOp::SysSqliteColumnText,
        Operation::SysSqliteColumnBytes => RuntimeOp::SysSqliteColumnBytes,
        Operation::SysSqliteChanges => RuntimeOp::SysSqliteChanges,
        Operation::SysSqliteLastInsertRowid => RuntimeOp::SysSqliteLastInsertRowid,
        Operation::SysSqliteExtendedResultCode => RuntimeOp::SysSqliteExtendedResultCode,
        Operation::SysSqliteBackup => RuntimeOp::SysSqliteBackup,
        Operation::SysPathExists => RuntimeOp::SysPathExists,
        Operation::SysWaitMs => RuntimeOp::SysWaitMs,
        Operation::SysNowMs => RuntimeOp::SysNowMs,
        Operation::SysSocket => RuntimeOp::SysSocket,
        Operation::SysBind => RuntimeOp::SysBind,
        Operation::SysListen => RuntimeOp::SysListen,
        Operation::SysAccept => RuntimeOp::SysAccept,
        Operation::SysRecv => RuntimeOp::SysRecv,
        Operation::SysSend => RuntimeOp::SysSend,
        Operation::SysPoll => RuntimeOp::SysPoll,
        Operation::SysTtyGet => RuntimeOp::SysTtyGet,
        Operation::SysTtySet => RuntimeOp::SysTtySet,
        Operation::Exit
        | Operation::And
        | Operation::Or
        | Operation::F64FromI64Exact
        | Operation::F64FromI64Rounded
        | Operation::I64FromF64Exact
        | Operation::I64FromF64Trunc
        | Operation::Ok
        | Operation::Err
        | Operation::IsOk
        | Operation::UnwrapOk
        | Operation::UnwrapErr
        | Operation::Some
        | Operation::IsSome
        | Operation::UnwrapSome => {
            return Err(Error::msg(format!(
                "control operation {operation:?} cannot lower as an SSA runtime operation"
            )));
        }
    })
}
