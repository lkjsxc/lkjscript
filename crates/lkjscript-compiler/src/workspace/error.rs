use std::fmt;
use std::sync::Arc;

use lkjscript_core::Error;

use super::{
    CompletenessBlocker, DraftTypeParameterId, EntityId, EntityKind, NodeKind, RevisionId,
    SemanticTrait, SemanticType,
};

/// Structured kind carried by wrong-kind workspace failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SemanticKind {
    Entity(EntityKind),
    Node(NodeKind),
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WorkspaceError {
    ForeignNamespace(Arc<str>),
    StaleRevision,
    StaleIdentity(Arc<str>),
    InvalidTransaction(Arc<str>),
    InvalidDraft(Arc<str>),
    TypeMismatch {
        expected: Box<SemanticType>,
        actual: Box<SemanticType>,
    },
    InvalidSemanticType {
        position: Arc<str>,
        reason: Arc<str>,
    },
    UnknownDraftTypeParameter {
        parameter: DraftTypeParameterId,
    },
    DuplicateDraftTypeParameter {
        parameter: DraftTypeParameterId,
    },
    DuplicateTypeParameterName {
        first: DraftTypeParameterId,
        duplicate: DraftTypeParameterId,
    },
    DuplicateTypeParameterBound {
        parameter: DraftTypeParameterId,
        trait_identity: SemanticTrait,
    },
    UnusedDraftTypeParameter {
        parameter: DraftTypeParameterId,
    },
    MissingTypeArgument {
        parameter: EntityId,
    },
    DuplicateTypeArgument {
        parameter: EntityId,
    },
    UnexpectedTypeArgument,
    WrongTypeParameterOwner {
        parameter: Box<EntityId>,
        expected: Box<EntityId>,
        actual: Option<Box<EntityId>>,
    },
    InvisibleTypeParameter {
        parameter: EntityId,
    },
    CallArity {
        callee: Box<EntityId>,
        expected: usize,
        actual: usize,
    },
    UnsatisfiedTraitBound {
        parameter: Box<EntityId>,
        trait_identity: Box<SemanticTrait>,
        argument: Box<SemanticType>,
    },
    GenericForwardingUnsupported,
    InvisibleEntity {
        operation: Arc<str>,
        entity: Box<EntityId>,
        reason: Arc<str>,
    },
    WrongEntityKind {
        operation: Arc<str>,
        expected: Arc<str>,
        actual: SemanticKind,
    },
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
            Self::InvalidSemanticType { position, reason } => {
                write!(formatter, "invalid semantic type for {position}: {reason}")
            }
            Self::UnknownDraftTypeParameter { parameter } => {
                write!(formatter, "declaration type references unknown {parameter:?}")
            }
            Self::DuplicateDraftTypeParameter { parameter } => {
                write!(formatter, "function declaration repeats {parameter:?}")
            }
            Self::DuplicateTypeParameterName { first, duplicate } => write!(
                formatter,
                "function declaration gives {first:?} and {duplicate:?} the same name"
            ),
            Self::DuplicateTypeParameterBound {
                parameter,
                trait_identity,
            } => write!(
                formatter,
                "function declaration repeats bound {trait_identity:?} for {parameter:?}"
            ),
            Self::UnusedDraftTypeParameter { parameter } => write!(
                formatter,
                "function declaration does not use {parameter:?} in its signature"
            ),
            Self::MissingTypeArgument { parameter } => {
                write!(formatter, "generic call is missing type argument for {parameter:?}")
            }
            Self::DuplicateTypeArgument { parameter } => {
                write!(formatter, "generic call repeats type argument for {parameter:?}")
            }
            Self::UnexpectedTypeArgument => {
                formatter.write_str("non-generic call does not accept type arguments")
            }
            Self::WrongTypeParameterOwner {
                parameter,
                expected,
                actual,
            } => write!(
                formatter,
                "type parameter {parameter:?} is owned by {actual:?}, not {expected:?}"
            ),
            Self::InvisibleTypeParameter { parameter } => {
                write!(formatter, "type parameter {parameter:?} is not visible here")
            }
            Self::CallArity {
                callee,
                expected,
                actual,
            } => write!(
                formatter,
                "call to {callee:?} requires {expected} value argument(s), got {actual}"
            ),
            Self::UnsatisfiedTraitBound {
                parameter,
                trait_identity,
                argument,
            } => write!(
                formatter,
                "type argument {argument} for {parameter:?} does not satisfy {trait_identity:?}"
            ),
            Self::GenericForwardingUnsupported => formatter.write_str(
                "forwarding a caller type parameter through a generic call is unavailable in the current transport route",
            ),
            Self::InvisibleEntity {
                operation,
                entity,
                reason,
            } => write!(
                formatter,
                "{operation} cannot use invisible entity {entity:?}: {reason}"
            ),
            Self::WrongEntityKind {
                operation,
                expected,
                actual,
            } => write!(
                formatter,
                "{operation} requires {expected}, but the identity names {actual:?}"
            ),
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
    pub blockers: Vec<CompletenessBlocker>,
}

impl fmt::Display for IncompleteSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "workspace snapshot is incomplete ({} blocker(s))",
            self.blockers.len()
        )
    }
}

impl std::error::Error for IncompleteSnapshotError {}

#[derive(Debug)]
#[non_exhaustive]
pub enum CompileSnapshotError {
    Incomplete(IncompleteSnapshotError),
    Package(Error),
    Compiler(Error),
}

impl CompileSnapshotError {
    pub(crate) fn into_core(self) -> Error {
        match self {
            Self::Incomplete(error) => Error::msg(error.to_string()),
            Self::Package(error) | Self::Compiler(error) => error,
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
            Self::Package(error) | Self::Compiler(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CompileSnapshotError {}
