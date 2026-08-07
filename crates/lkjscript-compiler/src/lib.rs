//! Import text into immutable typed semantic workspace snapshots, then compile
//! snapshots into verified typed SSA and validated reference bytecode.

mod analyze;
mod codegen;
mod effects;
mod hir;
#[path = "hir/memory_plan/mod.rs"]
pub mod memory_plan;
mod operation;
mod ownership;
pub mod package;
mod pipeline;
// Text parsing is private importer machinery. Semantic authority begins at the
// immutable workspace snapshot returned by the public import conveniences.
mod source;
mod ssa;
mod stack;
mod types;
pub mod workspace;

use std::path::Path;
use std::time::Duration;

use lkjscript_core::ValidatedChunk;
use lkjscript_ir::{BytecodeLinkMetadata, VerifiedProgram};

pub use memory_plan::{
    HirMemoryPlan, MemoryDomain, MemoryValueCategory, MemoryValueFailureCleanup,
    MemoryValuePlacement, MemoryValueRepresentationId, MemoryValueRoute,
};
pub use package::program::PreparedProgram;

pub use pipeline::{
    compile_path, compile_path_with_metrics, compile_path_with_sources, compile_snapshot,
    compile_source, validate_source,
};
pub use types::Type;
pub use workspace::{
    import_path, import_path_with_metrics, import_source, CallEdge, CompileSnapshotError,
    Continuation, DependencyEdge, DiagnosticHeader, DiagnosticSeverity, DraftNode, DraftNodeId,
    Edit, EntityHeader, EntityId, EntityKind, EntityPage, ExpressionDraft, HoleId, HoleState,
    ImportMetrics, IncompleteSnapshotError, InvalidatedDomain, LegalConstructor, NodeHeader,
    NodeId, NodeKind, NodeTypeFacts, PageRequest, PresentationAttachments, ProgramState,
    ProjectionSlice, QueryPage, ReferenceEdge, RevisionId, SemanticChild, SemanticDiff,
    SemanticDiffEntry, SemanticOwner, SourceAttachment, Transaction, TransactionOutcome, Workspace,
    WorkspaceError, WorkspaceNamespace, WorkspaceSnapshot,
};

pub const SOURCE_EXTENSION: &str = "lkjscript";

/// Monotonic direct phase timings and observed source-file count.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompileMetrics {
    pub total: Duration,
    pub source_loading: Duration,
    pub parsing: Duration,
    pub hir_analysis: Duration,
    pub effect_analysis: Duration,
    pub memory_planning: Duration,
    pub ssa_construction: Duration,
    pub ssa_verification: Duration,
    pub normalization: Duration,
    pub bytecode_lowering: Duration,
    pub bytecode_validation: Duration,
    pub preparation: Duration,
    pub source_files: usize,
}

/// One compiled semantic program shared by the reference VM and later backends.
#[derive(Debug, Clone)]
pub struct ExecutableProgram {
    prepared: PreparedProgram,
    bytecode: ValidatedChunk,
    ssa: VerifiedProgram,
    memory_plan: HirMemoryPlan,
    bytecode_links: BytecodeLinkMetadata,
}

impl ExecutableProgram {
    pub const fn prepared(&self) -> PreparedProgram {
        self.prepared
    }

    pub const fn prepared_identity(&self) -> lkjscript_contracts::PreparedProgramIdentity {
        self.prepared.identity()
    }

    pub fn bytecode(&self) -> &ValidatedChunk {
        &self.bytecode
    }

    pub fn ssa(&self) -> &VerifiedProgram {
        &self.ssa
    }

    pub fn memory_plan(&self) -> &HirMemoryPlan {
        &self.memory_plan
    }

    pub fn bytecode_links(&self) -> &BytecodeLinkMetadata {
        &self.bytecode_links
    }

    pub fn into_bytecode(self) -> ValidatedChunk {
        self.bytecode
    }
}

pub(crate) fn ensure_source_path(path: &Path) -> lkjscript_core::Result<()> {
    source::ensure_source_path_for_compiler(path)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
