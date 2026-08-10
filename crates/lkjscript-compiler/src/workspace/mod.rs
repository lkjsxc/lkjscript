//! Syntax-independent immutable typed semantic workspace snapshots.
//!
//! Text is accepted only by the importer in this module. Imported and
//! programmatically constructed programs share one partial-capable semantic
//! authority; complete revisions derive compiler HIR without source text.

mod compaction;
mod draft;
mod error;
mod identity;
mod ids;
mod importer;
mod index;
mod model;
mod program;
mod projection;
mod query;
mod transaction;
mod types;
mod validate;

use std::fmt;
use std::sync::Arc;

use lkjscript_core::{Error, Result};

pub use draft::{
    DeclarationType, DraftBindingId, DraftBindingRef, DraftFieldValue, DraftNode, DraftNodeId,
    DraftPatternField, DraftPatternNode, DraftPatternNodeId, DraftTypeParameterId, ExpressionDraft,
    LocalDraft, MatchArmDraft, PatternDraft, TypeArgumentDraft,
};
pub use error::{CompileSnapshotError, IncompleteSnapshotError, SemanticKind, WorkspaceError};
use identity::IdentityAllocator;
pub use ids::{EntityId, NodeId, RevisionId, WorkspaceNamespace};
pub(crate) use importer::{import_package_path, import_package_path_with_metrics};
pub use importer::{import_path, import_path_with_metrics, import_source};
pub use model::{
    CallEdge, CompletenessBlocker, ContainmentEdge, DependencyEdge, DiagnosticHeader,
    DiagnosticSeverity, EntityHeader, EntityKind, HoleId, HoleKind, HoleState, ImportMetrics,
    NodeHeader, NodeKind, PresentationAttachments, ProgramState, ReferenceEdge, SemanticChild,
    SemanticOwner, SourceAttachment,
};
use model::{HoleRecord, SnapshotIndexes};
use program::SemanticProgram;
pub use projection::ProjectionSlice;
pub use query::{
    CallInstantiationView, ConstructorStatus, Continuation, EffectSummary, EntityPage,
    EntityTypeFacts, FunctionSignatureView, LegalConstructor, MatchArmView, MatchPatternFieldView,
    MatchPatternKindView, MatchPatternLabel, MatchPatternNodeView, MatchView, NodeSemanticFacts,
    NodeTypeFacts, PageRequest, QueryPage, TraitWitnessKindView, TraitWitnessView,
    TypeArgumentView, TypeParameterBoundView, TypeParameterView, ValueParameterView,
};
pub use transaction::{
    Edit, EnumFieldDraft, EnumVariantDraft, InvalidatedDomain, ParameterDraft, ProductFieldDraft,
    SemanticDiff, SemanticDiffEntry, Transaction, TransactionOutcome, TypeParameterDraft,
    Workspace,
};
pub use types::{BuiltinEnum, BuiltinTrait, SemanticEnum, SemanticTrait, SemanticType};

#[derive(Clone)]
enum CapturedCompilationProvenance {
    Development,
    Locked(Arc<crate::package::CapturedPackageCompilation>),
}

impl CapturedCompilationProvenance {
    fn validate_memory_plan(&self, plan: &crate::HirMemoryPlan) -> Result<()> {
        match self {
            Self::Development => Ok(()),
            Self::Locked(captured) => captured.validate_memory_plan(plan),
        }
    }

    fn validate_required_capabilities(
        &self,
        required: &[lkjscript_core::CapabilityKind],
    ) -> Result<()> {
        match self {
            Self::Development => Ok(()),
            Self::Locked(captured) => captured.validate_required_capabilities(required),
        }
    }
}

/// One immutable, clone-safe semantic program revision.
///
/// All fields are private so dense HIR identities, vectors, and captured
/// package-boundary facts cannot become public semantic identities.
#[derive(Clone)]
pub struct WorkspaceSnapshot {
    namespace: WorkspaceNamespace,
    revision: RevisionId,
    state: ProgramState,
    program: Arc<SemanticProgram>,
    source_origins: Arc<[crate::hir::Source]>,
    provenance: Arc<CapturedCompilationProvenance>,
    attachments: Option<Arc<PresentationAttachments>>,
    indexes: Arc<SnapshotIndexes>,
    holes: Arc<[HoleRecord]>,
    blockers: Arc<[CompletenessBlocker]>,
    allocator: IdentityAllocator,
}

impl WorkspaceSnapshot {
    pub fn namespace(&self) -> WorkspaceNamespace {
        self.namespace
    }

