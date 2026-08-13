use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::hir::{
    Binding, BindingKind, BindingRef, BindingStorage, EffectSet, Expr, ExprKind, Origin, PlaceId,
    Type,
};

#[cfg(test)]
thread_local! {
    static PATTERN_LOWERING_NODE_VISITS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static DRAFT_LOWERING_NODE_VISITS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static DRAFT_SCOPE_NODE_VISITS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static BINDING_LOCATION_NODE_VISITS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static STABLE_BINDING_LOOKUPS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn reset_pattern_lowering_node_visits() {
    PATTERN_LOWERING_NODE_VISITS.with(|count| count.set(0));
}

#[cfg(test)]
pub(super) fn pattern_lowering_node_visits() -> u64 {
    PATTERN_LOWERING_NODE_VISITS.with(std::cell::Cell::get)
}

#[cfg(test)]
pub(super) fn reset_draft_imperative_node_visits() {
    DRAFT_LOWERING_NODE_VISITS.with(|count| count.set(0));
    DRAFT_SCOPE_NODE_VISITS.with(|count| count.set(0));
}

#[cfg(test)]
pub(super) fn draft_imperative_node_visits() -> (u64, u64) {
    (
        DRAFT_LOWERING_NODE_VISITS.with(std::cell::Cell::get),
        DRAFT_SCOPE_NODE_VISITS.with(std::cell::Cell::get),
    )
}

#[cfg(test)]
pub(super) fn reset_binding_location_work() {
    BINDING_LOCATION_NODE_VISITS.with(|count| count.set(0));
    STABLE_BINDING_LOOKUPS.with(|count| count.set(0));
}

#[cfg(test)]
pub(super) fn binding_location_work() -> (u64, u64) {
    (
        BINDING_LOCATION_NODE_VISITS.with(std::cell::Cell::get),
        STABLE_BINDING_LOOKUPS.with(std::cell::Cell::get),
    )
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct TransactionMeasurement {
    pub stage_wall: std::time::Duration,
    pub program_clone: std::time::Duration,
    pub edit_staging: std::time::Duration,
    pub compaction: std::time::Duration,
    pub effect_inference: std::time::Duration,
    pub complete_validation: std::time::Duration,
    pub index_build: std::time::Duration,
    pub identity_reconciliation: std::time::Duration,
    pub finalization: std::time::Duration,
    pub program_clones: usize,
    pub functions_cloned: usize,
    pub semantic_nodes_cloned: usize,
    pub bindings_cloned: usize,
    pub products_cloned: usize,
    pub enums_cloned: usize,
    pub implementations_cloned: usize,
    pub match_plans_cloned: usize,
    pub compaction_invocations: usize,
    pub compaction_roots: usize,
    pub effect_inference_invocations: usize,
    pub effect_roots: usize,
    pub complete_hir_derivations: usize,
    pub complete_hir_nodes: usize,
    pub index_build_invocations: usize,
    pub index_entities_built: usize,
    pub index_nodes_built: usize,
    pub identity_reconciliation_invocations: usize,
    pub identity_entity_records_examined: usize,
    pub identity_node_records_examined: usize,
    pub movement_child_blocks_examined: usize,
    pub movement_nodes_relocated: usize,
    pub metadata_only_path_used: bool,
}

#[cfg(test)]
thread_local! {
    static TRANSACTION_MEASUREMENT: std::cell::RefCell<TransactionMeasurement> =
        std::cell::RefCell::default();
    static FORCE_FULL_RECOMPUTATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(super) fn set_force_full_recomputation(force: bool) {
    FORCE_FULL_RECOMPUTATION.with(|value| value.set(force));
}

#[cfg(test)]
fn force_full_recomputation() -> bool {
    FORCE_FULL_RECOMPUTATION.with(std::cell::Cell::get)
}

#[cfg(test)]
fn reset_transaction_measurement() {
    TRANSACTION_MEASUREMENT.with(|measurement| {
        *measurement.borrow_mut() = TransactionMeasurement::default();
    });
}

#[cfg(test)]
fn record_transaction_measurement(action: impl FnOnce(&mut TransactionMeasurement)) {
    TRANSACTION_MEASUREMENT.with(|measurement| action(&mut measurement.borrow_mut()));
}

#[cfg(test)]
pub(super) fn take_transaction_measurement() -> TransactionMeasurement {
    TRANSACTION_MEASUREMENT.with(|measurement| std::mem::take(&mut *measurement.borrow_mut()))
}

use super::identity::{self, IdentityAllocator};
use super::model::{
    EntityAddress, HoleRecord, NodeAddress, NodeKey, SnapshotIndexes,
    UnresolvedValueReferenceRecord,
};
use super::program::SemanticProgram;
use super::{
    CompletenessBlocker, DeclarationType, DiagnosticHeader, DiagnosticSeverity, DraftBindingId,
    DraftBindingRef, DraftFieldValue, DraftNode, DraftNodeId, DraftPatternNode, DraftPatternNodeId,
    DraftTypeParameterId, EntityId, EntityKind, ExpressionDraft, HoleId, HoleKind, HoleState,
    NodeId, NodeKind, PatternDraft, ProgramState, RevisionId, SemanticChild, SemanticKind,
    SemanticOwner, SemanticTrait, SemanticType, UnresolvedValueReferenceId,
    UnresolvedValueReferenceState, ValueReferenceIntent, WorkspaceError, WorkspaceNamespace,
    WorkspaceSnapshot,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeParameterDraft {
    pub id: DraftTypeParameterId,
    pub name: String,
    pub bounds: Vec<SemanticTrait>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterDraft {
    pub name: String,
    pub ty: DeclarationType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductFieldDraft {
    pub name: String,
    pub ty: SemanticType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumTypeParameterDraft {
    pub id: DraftTypeParameterId,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumFieldDraft {
    pub name: String,
    pub ty: DeclarationType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumVariantDraft {
    pub name: String,
    pub fields: Vec<EnumFieldDraft>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transaction {
    pub base_revision: RevisionId,
    pub edits: Vec<Edit>,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Edit {
    CreateProduct {
        name: String,
        fields: Vec<ProductFieldDraft>,
    },
    CreateEnum {
        name: String,
        type_parameters: Vec<EnumTypeParameterDraft>,
        variants: Vec<EnumVariantDraft>,
    },
    CreateFunction {
        name: String,
        type_parameters: Vec<TypeParameterDraft>,
        parameters: Vec<ParameterDraft>,
        return_type: DeclarationType,
    },
    CreateMain {
        parameters: Vec<ParameterDraft>,
        return_type: SemanticType,
    },
    DeleteEntity {
        entity: EntityId,
    },
    RenameEntity {
        entity: EntityId,
        new_name: String,
    },
    ReplaceExpression {
        target: NodeId,
        draft: ExpressionDraft,
    },
    MoveSequenceChild {
        sequence: NodeId,
        child: NodeId,
        before: Option<NodeId>,
    },
    IntroduceHole {
        target: NodeId,
        goal: String,
    },
    IntroduceUnresolvedValueReference {
        target: NodeId,
        requested_name: String,
    },
    ResolveUnresolvedValueReference {
        reference: UnresolvedValueReferenceId,
        target: EntityId,
    },
    RefineHole {
        hole: HoleId,
        expected_type: Option<SemanticType>,
        goal: String,
    },
    FillHole {
        hole: HoleId,
        draft: ExpressionDraft,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum InvalidatedDomain {
    SemanticIndexes,
    Types,
    Effects,
    Ownership,
    Diagnostics,
    Executable,
    Provenance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SemanticDiffEntry {
    EntityCreated {
        entity: EntityId,
        kind: EntityKind,
        name: Arc<str>,
    },
    EntityRenamed {
        entity: EntityId,
        old_name: Arc<str>,
        new_name: Arc<str>,
    },
    EntityDeleted {
        entity: EntityId,
        kind: EntityKind,
        name: Arc<str>,
    },
    ExpressionReplaced {
        node: NodeId,
        old_kind: NodeKind,
        new_kind: NodeKind,
    },
    SequenceChildMoved {
        sequence: NodeId,
        child: NodeId,
        old_predecessor: Option<NodeId>,
        old_successor: Option<NodeId>,
        new_predecessor: Option<NodeId>,
        new_successor: Option<NodeId>,
    },
    DescendantCreated {
        parent: SemanticOwner,
        node: NodeId,
        kind: NodeKind,
    },
    DescendantDeleted {
        parent: SemanticOwner,
        node: NodeId,
        kind: NodeKind,
    },
    HoleIntroduced {
        hole: HoleId,
    },
    HoleRefined {
        hole: HoleId,
        old_goal: Arc<str>,
        new_goal: Arc<str>,
    },
    HoleFilled {
        hole: HoleId,
    },
    UnresolvedValueReferenceIntroduced {
        reference: UnresolvedValueReferenceId,
    },
    UnresolvedValueReferenceResolved {
        reference: UnresolvedValueReferenceId,
        target: EntityId,
    },
    ReferenceRewired {
        site: NodeId,
        old_target: Option<EntityId>,
        new_target: Option<EntityId>,
    },
    CallRewired {
        site: NodeId,
        old_callee: Option<EntityId>,
        new_callee: Option<EntityId>,
    },
    CallInstantiationChanged {
        site: NodeId,
        old: Box<super::CallInstantiationView>,
        new: Box<super::CallInstantiationView>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDiff {
    pub base_revision: RevisionId,
    pub revision: RevisionId,
    pub entries: Vec<SemanticDiffEntry>,
}

#[derive(Clone, Debug)]
pub struct TransactionOutcome {
    pub snapshot: Arc<WorkspaceSnapshot>,
    pub diff: SemanticDiff,
    pub diagnostics: Vec<DiagnosticHeader>,
    pub invalidated: Vec<InvalidatedDomain>,
}

pub struct Workspace {
    current: Arc<WorkspaceSnapshot>,
    allocator: IdentityAllocator,
}

impl Workspace {
    pub fn empty() -> Result<Self, WorkspaceError> {
        Self::empty_in_namespace(WorkspaceNamespace::fresh().map_err(WorkspaceError::from_core)?)
    }

    fn empty_in_namespace(namespace: WorkspaceNamespace) -> Result<Self, WorkspaceError> {
        Self::new(WorkspaceSnapshot::empty(namespace).map_err(WorkspaceError::from_core)?)
    }

    #[cfg(test)]
    pub(super) fn empty_deterministic(seed: u64) -> Result<Self, WorkspaceError> {
        Self::empty_in_namespace(WorkspaceNamespace::deterministic(seed))
    }

    pub fn new(snapshot: WorkspaceSnapshot) -> Result<Self, WorkspaceError> {
        let allocator = snapshot.allocator.clone();
        Ok(Self {
            current: Arc::new(snapshot),
            allocator,
        })
    }

    pub fn current(&self) -> Arc<WorkspaceSnapshot> {
        Arc::clone(&self.current)
    }

    pub fn apply(
        &mut self,
        transaction: Transaction,
    ) -> Result<TransactionOutcome, WorkspaceError> {
        self.current
            .check_query_revision(transaction.base_revision)?;
        if transaction.edits.is_empty() {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                "semantic transaction contains no edits",
            )));
        }
        let mut staged_allocator = self.allocator.clone();
        let (snapshot, diff, invalidated) =
            stage(&self.current, transaction, &mut staged_allocator)?;
        let snapshot = Arc::new(snapshot);
        let mut diagnostics = Vec::new();
        diagnostics
            .try_reserve(snapshot.diagnostics().len())
            .map_err(|_| {
                WorkspaceError::Host(Arc::from("transaction diagnostic allocation failed"))
            })?;
        diagnostics.extend(snapshot.diagnostics().iter().cloned());
        self.current = Arc::clone(&snapshot);
        self.allocator = staged_allocator;
        Ok(TransactionOutcome {
            snapshot,
            diff,
            diagnostics,
            invalidated,
        })
    }
}

#[derive(Clone)]
struct StructuralAction {
    target: NodeId,
    address: NodeAddress,
    replacement: Expr,
}

struct SequenceMovement {
    sequence: NodeId,
    child: NodeId,
    address: NodeAddress,
    path: Vec<usize>,
    old_children: Vec<NodeId>,
    final_old_ordinals: Vec<usize>,
    old_index: usize,
    new_index: usize,
    old_predecessor: Option<NodeId>,
    old_successor: Option<NodeId>,
    new_predecessor: Option<NodeId>,
    new_successor: Option<NodeId>,
    sequence_type: Type,
    ancestor_types: Vec<Type>,
}

struct NewHole {
    address: NodeAddress,
    kind: HoleKind,
    goal: Arc<str>,
}

struct NewEntity {
    address: EntityAddress,
    kind: EntityKind,
    name: Arc<str>,
}

#[derive(Clone)]
struct RequestedDeletion {
    entity: EntityId,
    address: EntityAddress,
    kind: EntityKind,
    name: Arc<str>,
}

struct DeletionPlan {
    requested: Vec<RequestedDeletion>,
    entities: HashSet<EntityId>,
    callable_roots: HashSet<EntityAddress>,
    callable_bindings: HashSet<crate::hir::BindingId>,
    products: HashSet<lkjscript_core::ProductId>,
    enum_vectors: HashSet<u64>,
    implementations: HashSet<crate::hir::ImplId>,
    product_requests: HashMap<lkjscript_core::ProductId, EntityId>,
    enum_requests: HashMap<crate::hir::EnumId, EntityId>,
    implementation_requests: HashMap<crate::hir::ImplId, EntityId>,
    binding_requests: HashMap<crate::hir::BindingId, EntityId>,
}

fn stage(
    base: &WorkspaceSnapshot,
    transaction: Transaction,
    allocator: &mut IdentityAllocator,
) -> Result<(WorkspaceSnapshot, SemanticDiff, Vec<InvalidatedDomain>), WorkspaceError> {
    #[cfg(test)]
    reset_transaction_measurement();
    #[cfg(test)]
    let stage_started = std::time::Instant::now();
    let revision = base.revision.next().map_err(WorkspaceError::from_core)?;
    let edit_count = transaction.edits.len();
    let refinements_only = transaction
        .edits
        .iter()
        .all(|edit| matches!(edit, Edit::RefineHole { .. }));
    #[cfg(test)]
    let refinements_only = refinements_only && !force_full_recomputation();
    if refinements_only {
        return stage_hole_refinements(base, revision, transaction.edits, allocator);
    }
    let mut movement = preflight_sequence_movement(base, &transaction.edits)?;
    #[cfg(test)]
    let clone_started = std::time::Instant::now();
    let mut program = try_clone_program(base.program.as_ref())?;
    #[cfg(test)]
    record_transaction_measurement(|measurement| {
        measurement.program_clone = clone_started.elapsed();
        measurement.program_clones = 1;
        measurement.functions_cloned = base.program.functions.len();
        measurement.semantic_nodes_cloned = base.indexes.nodes.len();
        measurement.bindings_cloned = base.program.bindings.len();
        measurement.products_cloned = base.program.products.len();
        measurement.enums_cloned = base.program.enums.len();
        measurement.implementations_cloned = base.program.implementations.len();
        measurement.match_plans_cloned = base.program.match_plans.len();
    });
    #[cfg(test)]
    let edit_staging_started = std::time::Instant::now();
    let deletions = preflight_deletions(base, &program, &transaction.edits)?;
    preflight_structural_edits(base, &transaction.edits, movement.as_ref())?;
    if let Some(movement) = movement.as_ref() {
        reject_deleted_root_edit(&deletions.callable_roots, movement.address.root)?;
    }
    let deleted_entities = &deletions.entities;
    let deleted_roots = &deletions.callable_roots;
    let deleted_bindings = &deletions.callable_bindings;
    let mut holes = Vec::new();
    holes
        .try_reserve(base.holes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole staging allocation failed")))?;
    holes.extend(base.holes.iter().cloned());
    let unresolved_capacity = base
        .unresolved_value_references
        .len()
        .checked_add(edit_count)
        .ok_or_else(|| {
            WorkspaceError::Host(Arc::from(
                "unresolved value-reference staging count overflow",
            ))
        })?;
    let mut unresolved_value_references = Vec::new();
    unresolved_value_references
        .try_reserve(unresolved_capacity)
        .map_err(|_| {
            WorkspaceError::Host(Arc::from(
                "unresolved value-reference staging allocation failed",
            ))
        })?;
    unresolved_value_references.extend(base.unresolved_value_references.iter().cloned());
    prune_replaced_incomplete_subtrees(
        base,
        &mut holes,
        &mut unresolved_value_references,
        &transaction.edits,
    )?;
    let mut structural = Vec::new();
    structural
        .try_reserve(edit_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("structural edit allocation failed")))?;
    let mut new_holes = Vec::new();
    new_holes
        .try_reserve(edit_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("new hole allocation failed")))?;
    let mut new_entities = Vec::new();
    new_entities
        .try_reserve(edit_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("new entity allocation failed")))?;
    let mut entries = Vec::new();
    entries
        .try_reserve(edit_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("semantic diff allocation failed")))?;
    let mut renamed = HashSet::new();
    renamed
        .try_reserve(edit_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("rename preflight allocation failed")))?;
    let mut forced_entities = HashMap::new();
    forced_entities
        .try_reserve(edit_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("forced entity allocation failed")))?;
    let mut lowering = LoweringState::new(&program)?;

    for edit in transaction.edits {
        match edit {
            Edit::CreateProduct { name, fields } => {
                create_product(
                    base,
                    &mut program,
                    allocator,
                    &mut forced_entities,
                    &mut new_entities,
                    name,
                    fields,
                )?;
            }
            Edit::CreateEnum {
                name,
                type_parameters,
                variants,
            } => {
                create_enum(
                    base,
                    &mut program,
                    allocator,
                    &mut forced_entities,
                    &mut new_entities,
                    name,
                    type_parameters,
                    variants,
                )?;
            }
            Edit::CreateFunction {
                name,
                type_parameters,
                parameters,
                return_type,
            } => create_function(
                base,
                &mut program,
                allocator,
                &mut forced_entities,
                &mut new_entities,
                &mut new_holes,
                name,
                type_parameters,
                parameters,
                return_type,
            )?,
            Edit::CreateMain {
                parameters,
                return_type,
            } => create_main(
                base,
                &mut program,
                &mut new_entities,
                &mut new_holes,
                parameters,
                return_type,
            )?,
            Edit::DeleteEntity { .. } => {}
            Edit::RenameEntity { entity, new_name } => {
                if deleted_entities.contains(&entity) {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "an entity cannot be renamed and deleted in one transaction",
                    )));
                }
                if !renamed.insert(entity) {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "entity is renamed more than once in one transaction",
                    )));
                }
                let header = base.workspace_entity(entity)?;
                if !matches!(
                    header.kind,
                    EntityKind::Function
                        | EntityKind::Parameter
                        | EntityKind::ImmutableLocal
                        | EntityKind::StaticBytesLocal
                        | EntityKind::MutableLocal
                        | EntityKind::Product
                        | EntityKind::ProductField
                        | EntityKind::Enum
                        | EntityKind::EnumVariant
                        | EntityKind::EnumField
                ) {
                    return Err(WorkspaceError::unsupported(
                        "rename-entity",
                        "this entity kind cannot be renamed",
                    ));
                }
                if header.name.as_ref() == new_name {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "rename must change the entity name",
                    )));
                }
                validate_name(&new_name)?;
                let address = base
                    .indexes
                    .entity_lookup
                    .get(&entity)
                    .and_then(|index| base.indexes.entity_addresses.get(*index))
                    .copied()
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("entity")))?;
                rename_entity(&mut program, address, header.kind, &new_name)?;
                entries.push(SemanticDiffEntry::EntityRenamed {
                    entity,
                    old_name: Arc::clone(&header.name),
                    new_name: Arc::from(new_name),
                });
            }
            Edit::ReplaceExpression { target, draft } => {
                let (address, _key, expected, visible) = edit_context(base, target)?;
                reject_deleted_root_edit(deleted_roots, address.root)?;
                let lowered = crate::stack::grow(|| {
                    lower_draft(
                        base,
                        &mut program,
                        &draft,
                        &expected,
                        Origin::Semantic,
                        &visible,
                        address,
                        &mut lowering,
                        deleted_entities,
                    )
                })?;
                new_entities.extend(lowered.entities);
                structural.push(StructuralAction {
                    target,
                    address,
                    replacement: lowered.expression,
                });
            }
            Edit::MoveSequenceChild { .. } => {}
            Edit::IntroduceHole { target, goal } => {
                if goal.is_empty() {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "typed hole goal must not be empty",
                    )));
                }
                let (address, key, expected, visible) = edit_context(base, target)?;
                reject_deleted_root_edit(deleted_roots, address.root)?;
                let owner = root_owner(base, address)?;
                holes.push(HoleRecord {
                    state: HoleState {
                        id: HoleId(target),
                        kind: HoleKind::TypedExpression,
                        expected_type: super::types::view(
                            &base.program,
                            &base.indexes,
                            &expected,
                            Some(owner),
                        )?,
                        goal: Arc::from(goal),
                        owner,
                        context: target,
                        visible_entities: visible.into(),
                    },
                    expected_internal: expected.clone(),
                    address,
                    key,
                });
                structural.push(StructuralAction {
                    target,
                    address,
                    replacement: Expr {
                        ty: expected,
                        effects: EffectSet::UNKNOWN,
                        origin: Origin::Semantic,
                        kind: ExprKind::Hole,
                    },
                });
                entries.push(SemanticDiffEntry::HoleIntroduced {
                    hole: HoleId(target),
                });
            }
            Edit::IntroduceUnresolvedValueReference {
                target,
                requested_name,
            } => {
                validate_name(&requested_name)?;
                let (address, key, expected, visible) =
                    unresolved_introduction_context(base, target)?;
                reject_deleted_root_edit(deleted_roots, address.root)?;
                let owner = root_owner(base, address)?;
                let requested_name: Arc<str> = Arc::from(requested_name);
                unresolved_value_references.push(UnresolvedValueReferenceRecord {
                    state: UnresolvedValueReferenceState {
                        revision,
                        id: UnresolvedValueReferenceId(target),
                        intent: ValueReferenceIntent::CopyLoad,
                        requested_name: Arc::clone(&requested_name),
                        expected_type: super::types::view(
                            &base.program,
                            &base.indexes,
                            &expected,
                            Some(owner),
                        )?,
                        owner,
                        context: target,
                        visible_entities: visible.into(),
                    },
                    expected_internal: expected.clone(),
                    address,
                    key,
                });
                structural.push(StructuralAction {
                    target,
                    address,
                    replacement: Expr {
                        ty: expected,
                        effects: EffectSet::UNKNOWN,
                        origin: Origin::Semantic,
                        kind: ExprKind::UnresolvedValueReference { requested_name },
                    },
                });
                entries.push(SemanticDiffEntry::UnresolvedValueReferenceIntroduced {
                    reference: UnresolvedValueReferenceId(target),
                });
            }
            Edit::ResolveUnresolvedValueReference { reference, target } => {
                require_unresolved_value_reference(base, reference)?;
                let index = unresolved_value_references
                    .iter()
                    .position(|record| record.state.id == reference)
                    .ok_or_else(|| {
                        WorkspaceError::StaleIdentity(Arc::from("unresolved value reference"))
                    })?;
                let record = unresolved_value_references[index].clone();
                reject_deleted_root_edit(deleted_roots, record.address.root)?;
                let resolution_visible = visible_entities(base, record.address)?;
                let mut draft_nodes = Vec::new();
                draft_nodes.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from(
                        "unresolved value-reference resolution allocation failed",
                    ))
                })?;
                draft_nodes.push(DraftNode::Load(DraftBindingRef::Entity(target)));
                let draft = ExpressionDraft::new(draft_nodes, DraftNodeId::new(0));
                let lowered = crate::stack::grow(|| {
                    lower_draft(
                        base,
                        &mut program,
                        &draft,
                        &record.expected_internal,
                        Origin::Semantic,
                        &resolution_visible,
                        record.address,
                        &mut lowering,
                        deleted_entities,
                    )
                })?;
                new_entities.extend(lowered.entities);
                structural.push(StructuralAction {
                    target: reference.0,
                    address: record.address,
                    replacement: lowered.expression,
                });
                unresolved_value_references.remove(index);
                entries.push(SemanticDiffEntry::UnresolvedValueReferenceResolved {
                    reference,
                    target,
                });
            }
            Edit::RefineHole {
                hole,
                expected_type,
                goal,
            } => entries.push(refine_hole(
                base,
                &program,
                &mut holes,
                Some(deleted_roots),
                hole,
                expected_type,
                goal,
            )?),
            Edit::FillHole { hole, draft } => {
                if hole.0.namespace() != base.namespace {
                    return Err(WorkspaceError::ForeignNamespace(Arc::from("hole")));
                }
                let index = holes
                    .iter()
                    .position(|record| record.state.id == hole)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole")))?;
                let record = holes[index].clone();
                reject_deleted_root_edit(deleted_roots, record.address.root)?;
                let lowered = crate::stack::grow(|| {
                    lower_draft(
                        base,
                        &mut program,
                        &draft,
                        &record.expected_internal,
                        Origin::Semantic,
                        &record.state.visible_entities,
                        record.address,
                        &mut lowering,
                        deleted_entities,
                    )
                })?;
                new_entities.extend(lowered.entities);
                structural.push(StructuralAction {
                    target: hole.0,
                    address: record.address,
                    replacement: lowered.expression,
                });
                holes.remove(index);
                entries.push(SemanticDiffEntry::HoleFilled { hole });
            }
        }
    }

    structural.sort_by(|left, right| {
        right
            .address
            .root
            .cmp(&left.address.root)
            .then_with(|| right.address.preorder.cmp(&left.address.preorder))
    });
    for action in &structural {
        replace_expression(&mut program, action.address, &action.replacement)?;
    }
    if let Some(movement) = movement.as_ref() {
        apply_sequence_movement(&mut program, movement)?;
        entries.push(SemanticDiffEntry::SequenceChildMoved {
            sequence: movement.sequence,
            child: movement.child,
            old_predecessor: movement.old_predecessor,
            old_successor: movement.old_successor,
            new_predecessor: movement.new_predecessor,
            new_successor: movement.new_successor,
        });
    }
    if (!structural.is_empty() || movement.is_some()) && !program.match_plans.is_empty() {
        refresh_semantic_match_types(&mut program, &structural, movement.as_ref())?;
    }

    reject_surviving_deleted_dependencies(base, &program, &deletions)?;
    holes.retain(|hole| !deleted_roots.contains(&hole.address.root));
    unresolved_value_references
        .retain(|reference| !deleted_roots.contains(&reference.address.root));
    new_holes.retain(|hole| !deleted_roots.contains(&hole.address.root));
    if deleted_roots.contains(&EntityAddress::Main) {
        program.main = None;
    }
    program
        .functions
        .retain(|function| !deleted_bindings.contains(&function.binding));
    program
        .global_layout
        .retain(|binding| !deleted_bindings.contains(binding));
    reject_external_incompleteness_for_movement(
        &program,
        &holes,
        &unresolved_value_references,
        &new_holes,
        movement.as_ref(),
    )?;

    #[cfg(test)]
    record_transaction_measurement(|measurement| {
        measurement.edit_staging = edit_staging_started.elapsed();
    });
    #[cfg(test)]
    let compaction_started = std::time::Instant::now();
    let compaction =
        super::compaction::compact(&mut program, &deletions.products, &deletions.enum_vectors)
            .map_err(WorkspaceError::from_core)?;
    for hole in &mut holes {
        hole.expected_internal = hole
            .expected_internal
            .try_remap_products(&compaction.products)
            .map_err(WorkspaceError::from_core)?;
    }
    for reference in &mut unresolved_value_references {
        reference.expected_internal = reference
            .expected_internal
            .try_remap_products(&compaction.products)
            .map_err(WorkspaceError::from_core)?;
    }
    #[cfg(test)]
    record_transaction_measurement(|measurement| {
        measurement.compaction = compaction_started.elapsed();
        measurement.compaction_invocations = 1;
        measurement.compaction_roots =
            program.functions.len() + usize::from(program.main.is_some());
    });
    if deletions
        .implementations
        .iter()
        .any(|implementation| compaction.implementations.contains_key(implementation))
    {
        return Err(WorkspaceError::Validation(Arc::from(
            "implementation selected for deletion survived compaction",
        )));
    }
    remap_forced_entity_addresses(&compaction, &mut forced_entities)?;
    remap_staged_addresses(&compaction, &mut new_entities, &mut new_holes)?;
    if let Some(movement) = movement.as_mut() {
        movement.address.root = remap_entity_address(&compaction, movement.address.root)
            .ok_or_else(|| {
                WorkspaceError::Validation(Arc::from("moved sequence owner was removed"))
            })?;
    }
    install_survivor_entity_relocations(base, &program, &compaction, &mut forced_entities)?;
    reserve_new_entity_identities(base, allocator, &mut forced_entities, &new_entities)?;

    #[cfg(test)]
    let effects_started = std::time::Instant::now();
    let binding_count = program.bindings.len();
    #[cfg(test)]
    let effect_roots = program.functions.len() + usize::from(program.main.is_some());
    let main_body = program.main.as_mut().map(|main| &mut main.body);
    crate::effects::infer_partial(binding_count, &mut program.functions, main_body);
    #[cfg(test)]
    record_transaction_measurement(|measurement| {
        measurement.effect_inference = effects_started.elapsed();
        measurement.effect_inference_invocations = 1;
        measurement.effect_roots = effect_roots;
    });

    let validates_complete = program.main.is_some()
        && holes.is_empty()
        && unresolved_value_references.is_empty()
        && new_holes.is_empty();
    #[cfg(test)]
    let validation_started = std::time::Instant::now();
    if validates_complete {
        let complete = program
            .try_complete(&base.source_origins)
            .map_err(WorkspaceError::from_core)?;
        crate::ownership::check(&complete).map_err(WorkspaceError::from_core)?;
        crate::analyze::verify_match_plans(&complete).map_err(WorkspaceError::from_core)?;
        super::validate::program(&complete).map_err(WorkspaceError::from_core)?;
    }
    #[cfg(test)]
    record_transaction_measurement(|measurement| {
        measurement.complete_validation = validation_started.elapsed();
        measurement.complete_hir_derivations = usize::from(validates_complete);
    });

    #[cfg(test)]
    let index_started = std::time::Instant::now();
    let canonical =
        super::index::build(&program, base.namespace).map_err(WorkspaceError::from_core)?;
    #[cfg(test)]
    record_transaction_measurement(|measurement| {
        measurement.index_build = index_started.elapsed();
        measurement.index_build_invocations = 1;
        measurement.index_entities_built = canonical.entities.len();
        measurement.index_nodes_built = canonical.nodes.len();
        measurement.complete_hir_nodes = if validates_complete {
            canonical.nodes.len()
        } else {
            0
        };
    });
    let forced = force_surviving_nodes(
        base,
        &canonical,
        &forced_entities,
        &structural,
        movement.as_ref(),
    )?;
    #[cfg(test)]
    let reconciliation_started = std::time::Instant::now();
    #[cfg(test)]
    let reconciled_entities = canonical.entities.len();
    #[cfg(test)]
    let reconciled_nodes = canonical.nodes.len();
    let mut indexes = identity::reconcile(
        canonical,
        &base.indexes,
        allocator,
        &forced_entities,
        &forced,
    )
    .map_err(WorkspaceError::from_core)?;
    #[cfg(test)]
    record_transaction_measurement(|measurement| {
        measurement.identity_reconciliation = reconciliation_started.elapsed();
        measurement.identity_reconciliation_invocations = 1;
        measurement.identity_entity_records_examined =
            reconciled_entities + base.indexes.entities.len();
        measurement.identity_node_records_examined = reconciled_nodes + base.indexes.nodes.len();
    });
    #[cfg(test)]
    let finalization_started = std::time::Instant::now();

    refresh_hole_addresses(&mut holes, &program, &indexes)?;
    refresh_unresolved_value_reference_addresses(
        &mut unresolved_value_references,
        revision,
        &program,
        &indexes,
    )?;
    install_new_holes(&mut holes, &new_holes, &program, &indexes)?;
    for pending in &new_holes {
        let node = indexes
            .address_nodes
            .get(&pending.address)
            .copied()
            .ok_or_else(|| WorkspaceError::Validation(Arc::from("new hole identity is missing")))?;
        entries.push(SemanticDiffEntry::HoleIntroduced { hole: HoleId(node) });
    }
    let diagnostics = apply_incomplete_diagnostics(
        &mut indexes,
        &holes,
        &unresolved_value_references,
        program.main.is_none(),
    )?;
    for created in new_entities {
        let entity = indexes
            .address_entities
            .get(&created.address)
            .copied()
            .ok_or_else(|| {
                WorkspaceError::Validation(Arc::from("created entity identity is missing"))
            })?;
        entries.push(SemanticDiffEntry::EntityCreated {
            entity,
            kind: created.kind,
            name: created.name,
        });
    }
    append_structural_diff(base, &indexes, &structural, &mut entries)?;
    append_graph_diff(base, &indexes, &mut entries)?;
    let blockers = completeness_blockers(&program, &holes, &unresolved_value_references);
    let state = if blockers.is_empty() {
        ProgramState::Complete
    } else {
        ProgramState::Incomplete
    };
    let snapshot = WorkspaceSnapshot {
        namespace: base.namespace,
        revision,
        state,
        program: Arc::new(program),
        source_origins: Arc::clone(&base.source_origins),
        provenance: Arc::new(super::CapturedCompilationProvenance::Development),
        attachments: None,
        indexes: Arc::new(indexes),
        holes: holes.into(),
        unresolved_value_references: unresolved_value_references.into(),
        diagnostics: diagnostics.into(),
        blockers: blockers.into(),
        allocator: allocator.clone(),
    };
    append_call_instantiation_diff(base, &snapshot, &mut entries)?;
    coalesce_hole_refinement_entries(&mut entries)?;
    sort_diff_entries(&mut entries);
    let diff = SemanticDiff {
        base_revision: base.revision,
        revision,
        entries,
    };
    let invalidated = invalidated_domains();
    #[cfg(test)]
    record_transaction_measurement(|measurement| {
        measurement.finalization = finalization_started.elapsed();
        measurement.stage_wall = stage_started.elapsed();
    });
    Ok((snapshot, diff, invalidated))
}

