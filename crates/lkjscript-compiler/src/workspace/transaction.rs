use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::hir::{
    Binding, BindingKind, BindingRef, BindingStorage, EffectSet, Expr, ExprKind, Origin, PlaceId,
    Type,
};

#[cfg(test)]
thread_local! {
    static PATTERN_LOWERING_NODE_VISITS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn reset_pattern_lowering_node_visits() {
    PATTERN_LOWERING_NODE_VISITS.with(|count| count.set(0));
}

#[cfg(test)]
pub(super) fn pattern_lowering_node_visits() -> u64 {
    PATTERN_LOWERING_NODE_VISITS.with(std::cell::Cell::get)
}

use super::identity::{self, IdentityAllocator};
use super::model::{EntityAddress, HoleRecord, NodeAddress, NodeKey, SnapshotIndexes};
use super::program::SemanticProgram;
use super::{
    CompletenessBlocker, DeclarationType, DiagnosticHeader, DiagnosticSeverity, DraftBindingId,
    DraftBindingRef, DraftFieldValue, DraftNode, DraftNodeId, DraftPatternNode, DraftPatternNodeId,
    DraftTypeParameterId, EntityId, EntityKind, ExpressionDraft, HoleId, HoleKind, HoleState,
    NodeId, NodeKind, PatternDraft, ProgramState, RevisionId, SemanticChild, SemanticOwner,
    SemanticTrait, SemanticType, WorkspaceError, WorkspaceNamespace, WorkspaceSnapshot,
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
pub struct EnumFieldDraft {
    pub name: String,
    pub ty: SemanticType,
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
        variants: Vec<EnumVariantDraft>,
    },
    CreateFunction {
        name: String,
        type_parameters: Vec<TypeParameterDraft>,
        parameters: Vec<ParameterDraft>,
        return_type: DeclarationType,
    },
    CreateMain {
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
    IntroduceHole {
        target: NodeId,
        goal: String,
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
    let revision = base.revision.next().map_err(WorkspaceError::from_core)?;
    let edit_count = transaction.edits.len();
    let mut program = try_clone_program(base.program.as_ref())?;
    let deletions = preflight_deletions(base, &program, &transaction.edits)?;
    preflight_structural_edits(base, &transaction.edits)?;
    let deleted_entities = &deletions.entities;
    let deleted_roots = &deletions.callable_roots;
    let deleted_bindings = &deletions.callable_bindings;
    let mut holes = Vec::new();
    holes
        .try_reserve(base.holes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole staging allocation failed")))?;
    holes.extend(base.holes.iter().cloned());
    prune_replaced_subtree_holes(base, &mut holes, &transaction.edits)?;
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
            Edit::CreateEnum { name, variants } => {
                create_enum(
                    base,
                    &mut program,
                    allocator,
                    &mut forced_entities,
                    &mut new_entities,
                    name,
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
            Edit::CreateMain { return_type } => {
                let return_type =
                    resolve_semantic_type(base, &program, return_type, "main return")?;
                reject_reference_result(&return_type, "main")?;
                if program.main.is_some() {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "main entry point already exists",
                    )));
                }
                new_entities.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("created entity allocation failed"))
                })?;
                new_holes.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("created hole allocation failed"))
                })?;
                program.main = Some(crate::hir::Main {
                    origin: Origin::Semantic,
                    params: Vec::new(),
                    param_places: Vec::new(),
                    param_types: Vec::new(),
                    return_type: return_type.clone(),
                    arity: 0,
                    local_count: 0,
                    body: Expr {
                        ty: return_type,
                        effects: EffectSet::UNKNOWN,
                        origin: Origin::Semantic,
                        kind: ExprKind::Hole,
                    },
                });
                new_entities.push(NewEntity {
                    address: EntityAddress::Main,
                    kind: EntityKind::Main,
                    name: Arc::from("main"),
                });
                new_holes.push(NewHole {
                    address: NodeAddress {
                        root: EntityAddress::Main,
                        preorder: 0,
                    },
                    kind: HoleKind::MissingBody,
                    goal: Arc::from("provide the entry-point body"),
                });
            }
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
                validate_name(&new_name)?;
                let header = base.workspace_entity(entity)?;
                if matches!(
                    header.kind,
                    EntityKind::Main
                        | EntityKind::BuiltinOperation
                        | EntityKind::TypeParameter
                        | EntityKind::Product
                        | EntityKind::ProductField
                        | EntityKind::Enum
                        | EntityKind::EnumVariant
                        | EntityKind::EnumField
                ) {
                    return Err(WorkspaceError::unsupported(
                        "rename-entity",
                        "main, builtin operations, and nominal declarations or members cannot be renamed",
                    ));
                }
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
                        address.root,
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
            Edit::RefineHole {
                hole,
                expected_type,
                goal,
            } => {
                if hole.0.namespace() != base.namespace {
                    return Err(WorkspaceError::ForeignNamespace(Arc::from("hole")));
                }
                let record = holes
                    .iter_mut()
                    .find(|record| record.state.id == hole)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole")))?;
                reject_deleted_root_edit(deleted_roots, record.address.root)?;
                if let Some(expected_type) = expected_type {
                    let expected_type =
                        resolve_semantic_type(base, &program, expected_type, "hole expectation")?;
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
                entries.push(SemanticDiffEntry::HoleRefined {
                    hole,
                    old_goal,
                    new_goal: Arc::clone(&record.state.goal),
                });
            }
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
                        record.address.root,
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

    reject_surviving_deleted_dependencies(base, &program, &deletions)?;
    holes.retain(|hole| !deleted_roots.contains(&hole.address.root));
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

    let compaction =
        super::compaction::compact(&mut program, &deletions.products, &deletions.enum_vectors)
            .map_err(WorkspaceError::from_core)?;
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
    install_survivor_entity_relocations(base, &program, &compaction, &mut forced_entities)?;
    reserve_new_entity_identities(base, allocator, &mut forced_entities, &new_entities)?;

    let binding_count = program.bindings.len();
    let main_body = program.main.as_mut().map(|main| &mut main.body);
    crate::effects::infer_partial(binding_count, &mut program.functions, main_body);

    if program.main.is_some() && holes.is_empty() && new_holes.is_empty() {
        let complete = program
            .try_complete(&base.source_origins)
            .map_err(WorkspaceError::from_core)?;
        crate::ownership::check(&complete).map_err(WorkspaceError::from_core)?;
        crate::analyze::verify_match_plans(&complete).map_err(WorkspaceError::from_core)?;
        super::validate::program(&complete).map_err(WorkspaceError::from_core)?;
    }

    let canonical =
        super::index::build(&program, base.namespace).map_err(WorkspaceError::from_core)?;
    let forced = force_surviving_nodes(base, &canonical, &forced_entities, &structural)?;
    let mut indexes = identity::reconcile(
        canonical,
        &base.indexes,
        allocator,
        &forced_entities,
        &forced,
    )
    .map_err(WorkspaceError::from_core)?;

    refresh_hole_addresses(&mut holes, &program, &indexes)?;
    install_new_holes(&mut holes, &new_holes, &program, &indexes)?;
    for pending in &new_holes {
        let node = indexes
            .address_nodes
            .get(&pending.address)
            .copied()
            .ok_or_else(|| WorkspaceError::Validation(Arc::from("new hole identity is missing")))?;
        entries.push(SemanticDiffEntry::HoleIntroduced { hole: HoleId(node) });
    }
    apply_hole_diagnostics(&mut indexes, &holes, program.main.is_none())?;
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
    let blockers = completeness_blockers(&program, &holes);
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
        blockers: blockers.into(),
        allocator: allocator.clone(),
    };
    append_call_instantiation_diff(base, &snapshot, &mut entries)?;
    sort_diff_entries(&mut entries);
    let diff = SemanticDiff {
        base_revision: base.revision,
        revision,
        entries,
    };
    let invalidated = vec![
        InvalidatedDomain::SemanticIndexes,
        InvalidatedDomain::Types,
        InvalidatedDomain::Effects,
        InvalidatedDomain::Ownership,
        InvalidatedDomain::Diagnostics,
        InvalidatedDomain::Executable,
        InvalidatedDomain::Provenance,
    ];
    Ok((snapshot, diff, invalidated))
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
            "a node or hole owned by a deleted declaration cannot be edited in the same transaction",
        )))
    } else {
        Ok(())
    }
}