    pub fn revision(&self) -> RevisionId {
        self.revision
    }

    pub const fn state(&self) -> ProgramState {
        self.state
    }

    pub fn attachments(&self) -> Option<&PresentationAttachments> {
        self.attachments.as_deref()
    }

    pub fn without_attachments(&self) -> Self {
        Self {
            namespace: self.namespace,
            revision: self.revision,
            state: self.state,
            program: Arc::clone(&self.program),
            source_origins: Arc::clone(&self.source_origins),
            provenance: Arc::clone(&self.provenance),
            attachments: None,
            indexes: Arc::clone(&self.indexes),
            holes: Arc::clone(&self.holes),
            blockers: Arc::clone(&self.blockers),
            allocator: self.allocator.clone(),
        }
    }

    pub fn entities(&self) -> &[EntityHeader] {
        &self.indexes.entities
    }

    pub fn nodes(&self) -> &[NodeHeader] {
        &self.indexes.nodes
    }

    pub fn containment(&self) -> &[ContainmentEdge] {
        &self.indexes.containment
    }

    pub fn references(&self) -> &[ReferenceEdge] {
        &self.indexes.references
    }

    pub fn calls(&self) -> &[CallEdge] {
        &self.indexes.calls
    }

    pub fn dependencies(&self) -> &[DependencyEdge] {
        &self.indexes.dependencies
    }

    pub fn diagnostics(&self) -> &[DiagnosticHeader] {
        &self.indexes.diagnostics
    }

    pub fn holes(&self) -> impl ExactSizeIterator<Item = &HoleState> {
        self.holes.iter().map(|record| &record.state)
    }

    pub fn completeness_blockers(&self) -> &[CompletenessBlocker] {
        &self.blockers
    }

    pub fn entity(&self, id: EntityId) -> Result<&EntityHeader> {
        self.indexes.entity(self.namespace, id)
    }

    pub fn node(&self, id: NodeId) -> Result<&NodeHeader> {
        self.indexes.node(self.namespace, id)
    }

    pub fn require_revision(&self, revision: RevisionId) -> Result<()> {
        model::require_revision(self.namespace, self.revision, revision)
    }

    pub fn check_consistency(&self) -> Result<()> {
        self.validate_consistency()
    }

    fn new(
        namespace: WorkspaceNamespace,
        hir: crate::hir::Program,
        provenance: CapturedCompilationProvenance,
        attachments: Option<PresentationAttachments>,
    ) -> Result<Self> {
        validate::program(&hir)?;
        let (program, source_origins) = SemanticProgram::from_hir(hir);
        let indexes = index::build(&program, namespace)?;
        let allocator = IdentityAllocator::from_indexes(namespace, &indexes)?;
        Ok(Self {
            namespace,
            revision: RevisionId::initial(namespace),
            state: ProgramState::Complete,
            program: Arc::new(program),
            source_origins,
            provenance: Arc::new(provenance),
            attachments: attachments.map(Arc::new),
            indexes: Arc::new(indexes),
            holes: Arc::from([]),
            blockers: Arc::from([]),
            allocator,
        })
    }

    fn empty(namespace: WorkspaceNamespace) -> Result<Self> {
        let program = SemanticProgram::empty()?;
        let mut indexes = index::build(&program, namespace)?;
        indexes.diagnostics.push(DiagnosticHeader {
            code: Arc::from("workspace.missing-entry-point"),
            severity: DiagnosticSeverity::Error,
            subject: None,
            message: Arc::from("program requires a main entry point"),
        });
        let allocator = IdentityAllocator::from_indexes(namespace, &indexes)?;
        Ok(Self {
            namespace,
            revision: RevisionId::initial(namespace),
            state: ProgramState::Incomplete,
            program: Arc::new(program),
            source_origins: Arc::from([]),
            provenance: Arc::new(CapturedCompilationProvenance::Development),
            attachments: None,
            indexes: Arc::new(indexes),
            holes: Arc::from([]),
            blockers: Arc::from([CompletenessBlocker::MissingEntryPoint]),
            allocator,
        })
    }