fn stage_hole_refinements(
    base: &WorkspaceSnapshot,
    revision: RevisionId,
    edits: Vec<Edit>,
    allocator: &IdentityAllocator,
) -> Result<(WorkspaceSnapshot, SemanticDiff, Vec<InvalidatedDomain>), WorkspaceError> {
    #[cfg(test)]
    let started = std::time::Instant::now();
    let mut holes = Vec::new();
    holes
        .try_reserve(base.holes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole staging allocation failed")))?;
    holes.extend(base.holes.iter().cloned());
    let mut unresolved_value_references = Vec::new();
    unresolved_value_references
        .try_reserve(base.unresolved_value_references.len())
        .map_err(|_| {
            WorkspaceError::Host(Arc::from(
                "unresolved value-reference staging allocation failed",
            ))
        })?;
    unresolved_value_references.extend(base.unresolved_value_references.iter().cloned());
    for reference in &mut unresolved_value_references {
        reference.state.revision = revision;
    }
    let mut entries = Vec::new();
    entries
        .try_reserve(edits.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("semantic diff allocation failed")))?;
    for edit in edits {
        let Edit::RefineHole {
            hole,
            expected_type,
            goal,
        } = edit
        else {
            return Err(WorkspaceError::Validation(Arc::from(
                "metadata-only transaction contains a semantic edit",
            )));
        };
        entries.push(refine_hole(
            base,
            &base.program,
            &mut holes,
            None,
            hole,
            expected_type,
            goal,
        )?);
    }
    let diagnostics = incomplete_diagnostics(
        &holes,
        &unresolved_value_references,
        base.program.main.is_none(),
    )?;
    let snapshot = WorkspaceSnapshot {
        namespace: base.namespace,
        revision,
        state: base.state,
        program: Arc::clone(&base.program),
        source_origins: Arc::clone(&base.source_origins),
        provenance: Arc::new(super::CapturedCompilationProvenance::Development),
        attachments: None,
        indexes: Arc::clone(&base.indexes),
        holes: holes.into(),
        unresolved_value_references: unresolved_value_references.into(),
        diagnostics: diagnostics.into(),
        blockers: Arc::clone(&base.blockers),
        allocator: allocator.clone(),
    };
    coalesce_hole_refinement_entries(&mut entries)?;
    sort_diff_entries(&mut entries);
    let diff = SemanticDiff {
        base_revision: base.revision,
        revision,
        entries,
    };
    #[cfg(test)]
    record_transaction_measurement(|measurement| {
        measurement.edit_staging = started.elapsed();
        measurement.stage_wall = started.elapsed();
        measurement.metadata_only_path_used = true;
    });
    Ok((snapshot, diff, invalidated_domains()))
}

fn refine_hole(
    base: &WorkspaceSnapshot,
    program: &SemanticProgram,
    holes: &mut [HoleRecord],
    deleted_roots: Option<&HashSet<EntityAddress>>,
    hole: HoleId,
    expected_type: Option<SemanticType>,
    goal: String,
) -> Result<SemanticDiffEntry, WorkspaceError> {
    if hole.0.namespace() != base.namespace {
        return Err(WorkspaceError::ForeignNamespace(Arc::from("hole")));
    }
    let record = holes
        .iter_mut()
        .find(|record| record.state.id == hole)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole")))?;
    if let Some(deleted_roots) = deleted_roots {
        reject_deleted_root_edit(deleted_roots, record.address.root)?;
    }
    if let Some(expected_type) = expected_type {
        let expected_type =
            resolve_semantic_type(base, program, expected_type, "hole expectation")?;
        if expected_type != record.expected_internal {
            return Err(WorkspaceError::TypeMismatch {
                expected: Box::new(record.state.expected_type.clone()),
                actual: Box::new(super::types::view(
                    &base.program,
                    &base.indexes,
                    &expected_type,
                    Some(record.state.owner),
                )?),
            });
        }
    }
    if goal.is_empty() {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "typed hole goal must not be empty",
        )));
    }
    let old_goal = Arc::clone(&record.state.goal);
    record.state.goal = Arc::from(goal);
    Ok(SemanticDiffEntry::HoleRefined {
        hole,
        old_goal,
        new_goal: Arc::clone(&record.state.goal),
    })
}

fn invalidated_domains() -> Vec<InvalidatedDomain> {
    vec![
        InvalidatedDomain::SemanticIndexes,
        InvalidatedDomain::Types,
        InvalidatedDomain::Effects,
        InvalidatedDomain::Ownership,
        InvalidatedDomain::Diagnostics,
        InvalidatedDomain::Executable,
        InvalidatedDomain::Provenance,
    ]
}

fn try_clone_program(program: &SemanticProgram) -> Result<SemanticProgram, WorkspaceError> {
    let mut functions = Vec::new();
    functions
        .try_reserve(program.functions.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("function staging allocation failed")))?;
    for function in &program.functions {
        functions.push(crate::hir::Function {
            binding: function.binding,
            origin: function.origin,
            params: try_clone_values(&function.params, "function parameter")?,
            param_places: try_clone_values(&function.param_places, "function place")?,
            bounds: try_clone_values(&function.bounds, "function bound")?,
            arity: function.arity,
            local_count: function.local_count,
            summary: function.summary,
            body: function
                .body
                .try_clone()
                .map_err(WorkspaceError::from_core)?,
        });
    }
    let main = program
        .main
        .as_ref()
        .map(|main| {
            Ok(crate::hir::Main {
                origin: main.origin,
                params: try_clone_values(&main.params, "main parameter")?,
                param_places: try_clone_values(&main.param_places, "main place")?,
                param_types: try_clone_values(&main.param_types, "main parameter type")?,
                return_type: main.return_type.clone(),
                arity: main.arity,
                local_count: main.local_count,
                body: main.body.try_clone().map_err(WorkspaceError::from_core)?,
            })
        })
        .transpose()?;
    Ok(SemanticProgram {
        bindings: try_clone_values(&program.bindings, "binding")?,
        products: try_clone_values(&program.products, "product")?,
        enums: try_clone_values(&program.enums, "enum")?,
        traits: try_clone_values(&program.traits, "trait")?,
        implementations: try_clone_values(&program.implementations, "implementation")?,
        match_plans: try_clone_values(&program.match_plans, "match plan")?,
        functions,
        main,
        global_layout: try_clone_values(&program.global_layout, "global layout")?,
    })
}

fn try_clone_values<T: Clone>(values: &[T], kind: &str) -> Result<Vec<T>, WorkspaceError> {
    let mut cloned = Vec::new();
    cloned.try_reserve(values.len()).map_err(|_| {
        WorkspaceError::Host(Arc::from(format!("{kind} staging allocation failed")))
    })?;
    cloned.extend(values.iter().cloned());
    Ok(cloned)
}

fn preflight_deletions(
    base: &WorkspaceSnapshot,
    program: &SemanticProgram,
    edits: &[Edit],
) -> Result<DeletionPlan, WorkspaceError> {
    let mut requested = Vec::new();
    requested
        .try_reserve(edits.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("deletion intent allocation failed")))?;
    let mut seen = HashSet::new();
    seen.try_reserve(edits.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("deletion intent set allocation failed")))?;
    for edit in edits {
        let Edit::DeleteEntity { entity } = edit else {
            continue;
        };
        let header = base.workspace_entity(*entity)?;
        if !seen.insert(*entity) {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                "an entity is deleted more than once in one transaction",
            )));
        }
        requested.push(RequestedDeletion {
            entity: *entity,
            address: entity_address(base, *entity)?,
            kind: header.kind,
            name: Arc::clone(&header.name),
        });
    }

    let mut requested_addresses = HashSet::new();
    requested_addresses
        .try_reserve(requested.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("deletion address allocation failed")))?;
    requested_addresses.extend(requested.iter().map(|item| item.address));

    let mut callable_roots = HashSet::new();
    let mut callable_bindings = HashSet::new();
    let mut products = HashSet::new();
    let mut enum_vectors = HashSet::new();
    let mut product_requests = HashMap::new();
    let mut enum_requests = HashMap::new();
    let mut binding_requests = HashMap::new();
    callable_roots
        .try_reserve(requested.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("callable root deletion allocation failed")))?;
    callable_bindings
        .try_reserve(requested.len())
        .map_err(|_| {
            WorkspaceError::Host(Arc::from("callable binding deletion allocation failed"))
        })?;
    products
        .try_reserve(requested.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("product deletion allocation failed")))?;
    enum_vectors
        .try_reserve(requested.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("enum deletion allocation failed")))?;
    product_requests
        .try_reserve(requested.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("product request allocation failed")))?;
    enum_requests
        .try_reserve(requested.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("enum request allocation failed")))?;
    binding_requests
        .try_reserve(requested.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("binding request allocation failed")))?;
    for item in &requested {
        match item.kind {
            EntityKind::Main if item.address == EntityAddress::Main && program.main.is_some() => {
                callable_roots.insert(EntityAddress::Main);
            }
            EntityKind::Function => {
                let EntityAddress::Binding(raw) = item.address else {
                    return Err(WorkspaceError::StaleIdentity(Arc::from("function")));
                };
                let binding = program
                    .bindings
                    .get(host_index(raw, "function")?)
                    .filter(|binding| {
                        binding.id.raw() == raw
                            && binding.kind == BindingKind::Function
                            && binding.origin != Origin::Builtin
                    })
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function")))?;
                callable_roots.insert(item.address);
                callable_bindings.insert(binding.id);
                binding_requests.insert(binding.id, item.entity);
            }
            EntityKind::Product => {
                let EntityAddress::Product(raw) = item.address else {
                    return Err(WorkspaceError::StaleIdentity(Arc::from("product")));
                };
                let definition = program
                    .products
                    .get(host_index(raw, "product")?)
                    .filter(|definition| {
                        definition.id.raw() == raw && definition.origin != Origin::Builtin
                    })
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product")))?;
                products.insert(definition.id);
                product_requests.insert(definition.id, item.entity);
            }
            EntityKind::Enum => {
                let EntityAddress::Enum(raw) = item.address else {
                    return Err(WorkspaceError::StaleIdentity(Arc::from("enum")));
                };
                let definition = program
                    .enums
                    .get(host_index(raw, "enum")?)
                    .filter(|definition| definition.origin != Origin::Builtin)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum")))?;
                enum_vectors.insert(raw);
                enum_requests.insert(definition.id, item.entity);
            }
            EntityKind::ProductField => {
                let EntityAddress::ProductField { product, .. } = item.address else {
                    return Err(WorkspaceError::StaleIdentity(Arc::from("product field")));
                };
                if requested_addresses.contains(&EntityAddress::Product(product)) {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "deleting a product and one of its owned fields is redundant",
                    )));
                }
                return Err(WorkspaceError::unsupported(
                    "delete-entity",
                    "product fields cannot be deleted independently of their product",
                ));
            }
            EntityKind::EnumVariant => {
                let EntityAddress::EnumVariant { enumeration, .. } = item.address else {
                    return Err(WorkspaceError::StaleIdentity(Arc::from("enum variant")));
                };
                if requested_addresses.contains(&EntityAddress::Enum(enumeration)) {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "deleting an enum and one of its owned variants is redundant",
                    )));
                }
                return Err(WorkspaceError::unsupported(
                    "delete-entity",
                    "enum variants cannot be deleted independently of their enum",
                ));
            }
            EntityKind::EnumField => {
                let EntityAddress::EnumField { enumeration, .. } = item.address else {
                    return Err(WorkspaceError::StaleIdentity(Arc::from("enum field")));
                };
                if requested_addresses.contains(&EntityAddress::Enum(enumeration)) {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "deleting an enum and one of its owned fields is redundant",
                    )));
                }
                return Err(WorkspaceError::unsupported(
                    "delete-entity",
                    "enum fields cannot be deleted independently of their enum",
                ));
            }
            EntityKind::BuiltinOperation => {
                return Err(WorkspaceError::unsupported(
                    "delete-entity",
                    "fixed compiler operations cannot be deleted",
                ));
            }
            EntityKind::Main => {
                return Err(WorkspaceError::StaleIdentity(Arc::from("main")));
            }
            _ => {
                return Err(WorkspaceError::unsupported(
                    "delete-entity",
                    "only main, ordinary functions, products, and enums can be deleted directly",
                ));
            }
        }
    }

    if callable_roots.contains(&EntityAddress::Main)
        && edits
            .iter()
            .any(|edit| matches!(edit, Edit::CreateMain { .. }))
    {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "main cannot be deleted and created in one transaction",
        )));
    }
    let mut deleted_nominal_names = HashSet::new();
    deleted_nominal_names
        .try_reserve(product_requests.len().saturating_add(enum_requests.len()))
        .map_err(|_| WorkspaceError::Host(Arc::from("deleted nominal name allocation failed")))?;
    for item in &requested {
        if matches!(item.kind, EntityKind::Product | EntityKind::Enum) {
            deleted_nominal_names.insert(item.name.as_ref());
        }
    }
    if edits.iter().any(|edit| match edit {
        Edit::CreateProduct { name, .. } | Edit::CreateEnum { name, .. } => {
            deleted_nominal_names.contains(name.as_str())
        }
        _ => false,
    }) {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "a nominal declaration cannot be deleted and recreated with the same name in one transaction",
        )));
    }

    let mut entities = HashSet::new();
    entities
        .try_reserve(base.indexes.entities.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("deletion closure allocation failed")))?;
    let mut requested_entities = HashSet::new();
    requested_entities
        .try_reserve(requested.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("requested deletion allocation failed")))?;
    requested_entities.extend(requested.iter().map(|item| item.entity));
    for header in &base.indexes.entities {
        let mut current = Some(header.id);
        while let Some(entity) = current {
            if requested_entities.contains(&entity) {
                entities.insert(header.id);
                break;
            }
            current = base
                .indexes
                .entity_lookup
                .get(&entity)
                .and_then(|index| base.indexes.entities.get(*index))
                .and_then(|item| item.owner);
        }
    }

    let mut implementations = HashSet::new();
    let mut implementation_requests = HashMap::new();
    implementations
        .try_reserve(program.implementations.len())
        .map_err(|_| {
            WorkspaceError::Host(Arc::from("implementation deletion allocation failed"))
        })?;
    implementation_requests
        .try_reserve(program.implementations.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("implementation request allocation failed")))?;
    for (index, implementation) in program.implementations.iter().enumerate() {
        let Some(request) = product_requests.get(&implementation.product).copied() else {
            continue;
        };
        let expected = u64::try_from(index)
            .map_err(|_| WorkspaceError::Host(Arc::from("implementation index exceeds u64")))?;
        if implementation.id.raw() != expected {
            return Err(WorkspaceError::Validation(Arc::from(
                "implementation dense identity is stale before deletion",
            )));
        }
        implementations.insert(implementation.id);
        implementation_requests.insert(implementation.id, request);
        let address = EntityAddress::Implementation(expected);
        let entity = base
            .indexes
            .address_entities
            .get(&address)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("implementation")))?;
        entities.insert(entity);
    }

    Ok(DeletionPlan {
        requested,
        entities,
        callable_roots,
        callable_bindings,
        products,
        enum_vectors,
        implementations,
        product_requests,
        enum_requests,
        implementation_requests,
        binding_requests,
    })
}

fn reject_deleted_root_edit(
    deleted_roots: &HashSet<EntityAddress>,
    root: EntityAddress,
) -> Result<(), WorkspaceError> {
    if deleted_roots.contains(&root) {
        Err(WorkspaceError::InvalidTransaction(Arc::from(
            "an incomplete node or expression owned by a deleted declaration cannot be edited in the same transaction",
        )))
    } else {
        Ok(())
    }
}

fn reject_external_incompleteness_for_movement(
    program: &SemanticProgram,
    holes: &[HoleRecord],
    unresolved_value_references: &[UnresolvedValueReferenceRecord],
    new_holes: &[NewHole],
    movement: Option<&SequenceMovement>,
) -> Result<(), WorkspaceError> {
    let Some(movement) = movement else {
        return Ok(());
    };
    if program.main.is_none() {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "sequence movement cannot defer canonical validation while the entry point is absent",
        )));
    }
    let has_external_incomplete = holes
        .iter()
        .map(|hole| hole.address.root)
        .chain(
            unresolved_value_references
                .iter()
                .map(|reference| reference.address.root),
        )
        .chain(new_holes.iter().map(|hole| hole.address.root))
        .any(|root| root != movement.address.root);
    if has_external_incomplete {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "sequence movement cannot defer canonical validation because another callable is incomplete",
        )));
    }
    Ok(())
}

fn preflight_sequence_movement(
    base: &WorkspaceSnapshot,
    edits: &[Edit],
) -> Result<Option<SequenceMovement>, WorkspaceError> {
    let mut requested = None;
    for edit in edits {
        let Edit::MoveSequenceChild {
            sequence,
            child,
            before,
        } = edit
        else {
            continue;
        };
        if requested.replace((*sequence, *child, *before)).is_some() {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                "a transaction may contain only one sequence movement",
            )));
        }
    }
    let Some((sequence, child, before)) = requested else {
        return Ok(None);
    };

    let sequence_header = base.workspace_node(sequence)?;
    if sequence_header.kind != NodeKind::Sequence {
        return Err(WorkspaceError::WrongEntityKind {
            operation: Arc::from("move-sequence-child"),
            expected: Arc::from("sequence node"),
            actual: SemanticKind::Node(sequence_header.kind),
        });
    }
    let child_header = base.workspace_node(child)?;
    if child_header.owner != SemanticOwner::Node(sequence) {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "moved node is not a direct child of the selected sequence",
        )));
    }
    if let Some(anchor) = before {
        let anchor_header = base.workspace_node(anchor)?;
        if anchor_header.owner != SemanticOwner::Node(sequence) {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                "movement anchor is not a direct child of the selected sequence",
            )));
        }
        if anchor == child {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                "a sequence child cannot be moved before itself",
            )));
        }
    }

    let indexed_children = base
        .indexes
        .node_children
        .get(&sequence)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("sequence children")))?;
    let mut old_children = Vec::new();
    old_children
        .try_reserve(indexed_children.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("sequence movement allocation failed")))?;
    old_children.extend(indexed_children.iter().copied());
    let old_index = old_children
        .iter()
        .position(|candidate| *candidate == child)
        .ok_or_else(|| {
            WorkspaceError::InvalidTransaction(Arc::from(
                "moved node is not a direct child of the selected sequence",
            ))
        })?;
    let anchor_index = before
        .map(|anchor| {
            old_children
                .iter()
                .position(|candidate| *candidate == anchor)
                .ok_or_else(|| {
                    WorkspaceError::InvalidTransaction(Arc::from(
                        "movement anchor is not a direct child of the selected sequence",
                    ))
                })
        })
        .transpose()?;

    let mut final_old_ordinals = Vec::new();
    final_old_ordinals
        .try_reserve(old_children.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("sequence order allocation failed")))?;
    final_old_ordinals.extend(0..old_children.len());
    final_old_ordinals.remove(old_index);
    let new_index = if let Some(anchor_index) = anchor_index {
        final_old_ordinals
            .iter()
            .position(|ordinal| *ordinal == anchor_index)
            .ok_or_else(|| {
                WorkspaceError::Validation(Arc::from("sequence movement anchor was removed"))
            })?
    } else {
        final_old_ordinals.len()
    };
    final_old_ordinals.insert(new_index, old_index);
    if final_old_ordinals.iter().copied().eq(0..old_children.len()) {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "sequence movement would not change semantic order",
        )));
    }
    let old_predecessor = old_index
        .checked_sub(1)
        .and_then(|index| old_children.get(index))
        .copied();
    let old_successor_index = old_index
        .checked_add(1)
        .ok_or_else(|| WorkspaceError::Host(Arc::from("sequence neighbor index overflow")))?;
    let old_successor = old_children.get(old_successor_index).copied();
    let new_predecessor = new_index
        .checked_sub(1)
        .and_then(|index| final_old_ordinals.get(index))
        .and_then(|old_ordinal| old_children.get(*old_ordinal))
        .copied();
    let new_successor_index = new_index
        .checked_add(1)
        .ok_or_else(|| WorkspaceError::Host(Arc::from("sequence neighbor index overflow")))?;
    let new_successor = final_old_ordinals
        .get(new_successor_index)
        .and_then(|old_ordinal| old_children.get(*old_ordinal))
        .copied();

    let sequence_index = base
        .indexes
        .node_lookup
        .get(&sequence)
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("sequence")))?;
    let address = base.indexes.node_addresses[sequence_index];
    let (ancestor_nodes, path) = movement_ancestry(base, sequence)?;
    let root = expression_root(&base.program, address.root)?;
    let mut current = root;
    let mut ancestors = Vec::new();
    ancestors
        .try_reserve(path.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("sequence ancestry allocation failed")))?;
    for (ordinal, expected_node) in path.iter().copied().zip(&ancestor_nodes) {
        let child_expression = current
            .try_child(ordinal)
            .map_err(WorkspaceError::from_core)?;
        ancestors.push((current, ordinal, *expected_node));
        current = child_expression;
    }
    let ExprKind::Do(values) = &current.kind else {
        return Err(WorkspaceError::Validation(Arc::from(
            "selected sequence identity does not resolve to sequence semantics",
        )));
    };
    if values.len() != old_children.len() {
        return Err(WorkspaceError::Validation(Arc::from(
            "selected sequence child index is inconsistent",
        )));
    }
    for (new_position, old_position) in final_old_ordinals.iter().copied().enumerate() {
        let value = values.get(old_position).ok_or_else(|| {
            WorkspaceError::Validation(Arc::from("sequence movement order is stale"))
        })?;
        if value.ty == Type::Never && new_position.checked_add(1) != Some(values.len()) {
            return Err(WorkspaceError::Validation(Arc::from(
                "sequence movement leaves an expression after a divergent expression",
            )));
        }
    }
    let final_old_ordinal = *final_old_ordinals
        .last()
        .ok_or_else(|| WorkspaceError::Validation(Arc::from("sequence movement has no child")))?;
    let sequence_type = values
        .get(final_old_ordinal)
        .ok_or_else(|| WorkspaceError::Validation(Arc::from("sequence result is stale")))?
        .ty
        .clone();

    let mut propagated_node = sequence;
    let mut propagated = sequence_type.clone();
    let mut reversed_ancestor_types = Vec::new();
    reversed_ancestor_types
        .try_reserve(ancestors.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("sequence type allocation failed")))?;
    for (ancestor, ordinal, node) in ancestors.into_iter().rev() {
        if !movement_child_result_is_derived(ancestor, ordinal) {
            require_movement_expected_type(base, propagated_node, &propagated)?;
        }
        propagated = ancestor
            .try_reconstructed_type_with_child(ordinal, &propagated)
            .map_err(WorkspaceError::from_core)?;
        propagated_node = node;
        reversed_ancestor_types.push(propagated.clone());
    }
    require_movement_expected_type(base, propagated_node, &propagated)?;
    reversed_ancestor_types.reverse();

    Ok(Some(SequenceMovement {
        sequence,
        child,
        address,
        path,
        old_children,
        final_old_ordinals,
        old_index,
        new_index,
        old_predecessor,
        old_successor,
        new_predecessor,
        new_successor,
        sequence_type,
        ancestor_types: reversed_ancestor_types,
    }))
}

fn movement_ancestry(
    base: &WorkspaceSnapshot,
    sequence: NodeId,
) -> Result<(Vec<NodeId>, Vec<usize>), WorkspaceError> {
    let mut reverse_ancestors = Vec::new();
    let mut reverse_path = Vec::new();
    let mut current = sequence;
    loop {
        let index = base
            .indexes
            .node_lookup
            .get(&current)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("sequence ancestry")))?;
        match base.indexes.nodes[index].owner {
            SemanticOwner::Entity(_) => break,
            SemanticOwner::Node(parent) => {
                reverse_ancestors.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("sequence ancestry allocation failed"))
                })?;
                reverse_path.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("sequence path allocation failed"))
                })?;
                reverse_ancestors.push(parent);
                reverse_path.push(
                    usize::try_from(base.indexes.node_keys[index].ordinal).map_err(|_| {
                        WorkspaceError::Host(Arc::from(
                            "sequence child ordinal is not host-addressable",
                        ))
                    })?,
                );
                current = parent;
            }
        }
    }
    reverse_ancestors.reverse();
    reverse_path.reverse();
    Ok((reverse_ancestors, reverse_path))
}

fn movement_child_result_is_derived(parent: &Expr, ordinal: usize) -> bool {
    match &parent.kind {
        ExprKind::Do(values) => ordinal.checked_add(1) == Some(values.len()),
        ExprKind::If { .. } => matches!(ordinal, 1 | 2),
        ExprKind::Let { bindings, .. } => ordinal == bindings.len(),
        ExprKind::MutableLocal { .. } => ordinal == 1,
        ExprKind::Match { .. } => ordinal > 0,
        _ => false,
    }
}

fn require_movement_expected_type(
    base: &WorkspaceSnapshot,
    node: NodeId,
    actual: &Type,
) -> Result<(), WorkspaceError> {
    let index = base
        .indexes
        .node_lookup
        .get(&node)
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("movement type context")))?;
    let Some(expected) = base.indexes.node_expected_types[index].as_ref() else {
        return Ok(());
    };
    if Type::join_control(actual, expected) == Some(expected.clone()) {
        return Ok(());
    }
    let owner = base.indexes.node_enclosing_entities[index];
    Err(WorkspaceError::TypeMismatch {
        expected: Box::new(super::types::view(
            &base.program,
            &base.indexes,
            expected,
            Some(owner),
        )?),
        actual: Box::new(super::types::view(
            &base.program,
            &base.indexes,
            actual,
            Some(owner),
        )?),
    })
}

fn preflight_structural_edits(
    base: &WorkspaceSnapshot,
    edits: &[Edit],
    movement: Option<&SequenceMovement>,
) -> Result<(), WorkspaceError> {
    let mut targets = Vec::new();
    targets
        .try_reserve(edits.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("structural preflight allocation failed")))?;
    for edit in edits {
        let target = match edit {
            Edit::ReplaceExpression { target, .. }
            | Edit::IntroduceHole { target, .. }
            | Edit::IntroduceUnresolvedValueReference { target, .. } => Some(*target),
            Edit::FillHole { hole, .. } => Some(hole.0),
            Edit::ResolveUnresolvedValueReference { reference, .. } => Some(reference.0),
            _ => None,
        };
        if let Some(target) = target {
            ensure_structural_nonoverlapping(base, &mut targets, target)?;
            if let Some(movement) = movement {
                let index = base
                    .indexes
                    .node_lookup
                    .get(&target)
                    .copied()
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node")))?;
                if base.indexes.node_addresses[index].root == movement.address.root {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "sequence movement cannot be combined with a structural edit in the same callable",
                    )));
                }
            }
        }
    }
    for edit in edits {
        let Edit::RefineHole { hole, .. } = edit else {
            continue;
        };
        if hole.0.namespace() != base.namespace {
            return Err(WorkspaceError::ForeignNamespace(Arc::from("hole")));
        }
        let record = base
            .holes
            .iter()
            .find(|record| record.state.id == *hole)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole")))?;
        if movement.is_some_and(|movement| movement.address.root == record.address.root) {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                "sequence movement cannot refine a hole in the same callable transaction",
            )));
        }
        for target in &targets {
            if *target == hole.0 || node_is_ancestor(base, *target, hole.0)? {
                return Err(WorkspaceError::InvalidTransaction(Arc::from(
                    "a hole cannot be refined and structurally removed in one transaction",
                )));
            }
        }
    }
    Ok(())
}

fn prune_replaced_incomplete_subtrees(
    base: &WorkspaceSnapshot,
    holes: &mut Vec<HoleRecord>,
    unresolved_value_references: &mut Vec<UnresolvedValueReferenceRecord>,
    edits: &[Edit],
) -> Result<(), WorkspaceError> {
    let mut roots = HashSet::new();
    roots
        .try_reserve(edits.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole-pruning root allocation failed")))?;
    for edit in edits {
        if let Edit::ReplaceExpression { target, .. }
        | Edit::IntroduceHole { target, .. }
        | Edit::IntroduceUnresolvedValueReference { target, .. } = edit
        {
            roots.insert(*target);
        }
    }
    if roots.is_empty() || (holes.is_empty() && unresolved_value_references.is_empty()) {
        return Ok(());
    }
    let mut removed = HashSet::new();
    removed
        .try_reserve(base.indexes.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole-pruning index allocation failed")))?;
    for node in &base.indexes.nodes {
        let is_removed = roots.contains(&node.id)
            || matches!(node.owner, SemanticOwner::Node(parent) if removed.contains(&parent));
        if is_removed {
            removed.insert(node.id);
        }
    }
    holes.retain(|hole| !removed.contains(&hole.state.id.0));
    unresolved_value_references.retain(|reference| !removed.contains(&reference.state.id.0));
    Ok(())
}

#[derive(Clone)]
struct DependencyOwner {
    entity: Option<EntityId>,
    kind: EntityKind,
    name: Arc<str>,
}

struct DependencyBlocker {
    requested: EntityId,
    owner: DependencyOwner,
    category: &'static str,
}

fn reject_surviving_deleted_dependencies(
    base: &WorkspaceSnapshot,
    program: &SemanticProgram,
    deletions: &DeletionPlan,
) -> Result<(), WorkspaceError> {
    if deletions.requested.is_empty() {
        return Ok(());
    }
    let mut blockers = Vec::new();

    if let Some(main) = &program.main {
        let address = EntityAddress::Main;
        let owner = dependency_owner(base, address, EntityKind::Main, "main");
        if !dependency_owner_is_deleted(deletions, &owner) {
            for ty in main
                .param_types
                .iter()
                .chain(std::iter::once(&main.return_type))
            {
                collect_type_deletion_blockers(
                    ty,
                    &owner,
                    "callable signature type",
                    deletions,
                    &mut blockers,
                )?;
            }
            collect_expression_deletion_blockers(
                program,
                &main.body,
                &owner,
                deletions,
                &mut blockers,
            )?;
        }
    }

    for function in &program.functions {
        let address = EntityAddress::Binding(function.binding.raw());
        let name = program
            .binding(function.binding)
            .map(|binding| binding.name.as_str())
            .unwrap_or("<stale-function>");
        let owner = dependency_owner(base, address, EntityKind::Function, name);
        if dependency_owner_is_deleted(deletions, &owner) {
            continue;
        }
        let signature = program
            .binding(function.binding)
            .ok_or_else(|| WorkspaceError::Validation(Arc::from("function binding is stale")))?;
        collect_type_deletion_blockers(
            &signature.ty,
            &owner,
            "callable signature type",
            deletions,
            &mut blockers,
        )?;
        collect_expression_deletion_blockers(
            program,
            &function.body,
            &owner,
            deletions,
            &mut blockers,
        )?;
    }

    for (index, product) in program.products.iter().enumerate() {
        if deletions.products.contains(&product.id) {
            continue;
        }
        let raw = u64::try_from(index)
            .map_err(|_| WorkspaceError::Host(Arc::from("product index exceeds u64")))?;
        let owner = dependency_owner(
            base,
            EntityAddress::Product(raw),
            EntityKind::Product,
            &product.name,
        );
        for field in &product.fields {
            collect_type_deletion_blockers(
                &field.ty,
                &owner,
                "product field type",
                deletions,
                &mut blockers,
            )?;
        }
    }

    for (index, definition) in program.enums.iter().enumerate() {
        let raw = u64::try_from(index)
            .map_err(|_| WorkspaceError::Host(Arc::from("enum index exceeds u64")))?;
        if deletions.enum_vectors.contains(&raw) {
            continue;
        }
        let owner = dependency_owner(
            base,
            EntityAddress::Enum(raw),
            EntityKind::Enum,
            &definition.name,
        );
        for field in definition
            .variants
            .iter()
            .flat_map(|variant| &variant.fields)
        {
            collect_type_deletion_blockers(
                &field.ty,
                &owner,
                "enum field type",
                deletions,
                &mut blockers,
            )?;
        }
    }

    blockers.sort_by(|left, right| {
        left.requested
            .cmp(&right.requested)
            .then_with(|| match (left.owner.entity, right.owner.entity) {
                (Some(left), Some(right)) => left.cmp(&right),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| left.owner.kind.cmp(&right.owner.kind))
            .then_with(|| left.owner.name.cmp(&right.owner.name))
            .then_with(|| left.category.cmp(right.category))
    });
    blockers.dedup_by(|left, right| {
        left.requested == right.requested
            && left.owner.entity == right.owner.entity
            && left.owner.kind == right.owner.kind
            && left.owner.name == right.owner.name
            && left.category == right.category
    });
    let Some(blocker) = blockers.first() else {
        return Ok(());
    };
    let requested = base.workspace_entity(blocker.requested)?;
    let dependent_identity = blocker.owner.entity.map_or_else(
        || "new entity without a published identity".to_owned(),
        |entity| entity_diagnostic_label(&entity),
    );
    Err(WorkspaceError::InvalidTransaction(Arc::from(format!(
        "cannot delete {} '{}' ({}) while surviving {} '{}' ({dependent_identity}) retains a {} dependency",
        entity_kind_name(requested.kind),
        requested.name,
        entity_diagnostic_label(&requested.id),
        entity_kind_name(blocker.owner.kind),
        blocker.owner.name,
        blocker.category,
    ))))
}

fn dependency_owner(
    base: &WorkspaceSnapshot,
    address: EntityAddress,
    kind: EntityKind,
    name: &str,
) -> DependencyOwner {
    let entity = base.indexes.address_entities.get(&address).copied();
    let (kind, name) = entity
        .and_then(|entity| base.indexes.entity_lookup.get(&entity).copied())
        .and_then(|index| base.indexes.entities.get(index))
        .map_or_else(
            || (kind, Arc::from(name)),
            |header| (header.kind, Arc::clone(&header.name)),
        );
    DependencyOwner { entity, kind, name }
}

fn dependency_owner_is_deleted(deletions: &DeletionPlan, owner: &DependencyOwner) -> bool {
    owner
        .entity
        .is_some_and(|entity| deletions.entities.contains(&entity))
}

fn collect_type_deletion_blockers(
    root: &Type,
    owner: &DependencyOwner,
    category: &'static str,
    deletions: &DeletionPlan,
    blockers: &mut Vec<DependencyBlocker>,
) -> Result<(), WorkspaceError> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("type dependency work allocation failed")))?;
    pending.push(root);
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Product(id) => {
                if let Some(requested) = deletions.product_requests.get(id) {
                    push_deletion_blocker(blockers, *requested, owner, category)?;
                }
            }
            Type::Enum { id, arguments, .. } => {
                if let Some(requested) = deletions.enum_requests.get(id) {
                    push_deletion_blocker(blockers, *requested, owner, category)?;
                }
                pending.try_reserve(arguments.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("type dependency work allocation failed"))
                })?;
                pending.extend(arguments);
            }
            Type::List(inner) => {
                pending.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("type dependency work allocation failed"))
                })?;
                pending.push(inner);
            }
            Type::Fn { params, ret } => {
                let additional = params.len().checked_add(1).ok_or_else(|| {
                    WorkspaceError::Host(Arc::from("type dependency child count overflow"))
                })?;
                pending.try_reserve(additional).map_err(|_| {
                    WorkspaceError::Host(Arc::from("type dependency work allocation failed"))
                })?;
                pending.push(ret);
                pending.extend(params);
            }
            Type::Forall { body, .. } => {
                pending.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("type dependency work allocation failed"))
                })?;
                pending.push(body);
            }
            _ => {}
        }
    }
    Ok(())
}