fn preflight_structural_edits(
    base: &WorkspaceSnapshot,
    edits: &[Edit],
) -> Result<(), WorkspaceError> {
    let mut targets = Vec::new();
    targets
        .try_reserve(edits.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("structural preflight allocation failed")))?;
    for edit in edits {
        let target = match edit {
            Edit::ReplaceExpression { target, .. } | Edit::IntroduceHole { target, .. } => {
                Some(*target)
            }
            Edit::FillHole { hole, .. } => Some(hole.0),
            _ => None,
        };
        if let Some(target) = target {
            ensure_structural_nonoverlapping(base, &mut targets, target)?;
        }
    }
    for edit in edits {
        let Edit::RefineHole { hole, .. } = edit else {
            continue;
        };
        if hole.0.namespace() != base.namespace {
            return Err(WorkspaceError::ForeignNamespace(Arc::from("hole")));
        }
        if !base.holes.iter().any(|record| record.state.id == *hole) {
            return Err(WorkspaceError::StaleIdentity(Arc::from("hole")));
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

fn prune_replaced_subtree_holes(
    base: &WorkspaceSnapshot,
    holes: &mut Vec<HoleRecord>,
    edits: &[Edit],
) -> Result<(), WorkspaceError> {
    let mut roots = HashSet::new();
    roots
        .try_reserve(edits.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole-pruning root allocation failed")))?;
    for edit in edits {
        if let Edit::ReplaceExpression { target, .. } | Edit::IntroduceHole { target, .. } = edit {
            roots.insert(*target);
        }
    }
    if roots.is_empty() || holes.is_empty() {
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
    let mut product_names = HashMap::new();
    product_names
        .try_reserve(program.products.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("product name lookup allocation failed")))?;
    for product in &program.products {
        if product_names
            .insert(product.name.as_str(), product.id)
            .is_some()
        {
            return Err(WorkspaceError::Validation(Arc::from(
                "product declaration name is duplicated",
            )));
        }
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
                    &product_names,
                    &mut blockers,
                )?;
            }
            collect_expression_deletion_blockers(
                program,
                &main.body,
                &owner,
                deletions,
                &product_names,
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
            &product_names,
            &mut blockers,
        )?;
        collect_expression_deletion_blockers(
            program,
            &function.body,
            &owner,
            deletions,
            &product_names,
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
                &product_names,
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
                &product_names,
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
    product_names: &HashMap<&str, lkjscript_core::ProductId>,
    blockers: &mut Vec<DependencyBlocker>,
) -> Result<(), WorkspaceError> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("type dependency work allocation failed")))?;
    pending.push(root);
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Product(name) => {
                if let Some(product) = product_names.get(name.as_str()) {
                    if let Some(requested) = deletions.product_requests.get(product) {
                        push_deletion_blocker(blockers, *requested, owner, category)?;
                    }
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
    product_names: &HashMap<&str, lkjscript_core::ProductId>,
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
            product_names,
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
                            product_names,
                            blockers,
                        )?;
                    }
                    for witness in &instantiation.witnesses {
                        collect_type_deletion_blockers(
                            &witness.ty,
                            owner,
                            "trait witness type",
                            deletions,
                            product_names,
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
                product_names,
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
                collect_match_plan_deletion_blockers(
                    program,
                    *plan,
                    owner,
                    deletions,
                    product_names,
                    blockers,
                )?;
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
    product_names: &HashMap<&str, lkjscript_core::ProductId>,
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
        product_names,
        blockers,
    )?;
    collect_type_deletion_blockers(
        &plan.result_type,
        owner,
        "match result type",
        deletions,
        product_names,
        blockers,
    )?;
    for arm in &plan.arms {
        collect_type_deletion_blockers(
            &arm.body_type,
            owner,
            "match arm type",
            deletions,
            product_names,
            blockers,
        )?;
        collect_pattern_deletion_blockers(&arm.pattern, owner, deletions, product_names, blockers)?;
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
        collect_type_deletion_blockers(
            &local.ty,
            owner,
            "match local type",
            deletions,
            product_names,
            blockers,
        )?;
    }
    Ok(())
}

fn collect_pattern_deletion_blockers(
    root: &crate::hir::MatchPattern,
    owner: &DependencyOwner,
    deletions: &DeletionPlan,
    product_names: &HashMap<&str, lkjscript_core::ProductId>,
    blockers: &mut Vec<DependencyBlocker>,
) -> Result<(), WorkspaceError> {
    let mut pending = Vec::new();
    pending.try_reserve(1).map_err(|_| {
        WorkspaceError::Host(Arc::from("pattern dependency work allocation failed"))
    })?;
    pending.push(root);
    while let Some(pattern) = pending.pop() {
        let ty = pattern.ty();
        collect_type_deletion_blockers(
            &ty,
            owner,
            "match pattern type",
            deletions,
            product_names,
            blockers,
        )?;
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
                            product_names,
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
                            product_names,
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
        let old = stable_owner.and_then(|owner| {
            old_by_key
                .get(&NodeKey {
                    owner,
                    ordinal: canonical.node_keys[index].ordinal,
                })
                .copied()
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

    let mut parameter_names = HashSet::new();
    parameter_names
        .try_reserve(parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("parameter name allocation failed")))?;
    for parameter in &parameters {
        validate_name(&parameter.name)?;
        if !parameter_names.insert(parameter.name.as_str()) {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                "function parameter name is duplicated",
            )));
        }
    }

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

    let mut parameter_bindings = Vec::new();
    let mut parameter_places = Vec::new();
    parameter_bindings
        .try_reserve(parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("parameter binding allocation failed")))?;
    parameter_places
        .try_reserve(parameters.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("parameter place allocation failed")))?;
    for (index, parameter) in parameters.into_iter().enumerate() {
        let raw = u64::try_from(program.bindings.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("binding identity exceeds u64")))?;
        let binding = crate::hir::BindingId::new(raw);
        program.bindings.push(Binding {
            id: binding,
            name: parameter.name,
            kind: BindingKind::Parameter,
            ty: resolved_parameter_types[index].clone(),
            origin: Origin::Semantic,
        });
        parameter_bindings.push(binding);
        parameter_places.push(PlaceId::new(u64::try_from(index).map_err(|_| {
            WorkspaceError::Host(Arc::from("parameter place exceeds u64"))
        })?));
    }
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
                        position: Arc::from("function declaration"),
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
                        position: Arc::from("function declaration"),
                        reason: Arc::from("list type child is incomplete"),
                    })?;
                completed.push(SemanticType::List(Box::new(inner)));
            }
            Work::Function(count) => {
                let result =
                    completed
                        .pop()
                        .ok_or_else(|| WorkspaceError::InvalidSemanticType {
                            position: Arc::from("function declaration"),
                            reason: Arc::from("function type result is incomplete"),
                        })?;
                let split = completed.len().checked_sub(count).ok_or_else(|| {
                    WorkspaceError::InvalidSemanticType {
                        position: Arc::from("function declaration"),
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
            position: Arc::from("function declaration"),
            reason: Arc::from("declaration type omitted its root"),
        })?;
    if completed.is_empty() {
        Ok(result)
    } else {
        Err(WorkspaceError::InvalidSemanticType {
            position: Arc::from("function declaration"),
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
    let created_count = variants.iter().try_fold(1_usize, |count, variant| {
        count
            .checked_add(1)
            .and_then(|count| count.checked_add(variant.fields.len()))
            .ok_or_else(|| WorkspaceError::Host(Arc::from("created enum entity count overflow")))
    })?;
    created
        .try_reserve(created_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("created enum entity allocation failed")))?;
    let raw = u64::try_from(program.enums.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("enum identity exceeds u64")))?;
    let address = EntityAddress::Enum(raw);
    let entity = reserve_forced_entity(allocator, forced, address)?;
    let nominal = entity_derived_identity(b"workspace-enum-nominal-v1", entity);
    let enum_id = crate::hir::EnumId::new(nominal);
    let mut variant_names = HashSet::new();
    variant_names
        .try_reserve(variants.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("enum variant name allocation failed")))?;
    let mut resolved_variants = Vec::new();
    resolved_variants
        .try_reserve(variants.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("enum variant allocation failed")))?;
    created.push(NewEntity {
        address,
        kind: EntityKind::Enum,
        name: Arc::from(name.as_str()),
    });
    for (variant_index, variant) in variants.into_iter().enumerate() {
        validate_name(&variant.name)?;
        if !variant_names.insert(variant.name.clone()) {
            return Err(WorkspaceError::InvalidTransaction(Arc::from(
                "enum variant name is duplicated",
            )));
        }
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
        let mut field_names = HashSet::new();
        field_names
            .try_reserve(variant.fields.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("enum field name allocation failed")))?;
        let mut fields = Vec::new();
        fields
            .try_reserve(variant.fields.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("enum field allocation failed")))?;
        for (field_index, field) in variant.fields.into_iter().enumerate() {
            validate_name(&field.name)?;
            if !field_names.insert(field.name.clone()) {
                return Err(WorkspaceError::InvalidTransaction(Arc::from(
                    "enum field name is duplicated within its variant",
                )));
            }
            let ty = resolve_semantic_type(base, program, field.ty, "enum field")?;
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
    program
        .enums
        .try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("enum allocation failed")))?;
    program.enums.push(crate::hir::EnumDefinition {
        id: enum_id,
        name,
        origin: Origin::Semantic,
        type_parameters: Vec::new(),
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
    function_name_conflicts(program, name, None)
}

fn function_name_conflicts(
    program: &SemanticProgram,
    name: &str,
    except: Option<crate::hir::BindingId>,
) -> bool {
    name == "main"
        || crate::hir::Operation::from_name(name).is_some()
        || crate::analyze::is_reserved_semantic_name(name)
        || program.bindings.iter().any(|binding| {
            binding.kind == BindingKind::Function
                && Some(binding.id) != except
                && binding.name == name
        })
        || program.products.iter().any(|product| product.name == name)
        || program
            .enums
            .iter()
            .any(|enumeration| enumeration.name == name)
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
    if !matches!(
        kind,
        EntityKind::Function
            | EntityKind::Parameter
            | EntityKind::ImmutableLocal
            | EntityKind::StaticBytesLocal
            | EntityKind::MutableLocal
    ) {
        return Err(WorkspaceError::unsupported(
            "rename-entity",
            "this declaration kind is not in the initial editing vertical",
        ));
    }
    let EntityAddress::Binding(raw) = address else {
        return Err(WorkspaceError::StaleIdentity(Arc::from("entity")));
    };
    let index =
        usize::try_from(raw).map_err(|_| WorkspaceError::StaleIdentity(Arc::from("entity")))?;
    let binding = program
        .bindings
        .get(index)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("entity")))?;
    let binding_id = binding.id;
    let is_function = matches!(binding.kind, BindingKind::Function);
    if is_function && function_name_conflicts(program, new_name, Some(binding_id)) {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "global declaration name already exists or is reserved",
        )));
    }
    let binding = &mut program.bindings[index];
    binding.name.clear();
    binding.name.push_str(new_name);
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

