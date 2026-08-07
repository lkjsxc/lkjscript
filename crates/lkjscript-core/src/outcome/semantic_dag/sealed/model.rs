use std::fmt;

use super::super::{SemanticDagSnapshot, SemanticDagType};
use super::cells::SealedDagCell;
use crate::structural::{
    SealedBorrow, SealedOwner, SealedRegionMetrics, StructuralError, StructuralRuntimeMetrics,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SealedSemanticDagError {
    AllocationFailed,
    ArithmeticOverflow,
    InvalidTypeClosure,
    MissingValidatedReturn,
    MissingAuthenticatedWitness,
    UnauthenticatedValidatedType,
    RootTypeMismatch,
    UnresolvedType(SemanticDagType),
    UnsupportedValidatedType,
    ValidatedShapeMismatch,
    CorruptRegion,
    Structural(StructuralError),
}

impl From<StructuralError> for SealedSemanticDagError {
    fn from(error: StructuralError) -> Self {
        Self::Structural(error)
    }
}

impl fmt::Display for SealedSemanticDagError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllocationFailed => output.write_str("sealed semantic DAG allocation failed"),
            Self::ArithmeticOverflow => output.write_str("sealed semantic DAG arithmetic overflow"),
            Self::InvalidTypeClosure => {
                output.write_str("invalid sealed semantic DAG type closure")
            }
            Self::MissingValidatedReturn => {
                output.write_str("validated bytecode has no exact structural return")
            }
            Self::MissingAuthenticatedWitness => {
                output.write_str("validated return has no installed authenticated witness")
            }
            Self::UnauthenticatedValidatedType => {
                output.write_str("validated return witness does not permit sealed decoding")
            }
            Self::RootTypeMismatch => output.write_str("sealed semantic DAG root type mismatch"),
            Self::UnresolvedType(value_type) => {
                write!(
                    output,
                    "unresolved sealed semantic DAG type: {value_type:?}"
                )
            }
            Self::UnsupportedValidatedType => {
                output.write_str("validated bytecode type is ineligible for sealed DAG import")
            }
            Self::ValidatedShapeMismatch => {
                output.write_str("semantic DAG disagrees with validated bytecode shape")
            }
            Self::CorruptRegion => output.write_str("corrupt sealed semantic DAG region"),
            Self::Structural(error) => error.fmt(output),
        }
    }
}

impl std::error::Error for SealedSemanticDagError {}

#[derive(Debug)]
pub struct SealedSemanticDagFailure {
    pub error: SealedSemanticDagError,
    pub snapshot: Box<SemanticDagSnapshot>,
}

impl SealedSemanticDagFailure {
    pub(super) fn new(error: SealedSemanticDagError, snapshot: SemanticDagSnapshot) -> Self {
        Self {
            error,
            snapshot: Box::new(snapshot),
        }
    }
}

#[derive(Debug)]
pub struct SealedSemanticDagOwner {
    pub(super) store: u64,
    pub(super) root: u64,
    pub(super) nodes: u64,
    pub(super) cells: u64,
    pub(super) value_type: SemanticDagType,
    pub(super) owner: SealedOwner<SealedDagCell, ()>,
}

impl SealedSemanticDagOwner {
    pub const fn value_type(&self) -> SemanticDagType {
        self.value_type
    }

    pub const fn node_count(&self) -> u64 {
        self.nodes
    }
}

#[derive(Debug)]
pub struct SealedSemanticDagBorrow {
    pub(super) store: u64,
    pub(super) root: u64,
    pub(super) nodes: u64,
    pub(super) cells: u64,
    pub(super) value_type: SemanticDagType,
    pub(super) borrow: SealedBorrow<SealedDagCell>,
}

#[derive(Debug)]
pub struct SealedSemanticDagBorrowFailure {
    pub error: SealedSemanticDagError,
    pub borrow: Box<SealedSemanticDagBorrow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedSemanticDagReleaseReport {
    pub regions_released: u64,
    pub cells_released: u64,
    pub dependency_releases: u64,
}

#[derive(Debug)]
pub struct SealedSemanticDagReleaseFailure {
    pub error: SealedSemanticDagError,
    pub owner: SealedSemanticDagOwner,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SealedSemanticDagMetrics {
    pub typed_stores: u64,
    pub runtime: StructuralRuntimeMetrics,
    pub sealed: SealedRegionMetrics,
    pub live_regions: u64,
    pub live_owners: u64,
    pub live_loans: u64,
    pub live_dependencies: u64,
    pub release_backlog: u64,
}