fn collect_expression_deletion_blockers(
    program: &SemanticProgram,
    root: &Expr,
    owner: &DependencyOwner,
    deletions: &DeletionPlan,
    blockers: &mut Vec<DependencyBlocker>,
) -> Result<(), WorkspaceError> {
    let mut pending = Vec::new();
    pending.try_reserve(1).map_err(|_| {
        WorkspaceError::Host(Arc::from("expression dependency work allocation failed"))
    })?;
    pending.push(root);
    let mut match_plans = HashSet::new();
    while let Some(expression) = pending.pop() {
        collect_type_deletion_blockers(
            &expression.ty,
            owner,
            "expression type",
            deletions,
            blockers,
        )?;
        match &expression.kind {
            ExprKind::Load(reference)
            | ExprKind::Move {
                binding: reference, ..
            }
            | ExprKind::Borrow {
                binding: reference, ..
            }
            | ExprKind::BorrowBytes {
                binding: reference, ..
            } => collect_binding_deletion_blocker(
                reference.binding,
                owner,
                "callable body reference",
                deletions,
                blockers,
            )?,
            ExprKind::Call {
                callee,
                instantiation,
                ..
            } => {
                collect_binding_deletion_blocker(
                    callee.binding,
                    owner,
                    "callable body reference",
                    deletions,
                    blockers,
                )?;
                if let Some(instantiation) = instantiation {
                    for substitution in &instantiation.substitutions {
                        collect_type_deletion_blockers(
                            &substitution.ty,
                            owner,
                            "generic substitution type",
                            deletions,
                            blockers,
                        )?;
                    }
                    for witness in &instantiation.witnesses {
                        collect_type_deletion_blockers(
                            &witness.ty,
                            owner,
                            "trait witness type",
                            deletions,
                            blockers,
                        )?;
                        if let crate::hir::TraitWitnessKind::Explicit(implementation) = witness.kind
                        {
                            if let Some(requested) =
                                deletions.implementation_requests.get(&implementation)
                            {
                                push_deletion_blocker(
                                    blockers,
                                    *requested,
                                    owner,
                                    "explicit implementation witness",
                                )?;
                            }
                        }
                    }
                }
            }
            ExprKind::Operation {
                resolved_signature, ..
            } => collect_type_deletion_blockers(
                resolved_signature,
                owner,
                "resolved operation signature",
                deletions,
                blockers,
            )?,
            ExprKind::SetLocal { target, .. } => collect_binding_deletion_blocker(
                *target,
                owner,
                "callable body reference",
                deletions,
                blockers,
            )?,
            ExprKind::ProductValue { product, .. }
            | ExprKind::ProductField { product, .. }
            | ExprKind::WithProductField { product, .. } => {
                if let Some(requested) = deletions.product_requests.get(product) {
                    push_deletion_blocker(blockers, *requested, owner, "product expression")?;
                }
            }
            ExprKind::EnumValue { enum_id, .. }
            | ExprKind::EnumIsVariant { enum_id, .. }
            | ExprKind::EnumField { enum_id, .. }
            | ExprKind::EnumUnwrap { enum_id, .. } => {
                if let Some(requested) = deletions.enum_requests.get(enum_id) {
                    push_deletion_blocker(blockers, *requested, owner, "enum expression")?;
                }
            }
            ExprKind::Match { plan, .. } | ExprKind::MatchUnreachable { plan }
                if !match_plans.contains(plan) =>
            {
                match_plans.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("match dependency set allocation failed"))
                })?;
                match_plans.insert(*plan);
                collect_match_plan_deletion_blockers(program, *plan, owner, deletions, blockers)?;
            }
            ExprKind::Match { .. } | ExprKind::MatchUnreachable { .. } => {}
            _ => {}
        }
        let mut allocation_failed = false;
        crate::hir::for_each_expression_child(expression, &mut |child| {
            if !allocation_failed && pending.try_reserve(1).is_err() {
                allocation_failed = true;
            }
            if !allocation_failed {
                pending.push(child);
            }
        });
        if allocation_failed {
            return Err(WorkspaceError::Host(Arc::from(
                "expression dependency work allocation failed",
            )));
        }
    }
    Ok(())
}

fn collect_match_plan_deletion_blockers(
    program: &SemanticProgram,
    id: crate::hir::MatchPlanId,
    owner: &DependencyOwner,
    deletions: &DeletionPlan,
    blockers: &mut Vec<DependencyBlocker>,
) -> Result<(), WorkspaceError> {
    let plan = id
        .index()
        .and_then(|index| program.match_plans.get(index))
        .filter(|plan| plan.id == id)
        .ok_or_else(|| WorkspaceError::Validation(Arc::from("match plan identity is stale")))?;
    collect_type_deletion_blockers(
        &plan.scrutinee.ty,
        owner,
        "match scrutinee type",
        deletions,
        blockers,
    )?;
    collect_type_deletion_blockers(
        &plan.result_type,
        owner,
        "match result type",
        deletions,
        blockers,
    )?;
    for arm in &plan.arms {
        collect_type_deletion_blockers(
            &arm.body_type,
            owner,
            "match arm type",
            deletions,
            blockers,
        )?;
        collect_pattern_deletion_blockers(&arm.pattern, owner, deletions, blockers)?;
    }
    for test in &plan.tests {
        if let crate::hir::MatchTestKind::Variant { enum_id, .. } = test.kind {
            if let Some(requested) = deletions.enum_requests.get(&enum_id) {
                push_deletion_blocker(blockers, *requested, owner, "match variant test")?;
            }
        }
    }
    for local in plan
        .projections
        .iter()
        .map(|item| &item.local)
        .chain(plan.bindings.iter().map(|item| &item.local))
    {
        collect_type_deletion_blockers(&local.ty, owner, "match local type", deletions, blockers)?;
    }
    Ok(())
}

fn collect_pattern_deletion_blockers(
    root: &crate::hir::MatchPattern,
    owner: &DependencyOwner,
    deletions: &DeletionPlan,
    blockers: &mut Vec<DependencyBlocker>,
) -> Result<(), WorkspaceError> {
    let mut pending = Vec::new();
    pending.try_reserve(1).map_err(|_| {
        WorkspaceError::Host(Arc::from("pattern dependency work allocation failed"))
    })?;
    pending.push(root);
    while let Some(pattern) = pending.pop() {
        let ty = pattern.ty();
        collect_type_deletion_blockers(&ty, owner, "match pattern type", deletions, blockers)?;
        match pattern {
            crate::hir::MatchPattern::Variant {
                enum_id, fields, ..
            } => {
                if let Some(requested) = deletions.enum_requests.get(enum_id) {
                    push_deletion_blocker(blockers, *requested, owner, "enum match pattern")?;
                }
                pending.try_reserve(fields.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("pattern dependency work allocation failed"))
                })?;
                for field in fields {
                    if let Some(local) = &field.projection {
                        collect_type_deletion_blockers(
                            &local.ty,
                            owner,
                            "match projection type",
                            deletions,
                            blockers,
                        )?;
                    }
                    pending.push(&field.pattern);
                }
            }
            crate::hir::MatchPattern::Product {
                product, fields, ..
            } => {
                if let Some(requested) = deletions.product_requests.get(product) {
                    push_deletion_blocker(blockers, *requested, owner, "product match pattern")?;
                }
                pending.try_reserve(fields.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("pattern dependency work allocation failed"))
                })?;
                for field in fields {
                    if let Some(local) = &field.projection {
                        collect_type_deletion_blockers(
                            &local.ty,
                            owner,
                            "match projection type",
                            deletions,
                            blockers,
                        )?;
                    }
                    pending.push(&field.pattern);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn collect_binding_deletion_blocker(
    binding: crate::hir::BindingId,
    owner: &DependencyOwner,
    category: &'static str,
    deletions: &DeletionPlan,
    blockers: &mut Vec<DependencyBlocker>,
) -> Result<(), WorkspaceError> {
    if let Some(requested) = deletions.binding_requests.get(&binding) {
        push_deletion_blocker(blockers, *requested, owner, category)?;
    }
    Ok(())
}

fn push_deletion_blocker(
    blockers: &mut Vec<DependencyBlocker>,
    requested: EntityId,
    owner: &DependencyOwner,
    category: &'static str,
) -> Result<(), WorkspaceError> {
    blockers
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("deletion blocker allocation failed")))?;
    blockers.push(DependencyBlocker {
        requested,
        owner: owner.clone(),
        category,
    });
    Ok(())
}

fn entity_diagnostic_label(entity: &EntityId) -> String {
    format!(
        "entity slot {} generation {}",
        entity.slot(),
        entity.generation()
    )
}

fn remap_forced_entity_addresses(
    compaction: &super::compaction::CompactionResult,
    forced: &mut HashMap<EntityAddress, EntityId>,
) -> Result<(), WorkspaceError> {
    let previous = std::mem::take(forced);
    forced
        .try_reserve(previous.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("forced relocation allocation failed")))?;
    for (address, entity) in previous {
        let relocated = remap_entity_address(compaction, address).ok_or_else(|| {
            WorkspaceError::Validation(Arc::from("new semantic entity was removed by compaction"))
        })?;
        if forced.insert(relocated, entity).is_some() {
            return Err(WorkspaceError::Validation(Arc::from(
                "new semantic entities collide after compaction",
            )));
        }
    }
    Ok(())
}

fn remap_staged_addresses(
    compaction: &super::compaction::CompactionResult,
    entities: &mut [NewEntity],
    holes: &mut [NewHole],
) -> Result<(), WorkspaceError> {
    for entity in entities {
        entity.address = remap_entity_address(compaction, entity.address).ok_or_else(|| {
            WorkspaceError::Validation(Arc::from("staged entity address was removed"))
        })?;
    }
    for hole in holes {
        hole.address.root =
            remap_entity_address(compaction, hole.address.root).ok_or_else(|| {
                WorkspaceError::Validation(Arc::from("staged hole owner was removed"))
            })?;
    }
    Ok(())
}

fn remap_entity_address(
    compaction: &super::compaction::CompactionResult,
    address: EntityAddress,
) -> Option<EntityAddress> {
    match address {
        EntityAddress::Main => Some(EntityAddress::Main),
        EntityAddress::Binding(raw) => compaction
            .bindings
            .get(&crate::hir::BindingId::new(raw))
            .map(|binding| EntityAddress::Binding(binding.raw())),
        EntityAddress::FunctionTypeParameter { function, ordinal } => compaction
            .bindings
            .get(&crate::hir::BindingId::new(function))
            .map(|binding| EntityAddress::FunctionTypeParameter {
                function: binding.raw(),
                ordinal,
            }),
        EntityAddress::Product(raw) => compaction
            .products
            .get(&lkjscript_core::ProductId::new(raw))
            .map(|product| EntityAddress::Product(product.raw())),
        EntityAddress::ProductField { product, field } => compaction
            .products
            .get(&lkjscript_core::ProductId::new(product))
            .map(|product| EntityAddress::ProductField {
                product: product.raw(),
                field,
            }),
        EntityAddress::Enum(raw) => compaction
            .enum_vectors
            .get(&raw)
            .map(|enumeration| EntityAddress::Enum(*enumeration)),
        EntityAddress::EnumTypeParameter {
            enumeration,
            ordinal,
        } => compaction
            .enum_vectors
            .get(&enumeration)
            .map(|enumeration| EntityAddress::EnumTypeParameter {
                enumeration: *enumeration,
                ordinal,
            }),
        EntityAddress::EnumVariant {
            enumeration,
            variant,
        } => compaction
            .enum_vectors
            .get(&enumeration)
            .map(|enumeration| EntityAddress::EnumVariant {
                enumeration: *enumeration,
                variant,
            }),
        EntityAddress::EnumField {
            enumeration,
            variant,
            field,
        } => compaction
            .enum_vectors
            .get(&enumeration)
            .map(|enumeration| EntityAddress::EnumField {
                enumeration: *enumeration,
                variant,
                field,
            }),
        EntityAddress::Trait(raw) => Some(EntityAddress::Trait(raw)),
        EntityAddress::Implementation(raw) => compaction
            .implementations
            .get(&crate::hir::ImplId::new(raw))
            .map(|implementation| EntityAddress::Implementation(implementation.raw())),
    }
}

fn install_survivor_entity_relocations(
    base: &WorkspaceSnapshot,
    program: &SemanticProgram,
    compaction: &super::compaction::CompactionResult,
    forced: &mut HashMap<EntityAddress, EntityId>,
) -> Result<(), WorkspaceError> {
    forced
        .try_reserve(base.indexes.entities.len())
        .map_err(|_| {
            WorkspaceError::Host(Arc::from("survivor entity relocation allocation failed"))
        })?;
    for (header, address) in base
        .indexes
        .entities
        .iter()
        .zip(&base.indexes.entity_addresses)
    {
        let relocated = if *address == EntityAddress::Main && program.main.is_none() {
            None
        } else {
            remap_entity_address(compaction, *address)
        };
        let Some(relocated) = relocated else {
            continue;
        };
        match forced.entry(relocated) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(header.id);
            }
            std::collections::hash_map::Entry::Occupied(entry) if *entry.get() == header.id => {}
            std::collections::hash_map::Entry::Occupied(_) => {
                return Err(WorkspaceError::Validation(Arc::from(
                    "surviving entities collide after dense compaction",
                )));
            }
        }
    }
    Ok(())
}

fn reserve_new_entity_identities(
    base: &WorkspaceSnapshot,
    allocator: &mut IdentityAllocator,
    forced: &mut HashMap<EntityAddress, EntityId>,
    entities: &[NewEntity],
) -> Result<(), WorkspaceError> {
    for entity in entities {
        if let Some(existing) = forced.get(&entity.address).copied() {
            if base.indexes.entity_lookup.contains_key(&existing) {
                return Err(WorkspaceError::Validation(Arc::from(
                    "new entity collides with a surviving semantic entity",
                )));
            }
            continue;
        }
        reserve_forced_entity(allocator, forced, entity.address)?;
    }
    Ok(())
}

fn force_surviving_nodes(
    base: &WorkspaceSnapshot,
    canonical: &SnapshotIndexes,
    forced_entities: &HashMap<EntityAddress, EntityId>,
    structural: &[StructuralAction],
    movement: Option<&SequenceMovement>,
) -> Result<HashMap<NodeAddress, NodeId>, WorkspaceError> {
    let mut old_by_key = HashMap::new();
    old_by_key
        .try_reserve(base.indexes.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("old node key allocation failed")))?;
    for (header, key) in base.indexes.nodes.iter().zip(&base.indexes.node_keys) {
        if old_by_key.insert(*key, header.id).is_some() {
            return Err(WorkspaceError::Validation(Arc::from(
                "old node key is duplicated",
            )));
        }
    }
    let mut targets = HashSet::new();
    targets
        .try_reserve(structural.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("structural target allocation failed")))?;
    targets.extend(structural.iter().map(|action| action.target));
    let mut canonical_to_old = HashMap::new();
    canonical_to_old
        .try_reserve(canonical.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("node survivor allocation failed")))?;
    let mut forced = HashMap::new();
    forced
        .try_reserve(base.indexes.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("forced node allocation failed")))?;
    if let Some(movement) = movement {
        install_sequence_movement_node_relocations(base, canonical, movement, &mut forced)?;
    }
    for index in 0..canonical.nodes.len() {
        let header = &canonical.nodes[index];
        let stable_owner = match header.owner {
            SemanticOwner::Entity(entity) => canonical
                .entity_lookup
                .get(&entity)
                .and_then(|entity_index| canonical.entity_addresses.get(*entity_index))
                .and_then(|address| forced_entities.get(address))
                .copied()
                .map(SemanticOwner::Entity),
            SemanticOwner::Node(parent) => canonical_to_old
                .get(&parent)
                .copied()
                .flatten()
                .map(SemanticOwner::Node),
        };
        let old = forced
            .get(&canonical.node_addresses[index])
            .copied()
            .or_else(|| {
                stable_owner.and_then(|owner| {
                    old_by_key
                        .get(&NodeKey {
                            owner,
                            ordinal: canonical.node_keys[index].ordinal,
                        })
                        .copied()
                })
            });
        if let Some(old) = old {
            insert_forced_node(&mut forced, canonical.node_addresses[index], old)?;
            canonical_to_old.insert(header.id, (!targets.contains(&old)).then_some(old));
        } else {
            canonical_to_old.insert(header.id, None);
        }
    }
    Ok(forced)
}

fn install_sequence_movement_node_relocations(
    base: &WorkspaceSnapshot,
    canonical: &SnapshotIndexes,
    movement: &SequenceMovement,
    forced: &mut HashMap<NodeAddress, NodeId>,
) -> Result<(), WorkspaceError> {
    let canonical_sequence = canonical
        .address_nodes
        .get(&movement.address)
        .copied()
        .ok_or_else(|| WorkspaceError::Validation(Arc::from("moved sequence is missing")))?;
    let canonical_sequence_index = canonical
        .node_lookup
        .get(&canonical_sequence)
        .copied()
        .ok_or_else(|| WorkspaceError::Validation(Arc::from("moved sequence index is missing")))?;
    if canonical.nodes[canonical_sequence_index].kind != NodeKind::Sequence {
        return Err(WorkspaceError::Validation(Arc::from(
            "moved sequence changed kind before identity reconciliation",
        )));
    }
    insert_forced_node(forced, movement.address, movement.sequence)?;

    if movement.final_old_ordinals.len() != movement.old_children.len() {
        return Err(WorkspaceError::Validation(Arc::from(
            "movement block permutation is incomplete",
        )));
    }
    let mut seen_ordinals = Vec::new();
    seen_ordinals
        .try_reserve(movement.old_children.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("movement ordinal allocation failed")))?;
    seen_ordinals.resize(movement.old_children.len(), false);
    let initial_identity_capacity = movement
        .old_children
        .len()
        .checked_add(1)
        .ok_or_else(|| WorkspaceError::Host(Arc::from("movement identity count overflow")))?;
    let mut mapped = HashSet::new();
    mapped
        .try_reserve(initial_identity_capacity)
        .map_err(|_| WorkspaceError::Host(Arc::from("movement identity allocation failed")))?;
    mapped.insert(movement.sequence);
    let mut movement_node_count = 1_usize;
    let mut preorder = movement
        .address
        .preorder
        .checked_add(1)
        .ok_or_else(|| WorkspaceError::Host(Arc::from("movement preorder overflow")))?;
    for old_ordinal in &movement.final_old_ordinals {
        let seen = seen_ordinals.get_mut(*old_ordinal).ok_or_else(|| {
            WorkspaceError::Validation(Arc::from("movement block ordinal is stale"))
        })?;
        if std::mem::replace(seen, true) {
            return Err(WorkspaceError::Validation(Arc::from(
                "movement block ordinal is duplicated",
            )));
        }
        let child = movement
            .old_children
            .get(*old_ordinal)
            .copied()
            .ok_or_else(|| {
                WorkspaceError::Validation(Arc::from("movement block child is stale"))
            })?;
        for old_node in movement_subtree_nodes(base, child)? {
            mapped.try_reserve(1).map_err(|_| {
                WorkspaceError::Host(Arc::from("movement identity allocation failed"))
            })?;
            if !mapped.insert(old_node) {
                return Err(WorkspaceError::Validation(Arc::from(
                    "movement continuity maps one node more than once",
                )));
            }
            movement_node_count = movement_node_count
                .checked_add(1)
                .ok_or_else(|| WorkspaceError::Host(Arc::from("movement node count overflow")))?;
            let address = NodeAddress {
                root: movement.address.root,
                preorder,
            };
            let temporary = canonical
                .address_nodes
                .get(&address)
                .copied()
                .ok_or_else(|| {
                    WorkspaceError::Validation(Arc::from(
                        "movement continuity destination is missing",
                    ))
                })?;
            let temporary_index =
                canonical
                    .node_lookup
                    .get(&temporary)
                    .copied()
                    .ok_or_else(|| {
                        WorkspaceError::Validation(Arc::from(
                            "movement continuity destination index is missing",
                        ))
                    })?;
            if base.workspace_node(old_node)?.kind != canonical.nodes[temporary_index].kind {
                return Err(WorkspaceError::Validation(Arc::from(
                    "movement continuity changed a surviving node kind",
                )));
            }
            insert_forced_node(forced, address, old_node)?;
            preorder = preorder
                .checked_add(1)
                .ok_or_else(|| WorkspaceError::Host(Arc::from("movement preorder overflow")))?;
        }
    }
    if seen_ordinals.iter().any(|seen| !seen) || mapped.len() != movement_node_count {
        return Err(WorkspaceError::Validation(Arc::from(
            "movement continuity omitted a surviving node",
        )));
    }
    let canonical_children = canonical
        .node_children
        .get(&canonical_sequence)
        .ok_or_else(|| {
            WorkspaceError::Validation(Arc::from("moved sequence children are missing"))
        })?;
    if canonical_children.len() != movement.old_children.len() {
        return Err(WorkspaceError::Validation(Arc::from(
            "moved sequence child count changed",
        )));
    }
    #[cfg(test)]
    record_transaction_measurement(|measurement| {
        measurement.movement_child_blocks_examined = movement.old_children.len();
        measurement.movement_nodes_relocated = movement_node_count;
    });
    Ok(())
}

fn movement_subtree_nodes(
    base: &WorkspaceSnapshot,
    root: NodeId,
) -> Result<Vec<NodeId>, WorkspaceError> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("movement traversal allocation failed")))?;
    pending.push(root);
    let mut nodes = Vec::new();
    while let Some(node) = pending.pop() {
        nodes
            .try_reserve(1)
            .map_err(|_| WorkspaceError::Host(Arc::from("movement subtree allocation failed")))?;
        nodes.push(node);
        if let Some(children) = base.indexes.node_children.get(&node) {
            pending.try_reserve(children.len()).map_err(|_| {
                WorkspaceError::Host(Arc::from("movement traversal allocation failed"))
            })?;
            pending.extend(children.iter().rev().copied());
        }
    }
    Ok(nodes)
}

fn ensure_structural_nonoverlapping(
    snapshot: &WorkspaceSnapshot,
    targets: &mut Vec<NodeId>,
    target: NodeId,
) -> Result<(), WorkspaceError> {
    snapshot.workspace_node(target)?;
    for existing in targets.iter().copied() {
        if existing == target
            || node_is_ancestor(snapshot, existing, target)?
            || node_is_ancestor(snapshot, target, existing)?
        {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                "structural edits in one transaction must target disjoint expression subtrees",
            )));
        }
    }
    targets.push(target);
    Ok(())
}

fn node_is_ancestor(
    snapshot: &WorkspaceSnapshot,
    ancestor: NodeId,
    mut node: NodeId,
) -> Result<bool, WorkspaceError> {
    loop {
        match snapshot.workspace_node(node)?.owner {
            SemanticOwner::Entity(_) => return Ok(false),
            SemanticOwner::Node(parent) if parent == ancestor => return Ok(true),
            SemanticOwner::Node(parent) => node = parent,
        }
    }
}

fn insert_forced_node(
    forced: &mut HashMap<NodeAddress, NodeId>,
    address: NodeAddress,
    node: NodeId,
) -> Result<(), WorkspaceError> {
    match forced.entry(address) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(node);
            Ok(())
        }
        std::collections::hash_map::Entry::Occupied(entry) if *entry.get() == node => Ok(()),
        std::collections::hash_map::Entry::Occupied(_) => Err(WorkspaceError::Validation(
            Arc::from("distinct edited nodes resolved to one canonical path"),
        )),
    }
}

fn create_main(
    base: &WorkspaceSnapshot,
    program: &mut SemanticProgram,
    created: &mut Vec<NewEntity>,
    holes: &mut Vec<NewHole>,
    parameters: Vec<ParameterDraft>,
    return_type: SemanticType,
) -> Result<(), WorkspaceError> {
    if program.main.is_some() {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "main entry point already exists",
        )));
    }
    let draft_entities = HashMap::new();
    let mut resolved_parameter_types = Vec::new();
    resolved_parameter_types
        .try_reserve(parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("main parameter type allocation failed")))?;
    for parameter in &parameters {
        let semantic = declaration_type_to_semantic(&parameter.ty, &draft_entities)?;
        resolved_parameter_types.push(resolve_semantic_type(
            base,
            program,
            semantic,
            "main parameter",
        )?);
    }
    crate::analyze::validate_semantic_main_parameters(
        parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .zip(&resolved_parameter_types),
    )
    .map_err(|message| WorkspaceError::InvalidTransaction(Arc::from(message)))?;

    let return_type = resolve_semantic_type(base, program, return_type, "main return")?;
    crate::analyze::validate_semantic_main_result(&return_type)
        .map_err(|message| WorkspaceError::InvalidTransaction(Arc::from(message)))?;

    let created_count = parameters
        .len()
        .checked_add(1)
        .ok_or_else(|| WorkspaceError::Host(Arc::from("created main entity count overflow")))?;
    created
        .try_reserve(created_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("created main entity allocation failed")))?;
    holes
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("created hole allocation failed")))?;
    created.push(NewEntity {
        address: EntityAddress::Main,
        kind: EntityKind::Main,
        name: Arc::from("main"),
    });
    let first_parameter_raw = u64::try_from(program.bindings.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("binding identity exceeds u64")))?;
    for (index, parameter) in parameters.iter().enumerate() {
        let raw = first_parameter_raw
            .checked_add(u64::try_from(index).map_err(|_| {
                WorkspaceError::Host(Arc::from("main parameter binding index exceeds u64"))
            })?)
            .ok_or_else(|| {
                WorkspaceError::Host(Arc::from("main parameter binding identity overflow"))
            })?;
        created.push(NewEntity {
            address: EntityAddress::Binding(raw),
            kind: EntityKind::Parameter,
            name: Arc::from(parameter.name.as_str()),
        });
    }

    let (parameter_bindings, parameter_places) =
        append_parameter_bindings(program, parameters, &resolved_parameter_types)?;
    let arity = resolved_parameter_types.len();
    program.main = Some(crate::hir::Main {
        origin: Origin::Semantic,
        params: parameter_bindings,
        param_places: parameter_places,
        param_types: resolved_parameter_types,
        return_type: return_type.clone(),
        arity,
        local_count: arity,
        body: Expr {
            ty: return_type,
            effects: EffectSet::UNKNOWN,
            origin: Origin::Semantic,
            kind: ExprKind::Hole,
        },
    });
    holes.push(NewHole {
        address: NodeAddress {
            root: EntityAddress::Main,
            preorder: 0,
        },
        kind: HoleKind::MissingBody,
        goal: Arc::from("provide the entry-point body"),
    });
    Ok(())
}

fn validate_parameter_drafts(
    parameters: &[ParameterDraft],
    duplicate_message: &'static str,
) -> Result<(), WorkspaceError> {
    let mut names = HashSet::new();
    names
        .try_reserve(parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("parameter name allocation failed")))?;
    for parameter in parameters {
        validate_name(&parameter.name)?;
        if !names.insert(parameter.name.as_str()) {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                duplicate_message,
            )));
        }
    }
    Ok(())
}

fn append_parameter_bindings(
    program: &mut SemanticProgram,
    parameters: Vec<ParameterDraft>,
    resolved_types: &[Type],
) -> Result<(Vec<crate::hir::BindingId>, Vec<PlaceId>), WorkspaceError> {
    if parameters.len() != resolved_types.len() {
        return Err(WorkspaceError::Validation(Arc::from(
            "callable parameter declarations and resolved types disagree",
        )));
    }
    program
        .bindings
        .try_reserve(parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("parameter binding allocation failed")))?;
    let mut bindings = Vec::new();
    let mut places = Vec::new();
    bindings
        .try_reserve(parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("parameter binding allocation failed")))?;
    places
        .try_reserve(parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("parameter place allocation failed")))?;
    for (index, (parameter, ty)) in parameters.into_iter().zip(resolved_types).enumerate() {
        let raw = u64::try_from(program.bindings.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("binding identity exceeds u64")))?;
        let binding = crate::hir::BindingId::new(raw);
        program.bindings.push(Binding {
            id: binding,
            name: parameter.name,
            kind: BindingKind::Parameter,
            ty: ty.clone(),
            origin: Origin::Semantic,
        });
        bindings.push(binding);
        places.push(PlaceId::new(u64::try_from(index).map_err(|_| {
            WorkspaceError::Host(Arc::from("parameter place exceeds u64"))
        })?));
    }
    Ok((bindings, places))
}

#[allow(clippy::too_many_arguments)]
fn create_function(
    base: &WorkspaceSnapshot,
    program: &mut SemanticProgram,
    allocator: &mut IdentityAllocator,
    forced: &mut HashMap<EntityAddress, EntityId>,
    created: &mut Vec<NewEntity>,
    holes: &mut Vec<NewHole>,
    name: String,
    type_parameters: Vec<TypeParameterDraft>,
    parameters: Vec<ParameterDraft>,
    return_type: DeclarationType,
) -> Result<(), WorkspaceError> {
    validate_declaration_name(&name)?;
    if declaration_name_exists(program, &name) {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "global declaration name already exists or is reserved",
        )));
    }

    let mut draft_parameters = HashMap::new();
    draft_parameters
        .try_reserve(type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("type-parameter draft allocation failed")))?;
    let mut type_parameter_names = HashMap::new();
    type_parameter_names
        .try_reserve(type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("type-parameter name allocation failed")))?;
    let mut resolved_bounds = Vec::new();
    let bound_count = type_parameters
        .iter()
        .try_fold(0_usize, |count, parameter| {
            count
                .checked_add(parameter.bounds.len())
                .ok_or_else(|| WorkspaceError::Host(Arc::from("function bound count overflow")))
        })?;
    resolved_bounds
        .try_reserve(bound_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("function bound allocation failed")))?;
    for parameter in &type_parameters {
        validate_name(&parameter.name)?;
        if crate::analyze::is_reserved_semantic_name(&parameter.name) {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                "function type-parameter name is reserved by the language",
            )));
        }
        if draft_parameters
            .insert(parameter.id, parameter.name.as_str())
            .is_some()
        {
            return Err(WorkspaceError::DuplicateDraftTypeParameter {
                parameter: parameter.id,
            });
        }
        if let Some(first) = type_parameter_names.insert(parameter.name.as_str(), parameter.id) {
            return Err(WorkspaceError::DuplicateTypeParameterName {
                first,
                duplicate: parameter.id,
            });
        }
        let mut seen_bounds = HashSet::new();
        seen_bounds
            .try_reserve(parameter.bounds.len())
            .map_err(|_| {
                WorkspaceError::Host(Arc::from("type-parameter bound set allocation failed"))
            })?;
        for identity in &parameter.bounds {
            let trait_id = resolve_bound_trait(base, program, *identity)?;
            let definition = program
                .traits
                .get(host_index(trait_id.raw(), "trait bound")?)
                .filter(|definition| definition.id == trait_id)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("trait")))?;
            if matches!(
                definition.core,
                Some(crate::hir::CoreTrait::Clone | crate::hir::CoreTrait::Drop)
            ) {
                return Err(WorkspaceError::unsupported(
                    "create-function",
                    "clone and drop require methods and are unavailable as marker bounds",
                ));
            }
            if !seen_bounds.insert(trait_id) {
                return Err(WorkspaceError::DuplicateTypeParameterBound {
                    parameter: parameter.id,
                    trait_identity: *identity,
                });
            }
            resolved_bounds.push(crate::hir::TraitBound {
                parameter: parameter.name.clone(),
                trait_id,
            });
        }
    }

    validate_parameter_drafts(&parameters, "function parameter name is duplicated")?;

    let mut used_parameters = HashSet::new();
    used_parameters
        .try_reserve(type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("used type-parameter allocation failed")))?;
    for parameter in &parameters {
        collect_declaration_type_parameters(
            &parameter.ty,
            &draft_parameters,
            &mut used_parameters,
        )?;
    }
    collect_declaration_type_parameters(&return_type, &draft_parameters, &mut used_parameters)?;
    for parameter in &type_parameters {
        if !used_parameters.contains(&parameter.id) {
            return Err(WorkspaceError::UnusedDraftTypeParameter {
                parameter: parameter.id,
            });
        }
    }

    let function_raw = u64::try_from(program.bindings.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("binding identity exceeds u64")))?;
    let root = EntityAddress::Binding(function_raw);
    let created_count = type_parameters
        .len()
        .checked_add(parameters.len())
        .and_then(|count| count.checked_add(1))
        .ok_or_else(|| WorkspaceError::Host(Arc::from("created function entity count overflow")))?;
    forced
        .try_reserve(created_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("forced function entity allocation failed")))?;
    created.try_reserve(created_count).map_err(|_| {
        WorkspaceError::Host(Arc::from("created function entity allocation failed"))
    })?;
    holes
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("created hole allocation failed")))?;

    let function_entity = reserve_forced_entity(allocator, forced, root)?;
    created.push(NewEntity {
        address: root,
        kind: EntityKind::Function,
        name: Arc::from(name.as_str()),
    });
    let mut staged_type_parameters = HashMap::new();
    staged_type_parameters
        .try_reserve(type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("staged type-parameter allocation failed")))?;
    let mut draft_entities = HashMap::new();
    draft_entities
        .try_reserve(type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("draft binder allocation failed")))?;
    let mut variables = Vec::new();
    variables
        .try_reserve(type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("function binder allocation failed")))?;
    for (ordinal, parameter) in type_parameters.iter().enumerate() {
        let ordinal = u64::try_from(ordinal)
            .map_err(|_| WorkspaceError::Host(Arc::from("type-parameter ordinal exceeds u64")))?;
        let address = EntityAddress::FunctionTypeParameter {
            function: function_raw,
            ordinal,
        };
        let entity = reserve_forced_entity(allocator, forced, address)?;
        staged_type_parameters.insert(entity, parameter.name.clone());
        draft_entities.insert(parameter.id, entity);
        variables.push(parameter.name.clone());
        created.push(NewEntity {
            address,
            kind: EntityKind::TypeParameter,
            name: Arc::from(parameter.name.as_str()),
        });
    }

    let first_parameter_raw = function_raw
        .checked_add(1)
        .ok_or_else(|| WorkspaceError::Host(Arc::from("parameter binding identity overflow")))?;
    for (index, parameter) in parameters.iter().enumerate() {
        let raw = first_parameter_raw
            .checked_add(u64::try_from(index).map_err(|_| {
                WorkspaceError::Host(Arc::from("parameter binding index exceeds u64"))
            })?)
            .ok_or_else(|| {
                WorkspaceError::Host(Arc::from("parameter binding identity overflow"))
            })?;
        let address = EntityAddress::Binding(raw);
        reserve_forced_entity(allocator, forced, address)?;
        created.push(NewEntity {
            address,
            kind: EntityKind::Parameter,
            name: Arc::from(parameter.name.as_str()),
        });
    }

    let mut resolved_parameter_types = Vec::new();
    resolved_parameter_types
        .try_reserve(parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("parameter type allocation failed")))?;
    for parameter in &parameters {
        let semantic = declaration_type_to_semantic(&parameter.ty, &draft_entities)?;
        resolved_parameter_types.push(super::types::resolve_with_staged_type_parameters(
            base,
            program,
            &semantic,
            Some(function_entity),
            &staged_type_parameters,
            false,
            false,
            "function parameter",
        )?);
    }
    let semantic_return = declaration_type_to_semantic(&return_type, &draft_entities)?;
    let resolved_return = super::types::resolve_with_staged_type_parameters(
        base,
        program,
        &semantic_return,
        Some(function_entity),
        &staged_type_parameters,
        false,
        false,
        "function return",
    )?;
    reject_reference_result(&resolved_return, "function")?;

    let created_binding_count = parameters
        .len()
        .checked_add(1)
        .ok_or_else(|| WorkspaceError::Host(Arc::from("created binding count overflow")))?;
    program
        .bindings
        .try_reserve(created_binding_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("binding allocation failed")))?;
    program
        .functions
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("function allocation failed")))?;
    program
        .global_layout
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("global layout allocation failed")))?;

    let function_binding = crate::hir::BindingId::new(function_raw);
    let signature = Type::Fn {
        params: resolved_parameter_types.clone(),
        ret: Box::new(resolved_return.clone()),
    };
    program.bindings.push(Binding {
        id: function_binding,
        name,
        kind: BindingKind::Function,
        ty: if variables.is_empty() {
            signature
        } else {
            Type::Forall {
                vars: variables,
                body: Box::new(signature),
            }
        },
        origin: Origin::Semantic,
    });

    let (parameter_bindings, parameter_places) =
        append_parameter_bindings(program, parameters, &resolved_parameter_types)?;
    program.functions.push(crate::hir::Function {
        binding: function_binding,
        origin: Origin::Semantic,
        params: parameter_bindings,
        param_places: parameter_places,
        bounds: resolved_bounds,
        arity: resolved_parameter_types.len(),
        local_count: resolved_parameter_types.len(),
        summary: EffectSet::UNKNOWN,
        body: Expr {
            ty: resolved_return,
            effects: EffectSet::UNKNOWN,
            origin: Origin::Semantic,
            kind: ExprKind::Hole,
        },
    });
    program.global_layout.push(function_binding);
    holes.push(NewHole {
        address: NodeAddress { root, preorder: 0 },
        kind: HoleKind::MissingBody,
        goal: Arc::from("provide the function body"),
    });
    Ok(())
}

