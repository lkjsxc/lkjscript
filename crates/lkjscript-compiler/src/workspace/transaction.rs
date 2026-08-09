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
    CompletenessBlocker, DiagnosticHeader, DiagnosticSeverity, DraftBindingId, DraftBindingRef,
    DraftFieldValue, DraftNode, DraftNodeId, DraftPatternNode, DraftPatternNodeId, EntityId,
    EntityKind, ExpressionDraft, HoleId, HoleKind, HoleState, NodeId, NodeKind, PatternDraft,
    ProgramState, RevisionId, SemanticChild, SemanticOwner, SemanticTypeRef, WorkspaceError,
    WorkspaceNamespace, WorkspaceSnapshot,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterDraft {
    pub name: String,
    pub ty: SemanticTypeRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductFieldDraft {
    pub name: String,
    pub ty: SemanticTypeRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumFieldDraft {
    pub name: String,
    pub ty: SemanticTypeRef,
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
        parameters: Vec<ParameterDraft>,
        return_type: SemanticTypeRef,
    },
    CreateMain {
        return_type: SemanticTypeRef,
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
        expected_type: Option<SemanticTypeRef>,
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

#[derive(Clone, Copy)]
struct CallableDeletion {
    entity: EntityId,
    address: EntityAddress,
    binding: Option<crate::hir::BindingId>,
}

fn stage(
    base: &WorkspaceSnapshot,
    transaction: Transaction,
    allocator: &mut IdentityAllocator,
) -> Result<(WorkspaceSnapshot, SemanticDiff, Vec<InvalidatedDomain>), WorkspaceError> {
    let revision = base.revision.next().map_err(WorkspaceError::from_core)?;
    let edit_count = transaction.edits.len();
    let mut program = try_clone_program(base.program.as_ref())?;
    let deletions = preflight_callable_deletions(base, &program, &transaction.edits)?;
    preflight_structural_edits(base, &transaction.edits)?;
    let mut deleted_entities = HashSet::new();
    let mut deleted_roots = HashSet::new();
    let mut deleted_bindings = HashSet::new();
    deleted_entities
        .try_reserve(deletions.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("deleted entity set allocation failed")))?;
    deleted_roots
        .try_reserve(deletions.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("deleted root set allocation failed")))?;
    deleted_bindings
        .try_reserve(deletions.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("deleted binding set allocation failed")))?;
    for deletion in &deletions {
        deleted_entities.insert(deletion.entity);
        deleted_roots.insert(deletion.address);
        if let Some(binding) = deletion.binding {
            deleted_bindings.insert(binding);
        }
    }
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
                parameters,
                return_type,
            } => {
                validate_declaration_name(&name)?;
                let return_type =
                    resolve_semantic_type(base, &program, return_type, "function return")?;
                reject_reference_result(&return_type, "function")?;
                if declaration_name_exists(&program, &name) {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "global declaration name already exists or is reserved",
                    )));
                }
                let mut parameter_names = HashSet::new();
                parameter_names.try_reserve(parameters.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("parameter name allocation failed"))
                })?;
                let mut resolved_parameter_types = Vec::new();
                resolved_parameter_types
                    .try_reserve(parameters.len())
                    .map_err(|_| {
                        WorkspaceError::Host(Arc::from("parameter type allocation failed"))
                    })?;
                for parameter in &parameters {
                    validate_name(&parameter.name)?;
                    resolved_parameter_types.push(resolve_semantic_type(
                        base,
                        &program,
                        parameter.ty,
                        "function parameter",
                    )?);
                    if !parameter_names.insert(parameter.name.as_str()) {
                        return Err(WorkspaceError::InvalidTransaction(Arc::from(
                            "function parameter name is duplicated",
                        )));
                    }
                }
                let created_binding_count = parameters.len().checked_add(1).ok_or_else(|| {
                    WorkspaceError::Host(Arc::from("created binding count overflow"))
                })?;
                program
                    .bindings
                    .try_reserve(created_binding_count)
                    .map_err(|_| WorkspaceError::Host(Arc::from("binding allocation failed")))?;
                program
                    .functions
                    .try_reserve(1)
                    .map_err(|_| WorkspaceError::Host(Arc::from("function allocation failed")))?;
                program.global_layout.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("global layout allocation failed"))
                })?;
                new_entities
                    .try_reserve(created_binding_count)
                    .map_err(|_| {
                        WorkspaceError::Host(Arc::from("created entity allocation failed"))
                    })?;
                new_holes.try_reserve(1).map_err(|_| {
                    WorkspaceError::Host(Arc::from("created hole allocation failed"))
                })?;
                let function_raw = u64::try_from(program.bindings.len())
                    .map_err(|_| WorkspaceError::Host(Arc::from("binding identity exceeds u64")))?;
                let function_binding = crate::hir::BindingId::new(function_raw);
                let parameter_types = resolved_parameter_types;
                program.bindings.push(Binding {
                    id: function_binding,
                    name: name.clone(),
                    kind: BindingKind::Function,
                    ty: Type::Fn {
                        params: parameter_types.clone(),
                        ret: Box::new(return_type.clone()),
                    },
                    origin: Origin::Semantic,
                });
                let mut parameter_bindings = Vec::new();
                let mut parameter_places = Vec::new();
                parameter_bindings
                    .try_reserve(parameters.len())
                    .map_err(|_| {
                        WorkspaceError::Host(Arc::from("parameter binding allocation failed"))
                    })?;
                parameter_places
                    .try_reserve(parameters.len())
                    .map_err(|_| {
                        WorkspaceError::Host(Arc::from("parameter place allocation failed"))
                    })?;
                for (index, parameter) in parameters.into_iter().enumerate() {
                    let raw = u64::try_from(program.bindings.len()).map_err(|_| {
                        WorkspaceError::Host(Arc::from("binding identity exceeds u64"))
                    })?;
                    let binding = crate::hir::BindingId::new(raw);
                    program.bindings.push(Binding {
                        id: binding,
                        name: parameter.name.clone(),
                        kind: BindingKind::Parameter,
                        ty: parameter_types[index].clone(),
                        origin: Origin::Semantic,
                    });
                    parameter_bindings.push(binding);
                    parameter_places.push(PlaceId::new(u64::try_from(index).map_err(|_| {
                        WorkspaceError::Host(Arc::from("parameter place exceeds u64"))
                    })?));
                    new_entities.push(NewEntity {
                        address: EntityAddress::Binding(raw),
                        kind: EntityKind::Parameter,
                        name: Arc::from(parameter.name),
                    });
                }
                let root = EntityAddress::Binding(function_raw);
                program.functions.push(crate::hir::Function {
                    binding: function_binding,
                    origin: Origin::Semantic,
                    params: parameter_bindings,
                    param_places: parameter_places,
                    bounds: Vec::new(),
                    arity: parameter_types.len(),
                    local_count: parameter_types.len(),
                    summary: EffectSet::UNKNOWN,
                    body: Expr {
                        ty: return_type.clone(),
                        effects: EffectSet::UNKNOWN,
                        origin: Origin::Semantic,
                        kind: ExprKind::Hole,
                    },
                });
                program.global_layout.push(function_binding);
                new_entities.push(NewEntity {
                    address: root,
                    kind: EntityKind::Function,
                    name: Arc::from(name),
                });
                new_holes.push(NewHole {
                    address: NodeAddress { root, preorder: 0 },
                    kind: HoleKind::MissingBody,
                    goal: Arc::from("provide the function body"),
                });
            }
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
                reject_deleted_root_edit(&deleted_roots, address.root)?;
                let lowered = lower_draft(
                    base,
                    &mut program,
                    &draft,
                    &expected,
                    Origin::Semantic,
                    &visible,
                    address.root,
                    &mut lowering,
                    &deleted_entities,
                )?;
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
                reject_deleted_root_edit(&deleted_roots, address.root)?;
                let owner = root_owner(base, address)?;
                holes.push(HoleRecord {
                    state: HoleState {
                        id: HoleId(target),
                        kind: HoleKind::TypedExpression,
                        expected_type: expected.clone(),
                        expected_semantic_type: super::types::view(
                            &base.program,
                            &base.indexes,
                            &expected,
                        )?,
                        goal: Arc::from(goal),
                        owner,
                        context: target,
                        visible_entities: visible.into(),
                    },
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
                reject_deleted_root_edit(&deleted_roots, record.address.root)?;
                if let Some(expected_type) = expected_type {
                    let expected_type =
                        resolve_semantic_type(base, &program, expected_type, "hole expectation")?;
                    if expected_type != record.state.expected_type {
                        return Err(WorkspaceError::TypeMismatch {
                            expected: Arc::from(record.state.expected_type.to_string()),
                            actual: Arc::from(expected_type.to_string()),
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
                reject_deleted_root_edit(&deleted_roots, record.address.root)?;
                let lowered = crate::stack::grow(|| {
                    lower_draft(
                        base,
                        &mut program,
                        &draft,
                        &record.state.expected_type,
                        Origin::Semantic,
                        &record.state.visible_entities,
                        record.address.root,
                        &mut lowering,
                        &deleted_entities,
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

    reject_surviving_deleted_references(base, &program, &deleted_roots, &deleted_bindings)?;
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

    let binding_map =
        super::compaction::compact(&mut program).map_err(WorkspaceError::from_core)?;
    remap_staged_addresses(&binding_map, &mut new_entities, &mut new_holes)?;
    install_survivor_entity_relocations(base, &program, &binding_map, &mut forced_entities)?;
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
    sort_diff_entries(&mut entries);

    let diff = SemanticDiff {
        base_revision: base.revision,
        revision,
        entries,
    };
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

fn preflight_callable_deletions(
    base: &WorkspaceSnapshot,
    program: &SemanticProgram,
    edits: &[Edit],
) -> Result<Vec<CallableDeletion>, WorkspaceError> {
    let mut result = Vec::new();
    result
        .try_reserve(edits.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("callable deletion allocation failed")))?;
    let mut seen = HashSet::new();
    seen.try_reserve(edits.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("callable deletion set allocation failed")))?;
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
        let index = base
            .indexes
            .entity_lookup
            .get(entity)
            .copied()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("entity")))?;
        let address = *base
            .indexes
            .entity_addresses
            .get(index)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("entity")))?;
        let binding = match header.kind {
            EntityKind::Main if address == EntityAddress::Main && program.main.is_some() => None,
            EntityKind::Function => {
                let EntityAddress::Binding(raw) = address else {
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
                Some(binding.id)
            }
            EntityKind::BuiltinOperation => {
                return Err(WorkspaceError::unsupported(
                    "delete-entity",
                    "fixed compiler operations cannot be deleted",
                ));
            }
            _ => {
                return Err(WorkspaceError::unsupported(
                    "delete-entity",
                    "only main and ordinary function declarations can be deleted directly",
                ));
            }
        };
        result.push(CallableDeletion {
            entity: *entity,
            address,
            binding,
        });
    }
    if result
        .iter()
        .any(|item| item.address == EntityAddress::Main)
        && edits
            .iter()
            .any(|edit| matches!(edit, Edit::CreateMain { .. }))
    {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "main cannot be deleted and created in one transaction",
        )));
    }
    Ok(result)
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

fn reject_surviving_deleted_references(
    base: &WorkspaceSnapshot,
    program: &SemanticProgram,
    deleted_roots: &HashSet<EntityAddress>,
    deleted_bindings: &HashSet<crate::hir::BindingId>,
) -> Result<(), WorkspaceError> {
    if deleted_bindings.is_empty() {
        return Ok(());
    }
    if let Some(main) = &program.main {
        if !deleted_roots.contains(&EntityAddress::Main)
            && expression_references_any(&main.body, deleted_bindings)?
        {
            let dependent = base
                .indexes
                .address_entities
                .get(&EntityAddress::Main)
                .map_or_else(|| "new main".to_owned(), entity_diagnostic_label);
            return Err(WorkspaceError::InvalidTransaction(Arc::from(format!(
                "cannot delete function while surviving callable {dependent} still references it"
            ))));
        }
    }
    for function in &program.functions {
        let root = EntityAddress::Binding(function.binding.raw());
        if deleted_roots.contains(&root) {
            continue;
        }
        if expression_references_any(&function.body, deleted_bindings)? {
            let dependent = base
                .indexes
                .address_entities
                .get(&root)
                .map_or_else(|| "new function".to_owned(), entity_diagnostic_label);
            return Err(WorkspaceError::InvalidTransaction(Arc::from(format!(
                "cannot delete function while surviving callable {dependent} still references it"
            ))));
        }
    }
    Ok(())
}

fn entity_diagnostic_label(entity: &EntityId) -> String {
    format!(
        "entity slot {} generation {}",
        entity.slot(),
        entity.generation()
    )
}

fn expression_references_any(
    root: &Expr,
    deleted: &HashSet<crate::hir::BindingId>,
) -> Result<bool, WorkspaceError> {
    let mut pending = Vec::new();
    pending.try_reserve(1).map_err(|_| {
        WorkspaceError::Host(Arc::from("deleted-reference traversal allocation failed"))
    })?;
    pending.push(root);
    while let Some(expression) = pending.pop() {
        let referenced = match &expression.kind {
            ExprKind::Load(reference)
            | ExprKind::Move {
                binding: reference, ..
            }
            | ExprKind::Borrow {
                binding: reference, ..
            }
            | ExprKind::BorrowBytes {
                binding: reference, ..
            }
            | ExprKind::Call {
                callee: reference, ..
            } => Some(reference.binding),
            ExprKind::SetLocal { target, .. } => Some(*target),
            _ => None,
        };
        if referenced.is_some_and(|binding| deleted.contains(&binding)) {
            return Ok(true);
        }
        let mut allocation_failed = false;
        crate::hir::for_each_expression_child(expression, &mut |child| {
            if allocation_failed {
                return;
            }
            if pending.try_reserve(1).is_err() {
                allocation_failed = true;
            } else {
                pending.push(child);
            }
        });
        if allocation_failed {
            return Err(WorkspaceError::Host(Arc::from(
                "deleted-reference traversal allocation failed",
            )));
        }
    }
    Ok(false)
}

fn remap_staged_addresses(
    bindings: &HashMap<crate::hir::BindingId, crate::hir::BindingId>,
    entities: &mut [NewEntity],
    holes: &mut [NewHole],
) -> Result<(), WorkspaceError> {
    for entity in entities {
        entity.address = remap_entity_address(bindings, entity.address)?;
    }
    for hole in holes {
        hole.address.root = remap_entity_address(bindings, hole.address.root)?;
    }
    Ok(())
}

fn remap_entity_address(
    bindings: &HashMap<crate::hir::BindingId, crate::hir::BindingId>,
    address: EntityAddress,
) -> Result<EntityAddress, WorkspaceError> {
    let EntityAddress::Binding(raw) = address else {
        return Ok(address);
    };
    let old = crate::hir::BindingId::new(raw);
    bindings
        .get(&old)
        .map(|binding| EntityAddress::Binding(binding.raw()))
        .ok_or_else(|| WorkspaceError::Validation(Arc::from("staged binding address was removed")))
}

fn install_survivor_entity_relocations(
    base: &WorkspaceSnapshot,
    program: &SemanticProgram,
    bindings: &HashMap<crate::hir::BindingId, crate::hir::BindingId>,
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
        let relocated = match *address {
            EntityAddress::Main if program.main.is_some() => Some(EntityAddress::Main),
            EntityAddress::Main => None,
            EntityAddress::Binding(raw) => bindings
                .get(&crate::hir::BindingId::new(raw))
                .map(|binding| EntityAddress::Binding(binding.raw())),
            other => Some(other),
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
    ty: SemanticTypeRef,
    subject: &str,
) -> Result<Type, WorkspaceError> {
    Ok(match ty {
        SemanticTypeRef::Unit => Type::Unit,
        SemanticTypeRef::Bool => Type::Bool,
        SemanticTypeRef::I64 => Type::I64,
        SemanticTypeRef::F64 => Type::F64,
        SemanticTypeRef::Bytes => Type::Bytes,
        SemanticTypeRef::ByteVector => Type::ByteVector,
        SemanticTypeRef::ByteSlice => Type::ByteSlice,
        SemanticTypeRef::ByteSliceMut => Type::ByteSliceMut,
        SemanticTypeRef::Product(entity) => {
            let header = base.workspace_entity(entity)?;
            if header.kind != EntityKind::Product {
                return Err(wrong_kind(subject, "product declaration", header.kind));
            }
            let address = entity_address(base, entity)?;
            let EntityAddress::Product(raw) = address else {
                return Err(WorkspaceError::StaleIdentity(Arc::from("product")));
            };
            let definition = program
                .products
                .get(host_index(raw, "product")?)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product")))?;
            Type::Product(definition.name.clone())
        }
        SemanticTypeRef::Enum(entity) => {
            let header = base.workspace_entity(entity)?;
            if header.kind != EntityKind::Enum {
                return Err(wrong_kind(subject, "enum declaration", header.kind));
            }
            let address = entity_address(base, entity)?;
            let EntityAddress::Enum(raw) = address else {
                return Err(WorkspaceError::StaleIdentity(Arc::from("enum")));
            };
            let definition = program
                .enums
                .get(host_index(raw, "enum")?)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("enum")))?;
            if !definition.type_parameters.is_empty() {
                return Err(WorkspaceError::unsupported(
                    "construct-type",
                    "generic enum authoring is not implemented",
                ));
            }
            Type::Enum {
                id: definition.id,
                name: definition.name.clone(),
                arguments: Vec::new(),
            }
        }
    })
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
            || (entity.kind == EntityKind::Parameter && entity.owner == Some(owner))
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
        Ok(Self {
            root_places: HashMap::new(),
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
                    return Err(WorkspaceError::TypeMismatch {
                        expected: Arc::from(Type::ByteVector.to_string()),
                        actual: Arc::from(resolved.ty.to_string()),
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
            DraftNode::Call { callee, arguments } => {
                snapshot.workspace_entity(*callee)?;
                if deleting_entities.contains(callee) {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "a newly lowered call cannot target a function deleted by the transaction",
                    )));
                }
                if !visible_set.contains(callee) {
                    return Err(WorkspaceError::InvisibleEntity);
                }
                let (binding, parameters, result, summary) =
                    callable_binding(snapshot, program, *callee)?;
                if parameters.len() != arguments.len() {
                    return Err(WorkspaceError::InvalidDraft(Arc::from(
                        "draft call arity does not match callee signature",
                    )));
                }
                let mut args = Vec::new();
                args.try_reserve(arguments.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("draft argument allocation failed"))
                })?;
                let mut effects = summary;
                for (argument, parameter) in arguments.iter().zip(&parameters) {
                    let value = take_draft_child(&mut completed, *argument)?;
                    require_type(&value.ty, parameter)?;
                    effects = effects.union(value.effects);
                    args.push(value);
                }
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
                        instantiation: None,
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
    require_type(&root_expression.ty, expected)?;
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

fn callable_binding(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    entity: EntityId,
) -> Result<(crate::hir::BindingId, Vec<Type>, Type, EffectSet), WorkspaceError> {
    let header = snapshot.workspace_entity(entity)?;
    if header.kind != EntityKind::Function {
        return Err(WorkspaceError::unsupported(
            "call",
            "initial draft calls support non-generic visible functions only",
        ));
    }
    let binding = binding_from_entity(snapshot, program, entity)?;
    let definition = program
        .binding(binding)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function")))?;
    let Type::Fn { params, ret } = &definition.ty else {
        return Err(WorkspaceError::unsupported(
            "call",
            "generic and non-function binding calls are not implemented",
        ));
    };
    let summary = program
        .functions
        .iter()
        .find(|function| function.binding == binding)
        .map(|function| function.summary)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("function")))?;
    Ok((binding, params.clone(), ret.as_ref().clone(), summary))
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
        actual: Arc::from(entity_kind_name(actual)),
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

