//! Compile canonical line-oriented `.lkjscript` source into verified typed SSA
//! and validated reference bytecode.

mod analyze;
mod budget;
mod codegen;
mod effects;
mod hir;
mod operation;
mod ownership;
mod pipeline;
pub mod semantic;
pub mod source;
mod ssa;
mod types;

use std::path::Path;
use std::time::Duration;

use lkjscript_core::ValidatedChunk;
use lkjscript_ir::{BytecodeLinkMetadata, VerifiedProgram};

pub use lkjscript_core::{
    BudgetAuthority, BudgetCause, BudgetError, BudgetErrorKind, BudgetLedger, BudgetPath,
    BudgetPrefix, BudgetRejectedEvent, BudgetScope, InvalidCeiling, Reservation, ReservationId,
    ReservationJournalRecord, ReservationState, ResourceCategory, ResourceCeilings,
    ResourceDiagnostic, ResourceProfile, ResourceProfileIdentity, ResourceProfileName,
    ResourceUsage, UnknownResourceProfile, IMPLEMENTATION_MAXIMA_VERSION,
    MAX_BUDGET_JOURNAL_ENTRIES, MAX_BUDGET_PATH_DEPTH, RESOURCE_PROFILE_SCHEMA,
    RESOURCE_PROFILE_VERSION,
};
pub use pipeline::{
    compile_path, compile_path_with_metrics, compile_path_with_profile,
    compile_path_with_profile_and_metrics, compile_path_with_sources,
    compile_path_with_sources_and_profile, compile_source, compile_source_with_profile,
    validate_source, validate_source_tree, validate_source_with_profile,
};

pub const SOURCE_EXTENSION: &str = "lkjscript";

/// Monotonic direct phase timings and exact logical resource totals for one compilation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompileMetrics {
    pub total: Duration,
    pub source_loading: Duration,
    pub parsing: Duration,
    pub hir_analysis: Duration,
    pub effect_analysis: Duration,
    pub ssa_construction: Duration,
    pub ssa_verification: Duration,
    pub normalization: Duration,
    pub bytecode_lowering: Duration,
    pub bytecode_validation: Duration,
    pub source_files: usize,
    pub profile: ResourceProfileIdentity,
    pub resources: ResourceUsage,
}

impl Default for CompileMetrics {
    fn default() -> Self {
        let profile = ResourceProfile::default().identity();
        Self {
            total: Duration::default(),
            source_loading: Duration::default(),
            parsing: Duration::default(),
            hir_analysis: Duration::default(),
            effect_analysis: Duration::default(),
            ssa_construction: Duration::default(),
            ssa_verification: Duration::default(),
            normalization: Duration::default(),
            bytecode_lowering: Duration::default(),
            bytecode_validation: Duration::default(),
            source_files: 0,
            profile,
            resources: ResourceUsage::default(),
        }
    }
}

/// One compiled semantic program shared by the reference VM and later backends.
#[derive(Debug, Clone)]
pub struct ExecutableProgram {
    bytecode: ValidatedChunk,
    ssa: VerifiedProgram,
    bytecode_links: BytecodeLinkMetadata,
    profile: ResourceProfileIdentity,
}

impl ExecutableProgram {
    pub fn bytecode(&self) -> &ValidatedChunk {
        &self.bytecode
    }

    pub fn ssa(&self) -> &VerifiedProgram {
        &self.ssa
    }

    pub fn bytecode_links(&self) -> &BytecodeLinkMetadata {
        &self.bytecode_links
    }

    pub const fn profile(&self) -> ResourceProfileIdentity {
        self.profile
    }

    pub fn into_bytecode(self) -> ValidatedChunk {
        self.bytecode
    }
}

pub(crate) fn ensure_source_path(path: &Path) -> lkjscript_core::Result<()> {
    source::ensure_source_path_for_compiler(path)
}

pub use lkjscript_core::Limits as CompileLimits;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