fn collect_declaration_type_parameters<'a>(
    root: &'a DeclarationType,
    declared: &HashMap<DraftTypeParameterId, &'a str>,
    used: &mut HashSet<DraftTypeParameterId>,
) -> Result<(), WorkspaceError> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("declaration type work allocation failed")))?;
    pending.push(root);
    while let Some(ty) = pending.pop() {
        match ty {
            DeclarationType::DraftTypeParameter(parameter) => {
                if !declared.contains_key(parameter) {
                    return Err(WorkspaceError::UnknownDraftTypeParameter {
                        parameter: *parameter,
                    });
                }
                used.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("used type-parameter allocation failed"))
                })?;
                used.insert(*parameter);
            }
            DeclarationType::Enum { arguments, .. } => {
                pending.try_reserve(arguments.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("declaration type work allocation failed"))
                })?;
                pending.extend(arguments);
            }
            DeclarationType::List(inner) => {
                pending.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("declaration type work allocation failed"))
                })?;
                pending.push(inner);
            }
            DeclarationType::Function { parameters, result } => {
                let additional = parameters.len().checked_add(1).ok_or_else(|| {
                    WorkspaceError::Host(Arc::from("declaration type child count overflow"))
                })?;
                pending.try_reserve(additional).map_err(|_| {
                    WorkspaceError::Host(Arc::from("declaration type work allocation failed"))
                })?;
                pending.push(result);
                pending.extend(parameters);
            }
            _ => {}
        }
    }
    Ok(())
}

fn declaration_type_to_semantic(
    root: &DeclarationType,
    draft_entities: &HashMap<DraftTypeParameterId, EntityId>,
) -> Result<SemanticType, WorkspaceError> {
    enum Work<'a> {
        Visit(&'a DeclarationType),
        Enum(super::SemanticEnum, usize),
        List,
        Function(usize),
    }
    let mut work = Vec::new();
    work.try_reserve(1).map_err(|_| {
        WorkspaceError::Host(Arc::from(
            "declaration type conversion work allocation failed",
        ))
    })?;
    work.push(Work::Visit(root));
    let mut completed = Vec::new();
    while let Some(item) = work.pop() {
        match item {
            Work::Visit(ty) => {
                completed.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("declaration type conversion allocation failed"))
                })?;
                match ty {
                    DeclarationType::Never => completed.push(SemanticType::Never),
                    DeclarationType::Unit => completed.push(SemanticType::Unit),
                    DeclarationType::Bool => completed.push(SemanticType::Bool),
                    DeclarationType::I64 => completed.push(SemanticType::I64),
                    DeclarationType::F64 => completed.push(SemanticType::F64),
                    DeclarationType::String => completed.push(SemanticType::String),
                    DeclarationType::Bytes => completed.push(SemanticType::Bytes),
                    DeclarationType::ByteVector => completed.push(SemanticType::ByteVector),
                    DeclarationType::ByteSlice => completed.push(SemanticType::ByteSlice),
                    DeclarationType::ByteSliceMut => completed.push(SemanticType::ByteSliceMut),
                    DeclarationType::Path => completed.push(SemanticType::Path),
                    DeclarationType::Capability(kind) => {
                        completed.push(SemanticType::Capability(*kind));
                    }
                    DeclarationType::Symbol => completed.push(SemanticType::Symbol),
                    DeclarationType::Resource(kind) => {
                        completed.push(SemanticType::Resource(*kind));
                    }
                    DeclarationType::Product(entity) => {
                        completed.push(SemanticType::Product(*entity));
                    }
                    DeclarationType::Enum {
                        constructor,
                        arguments,
                    } => {
                        let additional = arguments.len().checked_add(1).ok_or_else(|| {
                            WorkspaceError::Host(Arc::from("declaration enum child count overflow"))
                        })?;
                        work.try_reserve(additional).map_err(|_| {
                            WorkspaceError::Host(Arc::from(
                                "declaration type conversion work allocation failed",
                            ))
                        })?;
                        work.push(Work::Enum(*constructor, arguments.len()));
                        work.extend(arguments.iter().rev().map(Work::Visit));
                    }
                    DeclarationType::TypeParameter(entity) => {
                        completed.push(SemanticType::TypeParameter(*entity));
                    }
                    DeclarationType::DraftTypeParameter(parameter) => {
                        let entity = draft_entities.get(parameter).copied().ok_or(
                            WorkspaceError::UnknownDraftTypeParameter {
                                parameter: *parameter,
                            },
                        )?;
                        completed.push(SemanticType::TypeParameter(entity));
                    }
                    DeclarationType::List(inner) => {
                        work.try_reserve(2).map_err(|_| {
                            WorkspaceError::Host(Arc::from(
                                "declaration type conversion work allocation failed",
                            ))
                        })?;
                        work.push(Work::List);
                        work.push(Work::Visit(inner));
                    }
                    DeclarationType::Function { parameters, result } => {
                        let additional = parameters.len().checked_add(2).ok_or_else(|| {
                            WorkspaceError::Host(Arc::from(
                                "declaration function child count overflow",
                            ))
                        })?;
                        work.try_reserve(additional).map_err(|_| {
                            WorkspaceError::Host(Arc::from(
                                "declaration type conversion work allocation failed",
                            ))
                        })?;
                        work.push(Work::Function(parameters.len()));
                        work.push(Work::Visit(result));
                        work.extend(parameters.iter().rev().map(Work::Visit));
                    }
                }
            }
            Work::Enum(constructor, count) => {
                let split = completed.len().checked_sub(count).ok_or_else(|| {
                    WorkspaceError::InvalidSemanticType {
                        position: Arc::from("declaration"),
                        reason: Arc::from("enum type children are incomplete"),
                    }
                })?;
                let arguments = completed.split_off(split);
                completed.push(SemanticType::Enum {
                    constructor,
                    arguments,
                });
            }
            Work::List => {
                let inner = completed
                    .pop()
                    .ok_or_else(|| WorkspaceError::InvalidSemanticType {
                        position: Arc::from("declaration"),
                        reason: Arc::from("list type child is incomplete"),
                    })?;
                completed.push(SemanticType::List(Box::new(inner)));
            }
            Work::Function(count) => {
                let result =
                    completed
                        .pop()
                        .ok_or_else(|| WorkspaceError::InvalidSemanticType {
                            position: Arc::from("declaration"),
                            reason: Arc::from("function type result is incomplete"),
                        })?;
                let split = completed.len().checked_sub(count).ok_or_else(|| {
                    WorkspaceError::InvalidSemanticType {
                        position: Arc::from("declaration"),
                        reason: Arc::from("function type parameters are incomplete"),
                    }
                })?;
                let parameters = completed.split_off(split);
                completed.push(SemanticType::Function {
                    parameters,
                    result: Box::new(result),
                });
            }
        }
    }
    let result = completed
        .pop()
        .ok_or_else(|| WorkspaceError::InvalidSemanticType {
            position: Arc::from("declaration"),
            reason: Arc::from("declaration type omitted its root"),
        })?;
    if completed.is_empty() {
        Ok(result)
    } else {
        Err(WorkspaceError::InvalidSemanticType {
            position: Arc::from("declaration"),
            reason: Arc::from("declaration type left disconnected results"),
        })
    }
}

fn resolve_bound_trait(
    base: &WorkspaceSnapshot,
    program: &SemanticProgram,
    identity: SemanticTrait,
) -> Result<crate::hir::TraitId, WorkspaceError> {
    match identity {
        SemanticTrait::Builtin(kind) => {
            let core = match kind {
                super::BuiltinTrait::Copy => crate::hir::CoreTrait::Copy,
                super::BuiltinTrait::Clone => crate::hir::CoreTrait::Clone,
                super::BuiltinTrait::Drop => crate::hir::CoreTrait::Drop,
                super::BuiltinTrait::Send => crate::hir::CoreTrait::Send,
                super::BuiltinTrait::Sync => crate::hir::CoreTrait::Sync,
            };
            program
                .traits
                .iter()
                .find(|definition| definition.core == Some(core))
                .map(|definition| definition.id)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("builtin trait")))
        }
        SemanticTrait::Entity(entity) => {
            if entity.namespace() != base.namespace() {
                return Err(WorkspaceError::ForeignNamespace(Arc::from("trait")));
            }
            let header = base
                .workspace_entity(entity)
                .map_err(|_| WorkspaceError::StaleIdentity(Arc::from("trait")))?;
            if header.kind != EntityKind::Trait {
                return Err(wrong_kind(
                    "function type-parameter bound",
                    "trait",
                    header.kind,
                ));
            }
            let address = base
                .indexes
                .entity_lookup
                .get(&entity)
                .and_then(|index| base.indexes.entity_addresses.get(*index))
                .copied()
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("trait")))?;
            let EntityAddress::Trait(raw) = address else {
                return Err(WorkspaceError::StaleIdentity(Arc::from("trait")));
            };
            program
                .traits
                .get(host_index(raw, "trait")?)
                .filter(|definition| definition.id.raw() == raw)
                .map(|definition| definition.id)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("trait")))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn create_product(
    base: &WorkspaceSnapshot,
    program: &mut SemanticProgram,
    allocator: &mut IdentityAllocator,
    forced: &mut HashMap<EntityAddress, EntityId>,
    created: &mut Vec<NewEntity>,
    name: String,
    fields: Vec<ProductFieldDraft>,
) -> Result<(), WorkspaceError> {
    validate_declaration_name(&name)?;
    if declaration_name_exists(program, &name) {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "global declaration name already exists or is reserved",
        )));
    }
    let raw = u64::try_from(program.products.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("product identity exceeds u64")))?;
    let address = EntityAddress::Product(raw);
    let entity = reserve_forced_entity(allocator, forced, address)?;
    let identity = entity_derived_identity(b"workspace-product-nominal-v1", entity);
    let mut names = HashSet::new();
    names
        .try_reserve(fields.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("product field name allocation failed")))?;
    let mut resolved = Vec::new();
    resolved
        .try_reserve(fields.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("product field allocation failed")))?;
    created
        .try_reserve(fields.len().checked_add(1).ok_or_else(|| {
            WorkspaceError::Host(Arc::from("created product entity count overflow"))
        })?)
        .map_err(|_| WorkspaceError::Host(Arc::from("created product entity allocation failed")))?;
    created.push(NewEntity {
        address,
        kind: EntityKind::Product,
        name: Arc::from(name.as_str()),
    });
    for (index, field) in fields.into_iter().enumerate() {
        validate_name(&field.name)?;
        if !names.insert(field.name.clone()) {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                "product field name is duplicated",
            )));
        }
        let ty = resolve_semantic_type(base, program, field.ty, "product field")?;
        reject_ownership_field(&ty, "product field")?;
        let field_raw = u64::try_from(index)
            .map_err(|_| WorkspaceError::Host(Arc::from("product field index exceeds u64")))?;
        let field_address = EntityAddress::ProductField {
            product: raw,
            field: field_raw,
        };
        let field_entity = reserve_forced_entity(allocator, forced, field_address)?;
        resolved.push(crate::hir::ProductField {
            identity: entity_derived_identity(b"workspace-product-field-v1", field_entity),
            source_order: field_raw,
            name: field.name.clone(),
            ty,
        });
        created.push(NewEntity {
            address: field_address,
            kind: EntityKind::ProductField,
            name: Arc::from(field.name),
        });
    }
    program
        .products
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("product allocation failed")))?;
    program.products.push(crate::hir::ProductDefinition {
        id: lkjscript_core::ProductId::new(raw),
        identity,
        name,
        origin: Origin::Semantic,
        fields: resolved,
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create_enum(
    base: &WorkspaceSnapshot,
    program: &mut SemanticProgram,
    allocator: &mut IdentityAllocator,
    forced: &mut HashMap<EntityAddress, EntityId>,
    created: &mut Vec<NewEntity>,
    name: String,
    type_parameters: Vec<EnumTypeParameterDraft>,
    variants: Vec<EnumVariantDraft>,
) -> Result<(), WorkspaceError> {
    validate_declaration_name(&name)?;
    if declaration_name_exists(program, &name) {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "global declaration name already exists or is reserved",
        )));
    }
    if variants.is_empty() {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "enum must contain at least one variant",
        )));
    }

    let mut draft_parameters = HashMap::new();
    draft_parameters
        .try_reserve(type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("type-parameter draft allocation failed")))?;
    let mut type_parameter_names = HashMap::new();
    type_parameter_names
        .try_reserve(type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("type-parameter name allocation failed")))?;
    for parameter in &type_parameters {
        validate_name(&parameter.name)?;
        if crate::analyze::is_reserved_semantic_name(&parameter.name)
            || crate::hir::Operation::from_name(&parameter.name).is_some()
        {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                "enum type-parameter name is reserved by the language",
            )));
        }
        if draft_parameters
            .insert(parameter.id, parameter.name.as_str())
            .is_some()
        {
            return Err(WorkspaceError::DuplicateDraftTypeParameter {
                parameter: parameter.id,
            });
        }
        if let Some(first) = type_parameter_names.insert(parameter.name.as_str(), parameter.id) {
            return Err(WorkspaceError::DuplicateTypeParameterName {
                first,
                duplicate: parameter.id,
            });
        }
    }

    let mut variant_names = HashSet::new();
    variant_names
        .try_reserve(variants.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("enum variant name allocation failed")))?;
    let mut referenced_parameters = HashSet::new();
    referenced_parameters
        .try_reserve(type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("used type-parameter allocation failed")))?;
    for variant in &variants {
        validate_name(&variant.name)?;
        if !variant_names.insert(variant.name.as_str()) {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                "enum variant name is duplicated",
            )));
        }
        let mut field_names = HashSet::new();
        field_names
            .try_reserve(variant.fields.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("enum field name allocation failed")))?;
        for field in &variant.fields {
            validate_name(&field.name)?;
            if !field_names.insert(field.name.as_str()) {
                return Err(WorkspaceError::InvalidTransaction(Arc::from(
                    "enum field name is duplicated within its variant",
                )));
            }
            collect_declaration_type_parameters(
                &field.ty,
                &draft_parameters,
                &mut referenced_parameters,
            )?;
        }
    }

    let created_count = 1_usize
        .checked_add(type_parameters.len())
        .ok_or_else(|| WorkspaceError::Host(Arc::from("created enum entity count overflow")))?;
    let created_count = variants.iter().try_fold(created_count, |count, variant| {
        count
            .checked_add(1)
            .and_then(|count| count.checked_add(variant.fields.len()))
            .ok_or_else(|| WorkspaceError::Host(Arc::from("created enum entity count overflow")))
    })?;
    forced
        .try_reserve(created_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("forced enum entity allocation failed")))?;
    created
        .try_reserve(created_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("created enum entity allocation failed")))?;
    program
        .enums
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("enum allocation failed")))?;

    let raw = u64::try_from(program.enums.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("enum identity exceeds u64")))?;
    let address = EntityAddress::Enum(raw);
    let entity = reserve_forced_entity(allocator, forced, address)?;
    let nominal = entity_derived_identity(b"workspace-enum-nominal-v1", entity);
    let enum_id = crate::hir::EnumId::new(nominal);
    created.push(NewEntity {
        address,
        kind: EntityKind::Enum,
        name: Arc::from(name.as_str()),
    });

    let mut staged_type_parameters = HashMap::new();
    staged_type_parameters
        .try_reserve(type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("staged type-parameter allocation failed")))?;
    let mut draft_entities = HashMap::new();
    draft_entities
        .try_reserve(type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("draft binder allocation failed")))?;
    let mut variables = Vec::new();
    variables
        .try_reserve(type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("enum binder allocation failed")))?;
    for (ordinal, parameter) in type_parameters.iter().enumerate() {
        let ordinal = u64::try_from(ordinal)
            .map_err(|_| WorkspaceError::Host(Arc::from("type-parameter ordinal exceeds u64")))?;
        let parameter_address = EntityAddress::EnumTypeParameter {
            enumeration: raw,
            ordinal,
        };
        let parameter_entity = reserve_forced_entity(allocator, forced, parameter_address)?;
        staged_type_parameters.insert(parameter_entity, parameter.name.clone());
        draft_entities.insert(parameter.id, parameter_entity);
        variables.push(parameter.name.clone());
        created.push(NewEntity {
            address: parameter_address,
            kind: EntityKind::TypeParameter,
            name: Arc::from(parameter.name.as_str()),
        });
    }

    let mut resolved_variants = Vec::new();
    resolved_variants
        .try_reserve(variants.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("enum variant allocation failed")))?;
    for (variant_index, variant) in variants.into_iter().enumerate() {
        let variant_raw = u64::try_from(variant_index)
            .map_err(|_| WorkspaceError::Host(Arc::from("enum variant index exceeds u64")))?;
        let variant_address = EntityAddress::EnumVariant {
            enumeration: raw,
            variant: variant_raw,
        };
        let variant_entity = reserve_forced_entity(allocator, forced, variant_address)?;
        let variant_id = crate::hir::VariantId::new(entity_derived_identity(
            b"workspace-enum-variant-v1",
            variant_entity,
        ));
        created.push(NewEntity {
            address: variant_address,
            kind: EntityKind::EnumVariant,
            name: Arc::from(variant.name.as_str()),
        });
        let mut fields = Vec::new();
        fields
            .try_reserve(variant.fields.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("enum field allocation failed")))?;
        for (field_index, field) in variant.fields.into_iter().enumerate() {
            let semantic = declaration_type_to_semantic(&field.ty, &draft_entities)?;
            let ty = super::types::resolve_with_staged_type_parameters(
                base,
                program,
                &semantic,
                Some(entity),
                &staged_type_parameters,
                false,
                false,
                "enum field",
            )?;
            reject_ownership_field(&ty, "enum field")?;
            let field_raw = u64::try_from(field_index)
                .map_err(|_| WorkspaceError::Host(Arc::from("enum field index exceeds u64")))?;
            let field_address = EntityAddress::EnumField {
                enumeration: raw,
                variant: variant_raw,
                field: field_raw,
            };
            let field_entity = reserve_forced_entity(allocator, forced, field_address)?;
            fields.push(crate::hir::EnumVariantField {
                id: crate::hir::VariantFieldId::new(entity_derived_identity(
                    b"workspace-enum-field-v1",
                    field_entity,
                )),
                name: field.name.clone(),
                source_order: field_raw,
                indirect: type_contains_enum(&ty),
                ty,
            });
            created.push(NewEntity {
                address: field_address,
                kind: EntityKind::EnumField,
                name: Arc::from(field.name),
            });
        }
        resolved_variants.push(crate::hir::EnumVariant {
            id: variant_id,
            name: variant.name,
            source_order: variant_raw,
            fields,
        });
    }
    let layout = crate::hir::RuntimeLayoutId::new(derived_identity(
        b"workspace-enum-runtime-layout-v1",
        nominal,
    ));
    program.enums.push(crate::hir::EnumDefinition {
        id: enum_id,
        name,
        origin: Origin::Semantic,
        type_parameters: variables,
        variants: resolved_variants,
        layout: crate::hir::EnumLayoutFacts {
            identity: layout,
            recursive: false,
        },
    });
    Ok(())
}

fn reserve_forced_entity(
    allocator: &mut IdentityAllocator,
    forced: &mut HashMap<EntityAddress, EntityId>,
    address: EntityAddress,
) -> Result<EntityId, WorkspaceError> {
    if forced.contains_key(&address) {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "semantic entity address is created more than once",
        )));
    }
    forced
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("forced entity allocation failed")))?;
    let entity = allocator
        .reserve_entity()
        .map_err(WorkspaceError::from_core)?;
    forced.insert(address, entity);
    Ok(entity)
}

fn entity_derived_identity(domain: &[u8], entity: EntityId) -> [u8; 32] {
    let mut bytes = [0_u8; 80];
    bytes[..32].copy_from_slice(&lkjscript_core::sha256(domain));
    bytes[32..64].copy_from_slice(&entity.namespace().bytes());
    bytes[64..72].copy_from_slice(&entity.slot().to_be_bytes());
    bytes[72..].copy_from_slice(&entity.generation().to_be_bytes());
    lkjscript_core::sha256(&bytes)
}

fn derived_identity(domain: &[u8], parent: [u8; 32]) -> [u8; 32] {
    let mut bytes = [0_u8; 64];
    bytes[..32].copy_from_slice(&lkjscript_core::sha256(domain));
    bytes[32..].copy_from_slice(&parent);
    lkjscript_core::sha256(&bytes)
}

fn validate_name(name: &str) -> Result<(), WorkspaceError> {
    if !lkjscript_contracts::is_identifier(name) {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "entity name must be a non-empty semantic identifier",
        )));
    }
    Ok(())
}

fn validate_declaration_name(name: &str) -> Result<(), WorkspaceError> {
    validate_name(name)?;
    if crate::analyze::is_reserved_semantic_name(name)
        || crate::hir::Operation::from_name(name).is_some()
    {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "global declaration name is reserved by the language",
        )));
    }
    Ok(())
}

fn resolve_semantic_type(
    base: &WorkspaceSnapshot,
    program: &SemanticProgram,
    ty: SemanticType,
    subject: &str,
) -> Result<Type, WorkspaceError> {
    super::types::resolve(base, program, &ty, None, false, false, subject)
}

fn reject_reference_result(ty: &Type, subject: &str) -> Result<(), WorkspaceError> {
    if matches!(ty, Type::ByteSlice | Type::ByteSliceMut) {
        Err(WorkspaceError::unsupported(
            "create-declaration",
            &format!("{subject} cannot return a lexical reference"),
        ))
    } else {
        Ok(())
    }
}

fn reject_ownership_field(ty: &Type, subject: &str) -> Result<(), WorkspaceError> {
    if matches!(
        ty,
        Type::Bytes | Type::ByteVector | Type::ByteSlice | Type::ByteSliceMut | Type::Resource(_)
    ) {
        Err(WorkspaceError::unsupported(
            "create-declaration",
            &format!("{subject} cannot store ownership or reference type {ty}"),
        ))
    } else {
        Ok(())
    }
}

fn type_contains_enum(ty: &Type) -> bool {
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Enum { .. } => return true,
            Type::List(inner) => pending.push(inner),
            Type::Fn { params, ret } => {
                pending.push(ret);
                pending.extend(params);
            }
            Type::Forall { body, .. } => pending.push(body),
            _ => {}
        }
    }
    false
}

fn declaration_name_exists(program: &SemanticProgram, name: &str) -> bool {
    global_name_conflicts(program, name, None)
}

fn global_name_conflicts(
    program: &SemanticProgram,
    name: &str,
    except: Option<EntityAddress>,
) -> bool {
    name == "main"
        || crate::hir::Operation::from_name(name).is_some()
        || crate::analyze::is_reserved_semantic_name(name)
        || program.bindings.iter().any(|binding| {
            binding.kind == BindingKind::Function
                && except != Some(EntityAddress::Binding(binding.id.raw()))
                && binding.name == name
        })
        || program.products.iter().any(|product| {
            except != Some(EntityAddress::Product(product.id.raw())) && product.name == name
        })
        || program
            .enums
            .iter()
            .enumerate()
            .any(|(index, enumeration)| {
                let raw = u64::try_from(index).ok();
                raw.is_none_or(|raw| except != Some(EntityAddress::Enum(raw)))
                    && enumeration.name == name
            })
        || program
            .traits
            .iter()
            .any(|declaration| declaration.name == name)
}

fn rename_entity(
    program: &mut SemanticProgram,
    address: EntityAddress,
    kind: EntityKind,
    new_name: &str,
) -> Result<(), WorkspaceError> {
    match (address, kind) {
        (EntityAddress::Binding(raw), kind)
            if matches!(
                kind,
                EntityKind::Function
                    | EntityKind::Parameter
                    | EntityKind::ImmutableLocal
                    | EntityKind::StaticBytesLocal
                    | EntityKind::MutableLocal
            ) =>
        {
            let index = host_index(raw, "entity")?;
            program
                .bindings
                .get(index)
                .filter(|binding| binding.id.raw() == raw)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("entity")))?;
            if kind == EntityKind::Function
                && global_name_conflicts(program, new_name, Some(address))
            {
                return Err(global_name_collision());
            }
            replace_name(&mut program.bindings[index].name, new_name);
        }
        (EntityAddress::Product(raw), EntityKind::Product) => {
            let index = host_index(raw, "product")?;
            program
                .products
                .get(index)
                .filter(|definition| definition.id.raw() == raw)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product")))?;
            if global_name_conflicts(program, new_name, Some(address)) {
                return Err(global_name_collision());
            }
            replace_name(&mut program.products[index].name, new_name);
        }
        (EntityAddress::ProductField { product, field }, EntityKind::ProductField) => {
            let product = program
                .products
                .get_mut(host_index(product, "product")?)
                .filter(|definition| definition.id.raw() == product)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product")))?;
            let field_index = host_index(field, "product field")?;
            if product
                .fields
                .iter()
                .enumerate()
                .any(|(index, sibling)| index != field_index && sibling.name == new_name)
            {
                return Err(member_name_collision("product field"));
            }
            let target = product
                .fields
                .get_mut(field_index)
                .filter(|target| target.source_order == field)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product field")))?;
            replace_name(&mut target.name, new_name);
        }
        (EntityAddress::Enum(raw), EntityKind::Enum) => {
            let index = host_index(raw, "enum")?;
            if program.enums.get(index).is_none() {
                return Err(WorkspaceError::StaleIdentity(Arc::from("enum")));
            }
            if global_name_conflicts(program, new_name, Some(address)) {
                return Err(global_name_collision());
            }
            replace_name(&mut program.enums[index].name, new_name);
        }
        (
            EntityAddress::EnumVariant {
                enumeration,
                variant,
            },
            EntityKind::EnumVariant,
        ) => {
            let definition = program
                .enums
                .get_mut(host_index(enumeration, "enum")?)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum")))?;
            let variant_index = host_index(variant, "enum variant")?;
            if definition
                .variants
                .iter()
                .enumerate()
                .any(|(index, sibling)| index != variant_index && sibling.name == new_name)
            {
                return Err(member_name_collision("enum variant"));
            }
            let target = definition
                .variants
                .get_mut(variant_index)
                .filter(|target| target.source_order == variant)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum variant")))?;
            replace_name(&mut target.name, new_name);
        }
        (
            EntityAddress::EnumField {
                enumeration,
                variant,
                field,
            },
            EntityKind::EnumField,
        ) => {
            let variant_index = host_index(variant, "enum variant")?;
            let selected = program
                .enums
                .get_mut(host_index(enumeration, "enum")?)
                .and_then(|definition| definition.variants.get_mut(variant_index))
                .filter(|selected| selected.source_order == variant)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum variant")))?;
            let field_index = host_index(field, "enum field")?;
            if selected
                .fields
                .iter()
                .enumerate()
                .any(|(index, sibling)| index != field_index && sibling.name == new_name)
            {
                return Err(member_name_collision("enum field"));
            }
            let target = selected
                .fields
                .get_mut(field_index)
                .filter(|target| target.source_order == field)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum field")))?;
            replace_name(&mut target.name, new_name);
        }
        _ => {
            return Err(WorkspaceError::unsupported(
                "rename-entity",
                "this entity kind cannot be renamed",
            ))
        }
    }
    Ok(())
}

fn replace_name(name: &mut String, replacement: &str) {
    name.clear();
    name.push_str(replacement);
}

fn global_name_collision() -> WorkspaceError {
    WorkspaceError::InvalidTransaction(Arc::from(
        "global declaration name already exists or is reserved",
    ))
}

fn member_name_collision(kind: &str) -> WorkspaceError {
    WorkspaceError::InvalidTransaction(Arc::from(format!("{kind} name collides with a sibling")))
}

fn unresolved_introduction_context(
    snapshot: &WorkspaceSnapshot,
    target: NodeId,
) -> Result<(NodeAddress, NodeKey, Type, Vec<EntityId>), WorkspaceError> {
    let header = snapshot.workspace_node(target)?;
    if header.kind != NodeKind::Hole {
        return edit_context(snapshot, target);
    }
    let record = snapshot
        .holes
        .iter()
        .find(|record| record.state.id.node() == target)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole")))?;
    let mut visible = Vec::new();
    visible
        .try_reserve(record.state.visible_entities.len())
        .map_err(|_| {
            WorkspaceError::Host(Arc::from(
                "unresolved value-reference visibility allocation failed",
            ))
        })?;
    visible.extend(record.state.visible_entities.iter().copied());
    Ok((
        record.address,
        record.key,
        record.expected_internal.clone(),
        visible,
    ))
}

fn require_unresolved_value_reference(
    snapshot: &WorkspaceSnapshot,
    reference: UnresolvedValueReferenceId,
) -> Result<(), WorkspaceError> {
    let header = snapshot.workspace_node(reference.0)?;
    if header.kind != NodeKind::UnresolvedValueReference {
        return Err(WorkspaceError::WrongEntityKind {
            operation: Arc::from("resolve-unresolved-value-reference"),
            expected: Arc::from("unresolved value-reference node"),
            actual: SemanticKind::Node(header.kind),
        });
    }
    if !snapshot
        .unresolved_value_references
        .iter()
        .any(|record| record.state.id == reference)
    {
        return Err(WorkspaceError::StaleIdentity(Arc::from(
            "unresolved value reference",
        )));
    }
    Ok(())
}

fn edit_context(
    snapshot: &WorkspaceSnapshot,
    target: NodeId,
) -> Result<(NodeAddress, NodeKey, Type, Vec<EntityId>), WorkspaceError> {
    let header = snapshot.workspace_node(target)?;
    if header.kind == NodeKind::Hole {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "hole nodes must be refined or filled with a hole edit",
        )));
    }
    let index = snapshot
        .indexes
        .node_lookup
        .get(&target)
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node")))?;
    let address = snapshot.indexes.node_addresses[index];
    let key = snapshot.indexes.node_keys[index];
    let expression = expression_at(&snapshot.program, address)?;
    let visible = visible_entities(snapshot, address)?;
    Ok((
        address,
        key,
        base_expected_type(snapshot, index, expression)?,
        visible,
    ))
}

