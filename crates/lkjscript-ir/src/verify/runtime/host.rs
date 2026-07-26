use crate::verify::*;
use crate::{RuntimeOp, SsaType};

pub(super) fn host_signature(
    operation: RuntimeOp,
    parameters: &[SsaType],
    result: &SsaType,
) -> Option<bool> {
    let exact = |expected: &[SsaType], result_type: &SsaType| {
        parameters == expected && result == result_type
    };
    let valid = match operation {
        RuntimeOp::StdinHandle => exact(&[], &SsaType::Handle),
        RuntimeOp::SysIsatty => exact(&[SsaType::Handle], &system_result(SsaType::Bool)),
        RuntimeOp::SysClose => exact(&[SsaType::Handle], &system_result(SsaType::Unit)),
        RuntimeOp::SysReadByte => exact(&[SsaType::Handle], &system_result(SsaType::I64)),
        RuntimeOp::SysWriteByte => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysReadInto | RuntimeOp::SysWriteFrom => exact(
            &[SsaType::Handle, SsaType::Buf, SsaType::I64, SsaType::I64],
            &system_result(SsaType::I64),
        ),
        RuntimeOp::SysTtyGuardSave => exact(&[SsaType::Buf], &system_result(SsaType::Unit)),
        RuntimeOp::SysTtyGuardClear => exact(&[], &system_result(SsaType::Unit)),
        RuntimeOp::SysOpenRead
        | RuntimeOp::SysOpenWrite
        | RuntimeOp::SysOpenAppend
        | RuntimeOp::SysOpenCreateNew
        | RuntimeOp::SysOpenDir => exact(&[SsaType::Str], &system_result(SsaType::Handle)),
        RuntimeOp::SysFsync => exact(&[SsaType::Handle], &system_result(SsaType::Unit)),
        RuntimeOp::SysTruncate => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysRename => exact(&[SsaType::Str, SsaType::Str], &system_result(SsaType::Unit)),
        RuntimeOp::SysRandomFill => exact(
            &[SsaType::Buf, SsaType::I64, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSha256 => exact(
            &[SsaType::Buf, SsaType::I64, SsaType::I64],
            &system_result(SsaType::Buf),
        ),
        RuntimeOp::SysSqliteOpen => exact(
            &[SsaType::Str, SsaType::I64],
            &system_result(SsaType::Handle),
        ),
        RuntimeOp::SysSqliteClose
        | RuntimeOp::SysSqliteFinalize
        | RuntimeOp::SysSqliteReset
        | RuntimeOp::SysSqliteClearBindings => {
            exact(&[SsaType::Handle], &system_result(SsaType::Unit))
        }
        RuntimeOp::SysSqliteBusyTimeout | RuntimeOp::SysSqliteBindNull => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSqliteExec => exact(
            &[SsaType::Handle, SsaType::Str],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSqlitePrepare => exact(
            &[SsaType::Handle, SsaType::Str],
            &system_result(SsaType::Handle),
        ),
        RuntimeOp::SysSqliteBindI64 => exact(
            &[SsaType::Handle, SsaType::I64, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSqliteBindF64 => exact(
            &[SsaType::Handle, SsaType::I64, SsaType::F64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSqliteBindText => exact(
            &[SsaType::Handle, SsaType::I64, SsaType::Str],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSqliteBindBytes => exact(
            &[SsaType::Handle, SsaType::I64, SsaType::Buf],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSqliteStep
        | RuntimeOp::SysSqliteColumnCount
        | RuntimeOp::SysSqliteChanges
        | RuntimeOp::SysSqliteLastInsertRowid
        | RuntimeOp::SysSqliteExtendedResultCode => {
            exact(&[SsaType::Handle], &system_result(SsaType::I64))
        }
        RuntimeOp::SysSqliteColumnType => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::I64),
        ),
        RuntimeOp::SysSqliteColumnI64 => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(crate::prelude_contract::option(SsaType::I64)),
        ),
        RuntimeOp::SysSqliteColumnF64 => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(crate::prelude_contract::option(SsaType::F64)),
        ),
        RuntimeOp::SysSqliteColumnText => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(crate::prelude_contract::option(SsaType::Str)),
        ),
        RuntimeOp::SysSqliteColumnBytes => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(crate::prelude_contract::option(SsaType::Buf)),
        ),
        RuntimeOp::SysSqliteBackup => exact(
            &[SsaType::Handle, SsaType::Str, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysPathExists => exact(&[SsaType::Str], &system_result(SsaType::Bool)),
        RuntimeOp::SysWaitMs => exact(&[SsaType::I64], &system_result(SsaType::Unit)),
        RuntimeOp::SysNowMs => exact(&[], &system_result(SsaType::I64)),
        RuntimeOp::SysSocket => exact(&[], &system_result(SsaType::Handle)),
        RuntimeOp::SysBind | RuntimeOp::SysListen => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysAccept => exact(&[SsaType::Handle], &system_result(SsaType::Handle)),
        RuntimeOp::SysRecv => exact(&[SsaType::Handle], &system_result(SsaType::Str)),
        RuntimeOp::SysSend => exact(
            &[SsaType::Handle, SsaType::Str],
            &system_result(SsaType::I64),
        ),
        RuntimeOp::SysPoll => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::I64),
        ),
        RuntimeOp::SysTtyGet | RuntimeOp::SysTtySet => exact(
            &[SsaType::Handle, SsaType::Buf],
            &system_result(SsaType::Unit),
        ),
        _ => return None,
    };
    Some(valid)
}
