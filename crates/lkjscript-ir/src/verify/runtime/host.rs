use crate::verify::*;
use crate::{RuntimeOp, SsaType};
use lkjscript_contracts::ResourceKind;

pub(super) fn host_signature(
    operation: RuntimeOp,
    parameters: &[SsaType],
    result: &SsaType,
) -> Option<bool> {
    use lkjscript_contracts::CapabilityKind::{
        Clock, Entropy, FileSystem, Network, Sqlite, Stdio, Terminal,
    };
    use ResourceKind::{
        Directory, FileAppender, FileReader, FileWriter, InputStream, OutputStream, TcpListener,
        TcpStream,
    };

    let resource = SsaType::Resource;
    let exact = |expected: &[SsaType], result_type: &SsaType| {
        parameters == expected && result == result_type
    };
    let resource_input = |allowed: &[ResourceKind], tail: &[SsaType], result_type: &SsaType| {
        let Some((SsaType::Resource(kind), rest)) = parameters.split_first() else {
            return false;
        };
        allowed.contains(kind) && rest == tail && result == result_type
    };
    let valid = match operation {
        RuntimeOp::StdinHandle => exact(&[SsaType::Capability(Stdio)], &resource(InputStream)),
        RuntimeOp::SysIsatty => exact(&[resource(InputStream)], &system_result(SsaType::Bool)),
        RuntimeOp::SysClose => resource_input(
            &[
                OutputStream,
                FileReader,
                FileWriter,
                FileAppender,
                Directory,
                TcpListener,
                TcpStream,
                ResourceKind::SqliteConnection,
                ResourceKind::SqliteStatement,
                ResourceKind::TerminalSession,
            ],
            &[],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysReadByte => resource_input(
            &[InputStream, FileReader, TcpStream],
            &[],
            &system_result(SsaType::I64),
        ),
        RuntimeOp::SysWriteByte => resource_input(
            &[OutputStream, FileWriter, FileAppender, TcpStream],
            &[SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysReadInto => resource_input(
            &[InputStream, FileReader, TcpStream],
            &[SsaType::Buf, SsaType::I64, SsaType::I64],
            &system_result(SsaType::I64),
        ),
        RuntimeOp::SysWriteFrom => resource_input(
            &[OutputStream, FileWriter, FileAppender, TcpStream],
            &[SsaType::Buf, SsaType::I64, SsaType::I64],
            &system_result(SsaType::I64),
        ),
        RuntimeOp::SysTtyGuardSave => exact(
            &[SsaType::Capability(Terminal), SsaType::Buf],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysTtyGuardClear => exact(
            &[SsaType::Capability(Terminal)],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysOpenRead => file_open(FileReader, parameters, result),
        RuntimeOp::SysOpenWrite | RuntimeOp::SysOpenCreateNew => {
            file_open(FileWriter, parameters, result)
        }
        RuntimeOp::SysOpenAppend => file_open(FileAppender, parameters, result),
        RuntimeOp::SysOpenDir => file_open(Directory, parameters, result),
        RuntimeOp::SysFsync => resource_input(
            &[FileWriter, FileAppender, Directory],
            &[],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysTruncate => resource_input(
            &[FileWriter, FileAppender],
            &[SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysRename => exact(
            &[
                SsaType::Capability(FileSystem),
                SsaType::Path,
                SsaType::Path,
            ],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysRandomFill => exact(
            &[
                SsaType::Capability(Entropy),
                SsaType::Buf,
                SsaType::I64,
                SsaType::I64,
            ],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSha256 => exact(
            &[SsaType::Buf, SsaType::I64, SsaType::I64],
            &system_result(SsaType::Buf),
        ),
        RuntimeOp::SysSqliteOpen => exact(
            &[SsaType::Capability(Sqlite), SsaType::Path, SsaType::I64],
            &system_result(resource(ResourceKind::SqliteConnection)),
        ),
        RuntimeOp::SysSqliteClose => {
            sqlite_one(ResourceKind::SqliteConnection, result, SsaType::Unit)
        }
        RuntimeOp::SysSqliteFinalize
        | RuntimeOp::SysSqliteReset
        | RuntimeOp::SysSqliteClearBindings => {
            sqlite_one(ResourceKind::SqliteStatement, result, SsaType::Unit)
        }
        RuntimeOp::SysSqliteBusyTimeout => sqlite_tail(
            ResourceKind::SqliteConnection,
            &[SsaType::I64],
            parameters,
            result,
            SsaType::Unit,
        ),
        RuntimeOp::SysSqliteBindNull => sqlite_tail(
            ResourceKind::SqliteStatement,
            &[SsaType::I64],
            parameters,
            result,
            SsaType::Unit,
        ),
        RuntimeOp::SysSqliteExec => sqlite_tail(
            ResourceKind::SqliteConnection,
            &[SsaType::Str],
            parameters,
            result,
            SsaType::Unit,
        ),
        RuntimeOp::SysSqlitePrepare => sqlite_tail(
            ResourceKind::SqliteConnection,
            &[SsaType::Str],
            parameters,
            result,
            resource(ResourceKind::SqliteStatement),
        ),
        RuntimeOp::SysSqliteBindI64 => sqlite_tail(
            ResourceKind::SqliteStatement,
            &[SsaType::I64, SsaType::I64],
            parameters,
            result,
            SsaType::Unit,
        ),
        RuntimeOp::SysSqliteBindF64 => sqlite_tail(
            ResourceKind::SqliteStatement,
            &[SsaType::I64, SsaType::F64],
            parameters,
            result,
            SsaType::Unit,
        ),
        RuntimeOp::SysSqliteBindText => sqlite_tail(
            ResourceKind::SqliteStatement,
            &[SsaType::I64, SsaType::Str],
            parameters,
            result,
            SsaType::Unit,
        ),
        RuntimeOp::SysSqliteBindBytes => sqlite_tail(
            ResourceKind::SqliteStatement,
            &[SsaType::I64, SsaType::Buf],
            parameters,
            result,
            SsaType::Unit,
        ),
        RuntimeOp::SysSqliteStep | RuntimeOp::SysSqliteColumnCount => {
            sqlite_one(ResourceKind::SqliteStatement, result, SsaType::I64)
        }
        RuntimeOp::SysSqliteChanges
        | RuntimeOp::SysSqliteLastInsertRowid
        | RuntimeOp::SysSqliteExtendedResultCode => {
            sqlite_one(ResourceKind::SqliteConnection, result, SsaType::I64)
        }
        RuntimeOp::SysSqliteColumnType => sqlite_tail(
            ResourceKind::SqliteStatement,
            &[SsaType::I64],
            parameters,
            result,
            SsaType::I64,
        ),
        RuntimeOp::SysSqliteColumnI64 => sqlite_tail(
            ResourceKind::SqliteStatement,
            &[SsaType::I64],
            parameters,
            result,
            crate::prelude_contract::option(SsaType::I64),
        ),
        RuntimeOp::SysSqliteColumnF64 => sqlite_tail(
            ResourceKind::SqliteStatement,
            &[SsaType::I64],
            parameters,
            result,
            crate::prelude_contract::option(SsaType::F64),
        ),
        RuntimeOp::SysSqliteColumnText => sqlite_tail(
            ResourceKind::SqliteStatement,
            &[SsaType::I64],
            parameters,
            result,
            crate::prelude_contract::option(SsaType::Str),
        ),
        RuntimeOp::SysSqliteColumnBytes => sqlite_tail(
            ResourceKind::SqliteStatement,
            &[SsaType::I64],
            parameters,
            result,
            crate::prelude_contract::option(SsaType::Buf),
        ),
        RuntimeOp::SysSqliteBackup => exact(
            &[
                SsaType::Capability(Sqlite),
                resource(ResourceKind::SqliteConnection),
                SsaType::Path,
                SsaType::I64,
            ],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysPathExists => exact(
            &[SsaType::Capability(FileSystem), SsaType::Path],
            &system_result(SsaType::Bool),
        ),
        RuntimeOp::SysWaitMs => exact(
            &[SsaType::Capability(Clock), SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysNowMs => exact(&[SsaType::Capability(Clock)], &system_result(SsaType::I64)),
        RuntimeOp::SysSocket => exact(
            &[SsaType::Capability(Network)],
            &system_result(resource(TcpListener)),
        ),
        RuntimeOp::SysBind | RuntimeOp::SysListen => exact(
            &[resource(TcpListener), SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysAccept => exact(
            &[resource(TcpListener)],
            &system_result(resource(TcpStream)),
        ),
        RuntimeOp::SysRecv => exact(&[resource(TcpStream)], &system_result(SsaType::Str)),
        RuntimeOp::SysSend => exact(
            &[resource(TcpStream), SsaType::Str],
            &system_result(SsaType::I64),
        ),
        RuntimeOp::SysPoll => resource_input(
            &[InputStream, FileReader, TcpListener, TcpStream],
            &[SsaType::I64],
            &system_result(SsaType::I64),
        ),
        RuntimeOp::SysTtyGet | RuntimeOp::SysTtySet => exact(
            &[resource(InputStream), SsaType::Buf],
            &system_result(SsaType::Unit),
        ),
        _ => return None,
    };
    Some(valid)
}

fn file_open(kind: ResourceKind, parameters: &[SsaType], result: &SsaType) -> bool {
    parameters
        == [
            SsaType::Capability(lkjscript_contracts::CapabilityKind::FileSystem),
            SsaType::Path,
        ]
        && result == &system_result(SsaType::Resource(kind))
}

fn sqlite_one(kind: ResourceKind, result: &SsaType, output: SsaType) -> bool {
    sqlite_tail(kind, &[], &[SsaType::Resource(kind)], result, output)
}

fn sqlite_tail(
    kind: ResourceKind,
    tail: &[SsaType],
    parameters: &[SsaType],
    result: &SsaType,
    output: SsaType,
) -> bool {
    parameters.first() == Some(&SsaType::Resource(kind))
        && parameters.get(1..) == Some(tail)
        && result == &system_result(output)
}