fn visible_entities_in(
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

fn replace_expression(
    program: &mut SemanticProgram,
    address: NodeAddress,
    replacement: &Expr,
) -> Result<(), WorkspaceError> {
    let root = if address.root == EntityAddress::Main {
        &mut program
            .main
            .as_mut()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("main root")))?
            .body
    } else {
        let EntityAddress::Binding(raw) = address.root else {
            return Err(WorkspaceError::StaleIdentity(Arc::from("node root")));
        };
        &mut program
            .functions
            .iter_mut()
            .find(|function| function.binding.raw() == raw)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node root")))?
            .body
    };
    let replaced = root
        .try_replaced_preorder(address.preorder, replacement)
        .map_err(WorkspaceError::from_core)?
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node address")))?;
    *root = replaced;
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
    Lower(usize),
}

#[derive(Clone)]
struct ResolvedDraftBinding {
    binding: crate::hir::BindingId,
    slot: usize,
    place: crate::hir::PlaceId,
    ty: Type,
    static_bytes: bool,
}

struct LoweringState {
    root_places: HashMap<EntityAddress, u64>,
    product_names: HashMap<String, lkjscript_core::ProductId>,
    implementation_index:
        HashMap<(crate::hir::TraitId, lkjscript_core::ProductId), crate::hir::ImplId>,
    next_loan: u64,
}

