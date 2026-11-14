//! One public/raw signature table drives capability inference and construction.
use super::methods::Receiver;
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
}
pub(super) fn method(receiver: Receiver, name: &str) -> Option<Method> {
    Some(match (receiver, name) {
        (Receiver::World, "stdout") => Method::Stdout,
        (Receiver::World, "stdin") => Method::Stdin,
        (Receiver::World, "stderr") => Method::Stderr,
        (Receiver::World, "args") => Method::Args,
        (Receiver::World, "directory") => Method::Directory,
        (Receiver::World, "set_exit_status") => Method::ExitStatus,
        (Receiver::World, "__lkj_arg_count") => Method::ArgCount,
        (Receiver::World, "__lkj_arg_len") => Method::ArgLen,
        (Receiver::World, "__lkj_arg_copy") => Method::ArgCopy,
        (Receiver::World, "__lkj_directory") => Method::DirectoryRaw,
        (Receiver::Stdout, "write") => Method::StdoutWrite,
        (Receiver::Stderr, "write") => Method::StderrWrite,
        (Receiver::Stdin, "read") => Method::StdinRead,
        (Receiver::Stdin, "__lkj_read") => Method::StdinReadRaw,
        (Receiver::Stdin, "read_to_end") => Method::StdinReadToEnd,
        (Receiver::Directory, "open") => Method::DirectoryOpen,
        (Receiver::Directory, "create") => Method::DirectoryCreate,
        (Receiver::Directory, "__lkj_open") => Method::DirectoryOpenRaw,
        (Receiver::File, "read") => Method::FileRead,
        (Receiver::File, "write") => Method::FileWrite,
        (Receiver::File, "__lkj_read") => Method::FileReadRaw,
        (Receiver::File, "__lkj_write") => Method::FileWriteRaw,
        (Receiver::File, "read_to_end") => Method::FileReadToEnd,
        (Receiver::File, "write_all") => Method::FileWriteAll,
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
        }
    }
    pub(super) fn intrinsic(self) -> Option<&'static str> {
        Some(match self {
            Method::Stdout => "world_stdout",
            Method::Stdin => "world_stdin",
            Method::Stderr => "world_stderr",
            Method::ExitStatus => "world_exit_status",
            Method::ArgCount => "world_arg_count",
            Method::ArgLen => "world_arg_len",
            Method::ArgCopy => "world_arg_copy",
            Method::DirectoryRaw => "world_directory",
            Method::StdoutWrite => "stdout_write",
            Method::StderrWrite => "stderr_write",
            Method::StdinReadRaw => "stdin_read",
            Method::DirectoryOpenRaw => "directory_open",
            Method::FileReadRaw => "file_read",
            Method::FileWriteRaw => "file_write",
            _ => return None,
        })
    }
    pub(super) fn helper(self) -> Option<&'static str> {
        Some(match self {
            Method::Args => "__lkj_args",
            Method::Directory => "__lkj_directory",
            Method::StdinRead => "__lkj_stdin_read",
            Method::StdinReadToEnd => "__lkj_stdin_read_to_end",
            Method::DirectoryOpen => "__lkj_open",
            Method::DirectoryCreate => "__lkj_create",
            Method::FileRead => "__lkj_file_read",
            Method::FileWrite => "__lkj_file_write",
            Method::FileReadToEnd => "__lkj_file_read_to_end",
            Method::FileWriteAll => "__lkj_file_write_all",
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
