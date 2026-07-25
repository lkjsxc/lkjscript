use std::fmt;

use crate::optimize::*;
use crate::{BlockId, FunctionId, Program, RuntimeOp, ValueId, VerifiedProgram};

/// Deterministic resource bounds for the proof-producing optimization slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OptimizationLimits {
    pub max_work_units: u64,
    pub max_certificate_records: u64,
    pub max_certificate_bytes_estimate: u64,
    pub max_instruction_growth: u64,
    pub max_iterations: u64,
    pub max_functions: u64,
    pub max_blocks: u64,
    pub max_parameters: u64,
    pub max_instructions: u64,
    pub max_operands: u64,
    pub max_frame_facts: u64,
    pub max_type_nodes: u64,
    pub max_metadata_items: u64,
    pub max_string_and_metadata_bytes: u64,
}

impl Default for OptimizationLimits {
    fn default() -> Self {
        Self {
            max_work_units: 16 * 1024 * 1024,
            max_certificate_records: 65_536,
            max_certificate_bytes_estimate: 4 * 1024 * 1024,
            max_instruction_growth: 0,
            // Internal optimize performs the seven cleanup passes once while
            // constructing and once while independently checking the candidate.
            max_iterations: 16,
            max_functions: 4_096,
            max_blocks: 65_536,
            max_parameters: 1_048_576,
            max_instructions: 1_048_576,
            max_operands: 4_194_304,
            max_frame_facts: 1_048_576,
            max_type_nodes: 4_194_304,
            max_metadata_items: 4_194_304,
            max_string_and_metadata_bytes: 16 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptimizationFailureCode {
    InputVerification,
    BudgetExceeded,
    CertificateMismatch,
    IllegalEdit,
    CandidateMismatch,
    OutputVerification,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimizationError {
    code: OptimizationFailureCode,
    detail: String,
}

impl OptimizationError {
    pub(crate) fn new(code: OptimizationFailureCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    pub const fn code(&self) -> OptimizationFailureCode {
        self.code
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for OptimizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "optimization {:?}: {}", self.code, self.detail)
    }
}

impl std::error::Error for OptimizationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptimizationEditKind {
    AlgebraicIdentity,
    GlobalValueNumbering,
    CheckedI64GlobalValueNumbering,
}

/// One ordered edit over the stable IDs of the verified baseline input.
///
/// Operation and operands are repeated deliberately: the certificate checker
/// checks them against its private immutable input indexes rather than trusting
/// edit discovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimizationCertificateRecord {
    pub sequence: u64,
    pub function: FunctionId,
    pub block: BlockId,
    pub value: ValueId,
    pub kind: OptimizationEditKind,
    pub expected_operation: RuntimeOp,
    pub expected_operands: Vec<ValueId>,
    pub replacement: ValueId,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OptimizationCertificate {
    pub records: Vec<OptimizationCertificateRecord>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OptimizationStats {
    pub input_instructions: u64,
    pub output_instructions: u64,
    pub work_units: u64,
    pub certificate_records: u64,
    pub certificate_bytes_estimate: u64,
    pub instruction_growth: u64,
    pub iterations: u64,
    pub discovery_passes: u64,
    pub checker_passes: u64,
    pub reconstruction_passes: u64,
    pub cleanup_passes: u64,
    pub validation_passes: u64,
    pub optimizing_passes: u64,
    pub algebraic_rewrites: u64,
    pub gvn_rewrites: u64,
    pub checked_i64_rewrites: u64,
    pub cleanup_removed_instructions: u64,
}

/// Opaque authority required by optimizing lowering.
///
/// There is intentionally no constructor from `Program` or `VerifiedProgram`.
#[derive(Clone, Debug)]
pub struct VerifiedOptimizedProgram {
    pub(crate) verified: VerifiedProgram,
    pub(crate) certificate: OptimizationCertificate,
    pub(crate) stats: OptimizationStats,
}

impl PartialEq for VerifiedOptimizedProgram {
    fn eq(&self, other: &Self) -> bool {
        exact_program_equal(self.program(), other.program())
            && self.certificate == other.certificate
            && self.stats == other.stats
    }
}

impl VerifiedOptimizedProgram {
    pub fn program(&self) -> &Program {
        self.verified.program()
    }

    pub fn verified_program(&self) -> &VerifiedProgram {
        &self.verified
    }

    pub fn certificate(&self) -> &OptimizationCertificate {
        &self.certificate
    }

    pub const fn stats(&self) -> &OptimizationStats {
        &self.stats
    }
}