impl LoweringState {
    fn new(program: &SemanticProgram) -> Result<Self, WorkspaceError> {
        let mut next_loan = 0_u64;
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
                crate::hir::for_each_expression_child(expression, &mut |child| pending.push(child));
            }
        }
        let mut product_names = HashMap::new();
        product_names
            .try_reserve(program.products.len())
            .map_err(|_| {
                WorkspaceError::Host(Arc::from("generic product index allocation failed"))
            })?;
        for product in &program.products {
            product_names.insert(product.name.clone(), product.id);
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
            product_names,
            implementation_index,
            next_loan,
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
}

#[allow(clippy::too_many_arguments)]
fn lower_draft(
    snapshot: &WorkspaceSnapshot,
    program: &mut SemanticProgram,
    draft: &ExpressionDraft,
    expected: &Type,
    origin: Origin,
    visible: &[EntityId],
    root: EntityAddress,
    lowering: &mut LoweringState,
    deleting_entities: &HashSet<EntityId>,
) -> Result<LoweredDraft, WorkspaceError> {
    validate_draft_shape(draft)?;
    let order = draft_lowering_actions(draft)?;
    let mut definition_events = draft_definition_events(draft)?;
    validate_draft_binding_scopes(draft)?;
    let mut visible_set = HashSet::new();
    visible_set
        .try_reserve(visible.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("draft visibility allocation failed")))?;
    visible_set.extend(visible.iter().copied());
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
                if !matches!(scrutinee_type, Type::Enum { .. }) {
                    return Err(WorkspaceError::unsupported(
                        "match",
                        "the source-free pattern surface currently supports enum scrutinees",
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
                    root,
                    *reference,
                    &visible_set,
                    &locals,
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
                    root,
                    *reference,
                    &visible_set,
                    &locals,
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
                    root,
                    *reference,
                    &visible_set,
                    &locals,
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
                    return Err(WorkspaceError::InvisibleEntity);
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
                    product_names: &lowering.product_names,
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
                if !source_free_operation_supported(*operation) {
                    return Err(WorkspaceError::unsupported(
                        "operation",
                        "this canonical operation is outside the selected source-free ownership surface",
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
                        static_bytes: info.static_bytes,
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
            DraftNode::ProductValue { product, fields } => {
                lower_product_value(snapshot, program, *product, fields, &mut completed, origin)?
            }
            DraftNode::ProductField { field, value } => {
                lower_product_field(snapshot, program, *field, *value, &mut completed, origin)?
            }
            DraftNode::EnumValue { variant, fields } => {
                lower_enum_value(snapshot, program, *variant, fields, &mut completed, origin)?
            }
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
            for (binding, name) in events {
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
                let raw = u64::try_from(program.bindings.len())
                    .map_err(|_| WorkspaceError::Host(Arc::from("binding identity exceeds u64")))?;
                let hir_binding = crate::hir::BindingId::new(raw);
                let static_bytes = matches!(initializer.kind, ExprKind::LitBytes(_))
                    || matches!(
                        initializer.kind,
                        ExprKind::Load(reference)
                            if program.binding(reference.binding).is_some_and(|item| item.kind == BindingKind::StaticBytesLocal)
                    );
                let info = ResolvedDraftBinding {
                    binding: hir_binding,
                    slot: next_slot,
                    place: lowering.place(program, root)?,
                    ty: initializer.ty.clone(),
                    static_bytes,
                };
                next_slot = next_slot
                    .checked_add(1)
                    .ok_or_else(|| WorkspaceError::Host(Arc::from("local slot count overflow")))?;
                program.bindings.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("local binding allocation failed"))
                })?;
                program.bindings.push(Binding {
                    id: hir_binding,
                    name: name.clone(),
                    kind: if static_bytes {
                        BindingKind::StaticBytesLocal
                    } else {
                        BindingKind::ImmutableLocal
                    },
                    ty: info.ty.clone(),
                    origin,
                });
                if locals.insert(binding, info).is_some() {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "draft binding handle is defined more than once",
                    )));
                }
                entities.push(NewEntity {
                    address: EntityAddress::Binding(raw),
                    kind: if static_bytes {
                        EntityKind::StaticBytesLocal
                    } else {
                        EntityKind::ImmutableLocal
                    },
                    name: Arc::from(name),
                });
            }
        }
    }
    if !definition_events.is_empty() || !locals.is_empty() || !prepared_matches.is_empty() {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "draft binding scope did not close deterministically",
        )));
    }
    let root_expression = draft
        .root
        .index()
        .and_then(|index| completed.get_mut(index))
        .and_then(Option::take)
        .ok_or_else(|| WorkspaceError::InvalidDraft(Arc::from("draft root is unavailable")))?;
    if !Type::unify_assignable(&root_expression.ty, expected) {
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

struct ResolvedPatternVariant {
    ty: Type,
    enum_id: crate::hir::EnumId,
    variant: crate::hir::VariantId,
    layout: crate::hir::RuntimeLayoutId,
    fields: Vec<ResolvedPatternField>,
}

struct ResolvedPatternField {
    name: String,
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
    let mut variants = Vec::new();
    variants
        .try_reserve(draft.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("pattern metadata allocation failed")))?;
    variants.resize_with(draft.nodes.len(), || None);
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
        let DraftPatternNode::EnumVariant { variant, fields } = node else {
            continue;
        };
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
        if !definition.type_parameters.is_empty() {
            return Err(WorkspaceError::unsupported(
                "match-pattern",
                "generic enum match authoring is not implemented",
            ));
        }
        let expected_enum = Type::Enum {
            id: definition.id,
            name: definition.name.clone(),
            arguments: Vec::new(),
        };
        require_type(&ty, &expected_enum)?;
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
        ordered
            .try_reserve(selected.fields.len())
            .map_err(|_| WorkspaceError::Host(Arc::from("enum pattern field allocation failed")))?;
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
            let declared = selected
                .fields
                .get(host_index(field_index, "enum field")?)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum field")))?;
            let slot = ordered
                .get_mut(host_index(field_index, "enum field")?)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum field")))?;
            if slot.replace(field.pattern).is_some() {
                return Err(WorkspaceError::InvalidDraft(Arc::from(
                    "enum pattern field is duplicated",
                )));
            }
            let child = field.pattern.index().ok_or_else(|| {
                WorkspaceError::InvalidDraft(Arc::from("pattern child exceeds host index"))
            })?;
            let expected_slot = expected_types.get_mut(child).ok_or_else(|| {
                WorkspaceError::InvalidDraft(Arc::from("pattern child identity is stale"))
            })?;
            if expected_slot.replace(declared.ty.clone()).is_some() {
                return Err(WorkspaceError::InvalidDraft(Arc::from(
                    "pattern child receives more than one expected type",
                )));
            }
            pending.push(field.pattern);
        }
        let ordered = ordered
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                WorkspaceError::InvalidDraft(Arc::from("enum pattern field is missing"))
            })?;
        let mut resolved_fields = Vec::new();
        resolved_fields
            .try_reserve(selected.fields.len())
            .map_err(|_| {
                WorkspaceError::Host(Arc::from("resolved pattern field allocation failed"))
            })?;
        for (declared, pattern) in selected.fields.iter().zip(ordered) {
            resolved_fields.push(ResolvedPatternField {
                name: declared.name.clone(),
                field_index: declared.source_order,
                ty: declared.ty.clone(),
                projection: None,
                pattern,
            });
        }
        let enum_id = definition.id;
        let selected_id = selected.id;
        let layout = definition.layout.identity;
        for field in &mut resolved_fields {
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
        variants[index] = Some(ResolvedPatternVariant {
            ty,
            enum_id,
            variant: selected_id,
            layout,
            fields: resolved_fields,
        });
    }

    let order = pattern_postorder(draft)?;
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
                    static_bytes: false,
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
            DraftPatternNode::EnumVariant { .. } => {
                let variant = variants[index].take().ok_or_else(|| {
                    WorkspaceError::InvalidDraft(Arc::from("enum pattern metadata is missing"))
                })?;
                let mut fields = Vec::new();
                fields.try_reserve(variant.fields.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("match pattern field allocation failed"))
                })?;
                for field in variant.fields {
                    let child = field.pattern.index().ok_or_else(|| {
                        WorkspaceError::InvalidDraft(Arc::from("pattern child exceeds host index"))
                    })?;
                    let pattern =
                        completed
                            .get_mut(child)
                            .and_then(Option::take)
                            .ok_or_else(|| {
                                WorkspaceError::InvalidDraft(Arc::from(
                                    "pattern child is stale or reused",
                                ))
                            })?;
                    let projection = field.projection;
                    if projection.is_none()
                        != matches!(pattern, crate::hir::MatchPattern::Wildcard { .. })
                    {
                        return Err(WorkspaceError::InvalidDraft(Arc::from(
                            "pattern projection metadata is stale",
                        )));
                    }
                    fields.push(crate::hir::MatchFieldPattern {
                        name: field.name,
                        field_index: field.field_index,
                        projection,
                        pattern,
                    });
                }
                crate::hir::MatchPattern::Variant {
                    ty: variant.ty,
                    enum_id: variant.enum_id,
                    variant: variant.variant,
                    layout: variant.layout,
                    fields,
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

fn pattern_postorder(draft: &PatternDraft) -> Result<Vec<usize>, WorkspaceError> {
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
                let additional = node.child_count().checked_add(1).ok_or_else(|| {
                    WorkspaceError::Host(Arc::from("pattern order work overflow"))
                })?;
                work.try_reserve(additional).map_err(|_| {
                    WorkspaceError::Host(Arc::from("pattern order work allocation failed"))
                })?;
                work.push(Work::Finish(index));
                let mut children = Vec::new();
                children.try_reserve(node.child_count()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("pattern child allocation failed"))
                })?;
                node.for_each_child(|child| children.push(child));
                work.extend(children.into_iter().rev().map(Work::Visit));
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

