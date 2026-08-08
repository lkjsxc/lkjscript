use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::hir::{
    Binding, BindingKind, BindingRef, BindingStorage, EffectSet, Expr, ExprKind, Origin, PlaceId,
    Type,
};

use super::identity::{self, IdentityAllocator};
use super::model::{EntityAddress, HoleRecord, NodeAddress, NodeKey, SnapshotIndexes};
use super::program::SemanticProgram;
use super::{
    CompletenessBlocker, DiagnosticHeader, DiagnosticSeverity, DraftNode, EntityId, EntityKind,
    ExpressionDraft, HoleId, HoleKind, HoleState, NodeId, NodeKind, ProgramState, RevisionId,
    SemanticChild, SemanticOwner, WorkspaceError, WorkspaceNamespace, WorkspaceSnapshot,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterDraft {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transaction {
    pub base_revision: RevisionId,
    pub edits: Vec<Edit>,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Edit {
    CreateFunction {
        name: String,
        parameters: Vec<ParameterDraft>,
        return_type: Type,
    },
    CreateMain {
        return_type: Type,
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
        expected_type: Option<Type>,
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
    key: NodeKey,
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

fn stage(
    base: &WorkspaceSnapshot,
    transaction: Transaction,
    allocator: &mut IdentityAllocator,
) -> Result<(WorkspaceSnapshot, SemanticDiff, Vec<InvalidatedDomain>), WorkspaceError> {
    let revision = base.revision.next().map_err(WorkspaceError::from_core)?;
    let edit_count = transaction.edits.len();
    let mut program = try_clone_program(base.program.as_ref())?;
    let mut holes = Vec::new();
    holes
        .try_reserve(base.holes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole staging allocation failed")))?;
    holes.extend(base.holes.iter().cloned());
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
    let mut structural_targets = Vec::new();
    structural_targets
        .try_reserve(edit_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("structural preflight allocation failed")))?;

    for edit in transaction.edits {
        match edit {
            Edit::CreateFunction {
                name,
                parameters,
                return_type,
            } => {
                validate_name(&name)?;
                validate_constructed_type(&return_type, "function return")?;
                if function_name_exists(&program, &name) {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "global declaration name already exists or is reserved",
                    )));
                }
                let mut parameter_names = HashSet::new();
                parameter_names.try_reserve(parameters.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("parameter name allocation failed"))
                })?;
                for parameter in &parameters {
                    validate_name(&parameter.name)?;
                    validate_constructed_type(&parameter.ty, "function parameter")?;
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
                let mut parameter_types = Vec::new();
                parameter_types.try_reserve(parameters.len()).map_err(|_| {
                    WorkspaceError::Host(Arc::from("parameter type allocation failed"))
                })?;
                parameter_types.extend(parameters.iter().map(|parameter| parameter.ty.clone()));
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
                        ty: parameter.ty,
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
                validate_constructed_type(&return_type, "main return")?;
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
            Edit::RenameEntity { entity, new_name } => {
                if !renamed.insert(entity) {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "entity is renamed more than once in one transaction",
                    )));
                }
                validate_name(&new_name)?;
                let header = base.workspace_entity(entity)?;
                if matches!(header.kind, EntityKind::Main | EntityKind::BuiltinOperation) {
                    return Err(WorkspaceError::unsupported(
                        "rename-entity",
                        "main and builtin operations cannot be renamed",
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
                ensure_structural_nonoverlapping(base, &mut structural_targets, target)?;
                let (address, key, expected, visible) = edit_context(base, target)?;
                let replacement = lower_draft(
                    base,
                    &program,
                    &draft,
                    &expected,
                    Origin::Semantic,
                    &visible,
                )?;
                structural.push(StructuralAction {
                    target,
                    address,
                    key,
                    replacement,
                });
            }
            Edit::IntroduceHole { target, goal } => {
                ensure_structural_nonoverlapping(base, &mut structural_targets, target)?;
                if goal.is_empty() {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "typed hole goal must not be empty",
                    )));
                }
                let (address, key, expected, visible) = edit_context(base, target)?;
                let owner = root_owner(base, address)?;
                holes.push(HoleRecord {
                    state: HoleState {
                        id: HoleId(target),
                        kind: HoleKind::TypedExpression,
                        expected_type: expected.clone(),
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
                    key,
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
                if let Some(expected_type) = expected_type {
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
                ensure_structural_nonoverlapping(base, &mut structural_targets, hole.0)?;
                if hole.0.namespace() != base.namespace {
                    return Err(WorkspaceError::ForeignNamespace(Arc::from("hole")));
                }
                let index = holes
                    .iter()
                    .position(|record| record.state.id == hole)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole")))?;
                let record = holes[index].clone();
                let replacement = lower_draft(
                    base,
                    &program,
                    &draft,
                    &record.state.expected_type,
                    Origin::Semantic,
                    &record.state.visible_entities,
                )?;
                structural.push(StructuralAction {
                    target: hole.0,
                    address: record.address,
                    key: record.key,
                    replacement,
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
        let (complete_program, _) = SemanticProgram::from_hir(complete);
        program = complete_program;
    }

    let canonical =
        super::index::build(&program, base.namespace).map_err(WorkspaceError::from_core)?;
    let mut forced = HashMap::new();
    let forced_count = structural
        .len()
        .checked_add(holes.len())
        .ok_or_else(|| WorkspaceError::Host(Arc::from("forced identity count overflow")))?;
    forced
        .try_reserve(forced_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("forced identity allocation failed")))?;
    for action in &structural {
        forced.insert(action.key, action.target);
    }
    for hole in &holes {
        forced.insert(hole.key, hole.state.id.0);
    }
    let mut indexes = identity::reconcile(canonical, &base.indexes, allocator, &forced)
        .map_err(WorkspaceError::from_core)?;

    refresh_hole_addresses(&mut holes, &indexes)?;
    install_new_holes(&mut holes, &new_holes, &indexes)?;
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

fn validate_name(name: &str) -> Result<(), WorkspaceError> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "entity name must be a non-empty ASCII semantic identifier",
        )));
    }
    Ok(())
}

fn validate_constructed_type(ty: &Type, subject: &str) -> Result<(), WorkspaceError> {
    if matches!(ty, Type::I64 | Type::F64 | Type::Bool | Type::Unit) {
        Ok(())
    } else {
        Err(WorkspaceError::unsupported(
            "create-declaration",
            &format!("{subject} type {ty} is outside the implemented scalar construction surface"),
        ))
    }
}

fn function_name_exists(program: &SemanticProgram, name: &str) -> bool {
    function_name_conflicts(program, name, None)
}

fn function_name_conflicts(
    program: &SemanticProgram,
    name: &str,
    except: Option<crate::hir::BindingId>,
) -> bool {
    name == "main"
        || crate::hir::Operation::from_name(name).is_some()
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
    let owner = root_owner(snapshot, address)?;
    let mut visible = Vec::new();
    visible
        .try_reserve(snapshot.indexes.entities.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("visible entity allocation failed")))?;
    for entity in &snapshot.indexes.entities {
        if entity.kind == EntityKind::Function
            || (entity.kind == EntityKind::Parameter && entity.owner == Some(owner))
        {
            visible.push(entity.id);
        }
    }
    visible.sort();
    Ok(visible)
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

fn lower_draft(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    draft: &ExpressionDraft,
    expected: &Type,
    origin: Origin,
    visible: &[EntityId],
) -> Result<Expr, WorkspaceError> {
    validate_draft_shape(draft)?;
    let mut visible_set = HashSet::new();
    visible_set
        .try_reserve(visible.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("draft visibility allocation failed")))?;
    visible_set.extend(visible.iter().copied());
    let mut completed: Vec<Option<Expr>> = Vec::new();
    completed
        .try_reserve(draft.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("draft lowering allocation failed")))?;

    for node in &draft.nodes {
        let expression = match node {
            DraftNode::I64(value) => scalar(Type::I64, ExprKind::LitI64(*value), origin),
            DraftNode::F64(value) => scalar(Type::F64, ExprKind::LitF64(*value), origin),
            DraftNode::Bool(value) => scalar(Type::Bool, ExprKind::LitBool(*value), origin),
            DraftNode::Unit => scalar(Type::Unit, ExprKind::LitUnit, origin),
            DraftNode::Load(entity) => {
                snapshot.workspace_entity(*entity)?;
                if !visible_set.contains(entity) {
                    return Err(WorkspaceError::InvisibleEntity);
                }
                let (binding, slot, ty) = parameter_binding(snapshot, program, *entity)?;
                if !crate::ownership::draft_parameter_load_is_supported(&ty) {
                    return Err(WorkspaceError::unsupported(
                        "load",
                        "owned and affine parameters require move construction, which is not implemented",
                    ));
                }
                Expr {
                    ty,
                    effects: EffectSet::PURE,
                    origin,
                    kind: ExprKind::Load(BindingRef {
                        binding,
                        storage: BindingStorage::Local(slot),
                    }),
                }
            }
            DraftNode::Call { callee, arguments } => {
                snapshot.workspace_entity(*callee)?;
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
        };
        completed.push(Some(expression));
    }
    let root = draft
        .root
        .index()
        .and_then(|index| completed.get_mut(index))
        .and_then(Option::take)
        .ok_or_else(|| WorkspaceError::InvalidDraft(Arc::from("draft root is unavailable")))?;
    require_type(&root.ty, expected)?;
    Ok(root)
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
    let root = draft
        .root
        .index()
        .ok_or_else(|| WorkspaceError::InvalidDraft(Arc::from("draft root exceeds host index")))?;
    if root != draft.nodes.len() - 1 {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "draft root must be the final dense node",
        )));
    }
    let mut parents = Vec::new();
    parents
        .try_reserve(draft.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("draft shape allocation failed")))?;
    parents.resize(draft.nodes.len(), 0_u64);
    for (index, node) in draft.nodes.iter().enumerate() {
        let mut failure = None;
        node.for_each_child(|child| {
            let Some(child_index) = child.index() else {
                failure = Some("draft child exceeds host index");
                return;
            };
            if child_index >= index {
                failure = Some("draft is cyclic or not in child-before-parent order");
                return;
            }
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
    if parents[root] != 0 || parents[..root].iter().any(|count| *count != 1) {
        return Err(WorkspaceError::InvalidDraft(Arc::from(
            "draft must be a dense, fully reachable expression tree",
        )));
    }
    Ok(())
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

fn parameter_binding(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    entity: EntityId,
) -> Result<(crate::hir::BindingId, usize, Type), WorkspaceError> {
    let header = snapshot.workspace_entity(entity)?;
    if header.kind != EntityKind::Parameter {
        return Err(WorkspaceError::unsupported(
            "load",
            "initial draft loads support visible parameters; local storage edits are not implemented",
        ));
    }
    let binding = binding_from_entity(snapshot, program, entity)?;
    let owner = header
        .owner
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("parameter owner")))?;
    let owner_address = snapshot
        .indexes
        .entity_lookup
        .get(&owner)
        .and_then(|index| snapshot.indexes.entity_addresses.get(*index))
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("parameter owner")))?;
    let parameters = if owner_address == EntityAddress::Main {
        &program
            .main
            .as_ref()
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("parameter owner")))?
            .params
    } else {
        let EntityAddress::Binding(raw) = owner_address else {
            return Err(WorkspaceError::StaleIdentity(Arc::from("parameter owner")));
        };
        &program
            .functions
            .iter()
            .find(|function| function.binding.raw() == raw)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("parameter owner")))?
            .params
    };
    let slot = parameters
        .iter()
        .position(|candidate| *candidate == binding)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("parameter")))?;
    let ty = program
        .binding(binding)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("parameter")))?
        .ty
        .clone();
    Ok((binding, slot, ty))
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
        hole.state.visible_entities = hole_visibility(indexes, hole.state.owner)?.into();
    }
    Ok(())
}

