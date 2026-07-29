use std::fmt;

use lkjscript_host::HostError;

pub type DatabaseResult<T> = Result<T, DatabaseError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatabaseError {
    Host(HostError),
    InvalidDatabaseName,
    InvalidTenantLength { length: usize },
    InvalidKeyLength { length: usize },
    InvalidValueLength { length: usize },
    RangeLimit { requested: usize, maximum: usize },
    LogicalBufferLimit { requested: usize, maximum: usize },
    AlreadyExists,
    NotFound,
    WriterActive,
    Closed,
    TransactionClosed,
    NeedsReopen,
    CorruptCheckpoint,
    CorruptWal,
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(error) => error.fmt(formatter),
            Self::InvalidDatabaseName => formatter.write_str("invalid database name"),
            Self::InvalidTenantLength { length } => {
                write!(formatter, "invalid tenant length: {length}")
            }
            Self::InvalidKeyLength { length } => write!(formatter, "invalid key length: {length}"),
            Self::InvalidValueLength { length } => {
                write!(formatter, "invalid value length: {length}")
            }
            Self::RangeLimit { requested, maximum } => {
                write!(formatter, "range limit {requested} exceeds {maximum}")
            }
            Self::LogicalBufferLimit { requested, maximum } => {
                write!(formatter, "logical buffer {requested} exceeds {maximum}")
            }
            Self::AlreadyExists => formatter.write_str("database already exists"),
            Self::NotFound => formatter.write_str("database not found"),
            Self::WriterActive => formatter.write_str("a write transaction is already active"),
            Self::Closed => formatter.write_str("database is closed"),
            Self::TransactionClosed => formatter.write_str("transaction is closed"),
            Self::NeedsReopen => {
                formatter.write_str("database must be reopened after an I/O failure")
            }
            Self::CorruptCheckpoint => formatter.write_str("corrupt database checkpoint"),
            Self::CorruptWal => formatter.write_str("corrupt database WAL"),
        }
    }
}

impl std::error::Error for DatabaseError {}

impl From<HostError> for DatabaseError {
    fn from(error: HostError) -> Self {
        Self::Host(error)
    }
}