fn source_free_operation_supported(operation: crate::hir::Operation) -> bool {
    matches!(
        operation,
        crate::hir::Operation::Add
            | crate::hir::Operation::ByteVectorNew
            | crate::hir::Operation::ByteSliceLength
            | crate::hir::Operation::ByteSliceByteAt
            | crate::hir::Operation::BytesLength
            | crate::hir::Operation::ThawBytes
    )
}

fn draft_definition_events(
    draft: &ExpressionDraft,
) -> Result<HashMap<usize, Vec<(DraftBindingId, String)>>, WorkspaceError> {
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
                        .push((binding.binding, binding.name.clone()));
                }
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
        Finish(usize),
    }

    let mut work = Vec::new();
    work.try_reserve(1)
        .map_err(|_| WorkspaceError::Host(Arc::from("draft order allocation failed")))?;
    work.push(Work::Visit(draft.root));
    let mut order = Vec::new();
    let action_capacity = draft
        .nodes
        .iter()
        .try_fold(draft.nodes.len(), |count, node| {
            if let DraftNode::Match { arms, .. } = node {
                count
                    .checked_add(arms.len())
                    .and_then(|value| value.checked_add(1))
                    .ok_or_else(|| WorkspaceError::Host(Arc::from("draft action count overflow")))
            } else {
                Ok(count)
            }
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
            Work::Finish(index) => order.push(DraftLoweringAction::Lower(index)),
            Work::Visit(id) => {
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

fn resolve_draft_binding(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    root: EntityAddress,
    reference: DraftBindingRef,
    visible: &HashSet<EntityId>,
    locals: &HashMap<DraftBindingId, ResolvedDraftBinding>,
) -> Result<ResolvedDraftBinding, WorkspaceError> {
    match reference {
        DraftBindingRef::Local(binding) => locals.get(&binding).cloned().ok_or_else(|| {
            WorkspaceError::InvalidDraft(Arc::from(format!(
                "draft binding handle {} is forward, malformed, or out of scope",
                binding.raw()
            )))
        }),
        DraftBindingRef::Entity(entity) => {
            let header = snapshot.workspace_entity(entity)?;
            if !visible.contains(&entity) {
                return Err(WorkspaceError::InvisibleEntity);
            }
            if !matches!(
                header.kind,
                EntityKind::Parameter | EntityKind::ImmutableLocal | EntityKind::StaticBytesLocal
            ) {
                return Err(wrong_kind(
                    "binding reference",
                    "parameter or immutable local",
                    header.kind,
                ));
            }
            let binding = binding_from_entity(snapshot, program, entity)?;
            let (slot, place) = binding_location(program, root, binding)?;
            let definition = program
                .binding(binding)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("binding")))?;
            Ok(ResolvedDraftBinding {
                binding,
                slot,
                place,
                ty: definition.ty.clone(),
                static_bytes: definition.kind == BindingKind::StaticBytesLocal,
            })
        }
    }
}

fn binding_location(
    program: &SemanticProgram,
    root: EntityAddress,
    binding: crate::hir::BindingId,
) -> Result<(usize, crate::hir::PlaceId), WorkspaceError> {
    let (params, places, expression) = if root == EntityAddress::Main {
        let main = program
            .main
            .as_ref()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("main root")))?;
        (&main.params, &main.param_places, &main.body)
    } else {
        let EntityAddress::Binding(raw) = root else {
            return Err(WorkspaceError::StaleIdentity(Arc::from("expression root")));
        };
        let function = program
            .functions
            .iter()
            .find(|function| function.binding.raw() == raw)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function root")))?;
        (&function.params, &function.param_places, &function.body)
    };
    if let Some(index) = params.iter().position(|candidate| *candidate == binding) {
        let place = places
            .get(index)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("parameter place")))?;
        return Ok((index, place));
    }
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match &expression.kind {
            ExprKind::Let { bindings, .. } => {
                if let Some(local) = bindings.iter().find(|local| local.binding == binding) {
                    return Ok((local.slot, local.place));
                }
            }
            ExprKind::MutableLocal {
                binding: candidate,
                place,
                slot,
                ..
            } if *candidate == binding => return Ok((*slot, *place)),
            ExprKind::Match { plan, .. } => {
                let plan = program
                    .match_plans
                    .get(host_index(plan.raw(), "match plan")?)
                    .filter(|item| item.id == *plan)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("match plan")))?;
                if let Some(local) = match_plan_local(plan, binding)? {
                    return Ok((local.slot, local.place));
                }
            }
            _ => {}
        }
        crate::hir::for_each_expression_child(expression, &mut |child| pending.push(child));
    }
    Err(WorkspaceError::StaleIdentity(Arc::from("local binding")))
}

