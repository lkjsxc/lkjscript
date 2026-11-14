//! One public/raw signature table drives capability inference and construction.
use super::methods::Receiver;
use crate::ir;
#[derive(Clone, Copy)]
pub(super) enum Value {
    Unit,
    Byte,
    Count,
    Error,
    Boolean,
    Text,
    Bytes,
    MutableBytes,
    Stdout,
    Stdin,
    Stderr,
    Directory,
    File,
    Arguments,
    ResultDirectory,
    ResultFile,
    ResultCount,
    ResultBytes,
    ResultUnit,
    Process,
    Clock,
    Child,
    FileRef,
    DirectoryRef,
    ArgumentsView,
    ResultChild,
    ResultNames,
    ResultStatus,
    ResultMaybeStatus,
}
#[derive(Clone, Copy)]
pub(super) enum Method {
    Stdout,
    Stdin,
    Stderr,
    Args,
    Directory,
    ExitStatus,
    ArgCount,
    ArgLen,
    ArgCopy,
    DirectoryRaw,
    StdoutWrite,
    StderrWrite,
    StdinRead,
    StdinReadRaw,
    StdinReadToEnd,
    DirectoryOpen,
    DirectoryCreate,
    DirectoryOpenRaw,
    FileRead,
    FileWrite,
    FileReadRaw,
    FileWriteRaw,
    FileReadToEnd,
    FileWriteAll,
    Process,
    Clock,
    WorldFile,
    TemporaryFile,
    StdinFile,
    StdoutFile,
    StderrFile,
    Executable,
    ClockRead,
    ClockNow,
    ProcessSpawn,
    ProcessSpawnRaw,
    ChildWait,
    ChildTryWait,
    ChildWaitRaw,
    ChildTerminate,
    ChildTerminateRaw,
    FileRewind,
    FileRewindRaw,
    FileExecutable,
    FileExecutableRaw,
    FileDuplicate,
    FileReadOnly,
    FileReadOnlyRaw,
    OpenDirectory,
    OpenDirectoryRaw,
    DirectoryRead,
    DirectoryRewind,
    DirectoryList,
    CreateDirectory,
    CreateDirectoryRaw,
    DirectoryRemove,
    DirectoryRemoveDirectory,
    DirectoryRemoveRaw,
    DirectoryRename,
    DirectoryRenameRaw,
}
pub(super) fn method(receiver: Receiver, name: &str) -> Option<Method> {
    Some(match (receiver, name) {
        (Receiver::World, "stdout") => Method::Stdout,
        (Receiver::World, "stdin") => Method::Stdin,
        (Receiver::World, "stderr") => Method::Stderr,
        (Receiver::World, "args") => Method::Args,
        (Receiver::World, "directory") => Method::Directory,
        (Receiver::World, "set_exit_status") => Method::ExitStatus,
        (Receiver::World, "__lkjscript_arg_count") => Method::ArgCount,
        (Receiver::World, "__lkjscript_arg_len") => Method::ArgLen,
        (Receiver::World, "__lkjscript_arg_copy") => Method::ArgCopy,
        (Receiver::World, "__lkjscript_directory") => Method::DirectoryRaw,
        (Receiver::Stdout, "write") => Method::StdoutWrite,
        (Receiver::Stderr, "write") => Method::StderrWrite,
        (Receiver::Stdin, "read") => Method::StdinRead,
        (Receiver::Stdin, "__lkjscript_read") => Method::StdinReadRaw,
        (Receiver::Stdin, "read_to_end") => Method::StdinReadToEnd,
        (Receiver::Directory, "open") => Method::DirectoryOpen,
        (Receiver::Directory, "create") => Method::DirectoryCreate,
        (Receiver::Directory, "__lkjscript_open") => Method::DirectoryOpenRaw,
        (Receiver::File, "read") => Method::FileRead,
        (Receiver::File, "write") => Method::FileWrite,
        (Receiver::File, "__lkjscript_read") => Method::FileReadRaw,
        (Receiver::File, "__lkjscript_write") => Method::FileWriteRaw,
        (Receiver::File, "read_to_end") => Method::FileReadToEnd,
        (Receiver::File, "write_all") => Method::FileWriteAll,
        (Receiver::World, "process") => Method::Process,
        (Receiver::World, "clock") => Method::Clock,
        (Receiver::World, "__lkjscript_file") => Method::WorldFile,
        (Receiver::World, "temporary_file") => Method::TemporaryFile,
        (Receiver::World, "stdin_file") => Method::StdinFile,
        (Receiver::World, "stdout_file") => Method::StdoutFile,
        (Receiver::World, "stderr_file") => Method::StderrFile,
        (Receiver::World, "executable") => Method::Executable,
        (Receiver::Clock, "__lkjscript_read") => Method::ClockRead,
        (Receiver::Clock, "nanoseconds") => Method::ClockNow,
        (Receiver::Process, "spawn_file") => Method::ProcessSpawn,
        (Receiver::Process, "__lkjscript_spawn") => Method::ProcessSpawnRaw,
        (Receiver::Child, "wait") => Method::ChildWait,
        (Receiver::Child, "try_wait") => Method::ChildTryWait,
        (Receiver::Child, "__lkjscript_wait") => Method::ChildWaitRaw,
        (Receiver::Child, "terminate") => Method::ChildTerminate,
        (Receiver::Child, "__lkjscript_terminate") => Method::ChildTerminateRaw,
        (Receiver::File, "rewind") => Method::FileRewind,
        (Receiver::File, "__lkjscript_rewind") => Method::FileRewindRaw,
        (Receiver::File, "set_executable") => Method::FileExecutable,
        (Receiver::File, "duplicate" | "__lkjscript_duplicate") => Method::FileDuplicate,
        (Receiver::File, "readonly") => Method::FileReadOnly,
        (Receiver::File, "__lkjscript_readonly") => Method::FileReadOnlyRaw,
        (Receiver::File, "__lkjscript_executable") => Method::FileExecutableRaw,
        (Receiver::Directory, "open_directory") => Method::OpenDirectory,
        (Receiver::Directory, "__lkjscript_open_directory") => Method::OpenDirectoryRaw,
        (Receiver::Directory, "__lkjscript_read") => Method::DirectoryRead,
        (Receiver::Directory, "__lkjscript_rewind") => Method::DirectoryRewind,
        (Receiver::Directory, "list") => Method::DirectoryList,
        (Receiver::Directory, "create_directory") => Method::CreateDirectory,
        (Receiver::Directory, "__lkjscript_create_directory") => Method::CreateDirectoryRaw,
        (Receiver::Directory, "remove") => Method::DirectoryRemove,
        (Receiver::Directory, "remove_directory") => Method::DirectoryRemoveDirectory,
        (Receiver::Directory, "__lkjscript_remove") => Method::DirectoryRemoveRaw,
        (Receiver::Directory, "rename") => Method::DirectoryRename,
        (Receiver::Directory, "__lkjscript_rename") => Method::DirectoryRenameRaw,
        _ => return None,
    })
}
impl Method {
    pub(super) fn signature(self) -> (Vec<Value>, Value) {
        match self {
            Method::Stdout => (vec![], Value::Stdout),
            Method::Stdin => (vec![], Value::Stdin),
            Method::Stderr => (vec![], Value::Stderr),
            Method::Args => (vec![], Value::Arguments),
            Method::Directory => (vec![Value::Text], Value::ResultDirectory),
            Method::ExitStatus => (vec![Value::Byte], Value::Unit),
            Method::ArgCount => (vec![], Value::Count),
            Method::ArgLen => (vec![Value::Count], Value::Count),
            Method::ArgCopy => (vec![Value::Count, Value::MutableBytes], Value::Count),
            Method::DirectoryRaw => (vec![Value::Bytes], Value::ResultDirectory),
            Method::StdoutWrite | Method::StderrWrite => (vec![Value::Bytes], Value::Unit),
            Method::StdinRead | Method::FileRead => (vec![Value::MutableBytes], Value::ResultCount),
            Method::StdinReadRaw | Method::FileReadRaw => (vec![Value::MutableBytes], Value::Error),
            Method::StdinReadToEnd | Method::FileReadToEnd => (vec![], Value::ResultBytes),
            Method::DirectoryOpen | Method::DirectoryCreate => {
                (vec![Value::Text], Value::ResultFile)
            }
            Method::DirectoryOpenRaw => (vec![Value::Bytes, Value::Boolean], Value::ResultFile),
            Method::FileWrite => (vec![Value::Bytes], Value::ResultCount),
            Method::FileWriteRaw => (vec![Value::Bytes], Value::Error),
            Method::FileWriteAll => (vec![Value::Bytes], Value::ResultUnit),
            Method::Process => (vec![], Value::Process),
            Method::Clock => (vec![], Value::Clock),
            Method::WorldFile => (vec![Value::Byte], Value::ResultFile),
            Method::TemporaryFile => (vec![], Value::ResultFile),
            Method::StdinFile => (vec![], Value::ResultFile),
            Method::StdoutFile => (vec![], Value::ResultFile),
            Method::StderrFile => (vec![], Value::ResultFile),
            Method::Executable => (vec![], Value::ResultFile),
            Method::ClockRead => (vec![Value::MutableBytes], Value::Error),
            Method::ClockNow => (vec![], Value::ResultCount),
            Method::ProcessSpawn => (
                vec![
                    Value::FileRef,
                    Value::ArgumentsView,
                    Value::FileRef,
                    Value::FileRef,
                    Value::FileRef,
                    Value::FileRef,
                ],
                Value::ResultChild,
            ),
            Method::ProcessSpawnRaw => (
                vec![
                    Value::FileRef,
                    Value::Bytes,
                    Value::FileRef,
                    Value::FileRef,
                    Value::FileRef,
                    Value::FileRef,
                ],
                Value::ResultChild,
            ),
            Method::ChildWait => (vec![], Value::ResultStatus),
            Method::ChildTryWait => (vec![], Value::ResultMaybeStatus),
            Method::ChildWaitRaw => (vec![Value::Boolean], Value::Error),
            Method::ChildTerminate => (vec![], Value::ResultUnit),
            Method::ChildTerminateRaw => (vec![], Value::Error),
            Method::FileRewind => (vec![], Value::ResultUnit),
            Method::FileRewindRaw => (vec![], Value::Error),
            Method::FileExecutable => (vec![], Value::ResultUnit),
            Method::FileExecutableRaw => (vec![], Value::Error),
            Method::FileDuplicate | Method::FileReadOnly | Method::FileReadOnlyRaw => {
                (vec![], Value::ResultFile)
            }
            Method::OpenDirectory => (vec![Value::Text], Value::ResultDirectory),
            Method::OpenDirectoryRaw => (vec![Value::Bytes], Value::ResultDirectory),
            Method::DirectoryRead => (vec![Value::MutableBytes], Value::Error),
            Method::DirectoryRewind => (vec![], Value::Error),
            Method::DirectoryList => (vec![], Value::ResultNames),
            Method::CreateDirectory => (vec![Value::Text], Value::ResultUnit),
            Method::CreateDirectoryRaw => (vec![Value::Bytes], Value::Error),
            Method::DirectoryRemove => (vec![Value::Text], Value::ResultUnit),
            Method::DirectoryRemoveDirectory => (vec![Value::Text], Value::ResultUnit),
            Method::DirectoryRemoveRaw => (vec![Value::Bytes, Value::Boolean], Value::Error),
            Method::DirectoryRename => (vec![Value::Text, Value::Text], Value::ResultUnit),
            Method::DirectoryRenameRaw => (
                vec![Value::Bytes, Value::DirectoryRef, Value::Bytes],
                Value::Error,
            ),
        }
    }
    pub(super) fn intrinsic(self) -> Option<ir::IntrinsicOp> {
        Some(match self {
            Method::Stdout => ir::IntrinsicOp::WorldStdout,
            Method::Stdin => ir::IntrinsicOp::WorldStdin,
            Method::Stderr => ir::IntrinsicOp::WorldStderr,
            Method::ExitStatus => ir::IntrinsicOp::WorldExitStatus,
            Method::ArgCount => ir::IntrinsicOp::WorldArgCount,
            Method::ArgLen => ir::IntrinsicOp::WorldArgLen,
            Method::ArgCopy => ir::IntrinsicOp::WorldArgCopy,
            Method::DirectoryRaw => ir::IntrinsicOp::WorldDirectory,
            Method::StdoutWrite => ir::IntrinsicOp::StdoutWrite,
            Method::StderrWrite => ir::IntrinsicOp::StderrWrite,
            Method::StdinReadRaw => ir::IntrinsicOp::StdinRead,
            Method::DirectoryOpenRaw => ir::IntrinsicOp::DirectoryOpen,
            Method::FileReadRaw => ir::IntrinsicOp::FileRead,
            Method::FileWriteRaw => ir::IntrinsicOp::FileWrite,
            Method::Process => ir::IntrinsicOp::WorldProcess,
            Method::Clock => ir::IntrinsicOp::WorldClock,
            Method::WorldFile => ir::IntrinsicOp::WorldFile,
            Method::ClockRead => ir::IntrinsicOp::ClockRead,
            Method::ProcessSpawnRaw => ir::IntrinsicOp::ProcessSpawn,
            Method::ChildWaitRaw => ir::IntrinsicOp::ChildWait,
            Method::ChildTerminateRaw => ir::IntrinsicOp::ChildTerminate,
            Method::FileRewindRaw => ir::IntrinsicOp::FileRewind,
            Method::FileExecutableRaw => ir::IntrinsicOp::FileExecutable,
            Method::FileDuplicate => ir::IntrinsicOp::FileDuplicate,
            Method::FileReadOnlyRaw => ir::IntrinsicOp::FileReadOnly,
            Method::OpenDirectoryRaw => ir::IntrinsicOp::DirectoryOpenDirectory,
            Method::DirectoryRead => ir::IntrinsicOp::DirectoryRead,
            Method::DirectoryRewind => ir::IntrinsicOp::DirectoryRewind,
            Method::CreateDirectoryRaw => ir::IntrinsicOp::DirectoryCreate,
            Method::DirectoryRemoveRaw => ir::IntrinsicOp::DirectoryRemove,
            Method::DirectoryRenameRaw => ir::IntrinsicOp::DirectoryRename,
            _ => return None,
        })
    }
    pub(super) fn helper(self) -> Option<&'static str> {
        Some(match self {
            Method::Args => "__lkjscript_args",
            Method::Directory => "__lkjscript_directory",
            Method::StdinRead => "__lkjscript_stdin_read",
            Method::StdinReadToEnd => "__lkjscript_stdin_read_to_end",
            Method::DirectoryOpen => "__lkjscript_open",
            Method::DirectoryCreate => "__lkjscript_create",
            Method::FileRead => "__lkjscript_file_read",
            Method::FileWrite => "__lkjscript_file_write",
            Method::FileReadToEnd => "__lkjscript_file_read_to_end",
            Method::FileWriteAll => "__lkjscript_file_write_all",
            Method::TemporaryFile => "__lkjscript_temporary_file",
            Method::StdinFile => "__lkjscript_stdin_file",
            Method::StdoutFile => "__lkjscript_stdout_file",
            Method::StderrFile => "__lkjscript_stderr_file",
            Method::Executable => "__lkjscript_executable",
            Method::ClockNow => "__lkjscript_clock_nanoseconds",
            Method::ProcessSpawn => "__lkjscript_process_spawn",
            Method::ChildWait => "__lkjscript_child_wait",
            Method::ChildTryWait => "__lkjscript_child_try_wait",
            Method::ChildTerminate => "__lkjscript_child_terminate",
            Method::FileRewind => "__lkjscript_file_rewind",
            Method::FileExecutable => "__lkjscript_file_executable",
            Method::FileReadOnly => "__lkjscript_file_readonly",
            Method::OpenDirectory => "__lkjscript_open_directory",
            Method::DirectoryList => "__lkjscript_directory_list",
            Method::CreateDirectory => "__lkjscript_create_directory",
            Method::DirectoryRemove => "__lkjscript_directory_remove",
            Method::DirectoryRemoveDirectory => "__lkjscript_directory_remove_directory",
            Method::DirectoryRename => "__lkjscript_directory_rename",
            _ => return None,
        })
    }
}