fn base_expected_type(
    snapshot: &WorkspaceSnapshot,
    index: usize,
    expression: &Expr,
) -> Result<Type, WorkspaceError> {
    snapshot
        .indexes
        .node_expected_types
        .get(index)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node expectation")))
        .map(|expected| expected.clone().unwrap_or_else(|| expression.ty.clone()))
}

fn visible_entities(
    snapshot: &WorkspaceSnapshot,
    address: NodeAddress,
) -> Result<Vec<EntityId>, WorkspaceError> {
    visible_entities_in(&snapshot.program, &snapshot.indexes, address)
}

pub(super) fn visible_entities_in(
    program: &SemanticProgram,
    indexes: &SnapshotIndexes,
    address: NodeAddress,
) -> Result<Vec<EntityId>, WorkspaceError> {
    enum ScopeWork<'a> {
        Visit(&'a Expr),
        Add(crate::hir::BindingId),
        Remove(Vec<crate::hir::BindingId>),
    }

    let owner = indexes
        .address_entities
        .get(&address.root)
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node root")))?;
    let mut visible = Vec::new();
    visible
        .try_reserve(indexes.entities.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("visible entity allocation failed")))?;
    for entity in &indexes.entities {
        if entity.kind == EntityKind::Function
            || (matches!(
                entity.kind,
                EntityKind::Parameter | EntityKind::TypeParameter
            ) && entity.owner == Some(owner))
        {
            visible.push(entity.id);
        }
    }
    let root = expression_root(program, address.root)?;
    let mut work = vec![ScopeWork::Visit(root)];
    let mut active = HashSet::new();
    let mut preorder = 0_u64;
    while let Some(item) = work.pop() {
        match item {
            ScopeWork::Add(binding) => {
                active.insert(binding);
            }
            ScopeWork::Remove(bindings) => {
                for binding in bindings {
                    active.remove(&binding);
                }
            }
            ScopeWork::Visit(expression) => {
                if preorder == address.preorder {
                    for binding in active {
                        if let Some(entity) = indexes
                            .address_entities
                            .get(&EntityAddress::Binding(binding.raw()))
                            .copied()
                        {
                            visible.push(entity);
                        }
                    }
                    visible.sort();
                    visible.dedup();
                    return Ok(visible);
                }
                preorder = preorder.checked_add(1).ok_or_else(|| {
                    WorkspaceError::Host(Arc::from("visibility preorder overflow"))
                })?;
                match &expression.kind {
                    ExprKind::Let { bindings, body } => {
                        let mut removed = Vec::new();
                        removed.try_reserve(bindings.len()).map_err(|_| {
                            WorkspaceError::Host(Arc::from("visibility scope allocation failed"))
                        })?;
                        removed.extend(bindings.iter().map(|local| local.binding));
                        work.push(ScopeWork::Remove(removed));
                        work.push(ScopeWork::Visit(body));
                        for local in bindings.iter().rev() {
                            work.push(ScopeWork::Add(local.binding));
                            work.push(ScopeWork::Visit(&local.value));
                        }
                    }
                    ExprKind::MutableLocal {
                        binding,
                        initial,
                        body,
                        ..
                    } => {
                        work.push(ScopeWork::Remove(vec![*binding]));
                        work.push(ScopeWork::Visit(body));
                        work.push(ScopeWork::Add(*binding));
                        work.push(ScopeWork::Visit(initial));
                    }
                    ExprKind::Match {
                        plan,
                        scrutinee,
                        arms,
                    } => {
                        let plan = program
                            .match_plans
                            .get(host_index(plan.raw(), "match plan")?)
                            .filter(|item| item.id == *plan)
                            .ok_or_else(|| {
                                WorkspaceError::StaleIdentity(Arc::from("match plan"))
                            })?;
                        if arms.len() != plan.arms.len() {
                            return Err(WorkspaceError::Validation(Arc::from(
                                "semantic match arm count is stale",
                            )));
                        }
                        for (body, arm) in arms.iter().zip(&plan.arms).rev() {
                            let bindings = match_pattern_bindings(&arm.pattern)?;
                            work.push(ScopeWork::Remove(bindings.clone()));
                            work.push(ScopeWork::Visit(body));
                            work.extend(bindings.into_iter().rev().map(ScopeWork::Add));
                        }
                        work.push(ScopeWork::Visit(scrutinee));
                    }
                    _ => {
                        let mut children = Vec::new();
                        crate::hir::for_each_expression_child(expression, &mut |child| {
                            children.push(child)
                        });
                        work.extend(children.into_iter().rev().map(ScopeWork::Visit));
                    }
                }
            }
        }
    }
    Err(WorkspaceError::StaleIdentity(Arc::from("node preorder")))
}

fn match_pattern_bindings(
    pattern: &crate::hir::MatchPattern,
) -> Result<Vec<crate::hir::BindingId>, WorkspaceError> {
    let mut result = Vec::new();
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("match scope work allocation failed")))?;
    pending.push(pattern);
    while let Some(pattern) = pending.pop() {
        match pattern {
            crate::hir::MatchPattern::Binding { local } => {
                result.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("match scope binding allocation failed"))
                })?;
                result.push(local.binding);
            }
            crate::hir::MatchPattern::Variant { fields, .. }
            | crate::hir::MatchPattern::Product { fields, .. } => {
                pending.try_reserve(fields.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("match scope work allocation failed"))
                })?;
                pending.extend(fields.iter().map(|field| &field.pattern));
            }
            _ => {}
        }
    }
    Ok(result)
}

fn root_owner(
    snapshot: &WorkspaceSnapshot,
    address: NodeAddress,
) -> Result<EntityId, WorkspaceError> {
    snapshot
        .indexes
        .address_entities
        .get(&address.root)
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node root")))
}

fn expression_at(program: &SemanticProgram, address: NodeAddress) -> Result<&Expr, WorkspaceError> {
    expression_root(program, address.root)?
        .try_at_preorder(address.preorder)
        .map_err(WorkspaceError::from_core)?
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node address")))
}

fn expression_root(
    program: &SemanticProgram,
    address: EntityAddress,
) -> Result<&Expr, WorkspaceError> {
    if address == EntityAddress::Main {
        return program
            .main
            .as_ref()
            .map(|main| &main.body)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("main root")));
    }
    let EntityAddress::Binding(binding_raw) = address else {
        return Err(WorkspaceError::StaleIdentity(Arc::from("node root")));
    };
    program
        .functions
        .iter()
        .find(|function| function.binding.raw() == binding_raw)
        .map(|function| &function.body)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node root")))
}

fn expression_root_mut(
    program: &mut SemanticProgram,
    address: EntityAddress,
) -> Result<&mut Expr, WorkspaceError> {
    if address == EntityAddress::Main {
        return program
            .main
            .as_mut()
            .map(|main| &mut main.body)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("main root")));
    }
    let EntityAddress::Binding(raw) = address else {
        return Err(WorkspaceError::StaleIdentity(Arc::from("node root")));
    };
    program
        .functions
        .iter_mut()
        .find(|function| function.binding.raw() == raw)
        .map(|function| &mut function.body)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node root")))
}

fn replace_expression(
    program: &mut SemanticProgram,
    address: NodeAddress,
    replacement: &Expr,
) -> Result<(), WorkspaceError> {
    let root = expression_root_mut(program, address.root)?;
    let replaced = root
        .try_replaced_preorder(address.preorder, replacement)
        .map_err(WorkspaceError::from_core)?
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node address")))?;
    *root = replaced;
    Ok(())
}

fn apply_sequence_movement(
    program: &mut SemanticProgram,
    movement: &SequenceMovement,
) -> Result<(), WorkspaceError> {
    if movement.path.len() != movement.ancestor_types.len() {
        return Err(WorkspaceError::Validation(Arc::from(
            "sequence movement ancestry types are incomplete",
        )));
    }
    let mut current = expression_root_mut(program, movement.address.root)?;
    for (ordinal, ty) in movement.path.iter().copied().zip(&movement.ancestor_types) {
        current.ty = ty.clone();
        current = current
            .try_child_mut(ordinal)
            .map_err(WorkspaceError::from_core)?;
    }
    current.ty = movement.sequence_type.clone();
    let ExprKind::Do(values) = &mut current.kind else {
        return Err(WorkspaceError::Validation(Arc::from(
            "selected sequence changed kind during staging",
        )));
    };
    if values.len() != movement.old_children.len()
        || movement.old_index >= values.len()
        || movement.new_index >= values.len()
    {
        return Err(WorkspaceError::Validation(Arc::from(
            "sequence movement order changed during staging",
        )));
    }
    let child = values.remove(movement.old_index);
    values.insert(movement.new_index, child);
    Ok(())
}

fn refresh_semantic_match_types(
    program: &mut SemanticProgram,
    structural: &[StructuralAction],
    movement: Option<&SequenceMovement>,
) -> Result<(), WorkspaceError> {
    struct Update {
        plan: crate::hir::MatchPlanId,
        result: Type,
        arms: Vec<Type>,
    }

    let mut affected_roots = Vec::new();
    let affected_count = structural
        .len()
        .checked_add(usize::from(movement.is_some()))
        .ok_or_else(|| WorkspaceError::Host(Arc::from("match root count overflow")))?;
    affected_roots
        .try_reserve(affected_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("match root allocation failed")))?;
    affected_roots.extend(structural.iter().map(|action| action.address.root));
    if let Some(movement) = movement {
        affected_roots.push(movement.address.root);
    }
    affected_roots.sort_unstable();
    affected_roots.dedup();

    let mut pending = Vec::new();
    pending
        .try_reserve(affected_roots.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("match root allocation failed")))?;
    for root in affected_roots {
        pending.push(expression_root(program, root)?);
    }
    let mut updates = Vec::new();
    while let Some(expression) = pending.pop() {
        if let ExprKind::Match { plan, arms, .. } = &expression.kind {
            let mut arm_types = Vec::new();
            arm_types.try_reserve(arms.len()).map_err(|_| {
                WorkspaceError::Host(Arc::from("match type update allocation failed"))
            })?;
            arm_types.extend(arms.iter().map(|arm| arm.ty.clone()));
            updates.try_reserve(1).map_err(|_| {
                WorkspaceError::Host(Arc::from("match type update allocation failed"))
            })?;
            updates.push(Update {
                plan: *plan,
                result: expression.ty.clone(),
                arms: arm_types,
            });
        }
        let mut child_count = Some(0_usize);
        crate::hir::for_each_expression_child(expression, &mut |_| {
            child_count = child_count.and_then(|count| count.checked_add(1));
        });
        let child_count = child_count.ok_or_else(|| {
            WorkspaceError::Host(Arc::from("match type update child count overflow"))
        })?;
        pending.try_reserve(child_count).map_err(|_| {
            WorkspaceError::Host(Arc::from("match type update work allocation failed"))
        })?;
        crate::hir::for_each_expression_child(expression, &mut |child| pending.push(child));
    }

    let mut seen = HashSet::new();
    seen.try_reserve(updates.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("match type identity allocation failed")))?;
    for update in updates {
        if !seen.insert(update.plan) {
            return Err(WorkspaceError::Validation(Arc::from(
                "semantic match plan is used by more than one expression",
            )));
        }
        let planned = program
            .match_plans
            .get_mut(host_index(update.plan.raw(), "match plan")?)
            .filter(|planned| planned.id == update.plan)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("match plan")))?;
        if planned.arms.len() != update.arms.len() {
            return Err(WorkspaceError::Validation(Arc::from(
                "semantic match plan arm count is stale",
            )));
        }
        planned.result_type = update.result;
        for (arm, ty) in planned.arms.iter_mut().zip(update.arms) {
            arm.body_type = ty;
        }
    }
    Ok(())
}

struct LoweredDraft {
    expression: Expr,
    entities: Vec<NewEntity>,
}

struct PreparedMatch {
    scrutinee: crate::hir::MatchLocal,
    arms: Vec<PreparedMatchArm>,
}

struct PreparedMatchArm {
    pattern: crate::hir::MatchPattern,
    bindings: Vec<DraftBindingId>,
}

enum DraftLoweringAction {
    BeginMatch(usize),
    BeginArm { node: usize, arm: usize },
    EnterLoop(usize),
    ExitLoop(usize),
    Lower(usize),
}

#[derive(Clone)]
struct ResolvedDraftBinding {
    binding: crate::hir::BindingId,
    slot: usize,
    place: crate::hir::PlaceId,
    ty: Type,
    kind: BindingKind,
}

struct DraftDefinitionEvent {
    binding: DraftBindingId,
    name: String,
    mutable_type: Option<SemanticType>,
}

struct LoweringState {
    root_places: HashMap<EntityAddress, u64>,
    implementation_index:
        HashMap<(crate::hir::TraitId, lkjscript_core::ProductId), crate::hir::ImplId>,
    next_loan: u64,
    next_loop: u64,
}

impl LoweringState {
    fn new(program: &SemanticProgram) -> Result<Self, WorkspaceError> {
        let mut next_loan = 0_u64;
        let mut next_loop = 0_u64;
        let mut roots = Vec::new();
        roots
            .try_reserve(
                program.functions.len().checked_add(1).ok_or_else(|| {
                    WorkspaceError::Host(Arc::from("lowering root count overflow"))
                })?,
            )
            .map_err(|_| WorkspaceError::Host(Arc::from("lowering root allocation failed")))?;
        roots.extend(program.functions.iter().map(|function| &function.body));
        if let Some(main) = &program.main {
            roots.push(&main.body);
        }
        for root in roots {
            let mut pending = vec![root];
            while let Some(expression) = pending.pop() {
                if let ExprKind::Borrow { loan, .. } | ExprKind::BorrowBytes { loan, .. } =
                    &expression.kind
                {
                    next_loan = next_loan.max(loan.raw().checked_add(1).ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("loan identity exhausted"))
                    })?);
                }
                if let ExprKind::While { loop_id, .. } | ExprKind::Loop { loop_id, .. } =
                    &expression.kind
                {
                    next_loop = next_loop.max(loop_id.raw().checked_add(1).ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("loop identity exhausted"))
                    })?);
                }
                crate::hir::for_each_expression_child(expression, &mut |child| pending.push(child));
            }
        }
        let mut implementation_index = HashMap::new();
        implementation_index
            .try_reserve(program.implementations.len())
            .map_err(|_| {
                WorkspaceError::Host(Arc::from("generic implementation index allocation failed"))
            })?;
        for implementation in &program.implementations {
            if implementation_index
                .insert(
                    (implementation.trait_id, implementation.product),
                    implementation.id,
                )
                .is_some()
            {
                return Err(WorkspaceError::Validation(Arc::from(
                    "generic implementation index contains overlapping facts",
                )));
            }
        }
        Ok(Self {
            root_places: HashMap::new(),
            implementation_index,
            next_loan,
            next_loop,
        })
    }

    fn place(
        &mut self,
        program: &SemanticProgram,
        root: EntityAddress,
    ) -> Result<crate::hir::PlaceId, WorkspaceError> {
        if let std::collections::hash_map::Entry::Vacant(slot) = self.root_places.entry(root) {
            let mut next = 0_u64;
            let (parameters, expression) = if root == EntityAddress::Main {
                let main = program
                    .main
                    .as_ref()
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("main root")))?;
                (&main.param_places, &main.body)
            } else {
                let EntityAddress::Binding(raw) = root else {
                    return Err(WorkspaceError::StaleIdentity(Arc::from("expression root")));
                };
                let function = program
                    .functions
                    .iter()
                    .find(|function| function.binding.raw() == raw)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function root")))?;
                (&function.param_places, &function.body)
            };
            for place in parameters {
                next =
                    next.max(place.raw().checked_add(1).ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("place identity exhausted"))
                    })?);
            }
            let mut pending = vec![expression];
            while let Some(expression) = pending.pop() {
                match &expression.kind {
                    ExprKind::Move { place, .. }
                    | ExprKind::Borrow { place, .. }
                    | ExprKind::BorrowBytes { place, .. }
                    | ExprKind::MutableLocal { place, .. } => {
                        next = next.max(place.raw().checked_add(1).ok_or_else(|| {
                            WorkspaceError::Host(Arc::from("place identity exhausted"))
                        })?);
                    }
                    ExprKind::Let { bindings, .. } => {
                        for local in bindings {
                            next = next.max(local.place.raw().checked_add(1).ok_or_else(|| {
                                WorkspaceError::Host(Arc::from("place identity exhausted"))
                            })?);
                        }
                    }
                    ExprKind::Match { plan, .. } => {
                        let plan = program
                            .match_plans
                            .get(host_index(plan.raw(), "match plan")?)
                            .filter(|item| item.id == *plan)
                            .ok_or_else(|| {
                                WorkspaceError::StaleIdentity(Arc::from("match plan"))
                            })?;
                        next = next.max(plan.scrutinee.place.raw().checked_add(1).ok_or_else(
                            || WorkspaceError::Host(Arc::from("place identity exhausted")),
                        )?);
                        for local in plan
                            .projections
                            .iter()
                            .map(|projection| &projection.local)
                            .chain(plan.bindings.iter().map(|binding| &binding.local))
                        {
                            next = next.max(local.place.raw().checked_add(1).ok_or_else(|| {
                                WorkspaceError::Host(Arc::from("place identity exhausted"))
                            })?);
                        }
                    }
                    _ => {}
                }
                crate::hir::for_each_expression_child(expression, &mut |child| pending.push(child));
            }
            slot.insert(next);
        }
        let next = self
            .root_places
            .get_mut(&root)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("place allocator")))?;
        let place = crate::hir::PlaceId::new(*next);
        *next = next
            .checked_add(1)
            .ok_or_else(|| WorkspaceError::Host(Arc::from("place identity exhausted")))?;
        Ok(place)
    }

    fn loan(&mut self) -> Result<crate::hir::LoanId, WorkspaceError> {
        let loan = crate::hir::LoanId::new(self.next_loan);
        self.next_loan = self
            .next_loan
            .checked_add(1)
            .ok_or_else(|| WorkspaceError::Host(Arc::from("loan identity exhausted")))?;
        Ok(loan)
    }

    fn loop_id(&mut self) -> Result<crate::hir::LoopId, WorkspaceError> {
        let loop_id = crate::hir::LoopId::new(self.next_loop);
        self.next_loop = self
            .next_loop
            .checked_add(1)
            .ok_or_else(|| WorkspaceError::Host(Arc::from("loop identity exhausted")))?;
        Ok(loop_id)
    }
}

fn callable_return_type(
    program: &SemanticProgram,
    root: EntityAddress,
) -> Result<&Type, WorkspaceError> {
    if root == EntityAddress::Main {
        return program
            .main
            .as_ref()
            .map(|main| &main.return_type)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("main root")));
    }
    let EntityAddress::Binding(raw) = root else {
        return Err(WorkspaceError::StaleIdentity(Arc::from("callable root")));
    };
    let binding = program
        .binding(crate::hir::BindingId::new(raw))
        .filter(|binding| matches!(&binding.kind, BindingKind::Function))
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function root")))?;
    let signature = match &binding.ty {
        Type::Forall { body, .. } => body.as_ref(),
        other => other,
    };
    let Type::Fn { ret, .. } = signature else {
        return Err(WorkspaceError::Validation(Arc::from(
            "function root lost its callable signature",
        )));
    };
    Ok(ret)
}

#[allow(clippy::too_many_arguments)]
fn lower_draft(
    snapshot: &WorkspaceSnapshot,
    program: &mut SemanticProgram,
    draft: &ExpressionDraft,
    expected: &Type,
    origin: Origin,
    visible: &[EntityId],
    address: NodeAddress,
    lowering: &mut LoweringState,
    deleting_entities: &HashSet<EntityId>,
) -> Result<LoweredDraft, WorkspaceError> {
    validate_draft_shape(draft)?;
    let order = draft_lowering_actions(draft)?;
    let root = address.root;
    let published_control = expression_root(&snapshot.program, root)?
        .try_control_context(address.preorder)
        .map_err(WorkspaceError::from_core)?
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("draft target address")))?;
    let mut definition_events = draft_definition_events(draft)?;
    validate_draft_binding_scopes(draft)?;
    let mut visible_set = HashSet::new();
    visible_set
        .try_reserve(visible.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("draft visibility allocation failed")))?;
    visible_set.extend(visible.iter().copied());
    let callable = snapshot
        .indexes
        .address_entities
        .get(&root)
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("callable owner")))?;
    // A nested return is checked against its callable declaration, not the
    // immediate sequence, branch, loop, or local-body expectation.
    let declared_result_type = callable_return_type(program, root)?.clone();
    let published_locations = binding_locations(program, root)?;
    let mut completed: Vec<Option<Expr>> = Vec::new();
    completed
        .try_reserve(draft.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("draft lowering allocation failed")))?;
    completed.resize_with(draft.nodes.len(), || None);
    let mut locals: HashMap<DraftBindingId, ResolvedDraftBinding> = HashMap::new();
    locals
        .try_reserve(definition_events.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("draft local allocation failed")))?;
    let mut entities = Vec::new();
    entities
        .try_reserve(definition_events.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("draft local entity allocation failed")))?;
    let mut next_slot = root_local_count(program, root)?;
    let mut prepared_matches = HashMap::new();
    let loop_count = draft
        .nodes
        .iter()
        .filter(|node| matches!(node, DraftNode::While { .. } | DraftNode::Loop { .. }))
        .count();
    let mut draft_loops = HashMap::new();
    draft_loops
        .try_reserve(loop_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("draft loop context allocation failed")))?;
    let mut active_loops = Vec::new();
    active_loops
        .try_reserve(
            loop_count.checked_add(1).ok_or_else(|| {
                WorkspaceError::Host(Arc::from("draft loop context count overflow"))
            })?,
        )
        .map_err(|_| WorkspaceError::Host(Arc::from("draft loop context allocation failed")))?;
    if let Some(context) = published_control.enclosing_loop {
        active_loops.push(context);
    }
    let published_loop_depth = active_loops.len();

    for action in order {
        let node_index = match action {
            DraftLoweringAction::BeginMatch(node_index) => {
                let DraftNode::Match { scrutinee, arms } =
                    draft.nodes.get(node_index).ok_or_else(|| {
                        WorkspaceError::InvalidDraft(Arc::from("draft match identity is stale"))
                    })?
                else {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "draft match preparation targets a non-match node",
                    )));
                };
                if arms.is_empty() {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "match arms must not be empty",
                    )));
                }
                let scrutinee_type = scrutinee
                    .index()
                    .and_then(|index| completed.get(index))
                    .and_then(Option::as_ref)
                    .map(|expression| expression.ty.clone())
                    .ok_or_else(|| {
                        WorkspaceError::InvalidDraft(Arc::from(
                            "match scrutinee was not lowered before its patterns",
                        ))
                    })?;
                if !matches!(
                    scrutinee_type,
                    Type::Bool | Type::I64 | Type::Product(_) | Type::Enum { .. }
                ) {
                    return Err(WorkspaceError::unsupported(
                        "match",
                        "the source-free closed pattern surface supports Boolean, I64, product, and enum scrutinees",
                    ));
                }
                let scrutinee = allocate_workspace_match_local(
                    program,
                    root,
                    lowering,
                    &mut next_slot,
                    format!("$match{}", program.bindings.len()),
                    scrutinee_type,
                    BindingKind::MatchTemporary,
                    origin,
                )?;
                let mut prepared_arms = Vec::new();
                prepared_arms.try_reserve(arms.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("prepared match arm allocation failed"))
                })?;
                if prepared_matches
                    .insert(
                        node_index,
                        PreparedMatch {
                            scrutinee,
                            arms: prepared_arms,
                        },
                    )
                    .is_some()
                {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "draft match was prepared more than once",
                    )));
                }
                continue;
            }
            DraftLoweringAction::BeginArm { node, arm } => {
                let DraftNode::Match { arms, .. } = draft.nodes.get(node).ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("draft match identity is stale"))
                })?
                else {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "draft arm preparation targets a non-match node",
                    )));
                };
                let arm_draft = arms.get(arm).ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("draft match arm identity is stale"))
                })?;
                let scrutinee_type = prepared_matches
                    .get(&node)
                    .map(|prepared: &PreparedMatch| prepared.scrutinee.ty.clone())
                    .ok_or_else(|| {
                        WorkspaceError::InvalidDraft(Arc::from(
                            "draft match arm was prepared before its scrutinee",
                        ))
                    })?;
                let (pattern, bindings) = lower_pattern_draft(
                    snapshot,
                    program,
                    &arm_draft.pattern,
                    &scrutinee_type,
                    origin,
                    root,
                    lowering,
                    &mut next_slot,
                    &mut locals,
                    &mut entities,
                )?;
                let prepared = prepared_matches.get_mut(&node).ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("prepared draft match is missing"))
                })?;
                if prepared.arms.len() != arm {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "draft match arms were not prepared in semantic order",
                    )));
                }
                prepared.arms.push(PreparedMatchArm { pattern, bindings });
                continue;
            }
            DraftLoweringAction::EnterLoop(node_index) => {
                let node = draft.nodes.get(node_index).ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("draft loop identity is stale"))
                })?;
                let (result_type, is_while) = match node {
                    DraftNode::While { .. } => (Type::Unit, true),
                    DraftNode::Loop { result_type, .. } => (
                        super::types::resolve(
                            snapshot,
                            program,
                            result_type,
                            Some(callable),
                            false,
                            false,
                            "loop result type",
                        )?,
                        false,
                    ),
                    _ => {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(
                            "draft loop context targets a non-loop node",
                        )))
                    }
                };
                let context = crate::hir::LexicalLoopContext {
                    loop_id: lowering.loop_id()?,
                    result_type,
                    is_while,
                };
                if draft_loops.insert(node_index, context.clone()).is_some() {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "draft loop context was entered more than once",
                    )));
                }
                active_loops.push(context);
                continue;
            }
            DraftLoweringAction::ExitLoop(node_index) => {
                let expected = draft_loops.get(&node_index).ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from(
                        "draft loop context exited before entry",
                    ))
                })?;
                let actual = active_loops.pop().ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("draft loop context is missing"))
                })?;
                if actual.loop_id != expected.loop_id || active_loops.len() < published_loop_depth {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "draft loop contexts closed out of lexical order",
                    )));
                }
                continue;
            }
            DraftLoweringAction::Lower(node_index) => node_index,
        };
        let node = draft.nodes.get(node_index).ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from("draft lowering order is stale"))
        })?;
        let expression = match node {
            DraftNode::I64(value) => scalar(Type::I64, ExprKind::LitI64(*value), origin),
            DraftNode::F64(value) => scalar(Type::F64, ExprKind::LitF64(*value), origin),
            DraftNode::Bool(value) => scalar(Type::Bool, ExprKind::LitBool(*value), origin),
            DraftNode::Unit => scalar(Type::Unit, ExprKind::LitUnit, origin),
            DraftNode::Bytes(value) => scalar(
                Type::Bytes,
                ExprKind::LitBytes(try_clone_values(value, "bytes literal")?),
                origin,
            ),
            DraftNode::Load(reference) => {
                let resolved = resolve_draft_binding(
                    snapshot,
                    program,
                    *reference,
                    &visible_set,
                    &locals,
                    &published_locations,
                )?;
                if !crate::ownership::draft_parameter_load_is_supported(&resolved.ty) {
                    return Err(WorkspaceError::unsupported(
                        "load",
                        "affine values cannot be copied; use an explicit move",
                    ));
                }
                Expr {
                    ty: resolved.ty,
                    effects: EffectSet::PURE,
                    origin,
                    kind: ExprKind::Load(BindingRef {
                        binding: resolved.binding,
                        storage: BindingStorage::Local(resolved.slot),
                    }),
                }
            }
            DraftNode::Move(reference) => {
                let resolved = resolve_draft_binding(
                    snapshot,
                    program,
                    *reference,
                    &visible_set,
                    &locals,
                    &published_locations,
                )?;
                if !matches!(
                    resolved.ty,
                    Type::Bytes | Type::ByteVector | Type::Resource(_)
                ) {
                    return Err(WorkspaceError::unsupported(
                        "move",
                        "move requires affine bytes, byte-vector, or a typed resource",
                    ));
                }
                Expr {
                    ty: resolved.ty,
                    effects: EffectSet::PURE,
                    origin,
                    kind: ExprKind::Move {
                        place: resolved.place,
                        binding: BindingRef {
                            binding: resolved.binding,
                            storage: BindingStorage::Local(resolved.slot),
                        },
                    },
                }
            }
            DraftNode::BorrowShared(reference) => {
                let resolved = resolve_draft_binding(
                    snapshot,
                    program,
                    *reference,
                    &visible_set,
                    &locals,
                    &published_locations,
                )?;
                if resolved.ty != Type::ByteVector {
                    let owner = snapshot
                        .indexes
                        .address_entities
                        .get(&root)
                        .copied()
                        .ok_or_else(|| {
                            WorkspaceError::StaleIdentity(Arc::from("callable owner"))
                        })?;
                    return Err(WorkspaceError::TypeMismatch {
                        expected: Box::new(SemanticType::ByteVector),
                        actual: Box::new(super::types::view(
                            &snapshot.program,
                            &snapshot.indexes,
                            &resolved.ty,
                            Some(owner),
                        )?),
                    });
                }
                Expr {
                    ty: Type::ByteSlice,
                    effects: EffectSet::PURE,
                    origin,
                    kind: ExprKind::Borrow {
                        place: resolved.place,
                        loan: lowering.loan()?,
                        kind: crate::hir::BorrowKind::Shared,
                        binding: BindingRef {
                            binding: resolved.binding,
                            storage: BindingStorage::Local(resolved.slot),
                        },
                    },
                }
            }
            DraftNode::Call {
                callee,
                type_arguments,
                arguments,
            } => {
                let callee_header = snapshot.workspace_entity(*callee)?;
                if deleting_entities.contains(callee) {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "a newly lowered call cannot target a function deleted by the transaction",
                    )));
                }
                if !visible_set.contains(callee) {
                    return Err(invisible_entity("call", *callee));
                }
                if callee_header.kind != EntityKind::Function {
                    return Err(wrong_kind("call", "function", callee_header.kind));
                }
                let caller = snapshot
                    .indexes
                    .address_entities
                    .get(&root)
                    .copied()
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("callable owner")))?;
                let binding = binding_from_entity(snapshot, program, *callee)?;
                let declaration = program
                    .binding(binding)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function")))?;
                let function = program
                    .functions
                    .iter()
                    .find(|function| function.binding == binding)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function")))?;
                let variables = match &declaration.ty {
                    Type::Forall { vars, .. } => vars.as_slice(),
                    Type::Fn { .. } => &[][..],
                    _ => return Err(wrong_kind("call", "function", callee_header.kind)),
                };
                if variables.is_empty() && !type_arguments.is_empty() {
                    return Err(WorkspaceError::UnexpectedTypeArgument);
                }
                let mut supplied = HashMap::new();
                supplied.try_reserve(type_arguments.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("draft type-argument allocation failed"))
                })?;
                for argument in type_arguments {
                    if argument.parameter.namespace() != snapshot.namespace {
                        return Err(WorkspaceError::ForeignNamespace(Arc::from(
                            "type parameter",
                        )));
                    }
                    let parameter = snapshot
                        .workspace_entity(argument.parameter)
                        .map_err(|_| WorkspaceError::StaleIdentity(Arc::from("type parameter")))?;
                    if parameter.kind != EntityKind::TypeParameter {
                        return Err(wrong_kind(
                            "generic call type argument",
                            "type parameter",
                            parameter.kind,
                        ));
                    }
                    if parameter.owner != Some(*callee) {
                        return Err(WorkspaceError::WrongTypeParameterOwner {
                            parameter: Box::new(argument.parameter),
                            expected: Box::new(*callee),
                            actual: parameter.owner.map(Box::new),
                        });
                    }
                    let resolved = super::types::resolve(
                        snapshot,
                        program,
                        &argument.argument,
                        Some(caller),
                        false,
                        false,
                        "generic type argument",
                    )?;
                    if supplied.insert(argument.parameter, resolved).is_some() {
                        return Err(WorkspaceError::DuplicateTypeArgument {
                            parameter: argument.parameter,
                        });
                    }
                }
                let mut substitutions = Vec::new();
                substitutions.try_reserve(variables.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("generic substitution allocation failed"))
                })?;
                for variable in variables {
                    let parameter = snapshot
                        .indexes
                        .type_parameter_entities
                        .get(callee)
                        .and_then(|parameters| parameters.get(variable.as_str()))
                        .copied()
                        .ok_or_else(|| {
                            WorkspaceError::StaleIdentity(Arc::from("type parameter"))
                        })?;
                    let ty = supplied
                        .remove(&parameter)
                        .ok_or(WorkspaceError::MissingTypeArgument { parameter })?;
                    substitutions.push(crate::hir::TypeSubstitution {
                        parameter: variable.clone(),
                        ty,
                    });
                }
                if !supplied.is_empty() {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "generic call contains an undeclared type argument",
                    )));
                }
                let mut args = Vec::new();
                let mut argument_types = Vec::new();
                args.try_reserve(arguments.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("draft argument allocation failed"))
                })?;
                argument_types.try_reserve(arguments.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("draft argument type allocation failed"))
                })?;
                let mut effects = function.summary;
                for argument in arguments {
                    let value = take_draft_child(&mut completed, *argument)?;
                    argument_types.push(value.ty.clone());
                    effects = effects.union(value.effects);
                    args.push(value);
                }
                let facts = crate::generic_call::GenericFacts {
                    traits: &program.traits,
                    products: &program.products,
                    implementations: &program.implementations,
                    implementation_index: &lowering.implementation_index,
                };
                let exact = crate::generic_call::resolve_exact(
                    &declaration.ty,
                    substitutions,
                    &argument_types,
                    &function.bounds,
                    &facts,
                )
                .map_err(|error| {
                    generic_workspace_error(snapshot, program, caller, *callee, error)
                })?;
                debug_assert_eq!(exact.parameters.len(), args.len());
                let crate::generic_call::ExactCall {
                    parameters: _,
                    result,
                    instantiation,
                } = exact;
                Expr {
                    ty: result,
                    effects,
                    origin,
                    kind: ExprKind::Call {
                        callee: BindingRef {
                            binding,
                            storage: BindingStorage::Function,
                        },
                        args,
                        instantiation,
                    },
                }
            }
            DraftNode::Operation {
                operation,
                arguments,
            } => {
                if !operation.supports_direct_operation_expression() {
                    return Err(WorkspaceError::unsupported(
                        "operation",
                        "this canonical operation requires dedicated non-expression lowering",
                    ));
                }
                let mut args = Vec::new();
                let mut argument_types = Vec::new();
                args.try_reserve(arguments.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("operation argument allocation failed"))
                })?;
                argument_types.try_reserve(arguments.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("operation type allocation failed"))
                })?;
                let mut effects = operation.effects();
                for argument in arguments {
                    let value = take_draft_child(&mut completed, *argument)?;
                    argument_types.push(value.ty.clone());
                    effects = effects.union(value.effects);
                    args.push(value);
                }
                let (resolved_signature, ty) = operation
                    .resolve_types(&argument_types)
                    .map_err(|message| WorkspaceError::InvalidDraft(Arc::from(message)))?;
                Expr {
                    ty,
                    effects,
                    origin,
                    kind: ExprKind::Operation {
                        operation: *operation,
                        resolved_signature,
                        args,
                    },
                }
            }
            DraftNode::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = take_draft_child(&mut completed, *condition)?;
                require_type(&condition.ty, &Type::Bool)?;
                let then_branch = take_draft_child(&mut completed, *then_branch)?;
                let else_branch = take_draft_child(&mut completed, *else_branch)?;
                let ty = Type::join_control(&then_branch.ty, &else_branch.ty).ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from(
                        "draft if branches do not have a common type",
                    ))
                })?;
                let effects = condition
                    .effects
                    .union(then_branch.effects)
                    .union(else_branch.effects);
                Expr {
                    ty,
                    effects,
                    origin,
                    kind: ExprKind::If {
                        condition: Box::new(condition),
                        then_branch: Box::new(then_branch),
                        else_branch: Box::new(else_branch),
                    },
                }
            }
            DraftNode::Sequence(expressions) => {
                let mut values = Vec::new();
                values
                    .try_reserve(expressions.len())
                    .map_err(|_| WorkspaceError::Host(Arc::from("sequence allocation failed")))?;
                let mut effects = EffectSet::PURE;
                let mut ty = Type::Unit;
                for expression in expressions {
                    if ty == Type::Never {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(
                            "sequence contains an expression after a divergent expression",
                        )));
                    }
                    let value = take_draft_child(&mut completed, *expression)?;
                    ty = value.ty.clone();
                    effects = effects.union(value.effects);
                    values.push(value);
                }
                Expr {
                    ty,
                    effects,
                    origin,
                    kind: ExprKind::Do(values),
                }
            }
            DraftNode::Let { bindings, body } => {
                let mut definitions = Vec::new();
                definitions.try_reserve(bindings.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("let definition allocation failed"))
                })?;
                let mut effects = EffectSet::PURE;
                for binding in bindings {
                    let value = take_draft_child(&mut completed, binding.value)?;
                    let info = locals.get(&binding.binding).cloned().ok_or_else(|| {
                        WorkspaceError::InvalidDraft(Arc::from(
                            "draft local definition was not established by its initializer",
                        ))
                    })?;
                    require_type(&value.ty, &info.ty)?;
                    effects = effects.union(value.effects);
                    definitions.push(crate::hir::LocalDefinition {
                        binding: info.binding,
                        place: info.place,
                        static_bytes: info.kind == BindingKind::StaticBytesLocal,
                        slot: info.slot,
                        value,
                    });
                }
                let body = take_draft_child(&mut completed, *body)?;
                effects = effects.union(body.effects);
                let ty = body.ty.clone();
                for binding in bindings {
                    locals.remove(&binding.binding);
                }
                Expr {
                    ty,
                    effects,
                    origin,
                    kind: ExprKind::Let {
                        bindings: definitions,
                        body: Box::new(body),
                    },
                }
            }
            DraftNode::MutableLocal {
                binding,
                initial,
                body,
                ..
            } => {
                let initial = take_draft_child(&mut completed, *initial)?;
                let info = locals.get(binding).cloned().ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from(
                        "draft mutable local was not established after its initializer",
                    ))
                })?;
                require_type(&initial.ty, &info.ty)?;
                let body = take_draft_child(&mut completed, *body)?;
                let ty = body.ty.clone();
                let effects = initial.effects.union(body.effects);
                locals.remove(binding);
                Expr {
                    ty,
                    effects,
                    origin,
                    kind: ExprKind::MutableLocal {
                        binding: info.binding,
                        place: info.place,
                        slot: info.slot,
                        initial: Box::new(initial),
                        body: Box::new(body),
                    },
                }
            }
            DraftNode::SetLocal { target, value } => {
                let target = resolve_assignment_target(
                    snapshot,
                    program,
                    *target,
                    &visible_set,
                    &locals,
                    &published_locations,
                )?;
                let value = take_draft_child(&mut completed, *value)?;
                if value.ty == Type::Never {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "divergent value cannot fill a mutable local storage slot",
                    )));
                }
                require_workspace_type(snapshot, program, callable, &value.ty, &target.ty)?;
                Expr {
                    ty: Type::Unit,
                    effects: value.effects.union(EffectSet::MUTATES_LOCAL),
                    origin,
                    kind: ExprKind::SetLocal {
                        target: target.binding,
                        slot: target.slot,
                        value: Box::new(value),
                    },
                }
            }
            DraftNode::While { condition, body } => {
                let context = draft_loops.remove(&node_index).ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("draft while context is missing"))
                })?;
                if !context.is_while || context.result_type != Type::Unit {
                    return Err(WorkspaceError::Validation(Arc::from(
                        "draft while context has stale type facts",
                    )));
                }
                let condition = take_draft_child(&mut completed, *condition)?;
                require_workspace_type(snapshot, program, callable, &condition.ty, &Type::Bool)?;
                let mut values = Vec::new();
                values
                    .try_reserve(body.len())
                    .map_err(|_| WorkspaceError::Host(Arc::from("while body allocation failed")))?;
                let mut effects = condition.effects.union(EffectSet::MAY_DIVERGE);
                let mut previous = Type::Unit;
                for expression in body {
                    if previous == Type::Never {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(
                            "while body contains an expression after a divergent expression",
                        )));
                    }
                    let value = take_draft_child(&mut completed, *expression)?;
                    previous = value.ty.clone();
                    effects = effects.union(value.effects);
                    values.push(value);
                }
                Expr {
                    ty: Type::Unit,
                    effects,
                    origin,
                    kind: ExprKind::While {
                        loop_id: context.loop_id,
                        condition: Box::new(condition),
                        body: values,
                    },
                }
            }
            DraftNode::Loop { body, .. } => {
                let context = draft_loops.remove(&node_index).ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("draft typed-loop context is missing"))
                })?;
                if context.is_while {
                    return Err(WorkspaceError::Validation(Arc::from(
                        "draft typed-loop context has stale kind facts",
                    )));
                }
                let mut values = Vec::new();
                values.try_reserve(body.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("typed-loop body allocation failed"))
                })?;
                let mut effects = EffectSet::MAY_DIVERGE;
                let mut previous = Type::Unit;
                for expression in body {
                    if previous == Type::Never {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(
                            "typed-loop body contains an expression after a divergent expression",
                        )));
                    }
                    let value = take_draft_child(&mut completed, *expression)?;
                    previous = value.ty.clone();
                    effects = effects.union(value.effects);
                    values.push(value);
                }
                Expr {
                    ty: context.result_type.clone(),
                    effects,
                    origin,
                    kind: ExprKind::Loop {
                        loop_id: context.loop_id,
                        result_type: context.result_type,
                        body: values,
                    },
                }
            }
            DraftNode::Return { value } => {
                let value = take_draft_child(&mut completed, *value)?;
                if value.ty == Type::Never {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "return value is already divergent",
                    )));
                }
                require_workspace_type(
                    snapshot,
                    program,
                    callable,
                    &value.ty,
                    &declared_result_type,
                )?;
                let effects = value.effects.union(EffectSet::MAY_DIVERGE);
                Expr {
                    ty: Type::Never,
                    effects,
                    origin,
                    kind: ExprKind::Return {
                        value: Box::new(value),
                    },
                }
            }
            DraftNode::Break { value } => {
                let target = active_loops.last().cloned().ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from(
                        "break is only valid inside a lexical loop",
                    ))
                })?;
                let value = take_draft_child(&mut completed, *value)?;
                if value.ty == Type::Never {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "break value is already divergent",
                    )));
                }
                require_workspace_type(
                    snapshot,
                    program,
                    callable,
                    &value.ty,
                    &target.result_type,
                )?;
                if target.is_while && target.result_type != Type::Unit {
                    return Err(WorkspaceError::Validation(Arc::from(
                        "draft while break context is not unit-typed",
                    )));
                }
                let effects = value.effects.union(EffectSet::MAY_DIVERGE);
                Expr {
                    ty: Type::Never,
                    effects,
                    origin,
                    kind: ExprKind::Break {
                        loop_id: target.loop_id,
                        value: Box::new(value),
                    },
                }
            }
            DraftNode::Continue => {
                let target = active_loops.last().ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from(
                        "continue is only valid inside a lexical loop",
                    ))
                })?;
                Expr {
                    ty: Type::Never,
                    effects: EffectSet::MAY_DIVERGE,
                    origin,
                    kind: ExprKind::Continue {
                        loop_id: target.loop_id,
                    },
                }
            }
            DraftNode::ProductValue { product, fields } => {
                lower_product_value(snapshot, program, *product, fields, &mut completed, origin)?
            }
            DraftNode::ProductField { field, value } => {
                lower_product_field(snapshot, program, *field, *value, &mut completed, origin)?
            }
            DraftNode::EnumValue {
                variant,
                type_arguments,
                fields,
            } => lower_enum_value(
                snapshot,
                program,
                callable,
                *variant,
                type_arguments,
                fields,
                &mut completed,
                origin,
            )?,
            DraftNode::EnumIsVariant { variant, value } => {
                lower_enum_is_variant(snapshot, program, *variant, *value, &mut completed, origin)?
            }
            DraftNode::Match { scrutinee, arms } => lower_prepared_match(
                program,
                node_index,
                *scrutinee,
                arms,
                origin,
                &mut completed,
                &mut prepared_matches,
                &mut locals,
            )?,
        };
        let slot = completed.get_mut(node_index).ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from("draft lowering slot is stale"))
        })?;
        if slot.replace(expression).is_some() {
            return Err(WorkspaceError::InvalidDraft(Arc::from(
                "draft node was lowered more than once",
            )));
        }

        if let Some(events) = definition_events.remove(&node_index) {
            for event in events {
                let initializer = completed
                    .get(node_index)
                    .and_then(Option::as_ref)
                    .ok_or_else(|| {
                        WorkspaceError::InvalidDraft(Arc::from("local initializer is missing"))
                    })?;
                if initializer.ty == Type::Never {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "divergent expression cannot initialize a local",
                    )));
                }
                let (ty, kind, entity_kind) = if let Some(declared) = event.mutable_type.as_ref() {
                    let ty = super::types::resolve(
                        snapshot,
                        program,
                        declared,
                        Some(callable),
                        false,
                        false,
                        "mutable local type",
                    )?;
                    if let Some(reason) = crate::ownership::mutable_local_storage_restriction(&ty) {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(reason)));
                    }
                    require_workspace_type(snapshot, program, callable, &initializer.ty, &ty)?;
                    (ty, BindingKind::MutableLocal, EntityKind::MutableLocal)
                } else {
                    let static_bytes = matches!(initializer.kind, ExprKind::LitBytes(_))
                        || matches!(
                            initializer.kind,
                            ExprKind::Load(reference)
                                if program.binding(reference.binding).is_some_and(|item| item.kind == BindingKind::StaticBytesLocal)
                        );
                    (
                        initializer.ty.clone(),
                        if static_bytes {
                            BindingKind::StaticBytesLocal
                        } else {
                            BindingKind::ImmutableLocal
                        },
                        if static_bytes {
                            EntityKind::StaticBytesLocal
                        } else {
                            EntityKind::ImmutableLocal
                        },
                    )
                };
                let raw = u64::try_from(program.bindings.len())
                    .map_err(|_| WorkspaceError::Host(Arc::from("binding identity exceeds u64")))?;
                let hir_binding = crate::hir::BindingId::new(raw);
                let info = ResolvedDraftBinding {
                    binding: hir_binding,
                    slot: next_slot,
                    place: lowering.place(program, root)?,
                    ty: ty.clone(),
                    kind: kind.clone(),
                };
                next_slot = next_slot
                    .checked_add(1)
                    .ok_or_else(|| WorkspaceError::Host(Arc::from("local slot count overflow")))?;
                program.bindings.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("local binding allocation failed"))
                })?;
                program.bindings.push(Binding {
                    id: hir_binding,
                    name: event.name.clone(),
                    kind,
                    ty,
                    origin,
                });
                if locals.insert(event.binding, info).is_some() {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "draft binding handle is defined more than once",
                    )));
                }
                entities.push(NewEntity {
                    address: EntityAddress::Binding(raw),
                    kind: entity_kind,
                    name: Arc::from(event.name),
                });
            }
        }
    }
    if !definition_events.is_empty()
        || !locals.is_empty()
        || !prepared_matches.is_empty()
        || !draft_loops.is_empty()
        || active_loops.len() != published_loop_depth
    {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "draft lexical scope did not close deterministically",
        )));
    }
    let root_expression = draft
        .root
        .index()
        .and_then(|index| completed.get_mut(index))
        .and_then(Option::take)
        .ok_or_else(|| WorkspaceError::InvalidDraft(Arc::from("draft root is unavailable")))?;
    if root_expression.ty != Type::Never && !Type::unify_assignable(&root_expression.ty, expected) {
        let owner = snapshot
            .indexes
            .address_entities
            .get(&root)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("callable owner")))?;
        return Err(WorkspaceError::TypeMismatch {
            expected: Box::new(super::types::view(
                program,
                &snapshot.indexes,
                expected,
                Some(owner),
            )?),
            actual: Box::new(super::types::view(
                program,
                &snapshot.indexes,
                &root_expression.ty,
                Some(owner),
            )?),
        });
    }
    set_root_local_count(program, root, next_slot)?;
    Ok(LoweredDraft {
        expression: root_expression,
        entities,
    })
}

