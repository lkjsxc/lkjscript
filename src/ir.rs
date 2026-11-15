//! Explicit, monomorphic meaning. Only CheckedProgram reaches a backend.
#[derive(Clone, PartialEq)]
pub enum TypeKind {
    Unit,
    Bool,
    /// A Unicode scalar value: 0..=0x10ffff excluding 0xd800..=0xdfff.
    Char,
    Int {
        bits: u8,
        signed: bool,
    },
    F64,
    Struct {
        fields: Vec<Field>,
        copy: bool,
    },
    Enum {
        variants: Vec<Variant>,
        copy: bool,
    },
    Ref {
        inner: usize,
        mutable: bool,
    },
    Slice {
        inner: usize,
        mutable: bool,
    },
    Array {
        inner: usize,
        len: usize,
    },
    Vec {
        inner: usize,
    },
    Box {
        inner: usize,
    },
    /// Unsized immutable UTF-8 text; stored values use a shared Ref.
    Str,
    World,
    /// Affine access to invocation arguments, one environment name, or exit status.
    Arguments,
    Environment,
    ExitStatus,
    /// Affine authority to write standard output, derived from World.
    Stdout,
    Stdin,
    Stderr,
    /// Affine directory authority; descendants remain beneath this root.
    Directory,
    /// Affine open file; structural destruction closes its handle.
    File,
    Process,
    Clock,
    Child,
    /// Affine IPv4 TCP listener; structural destruction closes its handle.
    Listener,
    /// Affine authority to connect to one numeric IPv4 endpoint.
    Connector,
    /// Affine readiness set; destruction closes only its kernel event handle.
    Readiness,
    /// Affine authority to obtain kernel random bytes.
    Entropy,
    /// Affine SIGTERM endpoint; consumption is sticky within this owned value.
    Stop,
}

/// Explicit host provisioning data; executors validate every descriptor against
/// the selected entry's parameter type before creating owned values.
#[derive(Clone)]
pub enum EntryGrant {
    World,
    Arguments,
    /// Exactly one NUL-terminated environment name; never an enumeration grant.
    Environment(Vec<u8>),
    /// Launcher snapshot for native run; standalone builds retain the named grant.
    EnvironmentValue(Option<Vec<u8>>),
    ExitStatus,
    Stdout,
    Stdin,
    Stderr,
    Clock,
    Process,
    /// NUL-terminated path, matching the sealed WorldDirectory ABI.
    Directory(Vec<u8>),
    /// Directory selected at invocation by exactly one named environment value.
    DirectoryEnvironment(Vec<u8>),
    /// Exact sixteen-byte IPv4 sockaddr, matching WorldListen.
    Listener(Vec<u8>),
    Connector(Vec<u8>),
    Readiness,
    Entropy,
    Stop,
}

#[derive(Clone, PartialEq)]
pub struct Field {

