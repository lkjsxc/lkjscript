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

use std::collections::HashSet;
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
    SemanticOwner, SourceAttachment, UnresolvedValueReferenceId, UnresolvedValueReferenceState,
    ValueReferenceIntent,
};
use model::{HoleRecord, SnapshotIndexes, UnresolvedValueReferenceRecord};
use program::SemanticProgram;
pub use projection::ProjectionSlice;
pub use query::{
    CallInstantiationView, ConstructorStatus, Continuation, EffectSummary, EntityPage,
    EntityTypeFacts, FunctionSignatureView, LegalConstructor, MatchArmView, MatchPatternFieldView,
    MatchPatternKindView, MatchPatternLabel, MatchPatternNodeView, MatchView, NodeSemanticFacts,
    NodeTypeFacts, PageRequest, QueryPage, TraitWitnessKindView, TraitWitnessView,
    TypeArgumentView, TypeParameterBoundView, TypeParameterView, ValueParameterView,
    ValueReferenceCandidate, ValueReferenceCandidateStatus,
};
pub use transaction::{
    Edit, EnumFieldDraft, EnumTypeParameterDraft, EnumVariantDraft, InvalidatedDomain,
    ParameterDraft, ProductFieldDraft, SemanticDiff, SemanticDiffEntry, Transaction,
    TransactionOutcome, TypeParameterDraft, Workspace,
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
    unresolved_value_references: Arc<[UnresolvedValueReferenceRecord]>,
    diagnostics: Arc<[DiagnosticHeader]>,
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
            unresolved_value_references: Arc::clone(&self.unresolved_value_references),
            diagnostics: Arc::clone(&self.diagnostics),
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
        &self.diagnostics
    }

    pub fn holes(&self) -> impl ExactSizeIterator<Item = &HoleState> {
        self.holes.iter().map(|record| &record.state)
    }

    pub fn unresolved_value_references(
        &self,
    ) -> impl ExactSizeIterator<Item = &UnresolvedValueReferenceState> {
        self.unresolved_value_references
            .iter()
            .map(|record| &record.state)
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
            unresolved_value_references: Arc::from([]),
            diagnostics: Arc::from([]),
            blockers: Arc::from([]),
            allocator,
        })
    }

    fn empty(namespace: WorkspaceNamespace) -> Result<Self> {
        let program = SemanticProgram::empty()?;
        let indexes = index::build(&program, namespace)?;
        let diagnostics = Arc::from([DiagnosticHeader {
            code: Arc::from("workspace.missing-entry-point"),
            severity: DiagnosticSeverity::Error,
            subject: None,
            message: Arc::from("program requires a main entry point"),
        }]);
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
            unresolved_value_references: Arc::from([]),
            diagnostics,
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
        let indexed_unresolved = self
            .indexes
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::UnresolvedValueReference)
            .count();
        if indexed_holes != self.holes.len()
            || indexed_unresolved != self.unresolved_value_references.len()
            || self.blockers.len() != self.diagnostics.len()
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
        let mut unresolved_ids = HashSet::new();
        unresolved_ids
            .try_reserve(self.unresolved_value_references.len())
            .map_err(|_| Error::host("workspace unresolved identity allocation failed"))?;
        for reference in self.unresolved_value_references.iter() {
            if !unresolved_ids.insert(reference.state.id.node()) {
                return Err(Error::msg(
                    "workspace unresolved value-reference identity is duplicated",
                ));
            }
            let index = self
                .indexes
                .node_lookup
                .get(&reference.state.id.0)
                .copied()
                .ok_or_else(|| Error::msg("workspace unresolved reference identity is stale"))?;
            let expression = semantic_expression_at(&self.program, reference.address)?;
            let requested_name = match &expression.kind {
                crate::hir::ExprKind::UnresolvedValueReference { requested_name } => requested_name,
                _ => {
                    return Err(Error::msg(
                        "workspace unresolved reference expression is inconsistent",
                    ));
                }
            };
            let canonical_owner = self
                .indexes
                .address_entities
                .get(&reference.address.root)
                .copied()
                .ok_or_else(|| Error::msg("workspace unresolved reference owner is stale"))?;
            let canonical_expected = types::view(
                &self.program,
                &self.indexes,
                &reference.expected_internal,
                Some(canonical_owner),
            )
            .map_err(consistency_workspace_error)?;
            let canonical_visibility =
                transaction::visible_entities_in(&self.program, &self.indexes, reference.address)
                    .map_err(consistency_workspace_error)?;
            if reference.state.revision != self.revision
                || reference.state.intent != ValueReferenceIntent::CopyLoad
                || self.indexes.nodes[index].kind != NodeKind::UnresolvedValueReference
                || self.indexes.node_addresses[index] != reference.address
                || self.indexes.node_keys[index] != reference.key
                || self.indexes.node_expected_types[index].as_ref()
                    != Some(&reference.expected_internal)
                || self.indexes.node_actual_types[index] != reference.expected_internal
                || reference.state.owner != canonical_owner
                || reference.state.context != reference.state.id.node()
                || reference.state.expected_type != canonical_expected
                || requested_name.as_ref() != reference.state.requested_name.as_ref()
                || !lkjscript_contracts::is_identifier(requested_name)
                || reference.state.visible_entities.as_ref() != canonical_visibility.as_slice()
            {
                return Err(Error::msg(
                    "workspace unresolved value-reference facts are inconsistent",
                ));
            }
        }
        if self
            .indexes
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::UnresolvedValueReference)
            .any(|node| !unresolved_ids.contains(&node.id))
        {
            return Err(Error::msg(
                "workspace unresolved value-reference record is missing",
            ));
        }
        let mut expected_blockers = Vec::new();
        if self.program.main.is_none() {
            expected_blockers.push(CompletenessBlocker::MissingEntryPoint);
        }
        let mut expression_blockers = Vec::new();
        for hole in self.holes.iter() {
            expression_blockers.push(match hole.state.kind {
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
        for reference in self.unresolved_value_references.iter() {
            expression_blockers.push(CompletenessBlocker::UnresolvedValueReference {
                reference: reference.state.id,
                requested_name: Arc::clone(&reference.state.requested_name),
                expected_type: reference.state.expected_type.clone(),
                owner: reference.state.owner,
                context: reference.state.context,
            });
        }
        expression_blockers.sort_by_key(incomplete_blocker_node);
        expected_blockers.extend(expression_blockers);
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

fn consistency_workspace_error(error: WorkspaceError) -> Error {
    match error {
        WorkspaceError::Host(message) => Error::host(message.to_string()),
        other => Error::msg(other.to_string()),
    }
}

fn incomplete_blocker_node(blocker: &CompletenessBlocker) -> Option<NodeId> {
    match blocker {
        CompletenessBlocker::MissingEntryPoint => None,
        CompletenessBlocker::MissingBody { hole, .. }
        | CompletenessBlocker::TypedHole { hole, .. } => Some(hole.node()),
        CompletenessBlocker::UnresolvedValueReference { reference, .. } => Some(reference.node()),
    }
}

fn semantic_expression_at(
    program: &SemanticProgram,
    address: model::NodeAddress,
) -> Result<&crate::hir::Expr> {
    let root = match address.root {
        model::EntityAddress::Main => program
            .main
            .as_ref()
            .map(|main| &main.body)
            .ok_or_else(|| Error::msg("workspace main expression root is stale"))?,
        model::EntityAddress::Binding(raw) => program
            .functions
            .iter()
            .find(|function| function.binding.raw() == raw)
            .map(|function| &function.body)
            .ok_or_else(|| Error::msg("workspace function expression root is stale"))?,
        _ => return Err(Error::msg("workspace node has a non-callable root")),
    };
    root.try_at_preorder(address.preorder)?
        .ok_or_else(|| Error::msg("workspace expression address is stale"))
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
mod movement_tests;
#[cfg(test)]
mod recompute_measurement;
#[cfg(test)]
mod tests;