#[allow(clippy::too_many_arguments)]
fn lower_prepared_match(
    program: &mut SemanticProgram,
    node_index: usize,
    scrutinee: DraftNodeId,
    arms: &[super::MatchArmDraft],
    origin: Origin,
    completed: &mut [Option<Expr>],
    prepared_matches: &mut HashMap<usize, PreparedMatch>,
    locals: &mut HashMap<DraftBindingId, ResolvedDraftBinding>,
) -> Result<Expr, WorkspaceError> {
    let prepared = prepared_matches
        .remove(&node_index)
        .ok_or_else(|| WorkspaceError::InvalidDraft(Arc::from("draft match was not prepared")))?;
    if prepared.arms.len() != arms.len() {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "draft match arm preparation is incomplete",
        )));
    }
    let scrutinee_value = take_draft_child(completed, scrutinee)?;
    let mut bodies = Vec::new();
    bodies
        .try_reserve(arms.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("match body allocation failed")))?;
    let mut planned = Vec::new();
    planned
        .try_reserve(arms.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("planned match arm allocation failed")))?;
    for (index, (arm, prepared_arm)) in arms.iter().zip(prepared.arms).enumerate() {
        let body = take_draft_child(completed, arm.body)?;
        let id = u64::try_from(index)
            .map_err(|_| WorkspaceError::Host(Arc::from("match arm identity exceeds u64")))?;
        planned.push(crate::hir::PlannedMatchArm {
            id,
            pattern: prepared_arm.pattern,
            body_type: body.ty.clone(),
        });
        for binding in prepared_arm.bindings {
            locals.remove(&binding);
        }
        bodies.push(body);
    }
    let plan_id = crate::hir::MatchPlanId::new(
        u64::try_from(program.match_plans.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("match plan identity exceeds u64")))?,
    );
    let plan = crate::analyze::build_match_plan(
        plan_id,
        origin,
        prepared.scrutinee,
        planned,
        &program.enums,
        &program.products,
    )
    .map_err(match_build_error)?;
    let ty = plan.result_type.clone();
    let effects = scrutinee_value.effects.union(
        bodies
            .iter()
            .fold(EffectSet::PURE, |effects, body| effects.union(body.effects)),
    );
    program
        .match_plans
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("match plan allocation failed")))?;
    program.match_plans.push(plan);
    Ok(Expr {
        ty,
        effects,
        origin,
        kind: ExprKind::Match {
            plan: plan_id,
            scrutinee: Box::new(scrutinee_value),
            arms: bodies,
        },
    })
}

enum ResolvedPatternAggregate {
    EnumVariant {
        ty: Type,
        enum_id: crate::hir::EnumId,
        variant: crate::hir::VariantId,
        layout: crate::hir::RuntimeLayoutId,
        fields: Vec<ResolvedPatternField>,
    },
    Product {
        ty: Type,
        product: crate::hir::ProductId,
        fields: Vec<ResolvedPatternField>,
    },
}

impl ResolvedPatternAggregate {
    fn fields(&self) -> &[ResolvedPatternField] {
        match self {
            Self::EnumVariant { fields, .. } | Self::Product { fields, .. } => fields,
        }
    }

    fn fields_mut(&mut self) -> &mut [ResolvedPatternField] {
        match self {
            Self::EnumVariant { fields, .. } | Self::Product { fields, .. } => fields,
        }
    }
}

struct ResolvedPatternField {
    field_index: u64,
    ty: Type,
    projection: Option<crate::hir::MatchLocal>,
    pattern: DraftPatternNodeId,
}

#[allow(clippy::too_many_arguments)]
fn lower_pattern_draft(
    snapshot: &WorkspaceSnapshot,
    program: &mut SemanticProgram,
    draft: &PatternDraft,
    expected: &Type,
    origin: Origin,
    root: EntityAddress,
    lowering: &mut LoweringState,
    next_slot: &mut usize,
    locals: &mut HashMap<DraftBindingId, ResolvedDraftBinding>,
    entities: &mut Vec<NewEntity>,
) -> Result<(crate::hir::MatchPattern, Vec<DraftBindingId>), WorkspaceError> {
    validate_pattern_shape(draft)?;
    let mut expected_types = Vec::new();
    expected_types
        .try_reserve(draft.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("pattern type allocation failed")))?;
    expected_types.resize_with(draft.nodes.len(), || None);
    let root_index = draft.root.index().ok_or_else(|| {
        WorkspaceError::InvalidDraft(Arc::from("pattern root exceeds host index"))
    })?;
    expected_types[root_index] = Some(expected.clone());
    let mut aggregates = Vec::new();
    aggregates
        .try_reserve(draft.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("pattern metadata allocation failed")))?;
    aggregates.resize_with(draft.nodes.len(), || None);
    let mut pending = Vec::new();
    pending
        .try_reserve(draft.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("pattern type work allocation failed")))?;
    pending.push(draft.root);
    while let Some(id) = pending.pop() {
        let index = id.index().ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from("pattern node exceeds host index"))
        })?;
        let node = draft.nodes.get(index).ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from("pattern node identity is stale"))
        })?;
        let ty = expected_types
            .get(index)
            .and_then(Option::as_ref)
            .cloned()
            .ok_or_else(|| {
                WorkspaceError::InvalidDraft(Arc::from("pattern node has no expected type"))
            })?;
        let aggregate = match node {
            DraftPatternNode::Wildcard | DraftPatternNode::Binding { .. } => None,
            DraftPatternNode::Bool(_) => {
                require_pattern_type(&ty, &Type::Bool, "Boolean literal")?;
                None
            }
            DraftPatternNode::I64(_) => {
                require_pattern_type(&ty, &Type::I64, "I64 literal")?;
                None
            }
            DraftPatternNode::Product { product, fields } => {
                let header = snapshot.workspace_entity(*product)?;
                if header.kind != EntityKind::Product {
                    return Err(wrong_kind(
                        "product match pattern",
                        "product declaration",
                        header.kind,
                    ));
                }
                let EntityAddress::Product(product_index) = entity_address(snapshot, *product)?
                else {
                    return Err(WorkspaceError::StaleIdentity(Arc::from("product")));
                };
                let definition = program
                    .products
                    .get(host_index(product_index, "product")?)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product")))?;
                require_pattern_type(&ty, &Type::Product(definition.id), "product")?;
                if fields.len() != definition.fields.len() {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "product pattern must provide exactly one nested pattern per field",
                    )));
                }
                let mut ordered = Vec::new();
                ordered.try_reserve(definition.fields.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("product pattern field allocation failed"))
                })?;
                ordered.resize_with(definition.fields.len(), || None);
                for field in fields {
                    let field_header = snapshot.workspace_entity(field.field)?;
                    if field_header.kind != EntityKind::ProductField {
                        return Err(wrong_kind(
                            "product match pattern field",
                            "product field",
                            field_header.kind,
                        ));
                    }
                    let EntityAddress::ProductField {
                        product: field_product,
                        field: field_index,
                    } = entity_address(snapshot, field.field)?
                    else {
                        return Err(WorkspaceError::StaleIdentity(Arc::from("product field")));
                    };
                    if field_product != product_index {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(
                            "product pattern field belongs to a different product",
                        )));
                    }
                    let field_index = host_index(field_index, "product field")?;
                    definition
                        .fields
                        .get(field_index)
                        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product field")))?;
                    let slot = ordered
                        .get_mut(field_index)
                        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product field")))?;
                    if slot.replace(field.pattern).is_some() {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(
                            "product pattern field is duplicated",
                        )));
                    }
                }
                let mut resolved_fields = Vec::new();
                resolved_fields
                    .try_reserve(definition.fields.len())
                    .map_err(|_| {
                        WorkspaceError::Host(Arc::from(
                            "resolved product pattern field allocation failed",
                        ))
                    })?;
                for (declared, pattern) in definition.fields.iter().zip(ordered) {
                    let pattern = pattern.ok_or_else(|| {
                        WorkspaceError::InvalidDraft(Arc::from("product pattern field is missing"))
                    })?;
                    resolved_fields.push(ResolvedPatternField {
                        field_index: declared.source_order,
                        ty: declared.ty.clone(),
                        projection: None,
                        pattern,
                    });
                }
                Some(ResolvedPatternAggregate::Product {
                    ty,
                    product: definition.id,
                    fields: resolved_fields,
                })
            }
            DraftPatternNode::EnumVariant { variant, fields } => {
                let header = snapshot.workspace_entity(*variant)?;
                if header.kind != EntityKind::EnumVariant {
                    return Err(wrong_kind(
                        "enum match pattern",
                        "enum variant",
                        header.kind,
                    ));
                }
                let EntityAddress::EnumVariant {
                    enumeration,
                    variant: variant_index,
                } = entity_address(snapshot, *variant)?
                else {
                    return Err(WorkspaceError::StaleIdentity(Arc::from("enum variant")));
                };
                let definition = program
                    .enums
                    .get(host_index(enumeration, "enum")?)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum")))?;
                let Type::Enum { id, arguments } = &ty else {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(format!(
                        "enum variant pattern requires an enum type, got {ty}"
                    ))));
                };
                if *id != definition.id {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "enum pattern variant belongs to a different enum than the expected type",
                    )));
                }
                validate_concrete_enum_arguments(arguments, "match-pattern")?;
                let substitutions = enum_substitution_map(definition, arguments, "enum pattern")?;
                let selected = definition
                    .variants
                    .get(host_index(variant_index, "enum variant")?)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum variant")))?;
                if fields.len() != selected.fields.len() {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "enum pattern must provide exactly one nested pattern per field",
                    )));
                }
                let mut ordered = Vec::new();
                ordered.try_reserve(selected.fields.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("enum pattern field allocation failed"))
                })?;
                ordered.resize_with(selected.fields.len(), || None);
                for field in fields {
                    let field_header = snapshot.workspace_entity(field.field)?;
                    if field_header.kind != EntityKind::EnumField {
                        return Err(wrong_kind(
                            "enum match pattern field",
                            "enum field",
                            field_header.kind,
                        ));
                    }
                    let EntityAddress::EnumField {
                        enumeration: field_enum,
                        variant: field_variant,
                        field: field_index,
                    } = entity_address(snapshot, field.field)?
                    else {
                        return Err(WorkspaceError::StaleIdentity(Arc::from("enum field")));
                    };
                    if field_enum != enumeration || field_variant != variant_index {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(
                            "enum pattern field belongs to a different variant",
                        )));
                    }
                    let field_index = host_index(field_index, "enum field")?;
                    selected
                        .fields
                        .get(field_index)
                        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum field")))?;
                    let slot = ordered
                        .get_mut(field_index)
                        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum field")))?;
                    if slot.replace(field.pattern).is_some() {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(
                            "enum pattern field is duplicated",
                        )));
                    }
                }
                let mut resolved_fields = Vec::new();
                resolved_fields
                    .try_reserve(selected.fields.len())
                    .map_err(|_| {
                        WorkspaceError::Host(Arc::from(
                            "resolved enum pattern field allocation failed",
                        ))
                    })?;
                for (declared, pattern) in selected.fields.iter().zip(ordered) {
                    let pattern = pattern.ok_or_else(|| {
                        WorkspaceError::InvalidDraft(Arc::from("enum pattern field is missing"))
                    })?;
                    resolved_fields.push(ResolvedPatternField {
                        field_index: declared.source_order,
                        ty: substitute_enum_field_type(
                            &declared.ty,
                            &substitutions,
                            "enum pattern field",
                        )?,
                        projection: None,
                        pattern,
                    });
                }
                Some(ResolvedPatternAggregate::EnumVariant {
                    ty,
                    enum_id: definition.id,
                    variant: selected.id,
                    layout: definition.layout.identity,
                    fields: resolved_fields,
                })
            }
        };
        if let Some(mut aggregate) = aggregate {
            for field in aggregate.fields().iter().rev() {
                let child = field.pattern.index().ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("pattern child exceeds host index"))
                })?;
                let expected_slot = expected_types.get_mut(child).ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("pattern child identity is stale"))
                })?;
                if expected_slot.replace(field.ty.clone()).is_some() {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "pattern child receives more than one expected type",
                    )));
                }
                pending.push(field.pattern);
            }
            for field in aggregate.fields_mut() {
                let child = field.pattern.index().ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("pattern child exceeds host index"))
                })?;
                if !matches!(draft.nodes.get(child), Some(DraftPatternNode::Wildcard)) {
                    let name = format!("$match{}", program.bindings.len());
                    field.projection = Some(allocate_workspace_match_local(
                        program,
                        root,
                        lowering,
                        next_slot,
                        name,
                        field.ty.clone(),
                        BindingKind::MatchTemporary,
                        origin,
                    )?);
                }
            }
            aggregates[index] = Some(aggregate);
        }
    }

    let order = pattern_postorder(draft, &aggregates)?;
    let mut completed = Vec::new();
    completed
        .try_reserve(draft.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("pattern lowering allocation failed")))?;
    completed.resize_with(draft.nodes.len(), || None);
    let mut bindings = Vec::new();
    bindings
        .try_reserve(draft.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("pattern binding allocation failed")))?;
    for index in order {
        #[cfg(test)]
        PATTERN_LOWERING_NODE_VISITS.with(|count| {
            count.set(count.get().saturating_add(1));
        });
        let ty = expected_types[index].clone().ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from("pattern node has no expected type"))
        })?;
        let lowered = match &draft.nodes[index] {
            DraftPatternNode::Wildcard => crate::hir::MatchPattern::Wildcard { ty },
            DraftPatternNode::Binding { binding, name } => {
                let local = allocate_workspace_match_local(
                    program,
                    root,
                    lowering,
                    next_slot,
                    name.clone(),
                    ty.clone(),
                    BindingKind::ImmutableLocal,
                    origin,
                )?;
                let info = ResolvedDraftBinding {
                    binding: local.binding,
                    slot: local.slot,
                    place: local.place,
                    ty,
                    kind: BindingKind::ImmutableLocal,
                };
                if locals.insert(*binding, info).is_some() {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "pattern binding handle is defined more than once",
                    )));
                }
                entities.push(NewEntity {
                    address: EntityAddress::Binding(local.binding.raw()),
                    kind: EntityKind::ImmutableLocal,
                    name: Arc::from(name.as_str()),
                });
                bindings.push(*binding);
                crate::hir::MatchPattern::Binding { local }
            }
            DraftPatternNode::Bool(value) => crate::hir::MatchPattern::Bool(*value),
            DraftPatternNode::I64(value) => crate::hir::MatchPattern::I64(*value),
            DraftPatternNode::Product { .. } | DraftPatternNode::EnumVariant { .. } => {
                let aggregate = aggregates[index].take().ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("aggregate pattern metadata is missing"))
                })?;
                match aggregate {
                    ResolvedPatternAggregate::EnumVariant {
                        ty,
                        enum_id,
                        variant,
                        layout,
                        fields,
                    } => crate::hir::MatchPattern::Variant {
                        ty,
                        enum_id,
                        variant,
                        layout,
                        fields: lower_resolved_pattern_fields(fields, &mut completed)?,
                    },
                    ResolvedPatternAggregate::Product {
                        ty,
                        product,
                        fields,
                    } => crate::hir::MatchPattern::Product {
                        ty,
                        product,
                        fields: lower_resolved_pattern_fields(fields, &mut completed)?,
                    },
                }
            }
        };
        let slot = completed.get_mut(index).ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from("pattern lowering slot is stale"))
        })?;
        if slot.replace(lowered).is_some() {
            return Err(WorkspaceError::InvalidDraft(Arc::from(
                "pattern node was lowered more than once",
            )));
        }
    }
    let root = completed
        .get_mut(root_index)
        .and_then(Option::take)
        .ok_or_else(|| WorkspaceError::InvalidDraft(Arc::from("pattern root is unavailable")))?;
    Ok((root, bindings))
}

fn lower_resolved_pattern_fields(
    resolved: Vec<ResolvedPatternField>,
    completed: &mut [Option<crate::hir::MatchPattern>],
) -> Result<Vec<crate::hir::MatchFieldPattern>, WorkspaceError> {
    let mut fields = Vec::new();
    fields
        .try_reserve(resolved.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("match pattern field allocation failed")))?;
    for field in resolved {
        let child = field.pattern.index().ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from("pattern child exceeds host index"))
        })?;
        let pattern = completed
            .get_mut(child)
            .and_then(Option::take)
            .ok_or_else(|| {
                WorkspaceError::InvalidDraft(Arc::from("pattern child is stale or reused"))
            })?;
        let projection = field.projection;
        if projection.is_none() != matches!(pattern, crate::hir::MatchPattern::Wildcard { .. }) {
            return Err(WorkspaceError::InvalidDraft(Arc::from(
                "pattern projection metadata is stale",
            )));
        }
        fields.push(crate::hir::MatchFieldPattern {
            field_index: field.field_index,
            projection,
            pattern,
        });
    }
    Ok(fields)
}

fn require_pattern_type(
    actual: &Type,
    expected: &Type,
    subject: &str,
) -> Result<(), WorkspaceError> {
    if actual == expected {
        Ok(())
    } else {
        Err(WorkspaceError::InvalidDraft(Arc::from(format!(
            "{subject} pattern requires type {expected}, got {actual}"
        ))))
    }
}

fn pattern_postorder(
    draft: &PatternDraft,
    aggregates: &[Option<ResolvedPatternAggregate>],
) -> Result<Vec<usize>, WorkspaceError> {
    enum Work {
        Visit(DraftPatternNodeId),
        Finish(usize),
    }
    let mut work = Vec::new();
    work.try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("pattern order allocation failed")))?;
    work.push(Work::Visit(draft.root));
    let mut order = Vec::new();
    order
        .try_reserve(draft.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("pattern order allocation failed")))?;
    while let Some(item) = work.pop() {
        match item {
            Work::Finish(index) => order.push(index),
            Work::Visit(id) => {
                let index = id.index().ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("pattern node exceeds host index"))
                })?;
                let node = draft.nodes.get(index).ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("pattern node identity is stale"))
                })?;
                let fields = match node {
                    DraftPatternNode::Product { .. } | DraftPatternNode::EnumVariant { .. } => {
                        Some(
                            aggregates
                                .get(index)
                                .and_then(Option::as_ref)
                                .ok_or_else(|| {
                                    WorkspaceError::InvalidDraft(Arc::from(
                                        "aggregate pattern metadata is missing",
                                    ))
                                })?
                                .fields(),
                        )
                    }
                    _ => None,
                };
                let child_count = fields.map_or(0, <[ResolvedPatternField]>::len);
                let additional = child_count.checked_add(1).ok_or_else(|| {
                    WorkspaceError::Host(Arc::from("pattern order work overflow"))
                })?;
                work.try_reserve(additional).map_err(|_| {
                    WorkspaceError::Host(Arc::from("pattern order work allocation failed"))
                })?;
                work.push(Work::Finish(index));
                if let Some(fields) = fields {
                    work.extend(fields.iter().rev().map(|field| Work::Visit(field.pattern)));
                }
            }
        }
    }
    if order.len() == draft.nodes.len() {
        Ok(order)
    } else {
        Err(WorkspaceError::InvalidDraft(Arc::from(
            "pattern traversal did not cover every node",
        )))
    }
}

#[allow(clippy::too_many_arguments)]
fn allocate_workspace_match_local(
    program: &mut SemanticProgram,
    root: EntityAddress,
    lowering: &mut LoweringState,
    next_slot: &mut usize,
    name: String,
    ty: Type,
    kind: BindingKind,
    origin: Origin,
) -> Result<crate::hir::MatchLocal, WorkspaceError> {
    let raw = u64::try_from(program.bindings.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("binding identity exceeds u64")))?;
    let binding = crate::hir::BindingId::new(raw);
    let slot = *next_slot;
    *next_slot = next_slot
        .checked_add(1)
        .ok_or_else(|| WorkspaceError::Host(Arc::from("match local slot count overflow")))?;
    let place = lowering.place(program, root)?;
    program
        .bindings
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("match local allocation failed")))?;
    program.bindings.push(Binding {
        id: binding,
        name,
        kind,
        ty: ty.clone(),
        origin,
    });
    Ok(crate::hir::MatchLocal {
        binding,
        place,
        slot,
        ty,
    })
}

