use super::{NUMERIC_ERROR_VARIANTS, SYSTEM_ERROR_VARIANTS, UTF8_ERROR_VARIANTS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreludeEnum {
    Option,
    Result,
    NumericError,
    Utf8Error,
    SystemError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Utf8ErrorKind {
    UnexpectedContinuation,
    InvalidLeadingByte,
    MissingContinuation,
    OverlongEncoding,
    Surrogate,
    OutOfRange,
}

impl Utf8ErrorKind {
    pub const ALL: [Self; 6] = [
        Self::UnexpectedContinuation,
        Self::InvalidLeadingByte,
        Self::MissingContinuation,
        Self::OverlongEncoding,
        Self::Surrogate,
        Self::OutOfRange,
    ];

    pub const fn index(self) -> usize {
        match self {
            Self::UnexpectedContinuation => 0,
            Self::InvalidLeadingByte => 1,
            Self::MissingContinuation => 2,
            Self::OverlongEncoding => 3,
            Self::Surrogate => 4,
            Self::OutOfRange => 5,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::UnexpectedContinuation => "unexpected-continuation",
            Self::InvalidLeadingByte => "invalid-leading-byte",
            Self::MissingContinuation => "missing-continuation",
            Self::OverlongEncoding => "overlong-encoding",
            Self::Surrogate => "surrogate",
            Self::OutOfRange => "out-of-range",
        }
    }

    pub const fn variant_id(self) -> [u8; 32] {
        UTF8_ERROR_VARIANTS[self.index()]
    }

    pub const fn physical_tag(self) -> u64 {
        match self {
            Self::UnexpectedContinuation => 1,
            Self::InvalidLeadingByte => 0,
            Self::MissingContinuation => 4,
            Self::OverlongEncoding => 5,
            Self::Surrogate => 2,
            Self::OutOfRange => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Utf8Failure {
    pub offset: usize,
    pub kind: Utf8ErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemErrorKind {
    Io,
    Network,
    Terminal,
    Time,
    Random,
    Sqlite,
    Utf8,
    Unsupported,
}

impl SystemErrorKind {
    pub const ALL: [Self; 8] = [
        Self::Io,
        Self::Network,
        Self::Terminal,
        Self::Time,
        Self::Random,
        Self::Sqlite,
        Self::Utf8,
        Self::Unsupported,
    ];

    pub const fn index(self) -> usize {
        match self {
            Self::Io => 0,
            Self::Network => 1,
            Self::Terminal => 2,
            Self::Time => 3,
            Self::Random => 4,
            Self::Sqlite => 5,
            Self::Utf8 => 6,
            Self::Unsupported => 7,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Io => "io",
            Self::Network => "network",
            Self::Terminal => "terminal",
            Self::Time => "time",
            Self::Random => "random",
            Self::Sqlite => "sqlite",
            Self::Utf8 => "utf8",
            Self::Unsupported => "unsupported",
        }
    }

    pub const fn variant_id(self) -> [u8; 32] {
        SYSTEM_ERROR_VARIANTS[self.index()]
    }

    pub const fn physical_tag(self) -> u64 {
        match self {
            Self::Io => 0,
            Self::Network => 4,
            Self::Terminal => 1,
            Self::Time => 3,
            Self::Random => 7,
            Self::Sqlite => 2,
            Self::Utf8 => 5,
            Self::Unsupported => 6,
        }
    }
}

pub const fn numeric_variant(index: usize) -> [u8; 32] {
    NUMERIC_ERROR_VARIANTS[index]
}
