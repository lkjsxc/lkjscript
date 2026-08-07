use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstallError {
    UnsupportedPlatform,
    InvalidImage(ImageIntegrityError),
    ContractMismatch {
        expected: Box<ImageContracts>,
        actual: Box<ImageContracts>,
    },
    LimitExceeded(ExecutableLimitKind),
    AllocationFailed,
    RelocationRange,
    RelocationAddress,
    ProtectionFailed,
}

impl fmt::Display for InstallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("native executable images are supported only on Linux x86-64")
            }
            Self::InvalidImage(error) => write!(formatter, "invalid installable image: {error}"),
            Self::ContractMismatch { expected, actual } => write!(
                formatter,
                "native image contract mismatch: expected {expected:?}, received {actual:?}; rebuild the image"
            ),
            Self::LimitExceeded(kind) => {
                write!(formatter, "executable install limit exceeded: {kind:?}")
            }
            Self::AllocationFailed => formatter.write_str("RW executable-image allocation failed"),
            Self::RelocationRange => {
                formatter.write_str("executable-image relocation is out of range")
            }
            Self::RelocationAddress => {
                formatter.write_str("executable-image relocation address is invalid")
            }
            Self::ProtectionFailed => {
                formatter.write_str("RW to RX executable-image transition failed")
            }
        }
    }
}

impl std::error::Error for InstallError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InvocationError {
    UnknownEntry,
    ArgumentCount {
        expected: usize,
        actual: usize,
    },
    ArgumentType {
        index: usize,
        expected: Box<ValueType>,
        actual: Box<ValueType>,
    },
    UnsupportedSignature,
    InvalidNativeStatus(u32),
    InvalidNativeTrap(u32),
    InvalidBoolReturn(u64),
    NativeBookkeepingAllocationFailed,
    InvalidNativeEntryAccounting(u64),
    InvalidActiveFrame,
    LeakedActiveFrames(usize),
    ExecutionDomain,
}

impl fmt::Display for InvocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEntry => formatter.write_str("unknown native entry"),
            Self::ArgumentCount { expected, actual } => {
                write!(
                    formatter,
                    "native entry expected {expected} arguments, received {actual}"
                )
            }
            Self::ArgumentType {
                index,
                expected,
                actual,
            } => write!(
                formatter,
                "native argument {index} has type {actual:?}; expected {expected:?}"
            ),
            Self::UnsupportedSignature => {
                formatter.write_str("native entry signature is unsupported")
            }
            Self::InvalidNativeStatus(status) => {
                write!(formatter, "generated code returned invalid status {status}")
            }
            Self::InvalidNativeTrap(trap) => {
                write!(formatter, "generated code returned invalid trap {trap}")
            }
            Self::InvalidBoolReturn(value) => write!(
                formatter,
                "generated code returned non-canonical Bool {value}"
            ),
            Self::NativeBookkeepingAllocationFailed => {
                formatter.write_str("native invocation bookkeeping allocation failed")
            }
            Self::InvalidNativeEntryAccounting(source) => write!(
                formatter,
                "generated code accounted an unknown native source function {source}"
            ),
            Self::InvalidActiveFrame => {
                formatter.write_str("generated active-frame metadata is invalid")
            }
            Self::LeakedActiveFrames(depth) => {
                write!(
                    formatter,
                    "generated invocation leaked {depth} active frames"
                )
            }
            Self::ExecutionDomain => {
                formatter.write_str("native invocation selected the wrong execution domain")
            }
        }
    }
}

impl std::error::Error for InvocationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeResourceLimitKind {
    PollFuel,
    ActiveFrames,
    RuntimeService,
    NativeStackBytes,
    ActiveValues,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InvocationOutcome {
    Returned(NativeValue),
    Trapped(TrapCode),
    Exited(i64),
    DeadlineExceeded,
    ResourceLimitExceeded(NativeResourceLimitKind),
    HostFailure,
}