fn match_build_error(error: lkjscript_core::Error) -> WorkspaceError {
    if matches!(error.class(), lkjscript_core::ErrorClass::Host) {
        WorkspaceError::from_core(error)
    } else {
        WorkspaceError::InvalidDraft(Arc::from(error.as_str()))
    }
}

fn draft_definition_events(
    draft: &ExpressionDraft,
) -> Result<HashMap<usize, Vec<DraftDefinitionEvent>>, WorkspaceError> {
    let mut events = HashMap::new();
    let mut handles = HashSet::new();
    for node in &draft.nodes {
        match node {
            DraftNode::Let { bindings, .. } => {
                let mut names = HashSet::new();
                names
                    .try_reserve(bindings.len())
                    .map_err(|_| WorkspaceError::Host(Arc::from("let name allocation failed")))?;
                for binding in bindings {
                    if !lkjscript_contracts::is_identifier(&binding.name) {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(
                            "local name must be a non-empty semantic identifier",
                        )));
                    }
                    if !names.insert(binding.name.as_str()) {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(
                            "local name is duplicated in one lexical scope",
                        )));
                    }
                    if !handles.insert(binding.binding) {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(
                            "draft binding handle is defined more than once",
                        )));
                    }
                    let initializer = binding.value.index().ok_or_else(|| {
                        WorkspaceError::InvalidDraft(Arc::from(
                            "local initializer exceeds host index",
                        ))
                    })?;
                    events
                        .entry(initializer)
                        .or_insert_with(Vec::new)
                        .push(DraftDefinitionEvent {
                            binding: binding.binding,
                            name: binding.name.clone(),
                            mutable_type: None,
                        });
                }
            }
            DraftNode::MutableLocal {
                binding,
                name,
                ty,
                initial,
                ..
            } => {
                if !lkjscript_contracts::is_identifier(name) {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "mutable local name must be a non-empty semantic identifier",
                    )));
                }
                if !handles.insert(*binding) {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "draft binding handle is defined more than once",
                    )));
                }
                let initializer = initial.index().ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from(
                        "mutable local initializer exceeds host index",
                    ))
                })?;
                events
                    .entry(initializer)
                    .or_insert_with(Vec::new)
                    .push(DraftDefinitionEvent {
                        binding: *binding,
                        name: name.clone(),
                        mutable_type: Some(ty.clone()),
                    });
            }
            DraftNode::Match { arms, .. } => {
                for arm in arms {
                    let mut names = HashSet::new();
                    names.try_reserve(arm.pattern.nodes.len()).map_err(|_| {
                        WorkspaceError::Host(Arc::from("pattern binding name allocation failed"))
                    })?;
                    for pattern in &arm.pattern.nodes {
                        let DraftPatternNode::Binding { binding, name } = pattern else {
                            continue;
                        };
                        if !lkjscript_contracts::is_identifier(name) {
                            return Err(WorkspaceError::InvalidDraft(Arc::from(
                                "pattern binding name must be a semantic identifier",
                            )));
                        }
                        if !names.insert(name.as_str()) {
                            return Err(WorkspaceError::InvalidDraft(Arc::from(
                                "pattern binding name is duplicated in one arm",
                            )));
                        }
                        if !handles.insert(*binding) {
                            return Err(WorkspaceError::InvalidDraft(Arc::from(
                                "draft binding handle is defined more than once",
                            )));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(events)
}

fn draft_lowering_actions(
    draft: &ExpressionDraft,
) -> Result<Vec<DraftLoweringAction>, WorkspaceError> {
    enum Work {
        Visit(DraftNodeId),
        BeginMatch(usize),
        BeginArm { node: usize, arm: usize },
        EnterLoop(usize),
        ExitLoop(usize),
        Finish(usize),
    }

    let mut work = Vec::new();
    work.try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("draft order allocation failed")))?;
    work.push(Work::Visit(draft.root));
    let mut order = Vec::new();
    let action_capacity =
        draft
            .nodes
            .iter()
            .try_fold(draft.nodes.len(), |count, node| match node {
                DraftNode::Match { arms, .. } => count
                    .checked_add(arms.len())
                    .and_then(|value| value.checked_add(1))
                    .ok_or_else(|| WorkspaceError::Host(Arc::from("draft action count overflow"))),
                DraftNode::While { .. } | DraftNode::Loop { .. } => count
                    .checked_add(2)
                    .ok_or_else(|| WorkspaceError::Host(Arc::from("draft action count overflow"))),
                _ => Ok(count),
            })?;
    order
        .try_reserve(action_capacity)
        .map_err(|_| WorkspaceError::Host(Arc::from("draft order allocation failed")))?;
    while let Some(item) = work.pop() {
        match item {
            Work::BeginMatch(index) => order.push(DraftLoweringAction::BeginMatch(index)),
            Work::BeginArm { node, arm } => {
                order.push(DraftLoweringAction::BeginArm { node, arm });
            }
            Work::EnterLoop(index) => order.push(DraftLoweringAction::EnterLoop(index)),
            Work::ExitLoop(index) => order.push(DraftLoweringAction::ExitLoop(index)),
            Work::Finish(index) => order.push(DraftLoweringAction::Lower(index)),
            Work::Visit(id) => {
                #[cfg(test)]
                DRAFT_LOWERING_NODE_VISITS.with(|count| {
                    count.set(count.get().saturating_add(1));
                });
                let index = id.index().ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("draft node exceeds host index"))
                })?;
                let node = draft.nodes.get(index).ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("draft node identity is stale"))
                })?;
                if let DraftNode::Match { scrutinee, arms } = node {
                    let additional = arms
                        .len()
                        .checked_mul(2)
                        .and_then(|count| count.checked_add(3))
                        .ok_or_else(|| {
                            WorkspaceError::Host(Arc::from("draft match order work overflow"))
                        })?;
                    work.try_reserve(additional).map_err(|_| {
                        WorkspaceError::Host(Arc::from("draft order work allocation failed"))
                    })?;
                    work.push(Work::Finish(index));
                    for (arm, value) in arms.iter().enumerate().rev() {
                        work.push(Work::Visit(value.body));
                        work.push(Work::BeginArm { node: index, arm });
                    }
                    work.push(Work::BeginMatch(index));
                    work.push(Work::Visit(*scrutinee));
                    continue;
                }
                if let DraftNode::While { condition, body } = node {
                    let additional = body.len().checked_add(4).ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("draft while order work overflow"))
                    })?;
                    work.try_reserve(additional).map_err(|_| {
                        WorkspaceError::Host(Arc::from("draft order work allocation failed"))
                    })?;
                    work.push(Work::Finish(index));
                    work.push(Work::ExitLoop(index));
                    work.extend(body.iter().rev().copied().map(Work::Visit));
                    work.push(Work::EnterLoop(index));
                    work.push(Work::Visit(*condition));
                    continue;
                }
                if let DraftNode::Loop { body, .. } = node {
                    let additional = body.len().checked_add(3).ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("draft loop order work overflow"))
                    })?;
                    work.try_reserve(additional).map_err(|_| {
                        WorkspaceError::Host(Arc::from("draft order work allocation failed"))
                    })?;
                    work.push(Work::Finish(index));
                    work.push(Work::ExitLoop(index));
                    work.extend(body.iter().rev().copied().map(Work::Visit));
                    work.push(Work::EnterLoop(index));
                    continue;
                }
                let child_count = node
                    .child_count()
                    .ok_or_else(|| WorkspaceError::Host(Arc::from("draft child count overflow")))?;
                let mut children = Vec::new();
                children.try_reserve(child_count).map_err(|_| {
                    WorkspaceError::Host(Arc::from("draft child allocation failed"))
                })?;
                node.for_each_child(|child| children.push(child));
                work.try_reserve(
                    children.len().checked_add(1).ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("draft order work overflow"))
                    })?,
                )
                .map_err(|_| {
                    WorkspaceError::Host(Arc::from("draft order work allocation failed"))
                })?;
                work.push(Work::Finish(index));
                work.extend(children.into_iter().rev().map(Work::Visit));
            }
        }
    }
    let lowered = order
        .iter()
        .filter(|action| matches!(action, DraftLoweringAction::Lower(_)))
        .count();
    if lowered == draft.nodes.len() {
        Ok(order)
    } else {
        Err(WorkspaceError::InvalidDraft(Arc::from(
            "draft traversal did not cover every node",
        )))
    }
}

fn validate_draft_binding_scopes(draft: &ExpressionDraft) -> Result<(), WorkspaceError> {
    enum ScopeWork {
        Visit(DraftNodeId),
        Add(DraftBindingId),
        Remove(Vec<DraftBindingId>),
    }

    let mut work = vec![ScopeWork::Visit(draft.root)];
    let mut active = HashSet::new();
    while let Some(item) = work.pop() {
        match item {
            ScopeWork::Add(binding) => {
                if !active.insert(binding) {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "draft binding handle has overlapping lexical scopes",
                    )));
                }
            }
            ScopeWork::Remove(bindings) => {
                for binding in bindings {
                    if !active.remove(&binding) {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(
                            "draft binding scope closed without an active definition",
                        )));
                    }
                }
            }
            ScopeWork::Visit(id) => {
                #[cfg(test)]
                DRAFT_SCOPE_NODE_VISITS.with(|count| {
                    count.set(count.get().saturating_add(1));
                });
                let index = id.index().ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("draft node exceeds host index"))
                })?;
                let node = draft.nodes.get(index).ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("draft node identity is stale"))
                })?;
                match node {
                    DraftNode::Load(DraftBindingRef::Local(binding))
                    | DraftNode::Move(DraftBindingRef::Local(binding))
                    | DraftNode::BorrowShared(DraftBindingRef::Local(binding)) => {
                        if !active.contains(binding) {
                            return Err(WorkspaceError::InvalidDraft(Arc::from(format!(
                                "draft binding handle {} is forward or out of lexical scope",
                                binding.raw()
                            ))));
                        }
                    }
                    DraftNode::SetLocal {
                        target: DraftBindingRef::Local(binding),
                        value,
                    } => {
                        if !active.contains(binding) {
                            return Err(WorkspaceError::InvalidDraft(Arc::from(format!(
                                "draft binding handle {} is forward or out of lexical scope",
                                binding.raw()
                            ))));
                        }
                        work.try_reserve(1).map_err(|_| {
                            WorkspaceError::Host(Arc::from("draft scope work allocation failed"))
                        })?;
                        work.push(ScopeWork::Visit(*value));
                    }
                    DraftNode::MutableLocal {
                        binding,
                        initial,
                        body,
                        ..
                    } => {
                        work.try_reserve(4).map_err(|_| {
                            WorkspaceError::Host(Arc::from("draft scope work allocation failed"))
                        })?;
                        work.push(ScopeWork::Remove(vec![*binding]));
                        work.push(ScopeWork::Visit(*body));
                        work.push(ScopeWork::Add(*binding));
                        work.push(ScopeWork::Visit(*initial));
                    }
                    DraftNode::Let { bindings, body } => {
                        let mut removed = Vec::new();
                        removed.try_reserve(bindings.len()).map_err(|_| {
                            WorkspaceError::Host(Arc::from("draft scope allocation failed"))
                        })?;
                        removed.extend(bindings.iter().map(|binding| binding.binding));
                        work.try_reserve(
                            bindings
                                .len()
                                .checked_mul(2)
                                .and_then(|count| count.checked_add(2))
                                .ok_or_else(|| {
                                    WorkspaceError::Host(Arc::from("draft scope work overflow"))
                                })?,
                        )
                        .map_err(|_| {
                            WorkspaceError::Host(Arc::from("draft scope work allocation failed"))
                        })?;
                        work.push(ScopeWork::Remove(removed));
                        work.push(ScopeWork::Visit(*body));
                        for binding in bindings.iter().rev() {
                            work.push(ScopeWork::Add(binding.binding));
                            work.push(ScopeWork::Visit(binding.value));
                        }
                    }
                    DraftNode::Match { scrutinee, arms } => {
                        let binding_count = arms.iter().try_fold(0_usize, |count, arm| {
                            count
                                .checked_add(
                                    arm.pattern
                                        .nodes
                                        .iter()
                                        .filter(|node| {
                                            matches!(node, DraftPatternNode::Binding { .. })
                                        })
                                        .count(),
                                )
                                .ok_or_else(|| {
                                    WorkspaceError::Host(Arc::from(
                                        "match scope binding count overflow",
                                    ))
                                })
                        })?;
                        let additional = binding_count
                            .checked_mul(2)
                            .and_then(|count| count.checked_add(arms.len()))
                            .and_then(|count| count.checked_add(1))
                            .ok_or_else(|| {
                                WorkspaceError::Host(Arc::from("match scope work overflow"))
                            })?;
                        work.try_reserve(additional).map_err(|_| {
                            WorkspaceError::Host(Arc::from("match scope work allocation failed"))
                        })?;
                        for arm in arms.iter().rev() {
                            let mut bindings = Vec::new();
                            bindings.try_reserve(arm.pattern.nodes.len()).map_err(|_| {
                                WorkspaceError::Host(Arc::from(
                                    "match scope binding allocation failed",
                                ))
                            })?;
                            bindings.extend(arm.pattern.nodes.iter().filter_map(|node| {
                                if let DraftPatternNode::Binding { binding, .. } = node {
                                    Some(*binding)
                                } else {
                                    None
                                }
                            }));
                            work.push(ScopeWork::Remove(bindings.clone()));
                            work.push(ScopeWork::Visit(arm.body));
                            work.extend(bindings.into_iter().rev().map(ScopeWork::Add));
                        }
                        work.push(ScopeWork::Visit(*scrutinee));
                    }
                    _ => {
                        let mut children = Vec::new();
                        node.for_each_child(|child| children.push(child));
                        work.try_reserve(children.len()).map_err(|_| {
                            WorkspaceError::Host(Arc::from("draft scope work allocation failed"))
                        })?;
                        work.extend(children.into_iter().rev().map(ScopeWork::Visit));
                    }
                }
            }
        }
    }
    if active.is_empty() {
        Ok(())
    } else {
        Err(WorkspaceError::InvalidDraft(Arc::from(
            "draft binding scope did not close",
        )))
    }
}

fn root_local_count(
    program: &SemanticProgram,
    root: EntityAddress,
) -> Result<usize, WorkspaceError> {
    if root == EntityAddress::Main {
        return program
            .main
            .as_ref()
            .map(|main| main.local_count)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("main root")));
    }
    let EntityAddress::Binding(raw) = root else {
        return Err(WorkspaceError::StaleIdentity(Arc::from("expression root")));
    };
    program
        .functions
        .iter()
        .find(|function| function.binding.raw() == raw)
        .map(|function| function.local_count)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function root")))
}

fn set_root_local_count(
    program: &mut SemanticProgram,
    root: EntityAddress,
    local_count: usize,
) -> Result<(), WorkspaceError> {
    if root == EntityAddress::Main {
        program
            .main
            .as_mut()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("main root")))?
            .local_count = local_count;
        return Ok(());
    }
    let EntityAddress::Binding(raw) = root else {
        return Err(WorkspaceError::StaleIdentity(Arc::from("expression root")));
    };
    program
        .functions
        .iter_mut()
        .find(|function| function.binding.raw() == raw)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function root")))?
        .local_count = local_count;
    Ok(())
}

fn resolve_assignment_target(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    reference: DraftBindingRef,
    visible: &HashSet<EntityId>,
    locals: &HashMap<DraftBindingId, ResolvedDraftBinding>,
    published_locations: &HashMap<crate::hir::BindingId, (usize, crate::hir::PlaceId)>,
) -> Result<ResolvedDraftBinding, WorkspaceError> {
    if let DraftBindingRef::Entity(entity) = reference {
        let header = snapshot.workspace_entity(entity)?;
        if !visible.contains(&entity) {
            return Err(invisible_entity("set-local", entity));
        }
        if header.kind != EntityKind::MutableLocal {
            return Err(wrong_kind("set-local", "mutable local", header.kind));
        }
    }
    let resolved = resolve_draft_binding(
        snapshot,
        program,
        reference,
        visible,
        locals,
        published_locations,
    )?;
    if resolved.kind == BindingKind::MutableLocal {
        Ok(resolved)
    } else {
        Err(WorkspaceError::InvalidDraft(Arc::from(
            "set-local target is not a visible mutable local",
        )))
    }
}

fn resolve_draft_binding(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    reference: DraftBindingRef,
    visible: &HashSet<EntityId>,
    locals: &HashMap<DraftBindingId, ResolvedDraftBinding>,
    published_locations: &HashMap<crate::hir::BindingId, (usize, crate::hir::PlaceId)>,
) -> Result<ResolvedDraftBinding, WorkspaceError> {
    match reference {
        DraftBindingRef::Local(binding) => locals.get(&binding).cloned().ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from(format!(
                "draft binding handle {} is forward, malformed, or out of scope",
                binding.raw()
            )))
        }),
        DraftBindingRef::Entity(entity) => {
            #[cfg(test)]
            STABLE_BINDING_LOOKUPS.with(|count| count.set(count.get().saturating_add(1)));
            let header = snapshot.workspace_entity(entity)?;
            if !visible.contains(&entity) {
                return Err(invisible_entity("binding reference", entity));
            }
            if !matches!(
                header.kind,
                EntityKind::Parameter
                    | EntityKind::ImmutableLocal
                    | EntityKind::StaticBytesLocal
                    | EntityKind::MutableLocal
            ) {
                return Err(wrong_kind(
                    "binding reference",
                    "parameter or local",
                    header.kind,
                ));
            }
            let binding = binding_from_entity(snapshot, program, entity)?;
            let (slot, place) = published_locations
                .get(&binding)
                .copied()
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("local binding")))?;
            let definition = program
                .binding(binding)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("binding")))?;
            Ok(ResolvedDraftBinding {
                binding,
                slot,
                place,
                ty: definition.ty.clone(),
                kind: definition.kind.clone(),
            })
        }
    }
}

fn binding_locations(
    program: &SemanticProgram,
    root: EntityAddress,
) -> Result<HashMap<crate::hir::BindingId, (usize, crate::hir::PlaceId)>, WorkspaceError> {
    let (params, places, expression, local_count) = if root == EntityAddress::Main {
        let main = program
            .main
            .as_ref()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("main root")))?;
        (
            &main.params,
            &main.param_places,
            &main.body,
            main.local_count,
        )
    } else {
        let EntityAddress::Binding(raw) = root else {
            return Err(WorkspaceError::StaleIdentity(Arc::from("expression root")));
        };
        let function = program
            .functions
            .iter()
            .find(|function| function.binding.raw() == raw)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function root")))?;
        (
            &function.params,
            &function.param_places,
            &function.body,
            function.local_count,
        )
    };
    if params.len() != places.len() {
        return Err(WorkspaceError::Validation(Arc::from(
            "callable parameter places are inconsistent",
        )));
    }
    let mut locations = HashMap::new();
    locations
        .try_reserve(local_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("binding location allocation failed")))?;
    for (slot, (binding, place)) in params.iter().zip(places).enumerate() {
        record_binding_location(&mut locations, *binding, slot, *place)?;
    }
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("binding location work allocation failed")))?;
    pending.push(expression);
    while let Some(expression) = pending.pop() {
        #[cfg(test)]
        BINDING_LOCATION_NODE_VISITS.with(|count| count.set(count.get().saturating_add(1)));
        match &expression.kind {
            ExprKind::Let { bindings, .. } => {
                for local in bindings {
                    record_binding_location(
                        &mut locations,
                        local.binding,
                        local.slot,
                        local.place,
                    )?;
                }
            }
            ExprKind::MutableLocal {
                binding,
                place,
                slot,
                ..
            } => record_binding_location(&mut locations, *binding, *slot, *place)?,
            ExprKind::Match { plan, .. } => {
                let plan = program
                    .match_plans
                    .get(host_index(plan.raw(), "match plan")?)
                    .filter(|item| item.id == *plan)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("match plan")))?;
                record_binding_location(
                    &mut locations,
                    plan.scrutinee.binding,
                    plan.scrutinee.slot,
                    plan.scrutinee.place,
                )?;
                record_pattern_locations(&mut locations, &plan.arms)?;
            }
            _ => {}
        }
        crate::hir::for_each_expression_child(expression, &mut |child| pending.push(child));
    }
    Ok(locations)
}

fn record_pattern_locations(
    locations: &mut HashMap<crate::hir::BindingId, (usize, crate::hir::PlaceId)>,
    arms: &[crate::hir::PlannedMatchArm],
) -> Result<(), WorkspaceError> {
    let mut pending = Vec::new();
    pending
        .try_reserve(arms.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("match location work allocation failed")))?;
    pending.extend(arms.iter().map(|arm| &arm.pattern));
    while let Some(pattern) = pending.pop() {
        match pattern {
            crate::hir::MatchPattern::Binding { local } => {
                record_binding_location(locations, local.binding, local.slot, local.place)?;
            }
            crate::hir::MatchPattern::Variant { fields, .. }
            | crate::hir::MatchPattern::Product { fields, .. } => {
                pending.try_reserve(fields.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("match location work allocation failed"))
                })?;
                for field in fields {
                    if let Some(local) = &field.projection {
                        record_binding_location(locations, local.binding, local.slot, local.place)?;
                    }
                    pending.push(&field.pattern);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn record_binding_location(
    locations: &mut HashMap<crate::hir::BindingId, (usize, crate::hir::PlaceId)>,
    binding: crate::hir::BindingId,
    slot: usize,
    place: crate::hir::PlaceId,
) -> Result<(), WorkspaceError> {
    if locations.insert(binding, (slot, place)).is_some() {
        Err(WorkspaceError::Validation(Arc::from(
            "callable binding location is defined more than once",
        )))
    } else {
        Ok(())
    }
}

fn lower_product_value(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    product_entity: EntityId,
    fields: &[DraftFieldValue],
    completed: &mut [Option<Expr>],
    origin: Origin,
) -> Result<Expr, WorkspaceError> {
    let product_header = snapshot.workspace_entity(product_entity)?;
    if product_header.kind != EntityKind::Product {
        return Err(wrong_kind(
            "product value",
            "product declaration",
            product_header.kind,
        ));
    }
    let EntityAddress::Product(product_raw) = entity_address(snapshot, product_entity)? else {
        return Err(WorkspaceError::StaleIdentity(Arc::from("product")));
    };
    let definition = program
        .products
        .get(host_index(product_raw, "product")?)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product")))?;
    if fields.len() != definition.fields.len() {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "product value must provide exactly one value per field",
        )));
    }
    let mut ordered = Vec::new();
    ordered
        .try_reserve(definition.fields.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("product value allocation failed")))?;
    ordered.resize_with(definition.fields.len(), || None);
    for field in fields {
        let header = snapshot.workspace_entity(field.field)?;
        if header.kind != EntityKind::ProductField {
            return Err(wrong_kind(
                "product value field",
                "product field",
                header.kind,
            ));
        }
        let EntityAddress::ProductField {
            product,
            field: field_raw,
        } = entity_address(snapshot, field.field)?
        else {
            return Err(WorkspaceError::StaleIdentity(Arc::from("product field")));
        };
        if product != product_raw {
            return Err(WorkspaceError::InvalidDraft(Arc::from(
                "product value field belongs to a different product",
            )));
        }
        let index = host_index(field_raw, "product field")?;
        let value = take_draft_child(completed, field.value)?;
        let expected = definition
            .fields
            .get(index)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product field")))?;
        require_type(&value.ty, &expected.ty)?;
        let slot = ordered
            .get_mut(index)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product field")))?;
        if slot.replace(value).is_some() {
            return Err(WorkspaceError::InvalidDraft(Arc::from(
                "product value field is duplicated",
            )));
        }
    }
    let fields = ordered
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| WorkspaceError::InvalidDraft(Arc::from("product value field is missing")))?;
    let effects = fields
        .iter()
        .fold(EffectSet::PURE, |set, field| set.union(field.effects));
    Ok(Expr {
        ty: Type::Product(definition.id),
        effects,
        origin,
        kind: ExprKind::ProductValue {
            product: definition.id,
            fields,
        },
    })
}

fn lower_product_field(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    field_entity: EntityId,
    value: DraftNodeId,
    completed: &mut [Option<Expr>],
    origin: Origin,
) -> Result<Expr, WorkspaceError> {
    let header = snapshot.workspace_entity(field_entity)?;
    if header.kind != EntityKind::ProductField {
        return Err(wrong_kind(
            "product projection",
            "product field",
            header.kind,
        ));
    }
    let EntityAddress::ProductField { product, field } = entity_address(snapshot, field_entity)?
    else {
        return Err(WorkspaceError::StaleIdentity(Arc::from("product field")));
    };
    let definition = program
        .products
        .get(host_index(product, "product")?)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product")))?;
    let field_definition = definition
        .fields
        .get(host_index(field, "product field")?)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product field")))?;
    let value = take_draft_child(completed, value)?;
    require_type(&value.ty, &Type::Product(definition.id))?;
    Ok(Expr {
        ty: field_definition.ty.clone(),
        effects: value.effects,
        origin,
        kind: ExprKind::ProductField {
            product: definition.id,
            field,
            value: Box::new(value),
        },
    })
}

#[allow(clippy::too_many_arguments)]
fn lower_enum_value(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    callable: EntityId,
    variant_entity: EntityId,
    type_arguments: &[super::TypeArgumentDraft],
    fields: &[DraftFieldValue],
    completed: &mut [Option<Expr>],
    origin: Origin,
) -> Result<Expr, WorkspaceError> {
    let header = snapshot.workspace_entity(variant_entity)?;
    if header.kind != EntityKind::EnumVariant {
        return Err(wrong_kind("enum value", "enum variant", header.kind));
    }
    let enumeration_entity = header
        .owner
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum variant owner")))?;
    let enumeration_header = snapshot.workspace_entity(enumeration_entity)?;
    if enumeration_header.kind != EntityKind::Enum {
        return Err(wrong_kind(
            "enum value variant owner",
            "enum declaration",
            enumeration_header.kind,
        ));
    }
    let EntityAddress::EnumVariant {
        enumeration,
        variant,
    } = entity_address(snapshot, variant_entity)?
    else {
        return Err(WorkspaceError::StaleIdentity(Arc::from("enum variant")));
    };
    if entity_address(snapshot, enumeration_entity)? != EntityAddress::Enum(enumeration) {
        return Err(WorkspaceError::Validation(Arc::from(
            "enum variant owner identity is inconsistent",
        )));
    }
    let definition = program
        .enums
        .get(host_index(enumeration, "enum")?)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum")))?;
    let arguments = resolve_enum_type_arguments(
        snapshot,
        program,
        callable,
        enumeration_entity,
        definition,
        type_arguments,
    )?;
    let substitutions = enum_substitution_map(definition, &arguments, "enum value")?;
    let selected = definition
        .variants
        .get(host_index(variant, "enum variant")?)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum variant")))?;
    if fields.len() != selected.fields.len() {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "enum value must provide exactly one value per variant field",
        )));
    }
    let mut ordered = Vec::new();
    ordered
        .try_reserve(selected.fields.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("enum value allocation failed")))?;
    ordered.resize_with(selected.fields.len(), || None);
    for field in fields {
        let field_header = snapshot.workspace_entity(field.field)?;
        if field_header.kind != EntityKind::EnumField {
            return Err(wrong_kind(
                "enum value field",
                "enum field",
                field_header.kind,
            ));
        }
        let EntityAddress::EnumField {
            enumeration: field_enum,
            variant: field_variant,
            field: field_raw,
        } = entity_address(snapshot, field.field)?
        else {
            return Err(WorkspaceError::StaleIdentity(Arc::from("enum field")));
        };
        if field_enum != enumeration || field_variant != variant {
            return Err(WorkspaceError::InvalidDraft(Arc::from(
                "enum value field belongs to a different enum variant",
            )));
        }
        let index = host_index(field_raw, "enum field")?;
        let value = take_draft_child(completed, field.value)?;
        let declared = selected
            .fields
            .get(index)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum field")))?;
        let expected =
            substitute_enum_field_type(&declared.ty, &substitutions, "enum value field")?;
        require_workspace_type(snapshot, program, callable, &value.ty, &expected)?;
        let slot = ordered
            .get_mut(index)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum field")))?;
        if slot.replace(value).is_some() {
            return Err(WorkspaceError::InvalidDraft(Arc::from(
                "enum value field is duplicated",
            )));
        }
    }
    let fields = ordered
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| WorkspaceError::InvalidDraft(Arc::from("enum value field is missing")))?;
    let effects = fields
        .iter()
        .fold(EffectSet::PURE, |set, field| set.union(field.effects));
    drop(substitutions);
    Ok(Expr {
        ty: Type::Enum {
            id: definition.id,
            arguments,
        },
        effects,
        origin,
        kind: ExprKind::EnumValue {
            enum_id: definition.id,
            variant: selected.id,
            layout: definition.layout.identity,
            fields,
        },
    })
}

fn resolve_enum_type_arguments(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    callable: EntityId,
    enumeration: EntityId,
    definition: &crate::hir::EnumDefinition,
    type_arguments: &[super::TypeArgumentDraft],
) -> Result<Vec<Type>, WorkspaceError> {
    if definition.type_parameters.is_empty() && !type_arguments.is_empty() {
        return Err(WorkspaceError::UnexpectedTypeArgument);
    }
    let mut supplied = HashMap::new();
    supplied
        .try_reserve(type_arguments.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("enum type-argument allocation failed")))?;
    for argument in type_arguments {
        if argument.parameter.namespace() != snapshot.namespace {
            return Err(WorkspaceError::ForeignNamespace(Arc::from(
                "type parameter",
            )));
        }
        let parameter = snapshot
            .workspace_entity(argument.parameter)
            .map_err(|_| WorkspaceError::StaleIdentity(Arc::from("type parameter")))?;
        if parameter.kind != EntityKind::TypeParameter {
            return Err(wrong_kind(
                "enum value type argument",
                "type parameter",
                parameter.kind,
            ));
        }
        if parameter.owner != Some(enumeration) {
            return Err(WorkspaceError::WrongTypeParameterOwner {
                parameter: Box::new(argument.parameter),
                expected: Box::new(enumeration),
                actual: parameter.owner.map(Box::new),
            });
        }
        let resolved = super::types::resolve(
            snapshot,
            program,
            &argument.argument,
            Some(callable),
            false,
            false,
            "enum type argument",
        )?;
        validate_concrete_enum_arguments(std::slice::from_ref(&resolved), "enum value")?;
        if supplied.insert(argument.parameter, resolved).is_some() {
            return Err(WorkspaceError::DuplicateTypeArgument {
                parameter: argument.parameter,
            });
        }
    }
    let mut arguments = Vec::new();
    arguments
        .try_reserve(definition.type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("enum argument allocation failed")))?;
    for parameter_name in &definition.type_parameters {
        let parameter = snapshot
            .indexes
            .type_parameter_entities
            .get(&enumeration)
            .and_then(|parameters| parameters.get(parameter_name.as_str()))
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("type parameter")))?;
        arguments.push(
            supplied
                .remove(&parameter)
                .ok_or(WorkspaceError::MissingTypeArgument { parameter })?,
        );
    }
    if !supplied.is_empty() {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "enum value contains an undeclared type argument",
        )));
    }
    Ok(arguments)
}

fn validate_concrete_enum_arguments(
    arguments: &[Type],
    operation: &str,
) -> Result<(), WorkspaceError> {
    for argument in arguments {
        crate::generic_call::validate_concrete_enum_argument(argument).map_err(
            |error| match error {
                crate::generic_call::GenericCallError::OwnershipUnsupported => {
                    WorkspaceError::unsupported(
                        operation,
                        "ownership/reference-bearing generic enum arguments are unsupported",
                    )
                }
                crate::generic_call::GenericCallError::ForwardingUnsupported => {
                    WorkspaceError::GenericForwardingUnsupported
                }
                crate::generic_call::GenericCallError::Host(message) => {
                    WorkspaceError::Host(Arc::from(message))
                }
                other => WorkspaceError::Validation(Arc::from(other.to_string())),
            },
        )?;
    }
    Ok(())
}

fn enum_substitution_map<'a>(
    definition: &'a crate::hir::EnumDefinition,
    arguments: &'a [Type],
    subject: &str,
) -> Result<HashMap<&'a str, &'a Type>, WorkspaceError> {
    if definition.type_parameters.len() != arguments.len() {
        return Err(WorkspaceError::Validation(Arc::from(format!(
            "{subject} substitution arity is inconsistent"
        ))));
    }
    let mut substitutions = HashMap::new();
    substitutions
        .try_reserve(definition.type_parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("enum substitution allocation failed")))?;
    for (parameter, argument) in definition.type_parameters.iter().zip(arguments) {
        if substitutions.insert(parameter.as_str(), argument).is_some() {
            return Err(WorkspaceError::Validation(Arc::from(format!(
                "{subject} declaration repeats a type parameter"
            ))));
        }
    }
    Ok(substitutions)
}

fn substitute_enum_field_type(
    declared: &Type,
    substitutions: &HashMap<&str, &Type>,
    subject: &str,
) -> Result<Type, WorkspaceError> {
    crate::generic_call::substitute_type(declared, substitutions).map_err(|error| match error {
        crate::generic_call::GenericCallError::Host(message) => {
            WorkspaceError::Host(Arc::from(message))
        }
        other => {
            WorkspaceError::Validation(Arc::from(format!("{subject} substitution failed: {other}")))
        }
    })
}

fn lower_enum_is_variant(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    variant_entity: EntityId,
    value: DraftNodeId,
    completed: &mut [Option<Expr>],
    origin: Origin,
) -> Result<Expr, WorkspaceError> {
    let header = snapshot.workspace_entity(variant_entity)?;
    if header.kind != EntityKind::EnumVariant {
        return Err(wrong_kind("enum variant test", "enum variant", header.kind));
    }
    let EntityAddress::EnumVariant {
        enumeration,
        variant,
    } = entity_address(snapshot, variant_entity)?
    else {
        return Err(WorkspaceError::StaleIdentity(Arc::from("enum variant")));
    };
    let definition = program
        .enums
        .get(host_index(enumeration, "enum")?)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum")))?;
    let selected = definition
        .variants
        .get(host_index(variant, "enum variant")?)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum variant")))?;
    let value = take_draft_child(completed, value)?;
    let Type::Enum { id, arguments } = &value.ty else {
        return Err(WorkspaceError::InvalidDraft(Arc::from(format!(
            "enum variant test requires an enum value, got {}",
            value.ty
        ))));
    };
    if *id != definition.id {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "enum variant test belongs to a different enum than its value",
        )));
    }
    validate_concrete_enum_arguments(arguments, "enum variant test")?;
    Ok(Expr {
        ty: Type::Bool,
        effects: value.effects,
        origin,
        kind: ExprKind::EnumIsVariant {
            enum_id: definition.id,
            variant: selected.id,
            layout: definition.layout.identity,
            value: Box::new(value),
        },
    })
}

