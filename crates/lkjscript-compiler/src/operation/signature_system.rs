use crate::operation::instantiation::{forall, function};
use crate::operation::*;
use lkjscript_core::ResourceKind;

pub(in crate::operation) fn system_signature(operation: Operation) -> Type {
    use lkjscript_core::CapabilityKind::{
        Clock, Entropy, FileSystem, Network, Sqlite, Stdio, Terminal,
    };

    let system_result =
        |success| crate::types::result_type(success, crate::types::system_error_type());
    let resource = |kind| Type::Resource(kind);
    let any_resource = || Type::Param("resource".into());
    let resource_function =
        |params: Vec<Type>, result: Type| forall(&["resource"], function(params, result));
    match operation {
        Operation::StdinHandle => function(
            vec![Type::Capability(Stdio)],
            resource(ResourceKind::InputStream),
        ),
        Operation::SysIsatty => function(
            vec![resource(ResourceKind::InputStream)],
            system_result(Type::Bool),
        ),
        Operation::DropResource => {
            resource_function(vec![any_resource()], system_result(Type::Unit))
        }
        Operation::SysReadByte => resource_function(vec![any_resource()], system_result(Type::I64)),
        Operation::SysWriteByte => {
            resource_function(vec![any_resource(), Type::I64], system_result(Type::Unit))
        }
        Operation::SysReadInto | Operation::SysWriteFrom => resource_function(
            vec![any_resource(), Type::Buf, Type::I64, Type::I64],
            system_result(Type::I64),
        ),
        Operation::SysTtyGuardSave => function(
            vec![Type::Capability(Terminal), Type::Buf],
            system_result(Type::Unit),
        ),
        Operation::SysTtyGuardClear => {
            function(vec![Type::Capability(Terminal)], system_result(Type::Unit))
        }
        Operation::SysOpenRead => function(
            vec![Type::Capability(FileSystem), Type::Path],
            system_result(resource(ResourceKind::FileReader)),
        ),
        Operation::SysOpenWrite | Operation::SysOpenCreateNew => function(
            vec![Type::Capability(FileSystem), Type::Path],
            system_result(resource(ResourceKind::FileWriter)),
        ),
        Operation::SysOpenAppend => function(
            vec![Type::Capability(FileSystem), Type::Path],
            system_result(resource(ResourceKind::FileAppender)),
        ),
        Operation::SysOpenDir => function(
            vec![Type::Capability(FileSystem), Type::Path],
            system_result(resource(ResourceKind::Directory)),
        ),
        Operation::SysFsync => resource_function(vec![any_resource()], system_result(Type::Unit)),
        Operation::SysTruncate => {
            resource_function(vec![any_resource(), Type::I64], system_result(Type::Unit))
        }
        Operation::SysRename => function(
            vec![Type::Capability(FileSystem), Type::Path, Type::Path],
            system_result(Type::Unit),
        ),
        Operation::SysRandomFill => function(
            vec![Type::Capability(Entropy), Type::Buf, Type::I64, Type::I64],
            system_result(Type::Unit),
        ),
        Operation::SysSha256 => function(
            vec![Type::Buf, Type::I64, Type::I64],
            system_result(Type::Buf),
        ),
        Operation::SysSqliteOpen => function(
            vec![Type::Capability(Sqlite), Type::Path, Type::I64],
            system_result(resource(ResourceKind::SqliteConnection)),
        ),
        Operation::SysSqliteClose => function(
            vec![resource(ResourceKind::SqliteConnection)],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteFinalize
        | Operation::SysSqliteReset
        | Operation::SysSqliteClearBindings => function(
            vec![resource(ResourceKind::SqliteStatement)],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteBusyTimeout => function(
            vec![resource(ResourceKind::SqliteConnection), Type::I64],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteBindNull => function(
            vec![resource(ResourceKind::SqliteStatement), Type::I64],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteExec => function(
            vec![resource(ResourceKind::SqliteConnection), Type::Str],
            system_result(Type::Unit),
        ),
        Operation::SysSqlitePrepare => function(
            vec![resource(ResourceKind::SqliteConnection), Type::Str],
            system_result(resource(ResourceKind::SqliteStatement)),
        ),
        Operation::SysSqliteBindI64 => function(
            vec![
                resource(ResourceKind::SqliteStatement),
                Type::I64,
                Type::I64,
            ],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteBindF64 => function(
            vec![
                resource(ResourceKind::SqliteStatement),
                Type::I64,
                Type::F64,
            ],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteBindText => function(
            vec![
                resource(ResourceKind::SqliteStatement),
                Type::I64,
                Type::Str,
            ],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteBindBytes => function(
            vec![
                resource(ResourceKind::SqliteStatement),
                Type::I64,
                Type::Buf,
            ],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteStep | Operation::SysSqliteColumnCount => function(
            vec![resource(ResourceKind::SqliteStatement)],
            system_result(Type::I64),
        ),
        Operation::SysSqliteChanges
        | Operation::SysSqliteLastInsertRowid
        | Operation::SysSqliteExtendedResultCode => function(
            vec![resource(ResourceKind::SqliteConnection)],
            system_result(Type::I64),
        ),
        Operation::SysSqliteColumnType => function(
            vec![resource(ResourceKind::SqliteStatement), Type::I64],
            system_result(Type::I64),
        ),
        Operation::SysSqliteColumnI64 => function(
            vec![resource(ResourceKind::SqliteStatement), Type::I64],
            system_result(crate::types::option_type(Type::I64)),
        ),
        Operation::SysSqliteColumnF64 => function(
            vec![resource(ResourceKind::SqliteStatement), Type::I64],
            system_result(crate::types::option_type(Type::F64)),
        ),
        Operation::SysSqliteColumnText => function(
            vec![resource(ResourceKind::SqliteStatement), Type::I64],
            system_result(crate::types::option_type(Type::Str)),
        ),
        Operation::SysSqliteColumnBytes => function(
            vec![resource(ResourceKind::SqliteStatement), Type::I64],
            system_result(crate::types::option_type(Type::Buf)),
        ),
        Operation::SysSqliteBackup => function(
            vec![
                Type::Capability(Sqlite),
                resource(ResourceKind::SqliteConnection),
                Type::Path,
                Type::I64,
            ],
            system_result(Type::Unit),
        ),
        Operation::SysPathExists => function(
            vec![Type::Capability(FileSystem), Type::Path],
            system_result(Type::Bool),
        ),
        Operation::SysWaitMs => function(
            vec![Type::Capability(Clock), Type::I64],
            system_result(Type::Unit),
        ),
        Operation::SysNowMs => function(vec![Type::Capability(Clock)], system_result(Type::I64)),
        Operation::SysSocket => function(
            vec![Type::Capability(Network)],
            system_result(resource(ResourceKind::TcpListener)),
        ),
        Operation::SysBind | Operation::SysListen => function(
            vec![resource(ResourceKind::TcpListener), Type::I64],
            system_result(Type::Unit),
        ),
        Operation::SysAccept => function(
            vec![resource(ResourceKind::TcpListener)],
            system_result(resource(ResourceKind::TcpStream)),
        ),
        Operation::SysRecv => function(
            vec![resource(ResourceKind::TcpStream)],
            system_result(Type::Str),
        ),
        Operation::SysSend => function(
            vec![resource(ResourceKind::TcpStream), Type::Str],
            system_result(Type::I64),
        ),
        Operation::SysPoll => {
            resource_function(vec![any_resource(), Type::I64], system_result(Type::I64))
        }
        Operation::SysTtyGet | Operation::SysTtySet => function(
            vec![resource(ResourceKind::InputStream), Type::Buf],
            system_result(Type::Unit),
        ),
        _ => unreachable!("operation signature family mismatch"),
    }
}
