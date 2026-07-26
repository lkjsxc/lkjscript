use crate::operation::instantiation::function;
use crate::operation::*;

pub(in crate::operation) fn system_signature(operation: Operation) -> Type {
    let system_result =
        |success| crate::types::result_type(success, crate::types::system_error_type());
    match operation {
        Operation::StdinHandle => function(Vec::new(), Type::Handle),
        Operation::SysIsatty => function(vec![Type::Handle], system_result(Type::Bool)),
        Operation::SysClose => function(vec![Type::Handle], system_result(Type::Unit)),
        Operation::SysReadByte => function(vec![Type::Handle], system_result(Type::I64)),
        Operation::SysWriteByte => {
            function(vec![Type::Handle, Type::I64], system_result(Type::Unit))
        }
        Operation::SysReadInto | Operation::SysWriteFrom => function(
            vec![Type::Handle, Type::Buf, Type::I64, Type::I64],
            system_result(Type::I64),
        ),
        Operation::SysTtyGuardSave => function(vec![Type::Buf], system_result(Type::Unit)),
        Operation::SysTtyGuardClear => function(Vec::new(), system_result(Type::Unit)),
        Operation::SysOpenRead
        | Operation::SysOpenWrite
        | Operation::SysOpenAppend
        | Operation::SysOpenCreateNew
        | Operation::SysOpenDir => function(vec![Type::Str], system_result(Type::Handle)),
        Operation::SysFsync => function(vec![Type::Handle], system_result(Type::Unit)),
        Operation::SysTruncate => {
            function(vec![Type::Handle, Type::I64], system_result(Type::Unit))
        }
        Operation::SysRename => function(vec![Type::Str, Type::Str], system_result(Type::Unit)),
        Operation::SysRandomFill => function(
            vec![Type::Buf, Type::I64, Type::I64],
            system_result(Type::Unit),
        ),
        Operation::SysSha256 => function(
            vec![Type::Buf, Type::I64, Type::I64],
            system_result(Type::Buf),
        ),
        Operation::SysSqliteOpen => {
            function(vec![Type::Str, Type::I64], system_result(Type::Handle))
        }
        Operation::SysSqliteClose
        | Operation::SysSqliteFinalize
        | Operation::SysSqliteReset
        | Operation::SysSqliteClearBindings => {
            function(vec![Type::Handle], system_result(Type::Unit))
        }
        Operation::SysSqliteBindNull | Operation::SysSqliteBusyTimeout => {
            function(vec![Type::Handle, Type::I64], system_result(Type::Unit))
        }
        Operation::SysSqliteExec | Operation::SysSqlitePrepare => function(
            vec![Type::Handle, Type::Str],
            system_result(if matches!(operation, Operation::SysSqlitePrepare) {
                Type::Handle
            } else {
                Type::Unit
            }),
        ),
        Operation::SysSqliteBindI64 => function(
            vec![Type::Handle, Type::I64, Type::I64],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteBindF64 => function(
            vec![Type::Handle, Type::I64, Type::F64],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteBindText => function(
            vec![Type::Handle, Type::I64, Type::Str],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteBindBytes => function(
            vec![Type::Handle, Type::I64, Type::Buf],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteStep
        | Operation::SysSqliteColumnCount
        | Operation::SysSqliteChanges
        | Operation::SysSqliteLastInsertRowid
        | Operation::SysSqliteExtendedResultCode => {
            function(vec![Type::Handle], system_result(Type::I64))
        }
        Operation::SysSqliteColumnType => {
            function(vec![Type::Handle, Type::I64], system_result(Type::I64))
        }
        Operation::SysSqliteColumnI64 => function(
            vec![Type::Handle, Type::I64],
            system_result(crate::types::option_type(Type::I64)),
        ),
        Operation::SysSqliteColumnF64 => function(
            vec![Type::Handle, Type::I64],
            system_result(crate::types::option_type(Type::F64)),
        ),
        Operation::SysSqliteColumnText => function(
            vec![Type::Handle, Type::I64],
            system_result(crate::types::option_type(Type::Str)),
        ),
        Operation::SysSqliteColumnBytes => function(
            vec![Type::Handle, Type::I64],
            system_result(crate::types::option_type(Type::Buf)),
        ),
        Operation::SysSqliteBackup => function(
            vec![Type::Handle, Type::Str, Type::I64],
            system_result(Type::Unit),
        ),
        Operation::SysPathExists => function(vec![Type::Str], system_result(Type::Bool)),
        Operation::SysWaitMs => function(vec![Type::I64], system_result(Type::Unit)),
        Operation::SysNowMs => function(Vec::new(), system_result(Type::I64)),
        Operation::SysSocket => function(Vec::new(), system_result(Type::Handle)),
        Operation::SysBind | Operation::SysListen => {
            function(vec![Type::Handle, Type::I64], system_result(Type::Unit))
        }
        Operation::SysAccept => function(vec![Type::Handle], system_result(Type::Handle)),
        Operation::SysRecv => function(vec![Type::Handle], system_result(Type::Str)),
        Operation::SysSend => function(vec![Type::Handle, Type::Str], system_result(Type::I64)),
        Operation::SysPoll => function(vec![Type::Handle, Type::I64], system_result(Type::I64)),
        Operation::SysTtyGet | Operation::SysTtySet => {
            function(vec![Type::Handle, Type::Buf], system_result(Type::Unit))
        }
        _ => unreachable!("operation signature family mismatch"),
    }
}
