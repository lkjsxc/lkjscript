use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::hir::{
    BindingKind, BindingRef, BindingStorage, EffectSet, Expr, ExprKind, Program, Type,
};

use super::identity::{self, IdentityAllocator};
use super::model::{EntityAddress, HoleOverlay, NodeAddress, NodeKey, SnapshotIndexes};
use super::{
    DiagnosticHeader, DiagnosticSeverity, DraftNode, EntityId, EntityKind, ExpressionDraft, HoleId,
    HoleState, NodeId, NodeKind, ProgramState, RevisionId, SemanticChild, SemanticOwner,
    WorkspaceError, WorkspaceSnapshot,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Transaction {
    pub base_revision: RevisionId,
    pub edits: Vec<Edit>,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Edit {
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
    pub fn new(snapshot: WorkspaceSnapshot) -> Result<Self, WorkspaceError> {
        let allocator = IdentityAllocator::from_indexes(snapshot.namespace, &snapshot.indexes)
            .map_err(WorkspaceError::from_core)?;
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

fn stage(
    base: &WorkspaceSnapshot,
    transaction: Transaction,
    allocator: &mut IdentityAllocator,
) -> Result<(WorkspaceSnapshot, SemanticDiff, Vec<InvalidatedDomain>), WorkspaceError> {
    let revision = base.revision.next().map_err(WorkspaceError::from_core)?;
    let edit_count = transaction.edits.len();
    let mut program = try_clone_program(base.hir.as_ref())?;
    let mut holes = Vec::new();
    holes
        .try_reserve(base.holes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole staging allocation failed")))?;
    holes.extend(base.holes.iter().cloned());
    let mut structural = Vec::new();
    structural
        .try_reserve(edit_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("structural edit allocation failed")))?;
    let mut entries = Vec::new();
    entries
        .try_reserve(edit_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("semantic diff allocation failed")))?;
    let mut renamed = HashSet::new();
    renamed
        .try_reserve(edit_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("rename preflight allocation failed")))?;
    let mut structural_targets = HashSet::new();
    structural_targets
        .try_reserve(edit_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("structural preflight allocation failed")))?;

    for edit in transaction.edits {
        match edit {
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
                ensure_structural_once(&mut structural_targets, target)?;
                let (address, key, expected, origin, visible) = edit_context(base, target)?;
                let replacement = lower_draft(base, &program, &draft, &expected, origin, &visible)?;
                structural.push(StructuralAction {
                    target,
                    address,
                    key,
                    replacement,
                });
            }
            Edit::IntroduceHole { target, goal } => {
                ensure_structural_once(&mut structural_targets, target)?;
                if goal.is_empty() {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "typed hole goal must not be empty",
                    )));
                }
                let (address, key, expected, _origin, visible) = edit_context(base, target)?;
                let owner = root_owner(base, address)?;
                holes.push(HoleOverlay {
                    state: HoleState {
                        id: HoleId(target),
                        expected_type: expected,
                        goal: Arc::from(goal),
                        owner,
                        context: target,
                        visible_entities: visible.into(),
                    },
                    address,
                    key,
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
                let overlay = holes
                    .iter_mut()
                    .find(|overlay| overlay.state.id == hole)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole")))?;
                if let Some(expected_type) = expected_type {
                    if expected_type != overlay.state.expected_type {
                        return Err(WorkspaceError::TypeMismatch {
                            expected: Arc::from(overlay.state.expected_type.to_string()),
                            actual: Arc::from(expected_type.to_string()),
                        });
                    }
                }
                if goal.is_empty() {
                    return Err(WorkspaceError::InvalidTransaction(Arc::from(
                        "typed hole goal must not be empty",
                    )));
                }
                let old_goal = Arc::clone(&overlay.state.goal);
                overlay.state.goal = Arc::from(goal);
                entries.push(SemanticDiffEntry::HoleRefined {
                    hole,
                    old_goal,
                    new_goal: Arc::clone(&overlay.state.goal),
                });
            }
            Edit::FillHole { hole, draft } => {
                ensure_structural_once(&mut structural_targets, hole.0)?;
                if hole.0.namespace() != base.namespace {
                    return Err(WorkspaceError::ForeignNamespace(Arc::from("hole")));
                }
                let index = holes
                    .iter()
                    .position(|overlay| overlay.state.id == hole)
                    .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole")))?;
                let overlay = holes[index].clone();
                let origin = expression_at(&program, overlay.address)?.origin;
                let replacement = lower_draft(
                    base,
                    &program,
                    &draft,
                    &overlay.state.expected_type,
                    origin,
                    &overlay.state.visible_entities,
                )?;
                structural.push(StructuralAction {
                    target: hole.0,
                    address: overlay.address,
                    key: overlay.key,
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
            .0
            .cmp(&left.address.root.0)
            .then_with(|| right.address.preorder.cmp(&left.address.preorder))
    });
    for action in &structural {
        replace_expression(&mut program, action.address, &action.replacement)?;
    }

    crate::effects::infer(&mut program);
    crate::ownership::check(&program).map_err(WorkspaceError::from_core)?;
    crate::analyze::verify_match_plans(&program).map_err(WorkspaceError::from_core)?;
    super::validate::program(&program).map_err(WorkspaceError::from_core)?;

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
    apply_hole_overlay(&mut indexes, &holes, allocator)?;
    append_structural_diff(base, &indexes, &structural, &mut entries)?;
    append_graph_diff(base, &indexes, &mut entries)?;
    sort_diff_entries(&mut entries);

    let diff = SemanticDiff {
        base_revision: base.revision,
        revision,
        entries,
    };
    let semantic_digest = semantic_diff_digest(base.semantic_digest, &diff)?;
    let state = if holes.is_empty() {
        ProgramState::Complete
    } else {
        ProgramState::Incomplete
    };
    let snapshot = WorkspaceSnapshot {
        namespace: base.namespace,
        revision,
        state,
        hir: Arc::new(program),
        provenance: Arc::new(super::CapturedCompilationProvenance::edited(
            semantic_digest,
        )),
        semantic_digest,
        attachments: None,
        indexes: Arc::new(indexes),
        holes: holes.into(),
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

fn try_clone_program(program: &Program) -> Result<Program, WorkspaceError> {
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
    Ok(Program {
        sources: try_clone_values(&program.sources, "source")?,
        bindings: try_clone_values(&program.bindings, "binding")?,
        products: try_clone_values(&program.products, "product")?,
        enums: try_clone_values(&program.enums, "enum")?,
        traits: try_clone_values(&program.traits, "trait")?,
        implementations: try_clone_values(&program.implementations, "implementation")?,
        match_plans: try_clone_values(&program.match_plans, "match plan")?,
        functions,
        main: crate::hir::Main {
            origin: program.main.origin,
            params: try_clone_values(&program.main.params, "main parameter")?,
            param_places: try_clone_values(&program.main.param_places, "main place")?,
            param_types: try_clone_values(&program.main.param_types, "main parameter type")?,
            return_type: program.main.return_type.clone(),
            arity: program.main.arity,
            local_count: program.main.local_count,
            body: program
                .main
                .body
                .try_clone()
                .map_err(WorkspaceError::from_core)?,
        },
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

fn ensure_structural_once(
    targets: &mut HashSet<NodeId>,
    target: NodeId,
) -> Result<(), WorkspaceError> {
    if !targets.insert(target) {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "expression is structurally edited more than once in one transaction",
        )));
    }
    Ok(())
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

fn rename_entity(
    program: &mut Program,
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
    let raw = address
        .0
        .checked_sub(1)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("entity")))?;
    let index =
        usize::try_from(raw).map_err(|_| WorkspaceError::StaleIdentity(Arc::from("entity")))?;
    let binding = program
        .bindings
        .get(index)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("entity")))?;
    let binding_id = binding.id;
    let is_function = matches!(binding.kind, BindingKind::Function);
    if is_function
        && program.bindings.iter().any(|candidate| {
            matches!(candidate.kind, BindingKind::Function)
                && candidate.id != binding_id
                && candidate.name == new_name
        })
    {
        return Err(WorkspaceError::InvalidTransaction(Arc::from(
            "function name already exists",
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
) -> Result<
    (
        NodeAddress,
        NodeKey,
        Type,
        crate::hir::SourceId,
        Vec<EntityId>,
    ),
    WorkspaceError,
> {
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
    let expression = expression_at(snapshot.hir(), address)?;
    let visible = visible_entities(snapshot, address)?;
    Ok((
        address,
        key,
        base_expected_type(snapshot, index, expression)?,
        expression.origin,
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

fn expression_at(program: &Program, address: NodeAddress) -> Result<&Expr, WorkspaceError> {
    expression_root(program, address.root)?
        .try_at_preorder(address.preorder)
        .map_err(WorkspaceError::from_core)?
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node address")))
}

fn expression_root(program: &Program, address: EntityAddress) -> Result<&Expr, WorkspaceError> {
    if address.0 == 0 {
        return Ok(&program.main.body);
    }
    let binding_raw = address
        .0
        .checked_sub(1)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node root")))?;
    program
        .functions
        .iter()
        .find(|function| function.binding.raw() == binding_raw)
        .map(|function| &function.body)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node root")))
}

fn replace_expression(
    program: &mut Program,
    address: NodeAddress,
    replacement: &Expr,
) -> Result<(), WorkspaceError> {
    let root = if address.root.0 == 0 {
        &mut program.main.body
    } else {
        let raw = address
            .root
            .0
            .checked_sub(1)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("node root")))?;
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
    program: &Program,
    draft: &ExpressionDraft,
    expected: &Type,
    origin: crate::hir::SourceId,
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
                if !visible_set.contains(entity) {
                    return Err(WorkspaceError::InvisibleEntity);
                }
                let (binding, slot, ty) = parameter_binding(snapshot, program, *entity)?;
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
            DraftNode::Store { .. } => {
                return Err(WorkspaceError::unsupported(
                    "storage",
                    "storage creation is not in the initial semantic editing vertical",
                ));
            }
            DraftNode::GenericCall { .. } => {
                return Err(WorkspaceError::unsupported(
                    "generic-call",
                    "generic call creation is not in the initial semantic editing vertical",
                ));
            }
            DraftNode::Match { .. } => {
                return Err(WorkspaceError::unsupported(
                    "match",
                    "match creation is not in the initial semantic editing vertical",
                ));
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

fn scalar(ty: Type, kind: ExprKind, origin: crate::hir::SourceId) -> Expr {
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
    program: &Program,
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
    let parameters = if owner_address.0 == 0 {
        &program.main.params
    } else {
        let raw = owner_address
            .0
            .checked_sub(1)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("parameter owner")))?;
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
    program: &Program,
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
    program: &Program,
    entity: EntityId,
) -> Result<crate::hir::BindingId, WorkspaceError> {
    let address = snapshot
        .indexes
        .entity_lookup
        .get(&entity)
        .and_then(|index| snapshot.indexes.entity_addresses.get(*index))
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("binding")))?;
    let raw = address
        .0
        .checked_sub(1)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("binding")))?;
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
    holes: &mut [HoleOverlay],
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
    }
    Ok(())
}

fn apply_hole_overlay(
    indexes: &mut SnapshotIndexes,
    holes: &[HoleOverlay],
    allocator: &mut IdentityAllocator,
) -> Result<(), WorkspaceError> {
    let mut removed = HashSet::new();
    removed
        .try_reserve(indexes.nodes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole descendant allocation failed")))?;
    let mut roots = HashSet::new();
    roots
        .try_reserve(holes.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole root allocation failed")))?;
    roots.extend(holes.iter().map(|hole| hole.state.id.0));
    for header in &indexes.nodes {
        if let SemanticOwner::Node(parent) = header.owner {
            if roots.contains(&parent) || removed.contains(&parent) {
                removed.insert(header.id);
            }
        }
    }
    for id in &removed {
        allocator
            .tombstone_node(*id)
            .map_err(WorkspaceError::from_core)?;
    }

    let mut retained = Vec::new();
    let retained_count = indexes
        .nodes
        .len()
        .checked_sub(removed.len())
        .ok_or_else(|| WorkspaceError::Validation(Arc::from("hole removal set is inconsistent")))?;
    retained
        .try_reserve(retained_count)
        .map_err(|_| WorkspaceError::Host(Arc::from("hole node retention allocation failed")))?;
    for index in 0..indexes.nodes.len() {
        if !removed.contains(&indexes.nodes[index].id) {
            retained.push(index);
        }
    }
    let mut nodes = Vec::new();
    let mut addresses = Vec::new();
    let mut keys = Vec::new();
    let mut fingerprints = Vec::new();
    let mut expected_types = Vec::new();
    nodes
        .try_reserve(retained.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole node allocation failed")))?;
    addresses
        .try_reserve(retained.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole address allocation failed")))?;
    keys.try_reserve(retained.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole key allocation failed")))?;
    fingerprints
        .try_reserve(retained.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole fingerprint allocation failed")))?;
    expected_types
        .try_reserve(retained.len())
        .map_err(|_| WorkspaceError::Host(Arc::from("hole expectation allocation failed")))?;
    for index in retained {
        nodes.push(indexes.nodes[index].clone());
        addresses.push(indexes.node_addresses[index]);
        keys.push(indexes.node_keys[index]);
        fingerprints.push(indexes.node_fingerprints[index]);
        expected_types.push(indexes.node_expected_types[index].clone());
    }
    indexes.nodes = nodes;
    indexes.node_addresses = addresses;
    indexes.node_keys = keys;
    indexes.node_fingerprints = fingerprints;
    indexes.node_expected_types = expected_types;
    indexes.containment.retain(|edge| match edge.child {
        SemanticChild::Node(node) => !removed.contains(&node),
        SemanticChild::Entity(_) => true,
    });
    indexes.containment.retain(|edge| match edge.owner {
        SemanticOwner::Node(node) => !removed.contains(&node),
        SemanticOwner::Entity(_) => true,
    });
    indexes
        .references
        .retain(|edge| !removed.contains(&edge.site));
    indexes.calls.retain(|edge| !removed.contains(&edge.site));
    indexes.diagnostics.clear();
    for hole in holes {
        let index = indexes
            .nodes
            .iter()
            .position(|header| header.id == hole.state.id.0)
            .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("hole root")))?;
        indexes.nodes[index].kind = NodeKind::Hole;
        indexes.nodes[index].actual_type = Arc::from(hole.state.expected_type.to_string());
        indexes.nodes[index].expected_type = Some(Arc::from(hole.state.expected_type.to_string()));
        indexes.node_expected_types[index] = Some(hole.state.expected_type.clone());
        indexes
            .references
            .retain(|edge| edge.site != hole.state.id.0);
        indexes.calls.retain(|edge| edge.site != hole.state.id.0);
        indexes.diagnostics.push(DiagnosticHeader {
            code: Arc::from("workspace.typed-hole"),
            severity: DiagnosticSeverity::Error,
            subject: Some(SemanticChild::Node(hole.state.id.0)),
            message: Arc::from(format!(
                "typed hole requires {}: {}",
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
        SemanticDiffEntry::EntityRenamed { entity, .. } => (0, entity.slot(), entity.generation()),
        SemanticDiffEntry::ExpressionReplaced { node, .. } => (1, node.slot(), node.generation()),
        SemanticDiffEntry::DescendantCreated { node, .. } => (2, node.slot(), node.generation()),
        SemanticDiffEntry::DescendantDeleted { node, .. } => (3, node.slot(), node.generation()),
        SemanticDiffEntry::HoleIntroduced { hole } => (4, hole.0.slot(), hole.0.generation()),
        SemanticDiffEntry::HoleRefined { hole, .. } => (5, hole.0.slot(), hole.0.generation()),
        SemanticDiffEntry::HoleFilled { hole } => (6, hole.0.slot(), hole.0.generation()),
        SemanticDiffEntry::ReferenceRewired { site, .. } => (7, site.slot(), site.generation()),
        SemanticDiffEntry::CallRewired { site, .. } => (8, site.slot(), site.generation()),
    }
}

fn semantic_diff_digest(base: [u8; 32], diff: &SemanticDiff) -> Result<[u8; 32], WorkspaceError> {
    let mut capacity = 77_usize;
    for entry in &diff.entries {
        capacity = capacity
            .checked_add(17)
            .ok_or_else(|| WorkspaceError::Host(Arc::from("semantic digest size overflow")))?;
        match entry {
            SemanticDiffEntry::EntityRenamed {
                old_name, new_name, ..
            } => {
                capacity = capacity
                    .checked_add(old_name.len())
                    .and_then(|value| value.checked_add(new_name.len()))
                    .and_then(|value| value.checked_add(1))
                    .ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("semantic rename digest size overflow"))
                    })?;
            }
            SemanticDiffEntry::HoleRefined {
                old_goal, new_goal, ..
            } => {
                capacity = capacity
                    .checked_add(old_goal.len())
                    .and_then(|value| value.checked_add(new_goal.len()))
                    .and_then(|value| value.checked_add(1))
                    .ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("semantic hole digest size overflow"))
                    })?;
            }
            _ => {}
        }
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| WorkspaceError::Host(Arc::from("semantic digest allocation failed")))?;
    bytes.extend_from_slice(b"lkjscript.semantic-workspace-edit.v1");
    bytes.extend_from_slice(&base);
    bytes.extend_from_slice(&diff.revision.sequence().to_be_bytes());
    for entry in &diff.entries {
        let (tag, slot, generation) = diff_key(entry);
        bytes.push(tag);
        bytes.extend_from_slice(&slot.to_be_bytes());
        bytes.extend_from_slice(&generation.to_be_bytes());
        match entry {
            SemanticDiffEntry::EntityRenamed {
                old_name, new_name, ..
            } => {
                bytes.extend_from_slice(old_name.as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(new_name.as_bytes());
            }
            SemanticDiffEntry::HoleRefined {
                old_goal, new_goal, ..
            } => {
                bytes.extend_from_slice(old_goal.as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(new_goal.as_bytes());
            }
            _ => {}
        }
    }
    Ok(lkjscript_core::sha256(&bytes))
}
