//! Exact-base three-way semantic merge over stable graph identities.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::graph::{
    DependencyBinding, GraphRoot, ModuleObjectRef, TargetBinding, Tombstone, TombstoneIdentity,
};
use super::language::{Declaration, Import};
use super::meaning::{
    Annotation, DeclarationIdentity, Documentation, GRAPH_CONTRACT_VERSION, MeaningModule,
};
use super::repository::{
    MAXIMUM_HISTORY_ITEMS, PublicationOutcome, PublicationProposal, RevisionSnapshot,
    SemanticRepository,
};
use super::revision::{
    AffectedOwner, ParentRevision, REVISION_CONTRACT_VERSION, ReceiptStatus, RevisionCore,
    TransactionReceipt,
};
use super::semantic_diff::semantic_diff_digest;
use super::semantic_digest::{ModuleObjectDigest, SemanticDiffDigest, TransactionDigest};
use super::semantic_id::{ConflictId, DeclarationId, ModuleId, RevisionId, TargetId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SEMANTIC_MERGE_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_MERGE_CONFLICTS: usize = 10_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticMergeRequest {
    pub contract_version: u16,
    pub base_revision: RevisionId,
    pub left_revision: RevisionId,
    pub right_revision: RevisionId,
    #[serde(default = "default_merge_work")]
    pub maximum_work: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticMergeConflictKind {
    ConcurrentChange,
    ConcurrentCreation,
    DeleteModify,
    MissingModule,
    InvalidMergedGraph,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticMergeConflict {
    pub id: ConflictId,
    pub kind: SemanticMergeConflictKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    pub summary: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticMergeStatus {
    Ready,
    Conflicted,
    AcceptedChange,
    SemanticNoChange,
    StaleHead,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticMergeResult {
    pub contract_version: u16,
    pub base_revision: RevisionId,
    pub left_revision: RevisionId,
    pub right_revision: RevisionId,
    pub observed_current: RevisionId,
    pub status: SemanticMergeStatus,
    pub transaction: TransactionDigest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diff: Option<SemanticDiffDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predicted_revision: Option<RevisionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_revision: Option<RevisionId>,
    pub affected_owners: Vec<AffectedOwner>,
    pub conflicts: Vec<SemanticMergeConflict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<TransactionReceipt>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ModuleState {
    name: String,
    imports: Vec<Import>,
    documentation: Vec<Documentation>,
    annotations: Vec<Annotation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeclarationState {
    module: ModuleId,
    identity: DeclarationIdentity,
    declaration: Declaration,
    exported: bool,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PendingConflict {
    kind: SemanticMergeConflictKind,
    owner_id: Option<String>,
    summary: String,
}

struct MergedGraph {
    root: GraphRoot,
    modules: Vec<MeaningModule>,
    affected: Vec<AffectedOwner>,
}

pub fn merge_revisions(
    repository: &SemanticRepository,
    request: &SemanticMergeRequest,
    apply: bool,
) -> Result<SemanticMergeResult, Diagnostic> {
    validate_request(request)?;
    let current = repository.current()?;
    let transaction = merge_transaction_digest(request);
    let empty = |status, conflicts| SemanticMergeResult {
        contract_version: SEMANTIC_MERGE_CONTRACT_VERSION,
        base_revision: request.base_revision,
        left_revision: request.left_revision,
        right_revision: request.right_revision,
        observed_current: current.head.revision,
        status,
        transaction,
        semantic_diff: None,
        predicted_revision: None,
        published_revision: None,
        affected_owners: Vec::new(),
        conflicts,
        receipt: None,
    };

    if !repository.is_ancestor(
        request.base_revision,
        request.left_revision,
        request.maximum_work,
    )? || !repository.is_ancestor(
        request.base_revision,
        request.right_revision,
        request.maximum_work,
    )? {
        return Err(merge_error(
            DiagnosticClass::Source,
            "semantic_merge_base_ancestry",
            "merge base must be an ancestor of both exact branch revisions",
        ));
    }
    let base = repository.reconstruct_revision(request.base_revision)?;
    let left = repository.reconstruct_revision(request.left_revision)?;
    let right = repository.reconstruct_revision(request.right_revision)?;
    if base.record.core.repository_id != current.head.repository_id
        || left.record.core.repository_id != current.head.repository_id
        || right.record.core.repository_id != current.head.repository_id
    {
        return Err(merge_error(
            DiagnosticClass::Source,
            "semantic_merge_foreign_revision",
            "all merge revisions must belong to the opened repository authority",
        ));
    }
    let (mut merged, pending) = merge_snapshots(&base, &left, &right, request.maximum_work)?;
    if !pending.is_empty() {
        return Ok(empty(
            SemanticMergeStatus::Conflicted,
            materialize_conflicts(request, pending),
        ));
    }
    if let Err(error) = repository.canonicalize_proposal(&mut merged.root, &mut merged.modules) {
        return Ok(empty(
            SemanticMergeStatus::Conflicted,
            materialize_conflicts(
                request,
                vec![PendingConflict {
                    kind: SemanticMergeConflictKind::InvalidMergedGraph,
                    owner_id: None,
                    summary: format!("{}: {}", error.code, error.message),
                }],
            ),
        ));
    }

    if !apply {
        let current_root = if current.head.revision == request.left_revision {
            left.record.core.root
        } else if current.head.revision == request.right_revision {
            right.record.core.root
        } else {
            base.record.core.root
        };
        let result_root = merged.root.digest()?;
        return Ok(SemanticMergeResult {
            semantic_diff: Some(semantic_diff_digest(current_root, result_root)),
            affected_owners: merged.affected,
            ..empty(SemanticMergeStatus::Ready, Vec::new())
        });
    }
    let (current_branch, other_branch) = if current.head.revision == request.left_revision {
        (&left, &right)
    } else if current.head.revision == request.right_revision {
        (&right, &left)
    } else {
        return Ok(empty(SemanticMergeStatus::StaleHead, Vec::new()));
    };
    let result_root = merged.root.digest()?;
    if result_root == current_branch.record.core.root {
        return Ok(SemanticMergeResult {
            status: SemanticMergeStatus::SemanticNoChange,
            semantic_diff: Some(semantic_diff_digest(
                current_branch.record.core.root,
                result_root,
            )),
            affected_owners: merged.affected,
            ..empty(SemanticMergeStatus::SemanticNoChange, Vec::new())
        });
    }
    let semantic_diff = semantic_diff_digest(current_branch.record.core.root, result_root);
    let mut parents = vec![
        ParentRevision {
            revision: current_branch.record.revision,
            record: current_branch.record.digest()?,
        },
        ParentRevision {
            revision: other_branch.record.revision,
            record: other_branch.record.digest()?,
        },
    ];
    parents.sort();
    let predicted_revision = RevisionCore {
        contract_version: REVISION_CONTRACT_VERSION,
        graph_contract_version: GRAPH_CONTRACT_VERSION,
        repository_id: current.head.repository_id,
        parents,
        root: result_root,
        semantic_diff,
        transaction,
    }
    .revision_id()?;
    let additional_parent = ParentRevision {
        revision: other_branch.record.revision,
        record: other_branch.record.digest()?,
    };
    let (outcome, receipt) = repository.publish_merge(
        PublicationProposal {
            expected_base: current.head.revision,
            root: merged.root,
            modules: merged.modules,
            transaction,
            idempotency_key: None,
            semantic_diff,
            status: ReceiptStatus::MergeAccepted,
            affected_owners: merged.affected.clone(),
            intent: request.intent.clone(),
            dependency_artifacts: Vec::new(),
        },
        additional_parent,
    )?;
    match outcome {
        PublicationOutcome::Accepted { revision, .. } => Ok(SemanticMergeResult {
            status: SemanticMergeStatus::AcceptedChange,
            semantic_diff: Some(semantic_diff),
            predicted_revision: Some(predicted_revision),
            published_revision: Some(revision),
            affected_owners: merged.affected,
            receipt,
            ..empty(SemanticMergeStatus::AcceptedChange, Vec::new())
        }),
        PublicationOutcome::SemanticNoChange { .. } => Ok(SemanticMergeResult {
            status: SemanticMergeStatus::SemanticNoChange,
            semantic_diff: Some(semantic_diff),
            affected_owners: merged.affected,
            ..empty(SemanticMergeStatus::SemanticNoChange, Vec::new())
        }),
        PublicationOutcome::StaleBase { .. } => {
            Ok(empty(SemanticMergeStatus::StaleHead, Vec::new()))
        }
    }
}

fn merge_snapshots(
    base: &RevisionSnapshot,
    left: &RevisionSnapshot,
    right: &RevisionSnapshot,
    maximum_work: usize,
) -> Result<(MergedGraph, Vec<PendingConflict>), Diagnostic> {
    if base.root.package_id != left.root.package_id
        || base.root.package_id != right.root.package_id
        || base.root.repository_id != left.root.repository_id
        || base.root.repository_id != right.root.repository_id
    {
        return Err(merge_error(
            DiagnosticClass::Source,
            "semantic_merge_authority_mismatch",
            "merge snapshots do not describe one package authority",
        ));
    }
    let base_modules = module_states(base);
    let left_modules = module_states(left);
    let right_modules = module_states(right);
    let base_declarations = declaration_states(base);
    let left_declarations = declaration_states(left);
    let right_declarations = declaration_states(right);
    let work = base_modules
        .len()
        .saturating_add(left_modules.len())
        .saturating_add(right_modules.len())
        .saturating_add(base_declarations.len())
        .saturating_add(left_declarations.len())
        .saturating_add(right_declarations.len());
    if work > maximum_work {
        return Err(merge_error(
            DiagnosticClass::Resource,
            "semantic_merge_work_exhausted",
            "semantic merge exhausted its declared work budget",
        ));
    }

    let mut conflicts = Vec::new();
    let package_name = merge_value(
        &base.root.package_name,
        &left.root.package_name,
        &right.root.package_name,
    )
    .unwrap_or_else(|| {
        conflicts.push(concurrent("package", "package name changed differently"));
        base.root.package_name.clone()
    });
    let modules = merge_module_map(&base_modules, &left_modules, &right_modules, &mut conflicts);
    let declarations = merge_declaration_map(
        &base_declarations,
        &left_declarations,
        &right_declarations,
        &mut conflicts,
    );
    let dependencies = merge_keyed(
        keyed_dependencies(&base.root.dependencies),
        keyed_dependencies(&left.root.dependencies),
        keyed_dependencies(&right.root.dependencies),
        "dependency",
        &mut conflicts,
    );
    let targets = merge_keyed(
        keyed_targets(&base.root.targets),
        keyed_targets(&left.root.targets),
        keyed_targets(&right.root.targets),
        "target",
        &mut conflicts,
    );
    let tombstones = merge_keyed(
        keyed_tombstones(&base.root.tombstones),
        keyed_tombstones(&left.root.tombstones),
        keyed_tombstones(&right.root.tombstones),
        "tombstone",
        &mut conflicts,
    );

    for (id, declaration) in &declarations {
        if !modules.contains_key(&declaration.module) {
            conflicts.push(PendingConflict {
                kind: SemanticMergeConflictKind::MissingModule,
                owner_id: Some(id.to_string()),
                summary: format!(
                    "merged declaration references absent module {}",
                    declaration.module
                ),
            });
        }
    }
    conflicts.sort();
    conflicts.dedup();
    if conflicts.len() > MAXIMUM_MERGE_CONFLICTS {
        return Err(merge_error(
            DiagnosticClass::Resource,
            "semantic_merge_conflict_limit",
            format!("semantic merge exceeds {MAXIMUM_MERGE_CONFLICTS} conflicts"),
        ));
    }

    let mut meaning_modules = Vec::with_capacity(modules.len());
    for (module_id, state) in &modules {
        let mut entries = declarations
            .iter()
            .filter(|(_, declaration)| declaration.module == *module_id)
            .map(|(id, declaration)| (*id, declaration.clone()))
            .collect::<Vec<_>>();
        entries.sort_by_key(|(id, _)| *id);
        let mut exports = entries
            .iter()
            .filter(|(_, declaration)| declaration.exported)
            .map(|(_, declaration)| declaration.identity.name.clone())
            .collect::<Vec<_>>();
        exports.sort();
        exports.dedup();
        meaning_modules.push(MeaningModule {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            module_id: *module_id,
            module: super::language::Module {
                name: state.name.clone(),
                imports: state.imports.clone(),
                exports,
                declarations: entries
                    .iter()
                    .map(|(_, declaration)| declaration.declaration.clone())
                    .collect(),
            },
            declarations: entries
                .into_iter()
                .map(|(_, declaration)| declaration.identity)
                .collect(),
            relations: Vec::new(),
            documentation: state.documentation.clone(),
            annotations: state.annotations.clone(),
        });
    }
    meaning_modules.sort_by_key(|module| module.module_id);
    let placeholder = ModuleObjectDigest::of(b"semantic merge pending module object");
    let mut root = GraphRoot {
        graph_contract_version: GRAPH_CONTRACT_VERSION,
        repository_id: base.root.repository_id,
        package_id: base.root.package_id.clone(),
        package_name,
        modules: meaning_modules
            .iter()
            .map(|module| ModuleObjectRef {
                id: module.module_id,
                name: module.module.name.clone(),
                object: placeholder,
            })
            .collect(),
        dependencies: dependencies.into_values().collect(),
        targets: targets.into_values().collect(),
        tombstones: tombstones.into_values().collect(),
    };
    root.modules.sort();
    root.dependencies.sort();
    root.targets.sort();
    root.tombstones.sort();

    let mut affected = BTreeSet::new();
    collect_changed_keys(
        &base_modules,
        &modules,
        |id| AffectedOwner::Module(*id),
        &mut affected,
    );
    collect_changed_keys(
        &base_declarations,
        &declarations,
        |id| AffectedOwner::Declaration(*id),
        &mut affected,
    );
    let base_targets = keyed_targets(&base.root.targets);
    collect_changed_keys(
        &base_targets,
        &targets_from_root_map(&root.targets),
        |id| AffectedOwner::Target(*id),
        &mut affected,
    );
    Ok((
        MergedGraph {
            root,
            modules: meaning_modules,
            affected: affected.into_iter().collect(),
        },
        conflicts,
    ))
}

fn module_states(snapshot: &RevisionSnapshot) -> BTreeMap<ModuleId, ModuleState> {
    snapshot
        .modules
        .iter()
        .map(|module| {
            (
                module.module_id,
                ModuleState {
                    name: module.module.name.clone(),
                    imports: module.module.imports.clone(),
                    documentation: module.documentation.clone(),
                    annotations: module.annotations.clone(),
                },
            )
        })
        .collect()
}

fn declaration_states(snapshot: &RevisionSnapshot) -> BTreeMap<DeclarationId, DeclarationState> {
    let mut values = BTreeMap::new();
    for module in &snapshot.modules {
        for (identity, declaration) in module.declarations.iter().zip(&module.module.declarations) {
            values.insert(
                identity.id,
                DeclarationState {
                    module: module.module_id,
                    identity: identity.clone(),
                    declaration: declaration.clone(),
                    exported: module.module.exports.contains(&identity.name),
                },
            );
        }
    }
    values
}

fn merge_module_map(
    base: &BTreeMap<ModuleId, ModuleState>,
    left: &BTreeMap<ModuleId, ModuleState>,
    right: &BTreeMap<ModuleId, ModuleState>,
    conflicts: &mut Vec<PendingConflict>,
) -> BTreeMap<ModuleId, ModuleState> {
    let keys = union_keys(base, left, right);
    let mut merged = BTreeMap::new();
    for id in keys {
        match merge_optional(base.get(&id), left.get(&id), right.get(&id)) {
            MergeChoice::Value(Some(value)) => {
                merged.insert(id, value);
            }
            MergeChoice::Value(None) => {}
            MergeChoice::Conflict(kind) => {
                if let (Some(base), Some(left), Some(right)) =
                    (base.get(&id), left.get(&id), right.get(&id))
                    && let Some(value) = merge_module_fields(base, left, right)
                {
                    merged.insert(id, value);
                    continue;
                }
                conflicts.push(PendingConflict {
                    kind,
                    owner_id: Some(id.to_string()),
                    summary: "module changed incompatibly on both branches".to_owned(),
                });
                if let Some(value) = base.get(&id) {
                    merged.insert(id, value.clone());
                }
            }
        }
    }
    merged
}

fn merge_module_fields(
    base: &ModuleState,
    left: &ModuleState,
    right: &ModuleState,
) -> Option<ModuleState> {
    Some(ModuleState {
        name: merge_value(&base.name, &left.name, &right.name)?,
        imports: merge_value(&base.imports, &left.imports, &right.imports)?,
        documentation: merge_value(
            &base.documentation,
            &left.documentation,
            &right.documentation,
        )?,
        annotations: merge_value(&base.annotations, &left.annotations, &right.annotations)?,
    })
}

fn merge_declaration_map(
    base: &BTreeMap<DeclarationId, DeclarationState>,
    left: &BTreeMap<DeclarationId, DeclarationState>,
    right: &BTreeMap<DeclarationId, DeclarationState>,
    conflicts: &mut Vec<PendingConflict>,
) -> BTreeMap<DeclarationId, DeclarationState> {
    let keys = union_keys(base, left, right);
    let mut merged = BTreeMap::new();
    for id in keys {
        match merge_optional(base.get(&id), left.get(&id), right.get(&id)) {
            MergeChoice::Value(Some(value)) => {
                merged.insert(id, value);
            }
            MergeChoice::Value(None) => {}
            MergeChoice::Conflict(kind) => {
                if let (Some(base), Some(left), Some(right)) =
                    (base.get(&id), left.get(&id), right.get(&id))
                    && let Some(value) = merge_declaration_fields(base, left, right)
                {
                    merged.insert(id, value);
                    continue;
                }
                conflicts.push(PendingConflict {
                    kind,
                    owner_id: Some(id.to_string()),
                    summary: "declaration changed incompatibly on both branches".to_owned(),
                });
                if let Some(value) = base.get(&id) {
                    merged.insert(id, value.clone());
                }
            }
        }
    }
    merged
}

fn merge_declaration_fields(
    base: &DeclarationState,
    left: &DeclarationState,
    right: &DeclarationState,
) -> Option<DeclarationState> {
    let name = merge_value(
        &base.identity.name,
        &left.identity.name,
        &right.identity.name,
    )?;
    let mut base_identity = base.identity.clone();
    let mut left_identity = left.identity.clone();
    let mut right_identity = right.identity.clone();
    base_identity.name.clear();
    left_identity.name.clear();
    right_identity.name.clear();
    let mut identity = merge_value(&base_identity, &left_identity, &right_identity)?;
    identity.name.clone_from(&name);

    let mut base_declaration = base.declaration.clone();
    let mut left_declaration = left.declaration.clone();
    let mut right_declaration = right.declaration.clone();
    set_declaration_name(&mut base_declaration, String::new());
    set_declaration_name(&mut left_declaration, String::new());
    set_declaration_name(&mut right_declaration, String::new());
    let mut declaration = merge_value(&base_declaration, &left_declaration, &right_declaration)?;
    set_declaration_name(&mut declaration, name);
    Some(DeclarationState {
        module: merge_value(&base.module, &left.module, &right.module)?,
        identity,
        declaration,
        exported: merge_value(&base.exported, &left.exported, &right.exported)?,
    })
}

fn set_declaration_name(declaration: &mut Declaration, name: String) {
    match declaration {
        Declaration::Record(value) => value.name = name,
        Declaration::Variant(value) => value.name = name,
        Declaration::Interface(value) => value.name = name,
        Declaration::External(value) => value.name = name,
        Declaration::Function(value) => value.name = name,
        Declaration::Constant(value) => value.name = name,
        Declaration::Component(value) => value.name = name,
        Declaration::Test(value) => value.name = name,
    }
}

enum MergeChoice<T> {
    Value(Option<T>),
    Conflict(SemanticMergeConflictKind),
}

fn merge_optional<T: Clone + Eq>(
    base: Option<&T>,
    left: Option<&T>,
    right: Option<&T>,
) -> MergeChoice<T> {
    if left == right {
        return MergeChoice::Value(left.cloned());
    }
    if left == base {
        return MergeChoice::Value(right.cloned());
    }
    if right == base {
        return MergeChoice::Value(left.cloned());
    }
    let kind = match (base, left, right) {
        (None, Some(_), Some(_)) => SemanticMergeConflictKind::ConcurrentCreation,
        (Some(_), None, Some(_)) | (Some(_), Some(_), None) => {
            SemanticMergeConflictKind::DeleteModify
        }
        _ => SemanticMergeConflictKind::ConcurrentChange,
    };
    MergeChoice::Conflict(kind)
}

fn merge_value<T: Clone + Eq>(base: &T, left: &T, right: &T) -> Option<T> {
    if left == right {
        Some(left.clone())
    } else if left == base {
        Some(right.clone())
    } else if right == base {
        Some(left.clone())
    } else {
        None
    }
}

trait MergeKey: Clone + Ord {
    fn merge_label(&self) -> String;
}

impl MergeKey for String {
    fn merge_label(&self) -> String {
        self.clone()
    }
}

impl MergeKey for TargetId {
    fn merge_label(&self) -> String {
        self.to_string()
    }
}

impl MergeKey for TombstoneIdentity {
    fn merge_label(&self) -> String {
        match self {
            TombstoneIdentity::Module(id) => id.to_string(),
            TombstoneIdentity::Declaration(id) => id.to_string(),
            TombstoneIdentity::Field(id) => id.to_string(),
            TombstoneIdentity::Case(id) => id.to_string(),
            TombstoneIdentity::Operation(id) => id.to_string(),
            TombstoneIdentity::Parameter(id) => id.to_string(),
            TombstoneIdentity::Binding(id) => id.to_string(),
            TombstoneIdentity::Expression(id) => id.to_string(),
            TombstoneIdentity::Requirement(id) => id.to_string(),
            TombstoneIdentity::Port(id) => id.to_string(),
            TombstoneIdentity::Target(id) => id.to_string(),
            TombstoneIdentity::Documentation(id) => id.to_string(),
            TombstoneIdentity::Annotation(id) => id.to_string(),
        }
    }
}

fn merge_keyed<K: MergeKey, V: Clone + Eq>(
    base: BTreeMap<K, V>,
    left: BTreeMap<K, V>,
    right: BTreeMap<K, V>,
    label: &str,
    conflicts: &mut Vec<PendingConflict>,
) -> BTreeMap<K, V> {
    let keys = union_keys(&base, &left, &right);
    let mut merged = BTreeMap::new();
    for key in keys {
        match merge_optional(base.get(&key), left.get(&key), right.get(&key)) {
            MergeChoice::Value(Some(value)) => {
                merged.insert(key, value);
            }
            MergeChoice::Value(None) => {}
            MergeChoice::Conflict(kind) => conflicts.push(PendingConflict {
                kind,
                owner_id: Some(key.merge_label()),
                summary: format!("{label} changed incompatibly on both branches"),
            }),
        }
    }
    merged
}

fn union_keys<K: Clone + Ord, V>(
    base: &BTreeMap<K, V>,
    left: &BTreeMap<K, V>,
    right: &BTreeMap<K, V>,
) -> BTreeSet<K> {
    base.keys()
        .chain(left.keys())
        .chain(right.keys())
        .cloned()
        .collect()
}

fn keyed_dependencies(values: &[DependencyBinding]) -> BTreeMap<String, DependencyBinding> {
    values
        .iter()
        .map(|value| (value.alias.clone(), value.clone()))
        .collect()
}

fn keyed_targets(values: &[TargetBinding]) -> BTreeMap<TargetId, TargetBinding> {
    values
        .iter()
        .map(|value| (value.id, value.clone()))
        .collect()
}

fn targets_from_root_map(values: &[TargetBinding]) -> BTreeMap<TargetId, TargetBinding> {
    keyed_targets(values)
}

fn keyed_tombstones(values: &[Tombstone]) -> BTreeMap<TombstoneIdentity, Tombstone> {
    values
        .iter()
        .map(|value| (value.identity.clone(), value.clone()))
        .collect()
}

fn collect_changed_keys<K: Copy + Ord, V: Eq>(
    base: &BTreeMap<K, V>,
    merged: &BTreeMap<K, V>,
    owner: impl Fn(&K) -> AffectedOwner,
    output: &mut BTreeSet<AffectedOwner>,
) {
    for key in base.keys().chain(merged.keys()).collect::<BTreeSet<_>>() {
        if base.get(key) != merged.get(key) {
            output.insert(owner(key));
        }
    }
}

fn concurrent(owner: &str, summary: &str) -> PendingConflict {
    PendingConflict {
        kind: SemanticMergeConflictKind::ConcurrentChange,
        owner_id: Some(owner.to_owned()),
        summary: summary.to_owned(),
    }
}

fn materialize_conflicts(
    request: &SemanticMergeRequest,
    mut pending: Vec<PendingConflict>,
) -> Vec<SemanticMergeConflict> {
    pending.sort();
    pending.dedup();
    let mut seed = Vec::with_capacity(96);
    seed.extend_from_slice(&request.base_revision.bytes());
    seed.extend_from_slice(&request.left_revision.bytes());
    seed.extend_from_slice(&request.right_revision.bytes());
    pending
        .into_iter()
        .enumerate()
        .map(|(index, conflict)| SemanticMergeConflict {
            id: ConflictId::migrate(&seed, (index as u64).saturating_add(1)),
            kind: conflict.kind,
            owner_id: conflict.owner_id,
            summary: conflict.summary,
        })
        .collect()
}

fn merge_transaction_digest(request: &SemanticMergeRequest) -> TransactionDigest {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&request.contract_version.to_be_bytes());
    bytes.extend_from_slice(&request.base_revision.bytes());
    bytes.extend_from_slice(&request.left_revision.bytes());
    bytes.extend_from_slice(&request.right_revision.bytes());
    bytes.extend_from_slice(&(request.maximum_work as u64).to_be_bytes());
    if let Some(intent) = &request.intent {
        bytes.extend_from_slice(&(intent.len() as u64).to_be_bytes());
        bytes.extend_from_slice(intent.as_bytes());
    }
    TransactionDigest::of(&bytes)
}

fn validate_request(request: &SemanticMergeRequest) -> Result<(), Diagnostic> {
    if request.contract_version != SEMANTIC_MERGE_CONTRACT_VERSION {
        return Err(merge_error(
            DiagnosticClass::Source,
            "semantic_merge_contract",
            "merge request uses an unknown contract",
        ));
    }
    if request.left_revision == request.right_revision {
        return Err(merge_error(
            DiagnosticClass::Source,
            "semantic_merge_branch_duplicate",
            "merge branches must name two distinct accepted revisions",
        ));
    }
    if request.maximum_work == 0 || request.maximum_work > MAXIMUM_HISTORY_ITEMS {
        return Err(merge_error(
            DiagnosticClass::Resource,
            "semantic_merge_work_limit",
            format!("merge work must be 1 through {MAXIMUM_HISTORY_ITEMS}"),
        ));
    }
    if request
        .intent
        .as_ref()
        .is_some_and(|intent| intent.len() > 4_096)
    {
        return Err(merge_error(
            DiagnosticClass::Resource,
            "semantic_merge_intent_limit",
            "merge intent exceeds 4096 bytes",
        ));
    }
    Ok(())
}

const fn default_merge_work() -> usize {
    MAXIMUM_HISTORY_ITEMS
}

fn merge_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn independent_owner_changes_compose_and_same_owner_changes_conflict() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal");
        let repository = SemanticRepository::open(&project).expect("open graph fixture");
        let base = repository
            .reconstruct_current()
            .and_then(|current| repository.reconstruct_revision(current.current.head.revision))
            .expect("base snapshot");
        let mut left = base.clone();
        let declaration_id = left.modules[0].declarations[0].id;
        let old_name = left.modules[0].declarations[0].name.clone();
        let left_name = format!("{old_name}-left");
        left.modules[0].declarations[0].name = left_name.clone();
        set_declaration_name(
            &mut left.modules[0].module.declarations[0],
            left_name.clone(),
        );
        for export in &mut left.modules[0].module.exports {
            if export == &old_name {
                export.clone_from(&left_name);
            }
        }
        let mut right = base.clone();
        right.root.package_name.push_str("-right");

        let (merged, conflicts) =
            merge_snapshots(&base, &left, &right, 10_000).expect("merge snapshots");
        assert!(conflicts.is_empty());
        assert_eq!(merged.root.package_name, right.root.package_name);
        let merged_declaration = merged
            .modules
            .iter()
            .flat_map(|module| &module.declarations)
            .find(|identity| identity.id == declaration_id)
            .expect("merged declaration");
        assert_eq!(merged_declaration.name, left_name);

        let mut conflicting_right = base.clone();
        let right_name = format!("{old_name}-right");
        conflicting_right.modules[0].declarations[0].name = right_name.clone();
        set_declaration_name(
            &mut conflicting_right.modules[0].module.declarations[0],
            right_name,
        );
        let (_, conflicts) = merge_snapshots(&base, &left, &conflicting_right, 10_000)
            .expect("conflicting merge snapshots");
        assert!(conflicts.iter().any(|conflict| {
            conflict.kind == SemanticMergeConflictKind::ConcurrentChange
                && conflict.owner_id.as_deref() == Some(&declaration_id.to_string())
        }));
    }
}
