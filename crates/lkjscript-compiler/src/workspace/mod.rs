//! Syntax-independent immutable typed semantic workspace snapshots.
//!
//! Text is accepted only by the importer in this module. Once import has
//! produced a snapshot, compiler phases consume the owned typed HIR directly.

mod draft;
mod error;
mod identity;
mod ids;
mod importer;
mod index;
mod model;
mod projection;
mod query;
mod transaction;
mod validate;

use std::fmt;
use std::sync::Arc;

use lkjscript_core::{Error, Result};

pub use draft::{DraftNode, DraftNodeId, ExpressionDraft};
pub use error::{CompileSnapshotError, IncompleteSnapshotError, WorkspaceError};
pub use ids::{EntityId, NodeId, RevisionId, WorkspaceNamespace};
pub(crate) use importer::{import_package_path, import_package_path_with_metrics};
pub use importer::{import_path, import_path_with_metrics, import_source};
pub use model::{
    CallEdge, ContainmentEdge, DependencyEdge, DiagnosticHeader, DiagnosticSeverity, EntityHeader,
    EntityKind, HoleId, HoleState, ImportMetrics, NodeHeader, NodeKind, PresentationAttachments,
    ProgramState, ReferenceEdge, SemanticChild, SemanticOwner, SourceAttachment,
};
use model::{HoleOverlay, SnapshotIndexes};
pub use projection::ProjectionSlice;
pub use query::{
    Continuation, EntityPage, LegalConstructor, NodeTypeFacts, PageRequest, QueryPage,
};
pub use transaction::{
    Edit, InvalidatedDomain, SemanticDiff, SemanticDiffEntry, Transaction, TransactionOutcome,
    Workspace,
};

#[derive(Clone)]
enum CapturedCompilationProvenance {
    Development([u8; 32]),
    Locked(Arc<crate::package::CapturedPackageCompilation>),
}

impl CapturedCompilationProvenance {
    fn semantic_base_identity(&self) -> [u8; 32] {
        match self {
            Self::Development(source_identity) => *source_identity,
            Self::Locked(captured) => captured.semantic_base_identity(),
        }
    }

    const fn edited(identity: [u8; 32]) -> Self {
        Self::Development(identity)
    }

    fn validate_memory_plan(&self, plan: &crate::HirMemoryPlan) -> Result<()> {
        match self {
            Self::Development(_) => Ok(()),
            Self::Locked(captured) => captured.validate_memory_plan(plan),
        }
    }

    fn validate_required_capabilities(
        &self,
        required: &[lkjscript_core::CapabilityKind],
    ) -> Result<()> {
        match self {
            Self::Development(_) => Ok(()),
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
    hir: Arc<crate::hir::Program>,
    provenance: Arc<CapturedCompilationProvenance>,
    semantic_digest: [u8; 32],
    attachments: Option<Arc<PresentationAttachments>>,
    indexes: Arc<SnapshotIndexes>,
    holes: Arc<[HoleOverlay]>,
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
            hir: Arc::clone(&self.hir),
            provenance: Arc::clone(&self.provenance),
            semantic_digest: self.semantic_digest,
            attachments: None,
            indexes: Arc::clone(&self.indexes),
            holes: Arc::clone(&self.holes),
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
        self.holes.iter().map(|overlay| &overlay.state)
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
        let indexes = index::build(&hir, namespace)?;
        let semantic_digest = provenance.semantic_base_identity();
        Ok(Self {
            namespace,
            revision: RevisionId::initial(namespace),
            state: ProgramState::Complete,
            hir: Arc::new(hir),
            provenance: Arc::new(provenance),
            semantic_digest,
            attachments: attachments.map(Arc::new),
            indexes: Arc::new(indexes),
            holes: Arc::from([]),
        })
    }

    pub(crate) fn validate_consistency(&self) -> Result<()> {
        if self.revision.namespace() != self.namespace {
            return Err(Error::msg(
                "workspace revision and namespace are inconsistent",
            ));
        }
        if self.state != ProgramState::Complete || !self.holes.is_empty() {
            return Err(Error::msg("workspace snapshot is not executable"));
        }
        validate::program(&self.hir)?;
        if self.indexes.nodes.len() != self.indexes.node_addresses.len()
            || self.indexes.nodes.len() != self.indexes.node_expected_types.len()
            || self.indexes.entities.len() != self.indexes.entity_addresses.len()
        {
            return Err(Error::msg("workspace snapshot semantic indexes are stale"));
        }
        Ok(())
    }

    pub(crate) fn hir(&self) -> &crate::hir::Program {
        &self.hir
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

    #[cfg(test)]
    fn from_hir_for_test(
        namespace: WorkspaceNamespace,
        hir: crate::hir::Program,
        provenance: Arc<CapturedCompilationProvenance>,
    ) -> Result<Self> {
        validate::program(&hir)?;
        let indexes = index::build(&hir, namespace)?;
        Ok(Self {
            namespace,
            revision: RevisionId::initial(namespace),
            state: ProgramState::Complete,
            hir: Arc::new(hir),
            semantic_digest: provenance.semantic_base_identity(),
            provenance,
            attachments: None,
            indexes: Arc::new(indexes),
            holes: Arc::from([]),
        })
    }

    #[cfg(test)]
    fn malformed_hir_for_test(&self, hir: crate::hir::Program) -> Self {
        Self {
            namespace: self.namespace,
            revision: self.revision,
            state: self.state,
            hir: Arc::new(hir),
            provenance: Arc::clone(&self.provenance),
            semantic_digest: self.semantic_digest,
            attachments: self.attachments.clone(),
            indexes: Arc::clone(&self.indexes),
            holes: Arc::clone(&self.holes),
        }
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