    pub(crate) fn validate_consistency(&self) -> Result<()> {
        if self.revision.namespace() != self.namespace {
            return Err(Error::msg(
                "workspace revision and namespace are inconsistent",
            ));
        }
        if (self.state == ProgramState::Complete) != self.blockers.is_empty() {
            return Err(Error::msg("workspace completeness state is stale"));
        }
        if self.indexes.nodes.len() != self.indexes.node_addresses.len()
            || self.indexes.nodes.len() != self.indexes.node_enclosing_entities.len()
            || self.indexes.nodes.len() != self.indexes.node_actual_types.len()
            || self.indexes.nodes.len() != self.indexes.node_expected_types.len()
            || self.indexes.nodes.len() != self.indexes.node_operations.len()
            || self.indexes.nodes.len() != self.indexes.node_effects.len()
            || self.indexes.entities.len() != self.indexes.entity_addresses.len()
            || self.indexes.entities.len() != self.indexes.entity_types.len()
        {
            return Err(Error::msg("workspace snapshot semantic indexes are stale"));
        }
        let indexed_holes = self
            .indexes
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Hole)
            .count();
        if indexed_holes != self.holes.len()
            || self.blockers.len() != self.indexes.diagnostics.len()
        {
            return Err(Error::msg(
                "workspace incomplete-node indexes and diagnostics are inconsistent",
            ));
        }
        for hole in self.holes.iter() {
            let index = self
                .indexes
                .node_lookup
                .get(&hole.state.id.0)
                .copied()
                .ok_or_else(|| Error::msg("workspace hole identity is stale"))?;
            if self.indexes.nodes[index].kind != NodeKind::Hole
                || self.indexes.node_addresses[index] != hole.address
                || self.indexes.node_keys[index] != hole.key
                || self.indexes.node_expected_types[index].as_ref() != Some(&hole.expected_internal)
            {
                return Err(Error::msg("workspace hole facts are inconsistent"));
            }
        }
        let mut expected_blockers = Vec::new();
        if self.program.main.is_none() {
            expected_blockers.push(CompletenessBlocker::MissingEntryPoint);
        }
        for hole in self.holes.iter() {
            expected_blockers.push(match hole.state.kind {
                HoleKind::MissingBody => CompletenessBlocker::MissingBody {
                    declaration: hole.state.owner,
                    hole: hole.state.id,
                    expected_type: hole.state.expected_type.clone(),
                },
                HoleKind::TypedExpression => CompletenessBlocker::TypedHole {
                    hole: hole.state.id,
                    expected_type: hole.state.expected_type.clone(),
                    owner: hole.state.owner,
                    context: hole.state.context,
                },
            });
        }
        if expected_blockers.as_slice() != self.blockers.as_ref() {
            return Err(Error::msg("workspace completeness blockers are stale"));
        }
        if self.state == ProgramState::Complete {
            let _ = self.validated_complete_hir()?;
        }
        Ok(())
    }

    pub(crate) fn validated_complete_hir(&self) -> Result<crate::hir::Program> {
        if self.revision.namespace() != self.namespace {
            return Err(Error::msg(
                "workspace revision and namespace are inconsistent",
            ));
        }
        if self.state != ProgramState::Complete || !self.blockers.is_empty() {
            return Err(Error::msg("workspace snapshot is incomplete"));
        }
        if self.indexes.nodes.len() != self.indexes.node_addresses.len()
            || self.indexes.nodes.len() != self.indexes.node_enclosing_entities.len()
            || self.indexes.nodes.len() != self.indexes.node_actual_types.len()
            || self.indexes.nodes.len() != self.indexes.node_expected_types.len()
            || self.indexes.nodes.len() != self.indexes.node_operations.len()
            || self.indexes.nodes.len() != self.indexes.node_effects.len()
            || self.indexes.entities.len() != self.indexes.entity_addresses.len()
            || self.indexes.entities.len() != self.indexes.entity_types.len()
        {
            return Err(Error::msg("workspace snapshot semantic indexes are stale"));
        }
        let mut hir = self.program.try_complete(&self.source_origins)?;
        program::install_core_traits_if_absent(&mut hir)?;
        validate::program(&hir)?;
        Ok(hir)
    }

    pub(crate) fn validate_memory_plan(&self, plan: &crate::HirMemoryPlan) -> Result<()> {
        self.provenance.validate_memory_plan(plan)
    }

    pub(crate) fn validate_required_capabilities(
        &self,
        required: &[lkjscript_core::CapabilityKind],
    ) -> Result<()> {
        self.provenance.validate_required_capabilities(required)
    }
}

impl fmt::Debug for WorkspaceSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorkspaceSnapshot")
            .field("namespace", &self.namespace)
            .field("revision", &self.revision)
            .field("state", &self.state)
            .field("entities", &self.indexes.entities.len())
            .field("nodes", &self.indexes.nodes.len())
            .field("attachments", &self.attachments.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests;