fn require_type(actual: &Type, expected: &Type) -> Result<(), WorkspaceError> {
    if Type::unify_assignable(actual, expected) {
        Ok(())
    } else {
        Err(WorkspaceError::TypeMismatch {
            expected: Arc::from(expected.to_string()),
            actual: Arc::from(actual.to_string()),
        })
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
        hole.state.expected_semantic_type =
            super::types::view(program, indexes, &hole.state.expected_type)?;
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
                expected_semantic_type: super::types::view(program, indexes, &expected_type)?,
                expected_type,
                goal: Arc::clone(&hole.goal),
                owner,
                context: node,
                visible_entities: visible.into(),
            },
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
        .checked_add(indexes.references.len())
        .ok_or_else(|| WorkspaceError::Host(Arc::from("visible dependency count overflow")))?;
    dependencies
        .try_reserve(capacity)
        .map_err(|_| WorkspaceError::Host(Arc::from("visible dependency allocation failed")))?;
    dependencies.extend(indexes.declaration_dependencies.iter().copied());

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
    let mut sites = HashSet::new();
    let reference_count = base
        .indexes
        .references
        .len()
        .checked_add(next.references.len())
        .ok_or_else(|| WorkspaceError::Host(Arc::from("reference diff count overflow")))?;
    sites
        .try_reserve(reference_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("reference diff allocation failed")))?;
    sites.extend(base.indexes.references.iter().map(|edge| edge.site));
    sites.extend(next.references.iter().map(|edge| edge.site));
    for site in sites {
        let old = base
            .indexes
            .references
            .iter()
            .find(|edge| edge.site == site)
            .map(|edge| edge.target);
        let new = next
            .references
            .iter()
            .find(|edge| edge.site == site)
            .map(|edge| edge.target);
        if old != new {
            entries.push(SemanticDiffEntry::ReferenceRewired {
                site,
                old_target: old,
                new_target: new,
            });
        }
    }
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

