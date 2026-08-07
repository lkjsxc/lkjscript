use std::fmt;

use super::super::{StructuralError, StructuralRootTableError};
use super::SemanticValue;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructuralValueError {
    AllocationFailed,
    ArithmeticOverflow,
    RootTable(StructuralRootTableError),
    Domain(StructuralError),
    StaleObject,
    StaleDestination,
    StaleView,
    WrongLayout,
    WrongSemanticType,
    WrongPayloadKind,
    WrongOwnership,
    OwnerOverflow,
    InvalidUtf8,
    InvalidPath,
    InvalidRange,
    InvalidFieldPath,
    MixedValue,
    FieldAlreadyInitialized,
    FieldOutOfRange,
    IncompleteDestination,
    WrongDestinationKind,
    LiveDestination,
    LiveView,
    LiveObject,
    ReleaseBacklog,
    InvariantViolation,
}

impl From<StructuralRootTableError> for StructuralValueError {
    fn from(value: StructuralRootTableError) -> Self {
        Self::RootTable(value)
    }
}

impl From<StructuralError> for StructuralValueError {
    fn from(value: StructuralError) -> Self {
        Self::Domain(value)
    }
}

impl fmt::Display for StructuralValueError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllocationFailed => output.write_str("structural value allocation failed"),
            Self::ArithmeticOverflow => output.write_str("structural value arithmetic overflow"),
            Self::RootTable(error) => error.fmt(output),
            Self::Domain(error) => error.fmt(output),
            Self::StaleObject => output.write_str("stale structural object"),
            Self::StaleDestination => output.write_str("stale structural destination"),
            Self::StaleView => output.write_str("stale structural view"),
            Self::WrongLayout => output.write_str("wrong structural value layout"),
            Self::WrongSemanticType => output.write_str("wrong structural semantic type"),
            Self::WrongPayloadKind => output.write_str("wrong structural payload kind"),
            Self::WrongOwnership => output.write_str("wrong structural owner kind"),
            Self::OwnerOverflow => output.write_str("sealed structural owner count overflow"),
            Self::InvalidUtf8 => output.write_str("invalid UTF-8 structural string"),
            Self::InvalidPath => output.write_str("invalid structural path"),
            Self::InvalidRange => output.write_str("invalid structural range"),
            Self::InvalidFieldPath => output.write_str("invalid structural field path"),
            Self::MixedValue => {
                output.write_str("runtime key or mixed ownership cannot enter structure")
            }
            Self::FieldAlreadyInitialized => output.write_str("field already initialized"),
            Self::FieldOutOfRange => output.write_str("field is outside destination"),
            Self::IncompleteDestination => output.write_str("destination is incomplete"),
            Self::WrongDestinationKind => output.write_str("wrong destination kind"),
            Self::LiveDestination => output.write_str("live structural destination"),
            Self::LiveView => output.write_str("live structural view"),
            Self::LiveObject => output.write_str("live structural object"),
            Self::ReleaseBacklog => output.write_str("structural release backlog is not empty"),
            Self::InvariantViolation => output.write_str("structural value invariant violation"),
        }
    }
}

impl std::error::Error for StructuralValueError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralInitializationFailure {
    pub error: StructuralValueError,
    pub value: SemanticValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralPublishFailure {
    pub error: StructuralValueError,
    pub value: SemanticValue,
}