fn match_plan_local(
    plan: &crate::hir::MatchPlan,
    binding: crate::hir::BindingId,
) -> Result<Option<&crate::hir::MatchLocal>, WorkspaceError> {
    if plan.scrutinee.binding == binding {
        return Ok(Some(&plan.scrutinee));
    }
    let mut pending = Vec::new();
    for arm in &plan.arms {
        pending
            .try_reserve(1)
            .map_err(|_| WorkspaceError::Host(Arc::from("match local lookup allocation failed")))?;
        pending.push(&arm.pattern);
    }
    while let Some(pattern) = pending.pop() {
        match pattern {
            crate::hir::MatchPattern::Binding { local } if local.binding == binding => {
                return Ok(Some(local));
            }
            crate::hir::MatchPattern::Variant { fields, .. }
            | crate::hir::MatchPattern::Product { fields, .. } => {
                for field in fields {
                    if let Some(local) = &field.projection {
                        if local.binding == binding {
                            return Ok(Some(local));
                        }
                    }
                    pending.try_reserve(1).map_err(|_| {
                        WorkspaceError::Host(Arc::from("match local lookup allocation failed"))
                    })?;
                    pending.push(&field.pattern);
                }
            }
            _ => {}
        }
    }
    Ok(None)
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
        ty: Type::Product(definition.name.clone()),
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
    require_type(&value.ty, &Type::Product(definition.name.clone()))?;
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

fn lower_enum_value(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    variant_entity: EntityId,
    fields: &[DraftFieldValue],
    completed: &mut [Option<Expr>],
    origin: Origin,
) -> Result<Expr, WorkspaceError> {
    let header = snapshot.workspace_entity(variant_entity)?;
    if header.kind != EntityKind::EnumVariant {
        return Err(wrong_kind("enum value", "enum variant", header.kind));
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
    if !definition.type_parameters.is_empty() {
        return Err(WorkspaceError::unsupported(
            "enum value",
            "generic enum authoring is not implemented",
        ));
    }
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
        let expected = selected
            .fields
            .get(index)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum field")))?;
        require_type(&value.ty, &expected.ty)?;
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
    Ok(Expr {
        ty: Type::Enum {
            id: definition.id,
            name: definition.name.clone(),
            arguments: Vec::new(),
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
    require_type(
        &value.ty,
        &Type::Enum {
            id: definition.id,
            name: definition.name.clone(),
            arguments: Vec::new(),
        },
    )?;
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
    let mut reached = vec![false; pattern.nodes.len()];
    let mut pending = vec![pattern.root];
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

fn apply_hole_diagnostics(
    indexes: &mut SnapshotIndexes,
    holes: &[HoleRecord],
    missing_entry: bool,
) -> Result<(), WorkspaceError> {
    indexes.diagnostics.clear();
    if missing_entry {
        indexes.diagnostics.push(DiagnosticHeader {
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
        indexes.diagnostics.push(DiagnosticHeader {
            code: Arc::from(code),
            severity: DiagnosticSeverity::Error,
            subject: Some(SemanticChild::Node(hole.state.id.0)),
            message: Arc::from(format!(
                "{label} requires {}: {}",
                hole.state.expected_type, hole.state.goal
            )),
        });
    }
    rebuild_visible_dependencies(indexes)?;
    indexes
        .diagnostics
        .sort_by_key(|diagnostic| diagnostic.subject);
    indexes.rebuild_maps().map_err(WorkspaceError::from_core)
}

fn completeness_blockers(
    program: &SemanticProgram,
    holes: &[HoleRecord],
) -> Vec<CompletenessBlocker> {
    let mut blockers = Vec::new();
    if program.main.is_none() {
        blockers.push(CompletenessBlocker::MissingEntryPoint);
    }
    for hole in holes {
        blockers.push(match hole.state.kind {
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

fn sort_diff_entries(entries: &mut [SemanticDiffEntry]) {
    entries.sort_unstable_by_key(diff_key);
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
        SemanticDiffEntry::DescendantCreated { node, .. } => {
            (4, node.slot(), node.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::DescendantDeleted { node, .. } => {
            (5, node.slot(), node.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::HoleIntroduced { hole } => {
            (6, hole.0.slot(), hole.0.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::HoleRefined { hole, .. } => {
            (7, hole.0.slot(), hole.0.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::HoleFilled { hole } => {
            (8, hole.0.slot(), hole.0.generation(), 0, 0, 0, 0)
        }
        SemanticDiffEntry::ReferenceRewired {
            site,
            old_target,
            new_target,
        } => {
            let old = optional_entity(*old_target);
            let new = optional_entity(*new_target);
            (
                9,
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
                10,
                site.slot(),
                site.generation(),
                old.0,
                old.1,
                new.0,
                new.1,
            )
        }
        SemanticDiffEntry::CallInstantiationChanged { site, .. } => {
            (11, site.slot(), site.generation(), 0, 0, 0, 0)
        }
    }
}
