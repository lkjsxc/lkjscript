//! Source-independent owned Linux x86-64 scalar native backend foundation.
//!
//! This crate accepts only a closed typed target-lowering machine plan. It does
//! not consume source, HIR, SSA, or bytecode and does not own executable memory.

#![forbid(unsafe_code)]

mod encode;
mod image;
mod plan;
mod verify;

use std::fmt;

pub use encode::{encode, EncodingConfig};
pub use image::{
    CodeAccounting, EntryMetadata, ExactStackMap, FrameFacts, FrameHome, FrameHomeKind,
    HeapRuntimeSite, ImageContracts, ImageIntegrityError, InstallableImage, NativeReference,
    NativeValue, OutcomeKind, OutcomeMapEntry, Relocation, RelocationKind, RelocationTarget,
    RootLocation, Safepoint, SourceMapEntry, TrapMapEntry,
};
pub use plan::{
    AllocationClass, BlockId, BoolComparison, F64Comparison, FunctionBuilder, FunctionId,
    FunctionPlan, HeapCallDescriptor, HeapOperation, I64Comparison, InternalMachineArgument,
    InternalMachineResult, InternalRuntimeSignature, LayoutIdentity, LocalId, MachinePlanBuilder,
    PlanError, ReferenceType, RuntimeCallSlot, RuntimeOutcome, Signature, SourceFunctionId,
    SourceOrigin, StoreClass, TrapCode, ValueId, ValueType,
};
pub use verify::{VerificationError, VerifiedMachinePlan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendLimits {
    max_functions: usize,
    max_blocks: usize,
    max_values: usize,
    max_locals_per_function: usize,
    max_code_bytes: usize,
    max_metadata_bytes: u64,
    max_work_units: u64,
}

impl BackendLimits {
    #[must_use]
    pub const fn new(
        max_functions: usize,
        max_blocks: usize,
        max_values: usize,
        max_locals_per_function: usize,
        max_code_bytes: usize,
        max_metadata_bytes: u64,
        max_work_units: u64,
    ) -> Self {
        Self {
            max_functions,
            max_blocks,
            max_values,
            max_locals_per_function,
            max_code_bytes,
            max_metadata_bytes,
            max_work_units,
        }
    }

    #[must_use]
    pub const fn max_functions(self) -> usize {
        self.max_functions
    }

    #[must_use]
    pub const fn max_blocks(self) -> usize {
        self.max_blocks
    }

    #[must_use]
    pub const fn max_values(self) -> usize {
        self.max_values
    }

    #[must_use]
    pub const fn max_locals_per_function(self) -> usize {
        self.max_locals_per_function
    }

    #[must_use]
    pub const fn max_code_bytes(self) -> usize {
        self.max_code_bytes
    }

    #[must_use]
    pub const fn max_metadata_bytes(self) -> u64 {
        self.max_metadata_bytes
    }

    #[must_use]
    pub const fn max_work_units(self) -> u64 {
        self.max_work_units
    }
}

impl Default for BackendLimits {
    fn default() -> Self {
        Self::new(
            64,
            1_024,
            16_384,
            1_024,
            4 * 1024 * 1024,
            4 * 1024 * 1024,
            100_000,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EncodeError {
    MissingEntry,
    UnsupportedSignature,
    InvalidLabel,
    InvalidValue,
    InvalidCall,
    InvalidRelocation,
    FrameTooLarge,
    LimitExceeded(&'static str),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEntry => formatter.write_str("verified function is missing an entry"),
            Self::UnsupportedSignature => {
                formatter.write_str("function signature is unsupported by the scalar ABI")
            }
            Self::InvalidLabel => formatter.write_str("machine-code label is invalid"),
            Self::InvalidValue => formatter.write_str("machine-code value is invalid"),
            Self::InvalidCall => formatter.write_str("machine-code call target is invalid"),
            Self::InvalidRelocation => formatter.write_str("machine-code relocation is invalid"),
            Self::FrameTooLarge => formatter.write_str("machine-code frame is too large"),
            Self::LimitExceeded(limit) => {
                write!(formatter, "native encoding exceeds {limit} limit")
            }
        }
    }
}

impl std::error::Error for EncodeError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeError {
    Plan(PlanError),
    Verification(VerificationError),
    Encode(EncodeError),
    Image(ImageIntegrityError),
}

impl fmt::Display for NativeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan(error) => write!(formatter, "machine-plan construction failed: {error}"),
            Self::Verification(error) => {
                write!(formatter, "machine-plan verification failed: {error}")
            }
            Self::Encode(error) => write!(formatter, "native encoding failed: {error}"),
            Self::Image(error) => write!(formatter, "installable image is invalid: {error}"),
        }
    }
}

impl std::error::Error for NativeError {}

impl From<PlanError> for NativeError {
    fn from(error: PlanError) -> Self {
        Self::Plan(error)
    }
}

impl From<VerificationError> for NativeError {
    fn from(error: VerificationError) -> Self {
        Self::Verification(error)
    }
}

impl From<EncodeError> for NativeError {
    fn from(error: EncodeError) -> Self {
        Self::Encode(error)
    }
}

impl From<ImageIntegrityError> for NativeError {
    fn from(error: ImageIntegrityError) -> Self {
        Self::Image(error)
    }
}
