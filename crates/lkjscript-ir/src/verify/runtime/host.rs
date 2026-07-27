use crate::verify::*;
use crate::{RuntimeOp, SsaType};
use lkjscript_contracts::ResourceKind;

pub(super) fn host_signature(
    operation: RuntimeOp,
    parameters: &[SsaType],
    result: &SsaType,
) -> Option<bool> {
    use lkjscript_contracts::CapabilityKind::{
        Clock, Entropy, FileSystem, Network, Stdio, Terminal,
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
    if let Some(valid) = super::host_sqlite::sqlite_signature(operation, parameters, result) {
        return Some(valid);
    }
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
