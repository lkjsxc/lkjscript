use std::fmt;
use std::sync::Arc;

use lkjscript_core::Error;

use super::{HoleId, RevisionId};

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WorkspaceError {
    ForeignNamespace(Arc<str>),
    StaleRevision,
    StaleIdentity(Arc<str>),
    InvalidTransaction(Arc<str>),
    InvalidDraft(Arc<str>),
    TypeMismatch {
        expected: Arc<str>,
        actual: Arc<str>,
    },
    InvisibleEntity,
    UnsupportedEdit {
        operation: Arc<str>,
        reason: Arc<str>,
    },
    InvalidContinuation(Arc<str>),
    Validation(Arc<str>),
    Host(Arc<str>),
}

impl WorkspaceError {
    pub(super) fn from_core(error: Error) -> Self {
        if matches!(error.class(), lkjscript_core::ErrorClass::Host) {
            Self::Host(Arc::from(error.as_str()))
        } else {
            Self::Validation(Arc::from(error.as_str()))
        }
    }

    pub(super) fn unsupported(operation: &str, reason: &str) -> Self {
        Self::UnsupportedEdit {
            operation: Arc::from(operation),
            reason: Arc::from(reason),
        }
    }
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignNamespace(subject) => {
                write!(
                    formatter,
                    "{subject} belongs to a different workspace namespace"
                )
            }
            Self::StaleRevision => formatter.write_str("workspace revision is stale"),
            Self::StaleIdentity(subject) => {
                write!(formatter, "workspace {subject} identity is stale")
            }
            Self::InvalidTransaction(message)
            | Self::InvalidDraft(message)
            | Self::Validation(message)
            | Self::Host(message)
            | Self::InvalidContinuation(message) => formatter.write_str(message),
            Self::TypeMismatch { expected, actual } => {
                write!(
                    formatter,
                    "expression type mismatch: expected {expected}, got {actual}"
                )
            }
            Self::InvisibleEntity => {
                formatter.write_str("draft entity is not visible from the edited expression")
            }
            Self::UnsupportedEdit { operation, reason } => {
                write!(formatter, "unsupported semantic edit {operation}: {reason}")
            }
        }
    }
}

impl std::error::Error for WorkspaceError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncompleteSnapshotError {
    pub revision: RevisionId,
    pub holes: Vec<HoleId>,
}

impl fmt::Display for IncompleteSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "workspace snapshot is incomplete ({} typed hole(s))",
            self.holes.len()
        )
    }
}

impl std::error::Error for IncompleteSnapshotError {}

#[derive(Debug)]
#[non_exhaustive]
pub enum CompileSnapshotError {
    Incomplete(IncompleteSnapshotError),
    Compiler(Error),
}

impl CompileSnapshotError {
    pub(crate) fn into_core(self) -> Error {
        match self {
            Self::Incomplete(error) => Error::msg(error.to_string()),
            Self::Compiler(error) => error,
        }
    }
}

impl From<Error> for CompileSnapshotError {
    fn from(error: Error) -> Self {
        Self::Compiler(error)
    }
}

impl fmt::Display for CompileSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete(error) => error.fmt(formatter),
            Self::Compiler(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CompileSnapshotError {}
