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
pub enum PreEntryError {
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
    EntryAddressRepresentation,
    BookkeepingAllocationFailed,
    NativeStackUnavailable(NativeStackError),
    Cancelled,
    DeadlineExceeded,
    ResourceLimitExceeded(NativeResourceLimitKind),
    ExecutionDomain,
}

impl fmt::Display for PreEntryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEntry => formatter.write_str("unknown native entry"),
            Self::ArgumentCount { expected, actual } => write!(
                formatter,
                "native entry expected {expected} arguments, received {actual}"
            ),
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
            Self::EntryAddressRepresentation => {
                formatter.write_str("native entry offset exceeds host address representation")
            }
            Self::BookkeepingAllocationFailed => {
                formatter.write_str("native pre-entry bookkeeping allocation failed")
            }
            Self::NativeStackUnavailable(error) => {
                write!(formatter, "native stack pre-entry check failed: {error}")
            }
            Self::Cancelled => formatter.write_str("native invocation was cancelled before entry"),
            Self::DeadlineExceeded => {
                formatter.write_str("native invocation deadline expired before entry")
            }
            Self::ResourceLimitExceeded(kind) => {
                write!(
                    formatter,
                    "native invocation policy declined entry: {kind:?}"
                )
            }
            Self::ExecutionDomain => {
                formatter.write_str("native invocation selected the wrong execution domain")
            }
        }
    }
}

impl std::error::Error for PreEntryError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnteredInvocationError {
    InvalidNativeStatus(u32),
    InvalidNativeTrap(u32),
    InvalidBoolReturn(u64),
    InvalidNativeReturn,
    BookkeepingAllocationFailed,
    NativeStackViolation(NativeStackError),
    InvalidNativeEntryAccounting(u64),
    InvalidActiveFrame,
    LeakedActiveFrames(usize),
}

impl fmt::Display for EnteredInvocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Self::InvalidNativeReturn => {
                formatter.write_str("generated code returned an invalid typed value")
            }
            Self::BookkeepingAllocationFailed => {
                formatter.write_str("entered native invocation bookkeeping allocation failed")
            }
            Self::NativeStackViolation(error) => {
                write!(
                    formatter,
                    "entered native invocation reached a stack boundary: {error}"
                )
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
        }
    }
}

impl std::error::Error for EnteredInvocationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeStackError {
    ThreadExtentUnavailable,
    FrameArithmeticOverflow,
    FrameOutsideThreadExtent,
    GuardReached,
}

impl fmt::Display for NativeStackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ThreadExtentUnavailable => "current thread stack extent is unavailable",
            Self::FrameArithmeticOverflow => "native frame arithmetic exceeds host representation",
            Self::FrameOutsideThreadExtent => {
                "generated frame base is outside the discovered thread stack extent"
            }
            Self::GuardReached => "generated frame would reach the discovered thread stack guard",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeResourceLimitKind {
    PollFuel,
    ActiveFrames,
    RuntimeService,
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