fn sort_diff_entries(entries: &mut [SemanticDiffEntry]) {
    entries.sort_by_key(diff_key);
}

fn diff_key(entry: &SemanticDiffEntry) -> (u8, u64, u64) {
    match entry {
        SemanticDiffEntry::EntityCreated { entity, .. } => (0, entity.slot(), entity.generation()),
        SemanticDiffEntry::EntityRenamed { entity, .. } => (1, entity.slot(), entity.generation()),
        SemanticDiffEntry::EntityDeleted { entity, .. } => (2, entity.slot(), entity.generation()),
        SemanticDiffEntry::ExpressionReplaced { node, .. } => (3, node.slot(), node.generation()),
        SemanticDiffEntry::DescendantCreated { node, .. } => (4, node.slot(), node.generation()),
        SemanticDiffEntry::DescendantDeleted { node, .. } => (5, node.slot(), node.generation()),
        SemanticDiffEntry::HoleIntroduced { hole } => (6, hole.0.slot(), hole.0.generation()),
        SemanticDiffEntry::HoleRefined { hole, .. } => (7, hole.0.slot(), hole.0.generation()),
        SemanticDiffEntry::HoleFilled { hole } => (8, hole.0.slot(), hole.0.generation()),
        SemanticDiffEntry::ReferenceRewired { site, .. } => (9, site.slot(), site.generation()),
        SemanticDiffEntry::CallRewired { site, .. } => (10, site.slot(), site.generation()),
    }
}
