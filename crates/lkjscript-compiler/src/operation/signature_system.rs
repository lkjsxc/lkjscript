use crate::operation::instantiation::{forall, function};
use crate::operation::signature_system_sqlite::sqlite_signature;
use crate::operation::*;
use lkjscript_core::ResourceKind;

pub(in crate::operation) fn system_signature(operation: Operation) -> Type {
    use lkjscript_core::CapabilityKind::{Clock, Entropy, FileSystem, Network, Stdio, Terminal};

    let system_result =
        |success| crate::types::result_type(success, crate::types::system_error_type());
    let resource = |kind| Type::Resource(kind);
    let any_resource = || Type::Param("resource".into());
    let resource_function =
        |params: Vec<Type>, result: Type| forall(&["resource"], function(params, result));
    if let Some(signature) = sqlite_signature(operation) {
        return signature;
    }
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
        Operation::SysReadInto => resource_function(
            vec![any_resource(), Type::ByteSliceMut],
            system_result(Type::I64),
        ),
        Operation::SysWriteFrom => resource_function(
            vec![any_resource(), Type::ByteSlice],
            system_result(Type::I64),
        ),
        Operation::SysTtyGuardSave => function(
            vec![Type::Capability(Terminal), Type::ByteSlice],
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
            vec![Type::Capability(Entropy), Type::ByteSliceMut],
            system_result(Type::Unit),
        ),
        Operation::SysSha256 => function(vec![Type::ByteSlice], Type::Bytes),
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
        Operation::SysTtyGet => function(
            vec![resource(ResourceKind::InputStream), Type::ByteSliceMut],
            system_result(Type::Unit),
        ),
        Operation::SysTtySet => function(
            vec![resource(ResourceKind::InputStream), Type::ByteSlice],
            system_result(Type::Unit),
        ),
        _ => unreachable!("operation signature family mismatch"),
    }
}
