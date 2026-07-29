use std::fmt;
use std::io;

pub type HostResult<T> = Result<T, HostError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostError {
    InvalidName(String),
    NotFound(String),
    AlreadyExists(String),
    PermissionDenied(String),
    DiskFull(String),
    ShortWrite { expected: usize, written: usize },
    SyncFailed(String),
    Cancelled,
    Clock(String),
    Io { operation: String, message: String },
}

impl HostError {
    pub fn from_io(operation: impl Into<String>, error: io::Error) -> Self {
        let operation = operation.into();
        match error.kind() {
            io::ErrorKind::NotFound => Self::NotFound(operation),
            io::ErrorKind::AlreadyExists => Self::AlreadyExists(operation),
            io::ErrorKind::PermissionDenied => Self::PermissionDenied(operation),
            io::ErrorKind::StorageFull => Self::DiskFull(operation),
            _ => Self::Io {
                operation,
                message: error.to_string(),
            },
        }
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(name) => write!(formatter, "invalid storage name: {name}"),
            Self::NotFound(name) => write!(formatter, "not found: {name}"),
            Self::AlreadyExists(name) => write!(formatter, "already exists: {name}"),
            Self::PermissionDenied(name) => write!(formatter, "permission denied: {name}"),
            Self::DiskFull(name) => write!(formatter, "disk full: {name}"),
            Self::ShortWrite { expected, written } => {
                write!(
                    formatter,
                    "short write: expected {expected}, wrote {written}"
                )
            }
            Self::SyncFailed(name) => write!(formatter, "sync failed: {name}"),
            Self::Cancelled => formatter.write_str("cancelled"),
            Self::Clock(message) => write!(formatter, "clock failed: {message}"),
            Self::Io { operation, message } => write!(formatter, "{operation}: {message}"),
        }
    }
}

impl std::error::Error for HostError {}