fn reserved(name: &str) -> bool {
    let receivers = [
        Receiver::World,
        Receiver::Stdout,
        Receiver::Stdin,
        Receiver::Stderr,
        Receiver::Directory,
        Receiver::File,
        Receiver::Process,
        Receiver::Clock,
        Receiver::Child,
    ];
    let mut index = 0;
    while index < receivers.len() {
        if method(receivers[index], name).is_some() {
            return true;
        }
        index += 1;
    }
    false
}

impl super::Compiler<'_> {
    pub(super) fn check_capability_methods(&mut self) -> super::Result<()> {
        let mut implementation_index = 0;
        while implementation_index < self.source.impls.len() {
            let implementation = self.source.impls[implementation_index].clone();
            implementation_index += 1;
            if implementation.trait_ty.is_some() {
                continue;
            }
            let mut relevant = false;
            for method in &implementation.methods {
                if reserved(&self.source.functions[*method].name) {
                    relevant = true;
                }
            }
            if !relevant {
                continue;
            }
            let Ok(target) =
                self.resolve(implementation.target, &std::collections::BTreeMap::new())
            else {
                continue;
            };
            let receiver = super::methods::receiver(&self.types[target].kind);
            for index in &implementation.methods {
                let function = &self.source.functions[*index];
                if method(receiver, &function.name).is_some() {
                    return Err(super::err(
                        function.span,
                        "E_TRAIT",
                        &format!(
                            "inherent method {} duplicates builtin {}::{}; help: rename it or define an extension trait",
                            function.name,
                            self.display_type(target),
                            function.name
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}