fn install_new_holes(
    holes: &mut Vec<HoleRecord>,
    pending: &[NewHole],
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
        let visible = hole_visibility(indexes, owner)?;
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

fn hole_visibility(
    indexes: &SnapshotIndexes,
    owner: EntityId,
) -> Result<Vec<EntityId>, WorkspaceError> {
    let mut visible = Vec::new();
    visible
        .try_reserve(indexes.entities.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole visibility allocation failed")))?;
    for entity in &indexes.entities {
        if entity.kind == EntityKind::Function
            || (entity.kind == EntityKind::Parameter && entity.owner == Some(owner))
        {
            visible.push(entity.id);
        }
    }
    visible.sort();
    Ok(visible)
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
        .nodes
        .len()
        .checked_add(next.nodes.len())
        .and_then(|count| count.checked_add(base.indexes.references.len()))
        .and_then(|count| count.checked_add(next.references.len()))
        .and_then(|count| count.checked_add(base.indexes.calls.len()))
        .and_then(|count| count.checked_add(next.calls.len()))
        .ok_or_else(|| WorkspaceError::Host(Arc::from("semantic diff size overflow")))?;
    entries
        .try_reserve(possible_entries)
        .map_err(|_| WorkspaceError::Host(Arc::from("semantic diff allocation failed")))?;
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
        SemanticDiffEntry::ExpressionReplaced { node, .. } => (2, node.slot(), node.generation()),
        SemanticDiffEntry::DescendantCreated { node, .. } => (3, node.slot(), node.generation()),
        SemanticDiffEntry::DescendantDeleted { node, .. } => (4, node.slot(), node.generation()),
        SemanticDiffEntry::HoleIntroduced { hole } => (5, hole.0.slot(), hole.0.generation()),
        SemanticDiffEntry::HoleRefined { hole, .. } => (6, hole.0.slot(), hole.0.generation()),
        SemanticDiffEntry::HoleFilled { hole } => (7, hole.0.slot(), hole.0.generation()),
        SemanticDiffEntry::ReferenceRewired { site, .. } => (8, site.slot(), site.generation()),
        SemanticDiffEntry::CallRewired { site, .. } => (9, site.slot(), site.generation()),
    }
}