fn scalar(ty: Type, kind: ExprKind, origin: Origin) -> Expr {
    Expr {
        ty,
        effects: EffectSet::PURE,
        origin,
        kind,
    }
}

fn validate_draft_shape(draft: &ExpressionDraft) -> Result<(), WorkspaceError> {
    if draft.nodes.is_empty() {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "expression draft is empty",
        )));
    }
    for node in &draft.nodes {
        if let DraftNode::Match { arms, .. } = node {
            if arms.is_empty() {
                return Err(WorkspaceError::InvalidDraft(Arc::from(
                    "match arms must not be empty",
                )));
            }
            for arm in arms {
                validate_pattern_shape(&arm.pattern)?;
            }
        }
    }
    let root = draft
        .root
        .index()
        .filter(|index| *index < draft.nodes.len())
        .ok_or_else(|| WorkspaceError::InvalidDraft(Arc::from("draft root is stale")))?;
    let mut parents = Vec::new();
    parents
        .try_reserve(draft.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("draft shape allocation failed")))?;
    parents.resize(draft.nodes.len(), 0_u64);
    for node in &draft.nodes {
        let mut failure = None;
        node.for_each_child(|child| {
            let Some(child_index) = child.index().filter(|index| *index < draft.nodes.len()) else {
                failure = Some("draft child is stale");
                return;
            };
            let Some(count) = parents[child_index].checked_add(1) else {
                failure = Some("draft parent count overflow");
                return;
            };
            parents[child_index] = count;
        });
        if let Some(message) = failure {
            return Err(WorkspaceError::InvalidDraft(Arc::from(message)));
        }
    }
    if parents[root] != 0
        || parents
            .iter()
            .enumerate()
            .any(|(index, count)| index != root && *count != 1)
    {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "draft must be a connected expression tree with no reused child",
        )));
    }

    let mut reached = Vec::new();
    reached
        .try_reserve(draft.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("draft reachability allocation failed")))?;
    reached.resize(draft.nodes.len(), false);
    let mut pending = Vec::new();
    pending
        .try_reserve(draft.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("draft reachability allocation failed")))?;
    pending.push(draft.root);
    let mut reached_count = 0_usize;
    while let Some(node) = pending.pop() {
        let index = node.index().ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from("draft traversal identity is stale"))
        })?;
        let visited = reached.get_mut(index).ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from("draft traversal identity is stale"))
        })?;
        if *visited {
            return Err(WorkspaceError::InvalidDraft(Arc::from(
                "draft contains a cycle or reused child",
            )));
        }
        *visited = true;
        reached_count = reached_count
            .checked_add(1)
            .ok_or_else(|| WorkspaceError::Host(Arc::from("draft reachability overflow")))?;
        let current = draft.nodes.get(index).ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from("draft traversal identity is stale"))
        })?;
        current.for_each_child(|child| pending.push(child));
    }
    if reached_count != draft.nodes.len() {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "draft contains nodes disconnected from its root",
        )));
    }
    Ok(())
}

fn validate_pattern_shape(pattern: &PatternDraft) -> Result<(), WorkspaceError> {
    if pattern.nodes.is_empty() {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "match pattern draft is empty",
        )));
    }
    let root = pattern
        .root
        .index()
        .filter(|index| *index < pattern.nodes.len())
        .ok_or_else(|| WorkspaceError::InvalidDraft(Arc::from("pattern root is stale")))?;
    let mut parents = Vec::new();
    parents
        .try_reserve(pattern.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("pattern shape allocation failed")))?;
    parents.resize(pattern.nodes.len(), 0_u64);
    for node in &pattern.nodes {
        let mut failure = None;
        node.for_each_child(|child| {
            let Some(index) = child.index().filter(|index| *index < pattern.nodes.len()) else {
                failure = Some("pattern child is stale");
                return;
            };
            let Some(count) = parents[index].checked_add(1) else {
                failure = Some("pattern parent count overflow");
                return;
            };
            parents[index] = count;
        });
        if let Some(message) = failure {
            return Err(WorkspaceError::InvalidDraft(Arc::from(message)));
        }
    }
    if parents[root] != 0
        || parents
            .iter()
            .enumerate()
            .any(|(index, count)| index != root && *count != 1)
    {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "pattern draft must be one connected tree with no reused child",
        )));
    }
    let mut reached = Vec::new();
    reached
        .try_reserve(pattern.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("pattern reachability allocation failed")))?;
    reached.resize(pattern.nodes.len(), false);
    let mut pending = Vec::new();
    pending
        .try_reserve(pattern.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("pattern reachability allocation failed")))?;
    pending.push(pattern.root);
    let mut reached_count = 0_usize;
    while let Some(id) = pending.pop() {
        let index = id.index().ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from("pattern traversal identity is stale"))
        })?;
        let visited = reached.get_mut(index).ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from("pattern traversal identity is stale"))
        })?;
        if *visited {
            return Err(WorkspaceError::InvalidDraft(Arc::from(
                "pattern draft contains a cycle or reused child",
            )));
        }
        *visited = true;
        reached_count = reached_count
            .checked_add(1)
            .ok_or_else(|| WorkspaceError::Host(Arc::from("pattern reachability overflow")))?;
        let node = pattern.nodes.get(index).ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from("pattern traversal identity is stale"))
        })?;
        pending.try_reserve(node.child_count()).map_err(|_| {
            WorkspaceError::Host(Arc::from("pattern reachability allocation failed"))
        })?;
        node.for_each_child(|child| pending.push(child));
    }
    if reached_count == pattern.nodes.len() {
        Ok(())
    } else {
        Err(WorkspaceError::InvalidDraft(Arc::from(
            "pattern draft contains nodes disconnected from its root",
        )))
    }
}

fn take_draft_child(
    completed: &mut [Option<Expr>],
    id: super::DraftNodeId,
) -> Result<Expr, WorkspaceError> {
    id.index()
        .and_then(|index| completed.get_mut(index))
        .and_then(Option::take)
        .ok_or_else(|| WorkspaceError::InvalidDraft(Arc::from("draft child is stale or reused")))
}

fn entity_address(
    snapshot: &WorkspaceSnapshot,
    entity: EntityId,
) -> Result<EntityAddress, WorkspaceError> {
    snapshot.workspace_entity(entity)?;
    snapshot
        .indexes
        .entity_lookup
        .get(&entity)
        .and_then(|index| snapshot.indexes.entity_addresses.get(*index))
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("entity")))
}

fn host_index(raw: u64, subject: &str) -> Result<usize, WorkspaceError> {
    usize::try_from(raw).map_err(|_| WorkspaceError::StaleIdentity(Arc::from(subject.to_owned())))
}

fn invisible_entity(operation: &str, entity: EntityId) -> WorkspaceError {
    WorkspaceError::InvisibleEntity {
        operation: Arc::from(operation),
        entity: Box::new(entity),
        reason: Arc::from("entity is outside lexical visibility at the edited expression"),
    }
}

fn wrong_kind(operation: &str, expected: &str, actual: EntityKind) -> WorkspaceError {
    WorkspaceError::WrongEntityKind {
        operation: Arc::from(operation),
        expected: Arc::from(expected),
        actual: super::error::SemanticKind::Entity(actual),
    }
}

fn entity_kind_name(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Main => "main",
        EntityKind::Parameter => "parameter",
        EntityKind::ImmutableLocal => "immutable local",
        EntityKind::StaticBytesLocal => "static bytes local",
        EntityKind::MutableLocal => "mutable local",
        EntityKind::Function => "function",
        EntityKind::TypeParameter => "type parameter",
        EntityKind::BuiltinOperation => "builtin operation",
        EntityKind::Product => "product",
        EntityKind::ProductField => "product field",
        EntityKind::Enum => "enum",
        EntityKind::EnumVariant => "enum variant",
        EntityKind::EnumField => "enum field",
        EntityKind::Trait => "trait",
        EntityKind::Implementation => "implementation",
    }
}

fn binding_from_entity(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    entity: EntityId,
) -> Result<crate::hir::BindingId, WorkspaceError> {
    let address = snapshot
        .indexes
        .entity_lookup
        .get(&entity)
        .and_then(|index| snapshot.indexes.entity_addresses.get(*index))
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("binding")))?;
    let EntityAddress::Binding(raw) = address else {
        return Err(WorkspaceError::StaleIdentity(Arc::from("binding")));
    };
    program
        .bindings
        .get(
            usize::try_from(raw)
                .map_err(|_| WorkspaceError::StaleIdentity(Arc::from("binding")))?,
        )
        .map(|binding| binding.id)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("binding")))
}

fn generic_workspace_error(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    caller: EntityId,
    callee: EntityId,
    error: crate::generic_call::GenericCallError,
) -> WorkspaceError {
    match error {
        crate::generic_call::GenericCallError::Arity { expected, actual } => {
            WorkspaceError::CallArity {
                callee: Box::new(callee),
                expected,
                actual,
            }
        }
        crate::generic_call::GenericCallError::TypeMismatch {
            expected, actual, ..
        } => {
            let expected = super::types::view(program, &snapshot.indexes, &expected, Some(caller));
            let actual = super::types::view(program, &snapshot.indexes, &actual, Some(caller));
            match (expected, actual) {
                (Ok(expected), Ok(actual)) => WorkspaceError::TypeMismatch {
                    expected: Box::new(expected),
                    actual: Box::new(actual),
                },
                (Err(error), _) | (_, Err(error)) => error,
            }
        }
        crate::generic_call::GenericCallError::UnexpectedSubstitutions => {
            WorkspaceError::UnexpectedTypeArgument
        }
        crate::generic_call::GenericCallError::ForwardingUnsupported => {
            WorkspaceError::GenericForwardingUnsupported
        }
        crate::generic_call::GenericCallError::UnsatisfiedTrait {
            parameter,
            trait_id,
            ty,
        } => {
            let stable_parameter = snapshot
                .indexes
                .type_parameter_entities
                .get(&callee)
                .and_then(|parameters| parameters.get(parameter.as_str()))
                .copied();
            match (
                stable_parameter,
                super::types::semantic_trait(program, &snapshot.indexes, trait_id),
                super::types::view(program, &snapshot.indexes, &ty, Some(caller)),
            ) {
                (Some(parameter), Ok(trait_identity), Ok(argument)) => {
                    WorkspaceError::UnsatisfiedTraitBound {
                        parameter: Box::new(parameter),
                        trait_identity: Box::new(trait_identity),
                        argument: Box::new(argument),
                    }
                }
                _ => WorkspaceError::Validation(Arc::from(
                    "generic bound failure could not be mapped to stable semantic identities",
                )),
            }
        }
        crate::generic_call::GenericCallError::OwnershipUnsupported => {
            WorkspaceError::unsupported(
                "generic-call",
                "ownership/reference generic instantiation is unavailable in the initial ownership slice",
            )
        }
        crate::generic_call::GenericCallError::ReferenceResultUnsupported => {
            WorkspaceError::unsupported(
                "generic-call",
                "user-call results cannot be lexical references in the initial ownership slice",
            )
        }
        crate::generic_call::GenericCallError::Host(message) => {
            WorkspaceError::Host(Arc::from(message))
        }
        other => WorkspaceError::Validation(Arc::from(other.to_string())),
    }
}

fn require_type(actual: &Type, expected: &Type) -> Result<(), WorkspaceError> {
    if Type::unify_assignable(actual, expected) {
        Ok(())
    } else {
        Err(WorkspaceError::InvalidDraft(Arc::from(format!(
            "expression type mismatch: expected {expected}, got {actual}"
        ))))
    }
}

fn require_workspace_type(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    context: EntityId,
    actual: &Type,
    expected: &Type,
) -> Result<(), WorkspaceError> {
    if actual == expected {
        return Ok(());
    }
    Err(WorkspaceError::TypeMismatch {
        expected: Box::new(super::types::view(
            program,
            &snapshot.indexes,
            expected,
            Some(context),
        )?),
        actual: Box::new(super::types::view(
            program,
            &snapshot.indexes,
            actual,
            Some(context),
        )?),
    })
}

fn refresh_hole_addresses(
    holes: &mut [HoleRecord],
    program: &SemanticProgram,
    indexes: &SnapshotIndexes,
) -> Result<(), WorkspaceError> {
    for hole in holes {
        let index = indexes
            .node_lookup
            .get(&hole.state.id.0)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole root")))?;
        hole.address = indexes.node_addresses[index];
        hole.key = indexes.node_keys[index];
        hole.state.owner = indexes
            .address_entities
            .get(&hole.address.root)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole owner")))?;
        hole.state.context = hole.state.id.0;
        hole.state.visible_entities = visible_entities_in(program, indexes, hole.address)?.into();
        hole.state.expected_type = super::types::view(
            program,
            indexes,
            &hole.expected_internal,
            Some(hole.state.owner),
        )?;
    }
    Ok(())
}

fn refresh_unresolved_value_reference_addresses(
    references: &mut [UnresolvedValueReferenceRecord],
    revision: RevisionId,
    program: &SemanticProgram,
    indexes: &SnapshotIndexes,
) -> Result<(), WorkspaceError> {
    for reference in &mut *references {
        let index = indexes
            .node_lookup
            .get(&reference.state.id.0)
            .copied()
            .ok_or_else(|| {
                WorkspaceError::StaleIdentity(Arc::from("unresolved value-reference root"))
            })?;
        if indexes.nodes[index].kind != NodeKind::UnresolvedValueReference {
            return Err(WorkspaceError::Validation(Arc::from(
                "unresolved value-reference node kind is stale",
            )));
        }
        reference.address = indexes.node_addresses[index];
        reference.key = indexes.node_keys[index];
        reference.state.revision = revision;
        reference.state.owner = indexes
            .address_entities
            .get(&reference.address.root)
            .copied()
            .ok_or_else(|| {
                WorkspaceError::StaleIdentity(Arc::from("unresolved value-reference owner"))
            })?;
        reference.state.context = reference.state.id.0;
        reference.state.visible_entities =
            visible_entities_in(program, indexes, reference.address)?.into();
        reference.state.expected_type = super::types::view(
            program,
            indexes,
            &reference.expected_internal,
            Some(reference.state.owner),
        )?;
    }
    references.sort_by_key(|reference| reference.state.id);
    Ok(())
}

fn install_new_holes(
    holes: &mut Vec<HoleRecord>,
    pending: &[NewHole],
    program: &SemanticProgram,
    indexes: &SnapshotIndexes,
) -> Result<(), WorkspaceError> {
    for hole in pending {
        let node = indexes
            .address_nodes
            .get(&hole.address)
            .copied()
            .ok_or_else(|| WorkspaceError::Validation(Arc::from("new hole node is missing")))?;
        let index =
            indexes.node_lookup.get(&node).copied().ok_or_else(|| {
                WorkspaceError::Validation(Arc::from("new hole index is missing"))
            })?;
        let owner = indexes
            .address_entities
            .get(&hole.address.root)
            .copied()
            .ok_or_else(|| WorkspaceError::Validation(Arc::from("new hole owner is missing")))?;
        let visible = visible_entities_in(program, indexes, hole.address)?;
        let expected_type = indexes
            .node_expected_types
            .get(index)
            .and_then(Clone::clone)
            .ok_or_else(|| {
                WorkspaceError::Validation(Arc::from("new hole expected type is missing"))
            })?;
        holes.push(HoleRecord {
            state: HoleState {
                id: HoleId(node),
                kind: hole.kind,
                expected_type: super::types::view(program, indexes, &expected_type, Some(owner))?,
                goal: Arc::clone(&hole.goal),
                owner,
                context: node,
                visible_entities: visible.into(),
            },
            expected_internal: expected_type,
            address: hole.address,
            key: indexes.node_keys[index],
        });
    }
    holes.sort_by_key(|hole| hole.state.id);
    Ok(())
}

fn incomplete_diagnostics(
    holes: &[HoleRecord],
    unresolved_value_references: &[UnresolvedValueReferenceRecord],
    missing_entry: bool,
) -> Result<Vec<DiagnosticHeader>, WorkspaceError> {
    let capacity = holes
        .len()
        .checked_add(unresolved_value_references.len())
        .and_then(|count| count.checked_add(usize::from(missing_entry)))
        .ok_or_else(|| WorkspaceError::Host(Arc::from("diagnostic count overflow")))?;
    let mut diagnostics = Vec::new();
    diagnostics
        .try_reserve(capacity)
        .map_err(|_| WorkspaceError::Host(Arc::from("diagnostic allocation failed")))?;
    if missing_entry {
        diagnostics.push(DiagnosticHeader {
            code: Arc::from("workspace.missing-entry-point"),
            severity: DiagnosticSeverity::Error,
            subject: None,
            message: Arc::from("program requires a main entry point"),
        });
    }
    for hole in holes {
        let (code, label) = match hole.state.kind {
            HoleKind::MissingBody => ("workspace.missing-body", "missing body"),
            HoleKind::TypedExpression => ("workspace.typed-hole", "typed hole"),
        };
        diagnostics.push(DiagnosticHeader {
            code: Arc::from(code),
            severity: DiagnosticSeverity::Error,
            subject: Some(SemanticChild::Node(hole.state.id.0)),
            message: Arc::from(format!(
                "{label} requires {}: {}",
                hole.state.expected_type, hole.state.goal
            )),
        });
    }
    for reference in unresolved_value_references {
        diagnostics.push(DiagnosticHeader {
            code: Arc::from("workspace.unresolved-value-reference"),
            severity: DiagnosticSeverity::Error,
            subject: Some(SemanticChild::Node(reference.state.id.0)),
            message: Arc::from(format!(
                "unresolved value reference \"{}\" requires {}",
                reference.state.requested_name, reference.state.expected_type
            )),
        });
    }
    diagnostics.sort_by_key(|diagnostic| diagnostic.subject);
    Ok(diagnostics)
}

fn apply_incomplete_diagnostics(
    indexes: &mut SnapshotIndexes,
    holes: &[HoleRecord],
    unresolved_value_references: &[UnresolvedValueReferenceRecord],
    missing_entry: bool,
) -> Result<Vec<DiagnosticHeader>, WorkspaceError> {
    let diagnostics = incomplete_diagnostics(holes, unresolved_value_references, missing_entry)?;
    rebuild_visible_dependencies(indexes)?;
    indexes.rebuild_maps().map_err(WorkspaceError::from_core)?;
    Ok(diagnostics)
}

fn completeness_blockers(
    program: &SemanticProgram,
    holes: &[HoleRecord],
    unresolved_value_references: &[UnresolvedValueReferenceRecord],
) -> Vec<CompletenessBlocker> {
    let mut blockers = Vec::new();
    if program.main.is_none() {
        blockers.push(CompletenessBlocker::MissingEntryPoint);
    }
    let mut expression_blockers = Vec::new();
    for hole in holes {
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
    for reference in unresolved_value_references {
        expression_blockers.push(CompletenessBlocker::UnresolvedValueReference {
            reference: reference.state.id,
            requested_name: Arc::clone(&reference.state.requested_name),
            expected_type: reference.state.expected_type.clone(),
            owner: reference.state.owner,
            context: reference.state.context,
        });
    }
    expression_blockers.sort_by_key(|blocker| match blocker {
        CompletenessBlocker::MissingEntryPoint => None,
        CompletenessBlocker::MissingBody { hole, .. }
        | CompletenessBlocker::TypedHole { hole, .. } => Some(hole.node()),
        CompletenessBlocker::UnresolvedValueReference { reference, .. } => Some(reference.node()),
    });
    blockers.extend(expression_blockers);
    blockers
}

fn rebuild_visible_dependencies(indexes: &mut SnapshotIndexes) -> Result<(), WorkspaceError> {
    let mut dependencies = Vec::new();
    let capacity = indexes
        .declaration_dependencies
        .len()
        .checked_add(indexes.dependencies.len())
        .and_then(|count| count.checked_add(indexes.references.len()))
        .ok_or_else(|| WorkspaceError::Host(Arc::from("visible dependency count overflow")))?;
    dependencies
        .try_reserve(capacity)
        .map_err(|_| WorkspaceError::Host(Arc::from("visible dependency allocation failed")))?;
    dependencies.extend(indexes.declaration_dependencies.iter().copied());
    dependencies.extend(indexes.dependencies.iter().copied());

    let mut enclosing = HashMap::new();
    enclosing
        .try_reserve(indexes.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("dependency owner allocation failed")))?;
    for header in &indexes.nodes {
        let owner = match header.owner {
            SemanticOwner::Entity(entity) => entity,
            SemanticOwner::Node(parent) => enclosing
                .get(&parent)
                .copied()
                .ok_or_else(|| WorkspaceError::Validation(Arc::from("node owner is stale")))?,
        };
        enclosing.insert(header.id, owner);
    }
    for reference in &indexes.references {
        let dependent = enclosing
            .get(&reference.site)
            .copied()
            .ok_or_else(|| WorkspaceError::Validation(Arc::from("reference site is stale")))?;
        if dependent != reference.target {
            dependencies.push(super::DependencyEdge {
                dependent,
                dependency: reference.target,
            });
        }
    }
    dependencies.sort_by_key(|edge| (edge.dependent, edge.dependency));
    dependencies.dedup();
    indexes.dependencies = dependencies;
    Ok(())
}

fn append_structural_diff(
    base: &WorkspaceSnapshot,
    next: &SnapshotIndexes,
    structural: &[StructuralAction],
    entries: &mut Vec<SemanticDiffEntry>,
) -> Result<(), WorkspaceError> {
    for action in structural {
        let old = base.workspace_node(action.target)?;
        let new = next
            .node(base.namespace, action.target)
            .map_err(|_| WorkspaceError::StaleIdentity(Arc::from("replacement root")))?;
        entries.push(SemanticDiffEntry::ExpressionReplaced {
            node: action.target,
            old_kind: old.kind,
            new_kind: new.kind,
        });
    }
    Ok(())
}

fn append_graph_diff(
    base: &WorkspaceSnapshot,
    next: &SnapshotIndexes,
    entries: &mut Vec<SemanticDiffEntry>,
) -> Result<(), WorkspaceError> {
    let possible_entries = base
        .indexes
        .entities
        .len()
        .checked_add(base.indexes.nodes.len())
        .and_then(|count| count.checked_add(next.nodes.len()))
        .and_then(|count| count.checked_add(base.indexes.references.len()))
        .and_then(|count| count.checked_add(next.references.len()))
        .and_then(|count| count.checked_add(base.indexes.calls.len()))
        .and_then(|count| count.checked_add(next.calls.len()))
        .ok_or_else(|| WorkspaceError::Host(Arc::from("semantic diff size overflow")))?;
    entries
        .try_reserve(possible_entries)
        .map_err(|_| WorkspaceError::Host(Arc::from("semantic diff allocation failed")))?;
    let mut new_entity_ids = HashSet::new();
    new_entity_ids
        .try_reserve(next.entities.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("new entity diff allocation failed")))?;
    new_entity_ids.extend(next.entities.iter().map(|entity| entity.id));
    for entity in &base.indexes.entities {
        if !new_entity_ids.contains(&entity.id) {
            entries.push(SemanticDiffEntry::EntityDeleted {
                entity: entity.id,
                kind: entity.kind,
                name: Arc::clone(&entity.name),
            });
        }
    }

    let mut old_ids = HashSet::new();
    old_ids
        .try_reserve(base.indexes.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("old diff identity allocation failed")))?;
    old_ids.extend(base.indexes.nodes.iter().map(|node| node.id));
    let mut new_ids = HashSet::new();
    new_ids
        .try_reserve(next.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("new diff identity allocation failed")))?;
    new_ids.extend(next.nodes.iter().map(|node| node.id));
    for node in &next.nodes {
        if !old_ids.contains(&node.id) {
            entries.push(SemanticDiffEntry::DescendantCreated {
                parent: node.owner,
                node: node.id,
                kind: node.kind,
            });
        }
    }
    for node in &base.indexes.nodes {
        if !new_ids.contains(&node.id) {
            entries.push(SemanticDiffEntry::DescendantDeleted {
                parent: node.owner,
                node: node.id,
                kind: node.kind,
            });
        }
    }
    append_reference_diff(&base.indexes.references, &next.references, entries)?;
    let mut call_sites = HashSet::new();
    let call_count = base
        .indexes
        .calls
        .len()
        .checked_add(next.calls.len())
        .ok_or_else(|| WorkspaceError::Host(Arc::from("call diff count overflow")))?;
    call_sites
        .try_reserve(call_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("call diff allocation failed")))?;
    call_sites.extend(base.indexes.calls.iter().map(|edge| edge.site));
    call_sites.extend(next.calls.iter().map(|edge| edge.site));
    for site in call_sites {
        let old = base
            .indexes
            .calls
            .iter()
            .find(|edge| edge.site == site)
            .map(|edge| edge.callee);
        let new = next
            .calls
            .iter()
            .find(|edge| edge.site == site)
            .map(|edge| edge.callee);
        if old != new {
            entries.push(SemanticDiffEntry::CallRewired {
                site,
                old_callee: old,
                new_callee: new,
            });
        }
    }
    Ok(())
}

fn append_reference_diff(
    old: &[super::ReferenceEdge],
    new: &[super::ReferenceEdge],
    entries: &mut Vec<SemanticDiffEntry>,
) -> Result<(), WorkspaceError> {
    let mut old_index = 0_usize;
    let mut new_index = 0_usize;
    while old_index < old.len() || new_index < new.len() {
        let site = match (old.get(old_index), new.get(new_index)) {
            (Some(old), Some(new)) => old.site.min(new.site),
            (Some(old), None) => old.site,
            (None, Some(new)) => new.site,
            (None, None) => break,
        };
        let old_start = old_index;
        while old.get(old_index).is_some_and(|edge| edge.site == site) {
            old_index += 1;
        }
        let new_start = new_index;
        while new.get(new_index).is_some_and(|edge| edge.site == site) {
            new_index += 1;
        }
        let old_group = &old[old_start..old_index];
        let new_group = &new[new_start..new_index];
        let mut removed = Vec::new();
        let mut added = Vec::new();
        removed
            .try_reserve(old_group.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("reference removal allocation failed")))?;
        added
            .try_reserve(new_group.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("reference addition allocation failed")))?;
        let mut old_target = 0_usize;
        let mut new_target = 0_usize;
        while old_target < old_group.len() || new_target < new_group.len() {
            match (
                old_group.get(old_target).map(|edge| edge.target),
                new_group.get(new_target).map(|edge| edge.target),
            ) {
                (Some(left), Some(right)) if left == right => {
                    old_target += 1;
                    new_target += 1;
                }
                (Some(left), Some(right)) if left < right => {
                    removed.push(left);
                    old_target += 1;
                }
                (Some(_), Some(right)) => {
                    added.push(right);
                    new_target += 1;
                }
                (Some(left), None) => {
                    removed.push(left);
                    old_target += 1;
                }
                (None, Some(right)) => {
                    added.push(right);
                    new_target += 1;
                }
                (None, None) => break,
            }
        }
        let changed = removed.len().max(added.len());
        for index in 0..changed {
            entries.push(SemanticDiffEntry::ReferenceRewired {
                site,
                old_target: removed.get(index).copied(),
                new_target: added.get(index).copied(),
            });
        }
    }
    Ok(())
}

fn append_call_instantiation_diff(
    old: &WorkspaceSnapshot,
    new: &WorkspaceSnapshot,
    entries: &mut Vec<SemanticDiffEntry>,
) -> Result<(), WorkspaceError> {
    for node in old
        .indexes
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Call)
    {
        let Some(new_index) = new.indexes.node_lookup.get(&node.id).copied() else {
            continue;
        };
        if new.indexes.nodes[new_index].kind != NodeKind::Call {
            continue;
        }
        let old_view = old.call_instantiation(old.revision, node.id)?;
        let new_view = new.call_instantiation(new.revision, node.id)?;
        let changed = old_view.callee != new_view.callee
            || old_view.type_arguments != new_view.type_arguments
            || old_view.parameters != new_view.parameters
            || old_view.result != new_view.result
            || old_view.witnesses != new_view.witnesses
            || old_view.effects != new_view.effects;
        if changed {
            entries.try_reserve(1).map_err(|_| {
                WorkspaceError::Host(Arc::from("call instantiation diff allocation failed"))
            })?;
            entries.push(SemanticDiffEntry::CallInstantiationChanged {
                site: node.id,
                old: Box::new(old_view),
                new: Box::new(new_view),
            });
        }
    }
    Ok(())
}

fn coalesce_hole_refinement_entries(
    entries: &mut Vec<SemanticDiffEntry>,
) -> Result<(), WorkspaceError> {
    let refinement_count = entries
        .iter()
        .filter(|entry| matches!(entry, SemanticDiffEntry::HoleRefined { .. }))
        .count();
    if refinement_count == 0 {
        return Ok(());
    }
    if refinement_count == 1 {
        entries.retain(|entry| {
            !matches!(
                entry,
                SemanticDiffEntry::HoleRefined {
                    old_goal,
                    new_goal,
                    ..
                } if old_goal == new_goal
            )
        });
        return Ok(());
    }
    let mut positions = HashMap::<HoleId, usize>::new();
    positions.try_reserve(refinement_count).map_err(|_| {
        WorkspaceError::Host(Arc::from("hole refinement diff map allocation failed"))
    })?;
    let mut coalesced = Vec::<SemanticDiffEntry>::new();
    coalesced.try_reserve(entries.len()).map_err(|_| {
        WorkspaceError::Host(Arc::from("coalesced semantic diff allocation failed"))
    })?;
    for entry in entries.drain(..) {
        let SemanticDiffEntry::HoleRefined {
            hole,
            old_goal,
            new_goal,
        } = entry
        else {
            coalesced.push(entry);
            continue;
        };
        if let Some(index) = positions.get(&hole).copied() {
            let Some(SemanticDiffEntry::HoleRefined {
                new_goal: current_goal,
                ..
            }) = coalesced.get_mut(index)
            else {
                return Err(WorkspaceError::Validation(Arc::from(
                    "hole refinement diff map is inconsistent",
                )));
            };
            *current_goal = new_goal;
        } else {
            positions.insert(hole, coalesced.len());
            coalesced.push(SemanticDiffEntry::HoleRefined {
                hole,
                old_goal,
                new_goal,
            });
        }
    }
    coalesced.retain(|entry| {
        !matches!(
            entry,
            SemanticDiffEntry::HoleRefined {
                old_goal,
                new_goal,
                ..
            } if old_goal == new_goal
        )
    });
    *entries = coalesced;
    Ok(())
}

fn sort_diff_entries(entries: &mut [SemanticDiffEntry]) {
    entries.sort_unstable_by(|left, right| {
        diff_key(left)
            .cmp(&diff_key(right))
            .then_with(|| diff_tie_break(left, right))
    });
}

fn diff_tie_break(left: &SemanticDiffEntry, right: &SemanticDiffEntry) -> std::cmp::Ordering {
    match (left, right) {
        (
            SemanticDiffEntry::HoleRefined {
                old_goal: left_old,
                new_goal: left_new,
                ..
            },
            SemanticDiffEntry::HoleRefined {
                old_goal: right_old,
                new_goal: right_new,
                ..
            },
        ) => left_old
            .cmp(right_old)
            .then_with(|| left_new.cmp(right_new)),
        _ => std::cmp::Ordering::Equal,
    }
}

fn diff_key(entry: &SemanticDiffEntry) -> (u8, u64, u64, u64, u64, u64, u64) {
    let optional_entity = |entity: Option<EntityId>| {
        entity.map_or((u64::MAX, u64::MAX), |entity| {
            (entity.slot(), entity.generation())
        })
    };
    match entry {
        SemanticDiffEntry::EntityCreated { entity, .. } => {
            (0, entity.slot(), entity.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::EntityRenamed { entity, .. } => {
            (1, entity.slot(), entity.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::EntityDeleted { entity, .. } => {
            (2, entity.slot(), entity.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::ExpressionReplaced { node, .. } => {
            (3, node.slot(), node.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::SequenceChildMoved {
            sequence, child, ..
        } => (
            4,
            sequence.slot(),
            sequence.generation(),
            child.slot(),
            child.generation(),
            0,
            0,
        ),
        SemanticDiffEntry::DescendantCreated { node, .. } => {
            (5, node.slot(), node.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::DescendantDeleted { node, .. } => {
            (6, node.slot(), node.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::HoleIntroduced { hole } => {
            (7, hole.0.slot(), hole.0.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::HoleRefined { hole, .. } => {
            (8, hole.0.slot(), hole.0.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::HoleFilled { hole } => {
            (9, hole.0.slot(), hole.0.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::UnresolvedValueReferenceIntroduced { reference } => {
            (10, reference.0.slot(), reference.0.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::UnresolvedValueReferenceResolved { reference, target } => (
            11,
            reference.0.slot(),
            reference.0.generation(),
            target.slot(),
            target.generation(),
            0,
            0,
        ),
        SemanticDiffEntry::ReferenceRewired {
            site,
            old_target,
            new_target,
        } => {
            let old = optional_entity(*old_target);
            let new = optional_entity(*new_target);
            (
                12,
                site.slot(),
                site.generation(),
                old.0,
                old.1,
                new.0,
                new.1,
            )
        }
        SemanticDiffEntry::CallRewired {
            site,
            old_callee,
            new_callee,
        } => {
            let old = optional_entity(*old_callee);
            let new = optional_entity(*new_callee);
            (
                13,
                site.slot(),
                site.generation(),
                old.0,
                old.1,
                new.0,
                new.1,
            )
        }
        SemanticDiffEntry::CallInstantiationChanged { site, .. } => {
            (14, site.slot(), site.generation(), 0, 0, 0, 0)
        }
    }
}