    pub name: String,
    pub ty: usize,
}
#[derive(Clone, PartialEq)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
}
#[derive(Clone, PartialEq)]
pub struct Type {
    pub name: String,
    pub kind: TypeKind,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Projection {
    Deref,
    Field(usize),
    VariantField(usize, usize),
    Index(usize),
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Place {
    pub local: usize,
    pub projections: Vec<Projection>,
}
impl Place {
    pub fn local(local: usize) -> Place {
        Place {
            local,
            projections: Vec::new(),
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct Local {
    pub name: String,
    pub ty: usize,
    pub mutable: bool,
}
/// Closed operation vocabulary; textual spellings are only an interchange format.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}
impl UnaryOp {
    pub fn parse(text: &str) -> Option<UnaryOp> {
        match text {
            "-" => Some(UnaryOp::Neg),
            "!" => Some(UnaryOp::Not),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
        }
    }
}
/// Closed operation vocabulary; textual spellings are only an interchange format.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    WrappingAdd,
    WrappingSub,
    WrappingMul,
}
impl BinaryOp {
    pub fn parse(text: &str) -> Option<BinaryOp> {
        match text {
            "+" => Some(BinaryOp::Add),
            "-" => Some(BinaryOp::Sub),
            "*" => Some(BinaryOp::Mul),
            "/" => Some(BinaryOp::Div),
            "%" => Some(BinaryOp::Rem),
            "&" => Some(BinaryOp::BitAnd),
            "|" => Some(BinaryOp::BitOr),
            "^" => Some(BinaryOp::BitXor),
            "<<" => Some(BinaryOp::Shl),
            ">>" => Some(BinaryOp::Shr),
            "==" => Some(BinaryOp::Eq),
            "!=" => Some(BinaryOp::Ne),
            "<" => Some(BinaryOp::Lt),
            "<=" => Some(BinaryOp::Le),
            ">" => Some(BinaryOp::Gt),
            ">=" => Some(BinaryOp::Ge),
            "&&" => Some(BinaryOp::And),
            "||" => Some(BinaryOp::Or),
            "wrapping_add" => Some(BinaryOp::WrappingAdd),
            "wrapping_sub" => Some(BinaryOp::WrappingSub),
            "wrapping_mul" => Some(BinaryOp::WrappingMul),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::Shl => "<<",
            BinaryOp::Shr => ">>",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
            BinaryOp::WrappingAdd => "wrapping_add",
            BinaryOp::WrappingSub => "wrapping_sub",
            BinaryOp::WrappingMul => "wrapping_mul",
        }
    }
}
/// Closed operation vocabulary; textual spellings are only an interchange format.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IntrinsicOp {
    CharFromU32,
    F64FromBits,
    F64ToBits,
    ArrayRepeat,
    StrLen,
    StrAsBytes,
    StrFromUtf8,
    Assert,
    VecNew,
    VecLen,
    VecPush,
    VecPop,
    Replace,
    BoxNew,
    Cast,
    Clone,
    WorldStdout,
    WorldStdin,
    WorldStderr,
    WorldArgCount,
    WorldArgLen,
    WorldArgCopy,
    WorldExitStatus,
    EnvironmentLen,
    EnvironmentCopy,
    WorldDirectory,
    DirectoryOpen,
    StdinRead,
    FileRead,
    FileWrite,
    StdoutWrite,
    StderrWrite,
    WorldProcess,
    WorldClock,
    WorldFile,
    ClockRead,
    ProcessSpawn,
    ChildWait,
    ChildTerminate,
    DirectoryOpenDirectory,
    DirectoryRead,
    DirectoryRewind,
    DirectoryCreate,
    DirectoryRemove,
    DirectoryRename,
    FileRewind,
    FileExecutable,
    FileDuplicate,
    FileReadOnly,
    WorldListen,
    ListenerAccept,
    ListenerAcceptNoDelay,
    WorldConnector,
    ConnectorConnect,
    ListenerAddress,
    WorldReadiness,
    FileSetNonblocking,
    ListenerSetNonblocking,
    ReadinessWatchFile,
    ReadinessWatchListener,
    ReadinessUnwatchFile,
    ReadinessUnwatchListener,
    ReadinessWait,
    DirectoryCreateNew,
    DirectoryOpenAppend,
    DirectorySync,
    FileSync,
    FileTruncate,
    FileLockExclusive,
    FileLockWait,
    WorldEntropy,
    EntropyFill,
    ClockUnixSeconds,
    ClockUnixNanoseconds,
    FileSetPrivate,
    WorldStop,
    StopRequested,
    StopTarget,
    ReadinessWatchStop,
    ReadinessUnwatchStop,
    ChildRequestStop,
    DirectoryCreatePrivate,
}
impl IntrinsicOp {
    pub fn parse(text: &str) -> Option<IntrinsicOp> {
        match text {
            "char_from_u32" => Some(IntrinsicOp::CharFromU32),
            "f64_from_bits" => Some(IntrinsicOp::F64FromBits),
            "f64_to_bits" => Some(IntrinsicOp::F64ToBits),
            "array_repeat" => Some(IntrinsicOp::ArrayRepeat),
            "str_len" => Some(IntrinsicOp::StrLen),
            "str_as_bytes" => Some(IntrinsicOp::StrAsBytes),
            "str_from_utf8" => Some(IntrinsicOp::StrFromUtf8),
            "assert" => Some(IntrinsicOp::Assert),
            "vec_new" => Some(IntrinsicOp::VecNew),
            "vec_len" => Some(IntrinsicOp::VecLen),
            "vec_push" => Some(IntrinsicOp::VecPush),
            "vec_pop" => Some(IntrinsicOp::VecPop),
            "replace" => Some(IntrinsicOp::Replace),
            "box_new" => Some(IntrinsicOp::BoxNew),
            "cast" => Some(IntrinsicOp::Cast),
            "clone" => Some(IntrinsicOp::Clone),
            "world_stdout" => Some(IntrinsicOp::WorldStdout),
            "world_stdin" => Some(IntrinsicOp::WorldStdin),
            "world_stderr" => Some(IntrinsicOp::WorldStderr),
            "world_arg_count" => Some(IntrinsicOp::WorldArgCount),
            "world_arg_len" => Some(IntrinsicOp::WorldArgLen),
            "world_arg_copy" => Some(IntrinsicOp::WorldArgCopy),
            "world_exit_status" => Some(IntrinsicOp::WorldExitStatus),
            "environment_len" => Some(IntrinsicOp::EnvironmentLen),
            "environment_copy" => Some(IntrinsicOp::EnvironmentCopy),
            "world_directory" => Some(IntrinsicOp::WorldDirectory),
            "directory_open" => Some(IntrinsicOp::DirectoryOpen),
            "stdin_read" => Some(IntrinsicOp::StdinRead),
            "file_read" => Some(IntrinsicOp::FileRead),
            "file_write" => Some(IntrinsicOp::FileWrite),
            "stdout_write" => Some(IntrinsicOp::StdoutWrite),
            "stderr_write" => Some(IntrinsicOp::StderrWrite),
            "world_process" => Some(IntrinsicOp::WorldProcess),
            "world_clock" => Some(IntrinsicOp::WorldClock),
            "world_file" => Some(IntrinsicOp::WorldFile),
            "clock_read" => Some(IntrinsicOp::ClockRead),
            "process_spawn" => Some(IntrinsicOp::ProcessSpawn),
            "child_wait" => Some(IntrinsicOp::ChildWait),
            "child_terminate" => Some(IntrinsicOp::ChildTerminate),
            "directory_open_directory" => Some(IntrinsicOp::DirectoryOpenDirectory),
            "directory_read" => Some(IntrinsicOp::DirectoryRead),
            "directory_rewind" => Some(IntrinsicOp::DirectoryRewind),
            "directory_create" => Some(IntrinsicOp::DirectoryCreate),
            "directory_remove" => Some(IntrinsicOp::DirectoryRemove),
            "directory_rename" => Some(IntrinsicOp::DirectoryRename),
            "file_rewind" => Some(IntrinsicOp::FileRewind),
            "file_executable" => Some(IntrinsicOp::FileExecutable),
            "file_duplicate" => Some(IntrinsicOp::FileDuplicate),
            "file_readonly" => Some(IntrinsicOp::FileReadOnly),
            "world_listen" => Some(IntrinsicOp::WorldListen),
            "world_connector" => Some(IntrinsicOp::WorldConnector),
            "connector_connect" => Some(IntrinsicOp::ConnectorConnect),
            "listener_accept" => Some(IntrinsicOp::ListenerAccept),
            "listener_accept_nodelay" => Some(IntrinsicOp::ListenerAcceptNoDelay),
            "listener_address" => Some(IntrinsicOp::ListenerAddress),
            "world_readiness" => Some(IntrinsicOp::WorldReadiness),
            "file_set_nonblocking" => Some(IntrinsicOp::FileSetNonblocking),
            "listener_set_nonblocking" => Some(IntrinsicOp::ListenerSetNonblocking),
            "readiness_watch_file" => Some(IntrinsicOp::ReadinessWatchFile),
            "readiness_watch_listener" => Some(IntrinsicOp::ReadinessWatchListener),
            "readiness_unwatch_file" => Some(IntrinsicOp::ReadinessUnwatchFile),
            "readiness_unwatch_listener" => Some(IntrinsicOp::ReadinessUnwatchListener),
            "readiness_wait" => Some(IntrinsicOp::ReadinessWait),
            "directory_create_new" => Some(IntrinsicOp::DirectoryCreateNew),
            "directory_open_append" => Some(IntrinsicOp::DirectoryOpenAppend),
            "directory_sync" => Some(IntrinsicOp::DirectorySync),
            "file_sync" => Some(IntrinsicOp::FileSync),
            "file_truncate" => Some(IntrinsicOp::FileTruncate),
            "file_lock_exclusive" => Some(IntrinsicOp::FileLockExclusive),
            "file_lock_wait" => Some(IntrinsicOp::FileLockWait),
            "world_entropy" => Some(IntrinsicOp::WorldEntropy),
            "entropy_fill" => Some(IntrinsicOp::EntropyFill),
            "clock_unix_seconds" => Some(IntrinsicOp::ClockUnixSeconds),
            "clock_unix_nanoseconds" => Some(IntrinsicOp::ClockUnixNanoseconds),
            "file_set_private" => Some(IntrinsicOp::FileSetPrivate),
            "world_stop" => Some(IntrinsicOp::WorldStop),
            "stop_requested" => Some(IntrinsicOp::StopRequested),
            "stop_target" => Some(IntrinsicOp::StopTarget),
            "readiness_watch_stop" => Some(IntrinsicOp::ReadinessWatchStop),
            "readiness_unwatch_stop" => Some(IntrinsicOp::ReadinessUnwatchStop),
            "child_request_stop" => Some(IntrinsicOp::ChildRequestStop),
            "directory_create_private" => Some(IntrinsicOp::DirectoryCreatePrivate),

            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            IntrinsicOp::CharFromU32 => "char_from_u32",
            IntrinsicOp::F64FromBits => "f64_from_bits",
            IntrinsicOp::F64ToBits => "f64_to_bits",
            IntrinsicOp::ArrayRepeat => "array_repeat",
            IntrinsicOp::StrLen => "str_len",
            IntrinsicOp::StrAsBytes => "str_as_bytes",
            IntrinsicOp::StrFromUtf8 => "str_from_utf8",
            IntrinsicOp::Assert => "assert",
            IntrinsicOp::VecNew => "vec_new",
            IntrinsicOp::VecLen => "vec_len",
            IntrinsicOp::VecPush => "vec_push",
            IntrinsicOp::VecPop => "vec_pop",
            IntrinsicOp::Replace => "replace",
            IntrinsicOp::BoxNew => "box_new",
            IntrinsicOp::Cast => "cast",
            IntrinsicOp::Clone => "clone",
            IntrinsicOp::WorldStdout => "world_stdout",
            IntrinsicOp::WorldStdin => "world_stdin",
            IntrinsicOp::WorldStderr => "world_stderr",
            IntrinsicOp::WorldArgCount => "world_arg_count",
            IntrinsicOp::WorldArgLen => "world_arg_len",
            IntrinsicOp::WorldArgCopy => "world_arg_copy",
            IntrinsicOp::WorldExitStatus => "world_exit_status",
            IntrinsicOp::EnvironmentLen => "environment_len",
            IntrinsicOp::EnvironmentCopy => "environment_copy",
            IntrinsicOp::WorldDirectory => "world_directory",
            IntrinsicOp::DirectoryOpen => "directory_open",
            IntrinsicOp::StdinRead => "stdin_read",
            IntrinsicOp::FileRead => "file_read",
            IntrinsicOp::FileWrite => "file_write",
            IntrinsicOp::StdoutWrite => "stdout_write",
            IntrinsicOp::StderrWrite => "stderr_write",
            IntrinsicOp::WorldProcess => "world_process",
            IntrinsicOp::WorldClock => "world_clock",
            IntrinsicOp::WorldFile => "world_file",
            IntrinsicOp::ClockRead => "clock_read",
            IntrinsicOp::ProcessSpawn => "process_spawn",
            IntrinsicOp::ChildWait => "child_wait",
            IntrinsicOp::ChildTerminate => "child_terminate",
            IntrinsicOp::DirectoryOpenDirectory => "directory_open_directory",
            IntrinsicOp::DirectoryRead => "directory_read",
            IntrinsicOp::DirectoryRewind => "directory_rewind",
            IntrinsicOp::DirectoryCreate => "directory_create",
            IntrinsicOp::DirectoryRemove => "directory_remove",
            IntrinsicOp::DirectoryRename => "directory_rename",
            IntrinsicOp::FileRewind => "file_rewind",
            IntrinsicOp::FileExecutable => "file_executable",
            IntrinsicOp::FileDuplicate => "file_duplicate",
            IntrinsicOp::FileReadOnly => "file_readonly",
            IntrinsicOp::WorldListen => "world_listen",
            IntrinsicOp::WorldConnector => "world_connector",
            IntrinsicOp::ConnectorConnect => "connector_connect",
            IntrinsicOp::ListenerAccept => "listener_accept",
            IntrinsicOp::ListenerAcceptNoDelay => "listener_accept_nodelay",
            IntrinsicOp::ListenerAddress => "listener_address",
            IntrinsicOp::WorldReadiness => "world_readiness",
            IntrinsicOp::FileSetNonblocking => "file_set_nonblocking",
            IntrinsicOp::ListenerSetNonblocking => "listener_set_nonblocking",
            IntrinsicOp::ReadinessWatchFile => "readiness_watch_file",
            IntrinsicOp::ReadinessWatchListener => "readiness_watch_listener",
            IntrinsicOp::ReadinessUnwatchFile => "readiness_unwatch_file",
            IntrinsicOp::ReadinessUnwatchListener => "readiness_unwatch_listener",
            IntrinsicOp::ReadinessWait => "readiness_wait",
            IntrinsicOp::DirectoryCreateNew => "directory_create_new",
            IntrinsicOp::DirectoryOpenAppend => "directory_open_append",
            IntrinsicOp::DirectorySync => "directory_sync",
            IntrinsicOp::FileSync => "file_sync",
            IntrinsicOp::FileTruncate => "file_truncate",
            IntrinsicOp::FileLockExclusive => "file_lock_exclusive",
            IntrinsicOp::FileLockWait => "file_lock_wait",
            IntrinsicOp::WorldEntropy => "world_entropy",
            IntrinsicOp::EntropyFill => "entropy_fill",
            IntrinsicOp::ClockUnixSeconds => "clock_unix_seconds",
            IntrinsicOp::ClockUnixNanoseconds => "clock_unix_nanoseconds",
            IntrinsicOp::FileSetPrivate => "file_set_private",
            IntrinsicOp::WorldStop => "world_stop",
            IntrinsicOp::StopRequested => "stop_requested",
            IntrinsicOp::StopTarget => "stop_target",
            IntrinsicOp::ReadinessWatchStop => "readiness_watch_stop",
            IntrinsicOp::ReadinessUnwatchStop => "readiness_unwatch_stop",
            IntrinsicOp::ChildRequestStop => "child_request_stop",
            IntrinsicOp::DirectoryCreatePrivate => "directory_create_private",

        }
    }
}
#[derive(Clone, PartialEq)]
pub enum Op {
    /// Immutable module-long bytes: a UTF-8 str or an exact-length u8 array.
    Text {
        dst: usize,
        bytes: Vec<u8>,
    },
    Int {
        dst: usize,
        value: i64,
    },
    Float {
        dst: usize,
        bits: u64,
    },
    Unit {
        dst: usize,
    },
    Read {
        dst: usize,
        place: Place,
        moving: bool,
    },
    Borrow {
        dst: usize,
        place: Place,
        mutable: bool,
    },
    SliceBorrow {
        dst: usize,
        place: Place,
        mutable: bool,
    },
    /// Borrow a checked subrange without narrowing its static alias footprint.
    /// Bounds are consumed usize values; failure traps with the bounds code.
    ViewRange {
        dst: usize,
        place: Place,
        start: usize,
        end: usize,
        inclusive: bool,
        mutable: bool,
    },
    Array {
        dst: usize,
        elements: Vec<usize>,
    },
    Assign {
        place: Place,
        src: usize,
    },
    Drop {
        place: Place,
    },
    /// Destroy this place only when its own storage is initialized. Every
    /// projected ancestor must still exist; this does not authorize a hole
    /// through a reference or an indexed container.
    DropIf {
        place: Place,
    },
    Unary {
        dst: usize,
        op: UnaryOp,
        src: usize,
    },
    Binary {
        dst: usize,
        op: BinaryOp,
        left: usize,
        right: usize,
    },
    Call {
        dst: usize,
        function: usize,
        args: Vec<usize>,
    },
    Struct {
        dst: usize,
        fields: Vec<usize>,
    },
    Variant {
        dst: usize,
        variant: usize,
        fields: Vec<usize>,
    },
    Discriminant {
        dst: usize,
        src: usize,
    },
    TagTest {
        dst: usize,
        place: Place,
        variant: usize,
    },
    Intrinsic {
        dst: usize,
        name: IntrinsicOp,
        args: Vec<usize>,
    },
    If {
        condition: usize,
    },
    Else,
    End,
    Loop,
    Break,
    Continue,
    /// Select an enclosing lexical loop, counting only loops (zero is nearest).
    BreakTo {
        depth: usize,
    },
    ContinueTo {
        depth: usize,
    },
    Return {
        src: usize,
    },
    Trap {
        message: String,
    },
}
#[derive(Clone, PartialEq)]
pub struct Instruction {
    pub op: Op,
    /// None inherits the enclosing function's source file.
    pub file: Option<usize>,
    pub line: usize,
    pub column: usize,
}
/// An input selector names one stored reference leaf in a parameter value.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InputLeaf {
    pub index: usize,
    pub projections: Vec<Projection>,
}
/// Every result reference leaf selects a declared input alternative or static storage.
#[derive(Clone, PartialEq, Eq)]
pub enum LeafOrigin {
    Inputs {
        inputs: Vec<InputLeaf>,
        /// Exact preserves an input referent; bounded permits a descendant.
        exact: bool,
    },
    Static,
}
#[derive(Clone, PartialEq, Eq)]
pub struct ReturnLeaf {
    pub projections: Vec<Projection>,
    pub origin: LeafOrigin,
}
#[derive(Clone, PartialEq, Eq)]
pub enum ReturnSource {
    /// A complete, unique contract for every reference leaf in the result.
    Leaves(Vec<ReturnLeaf>),
    /// Reference leaves in a parameter value, after the declared dereferences.
    /// Projected sources currently require shared leaves; exclusive reborrows
    /// retain the lifetime of their outer reference.
    Parameter {
        index: usize,
        projections: Vec<Projection>,
    },
    Static,
}
#[derive(Clone, PartialEq)]
pub struct Function {
    pub file: usize,
    pub name: String,
    pub params: usize,
    pub result: usize,
    pub return_source: Option<ReturnSource>,
    pub locals: Vec<Local>,
    pub body: Vec<Instruction>,
    pub test: bool,
}
#[derive(Clone, PartialEq)]
pub struct Program {
    pub types: Vec<Type>,
    pub functions: Vec<Function>,
    pub destructors: Vec<(usize, usize)>,
}
pub struct CheckedProgram {
    pub(crate) program: Program,
}
impl CheckedProgram {
    pub fn program(&self) -> &Program {
        &self.program
    }
}
pub const UNIT: usize = 0;
pub const BOOL: usize = 1;
pub const I32: usize = 2;
pub const I64: usize = 3;
pub const U64: usize = 4;
pub const U8: usize = 5;
pub const F64: usize = 6;
pub fn primitives() -> Vec<Type> {
    vec![
        Type {
            name: "()".to_string(),
            kind: TypeKind::Unit,
        },
        Type {
            name: "bool".to_string(),
            kind: TypeKind::Bool,
        },
        Type {
            name: "i32".to_string(),
            kind: TypeKind::Int {
                bits: 32,
                signed: true,
            },
        },
        Type {
            name: "i64".to_string(),
            kind: TypeKind::Int {
                bits: 64,
                signed: true,
            },
        },
        Type {
            name: "usize".to_string(),
            kind: TypeKind::Int {
                bits: 64,
                signed: false,
            },
        },
        Type {
            name: "u8".to_string(),
            kind: TypeKind::Int {
                bits: 8,
                signed: false,
            },
        },
        Type {
            name: "f64".to_string(),
            kind: TypeKind::F64,
        },
    ]
}
