//! Import text into immutable typed semantic workspace snapshots, then compile
//! snapshots into verified typed SSA and validated reference bytecode.

mod analyze;
mod codegen;
mod effects;
mod generic_call;
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
pub use operation::Operation;
pub use pipeline::{
    compile_package_path, compile_package_path_with_metrics, compile_path,
    compile_path_with_metrics, compile_path_with_sources, compile_snapshot, compile_source,
    validate_source, PackageCompileError, PackageCompileResult,
};
pub use source::{
    DiagnosticCategory as SourceDiagnosticCategory, DiagnosticSeverity as SourceDiagnosticSeverity,
    RelatedSourceSpan, SourceDiagnostic, SourceOrigin, SourcePosition, SourceSpan,
};
pub(crate) use types::Type;
pub use workspace::{
    import_path, import_path_with_metrics, import_source, BuiltinEnum, BuiltinTrait, CallEdge,
    CallInstantiationView, CompileSnapshotError, CompletenessBlocker, ConstructorStatus,
    Continuation, DeclarationType, DependencyEdge, DiagnosticHeader, DiagnosticSeverity,
    DraftBindingId, DraftBindingRef, DraftFieldValue, DraftNode, DraftNodeId, DraftPatternField,
    DraftPatternNode, DraftPatternNodeId, DraftTypeParameterId, Edit, EffectSummary, EntityHeader,
    EntityId, EntityKind, EntityPage, EntityTypeFacts, EnumFieldDraft, EnumVariantDraft,
    ExpressionDraft, FunctionSignatureView, HoleId, HoleKind, HoleState, ImportMetrics,
    IncompleteSnapshotError, InvalidatedDomain, LegalConstructor, LocalDraft, MatchArmDraft,
    MatchArmView, MatchPatternFieldView, MatchPatternKindView, MatchPatternLabel,
    MatchPatternNodeView, MatchView, NodeHeader, NodeId, NodeKind, NodeTypeFacts, PageRequest,
    ParameterDraft, PatternDraft, PresentationAttachments, ProductFieldDraft, ProgramState,
    ProjectionSlice, QueryPage, ReferenceEdge, RevisionId, SemanticChild, SemanticDiff,
    SemanticDiffEntry, SemanticEnum, SemanticKind, SemanticOwner, SemanticTrait, SemanticType,
    SourceAttachment, TraitWitnessKindView, TraitWitnessView, Transaction, TransactionOutcome,
    TypeArgumentDraft, TypeArgumentView, TypeParameterBoundView, TypeParameterDraft,
    TypeParameterView, ValueParameterView, Workspace, WorkspaceError, WorkspaceNamespace,
    WorkspaceSnapshot,
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
    pub package_validation: Duration,
    pub source_files: usize,
}

/// One compiled semantic program shared by the reference VM and later backends.
#[derive(Debug, Clone)]
pub struct ExecutableProgram {
    bytecode: ValidatedChunk,
    ssa: VerifiedProgram,
    memory_plan: HirMemoryPlan,
    bytecode_links: BytecodeLinkMetadata,
}

impl ExecutableProgram {
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
    source::ensure_source_path(path).map_err(source::SourceDiagnostic::into_core)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
