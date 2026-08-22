//! One exact-base public semantic transaction protocol and high-level owner operations.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::graph::{
    DependencyBinding, GraphRoot, ModuleObjectRef, StoredGraphRootDelta, TargetBinding, Tombstone,
    TombstoneIdentity,
};
use super::language::{
    Declaration, DeclarationReference, Effect, Expression, Field, InterfaceOperation, Module,
    Parameter, Type, VariantCase,
};
use super::meaning::{
    BindingIdentity, DeclarationIdentity, ExpressionIdentity, MeaningModule, MemberIdentity,
    RelationRole, RelationSource, RelationTarget,
};
use super::repository::{
    CurrentBinding, PreparedValidation, PublicationOutcome, PublicationProposal, SemanticRepository,
};
use super::revision::{
    AffectedOwner, ParentRevision, REVISION_CONTRACT_VERSION, ReceiptStatus, RevisionCore,
    TransactionReceipt,
};
use super::semantic_diff::semantic_diff_digest;
use super::semantic_digest::{RootObjectDigest, SemanticDiffDigest, TransactionDigest};
use super::semantic_id::{
    BindingId, CaseId, DeclarationId, DraftId, ExpressionId, FieldId, ModuleId, OperationId,
    ParameterId, RepositoryId, RevisionId, TargetId,
};
use super::syntax::SourceSpan;
use super::{OwnerKind, SemanticQueryIndex};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const TRANSACTION_CONTRACT_VERSION: u16 = 4;
pub const MAXIMUM_TRANSACTION_OPERATIONS: usize = 10_000;
pub const MAXIMUM_TRANSACTION_WORK: usize = 10_000_000;
pub const MAXIMUM_TRANSACTION_AFFECTED_OWNERS: usize = 100_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionBudget {
    pub maximum_operations: usize,
    pub maximum_work: usize,
    pub maximum_affected_owners: usize,
}

impl Default for TransactionBudget {
    fn default() -> Self {
        Self {
            maximum_operations: 1_000,
            maximum_work: 1_000_000,
            maximum_affected_owners: 10_000,
        }
    }
}

impl TransactionBudget {
    fn validate(self) -> Result<Self, Diagnostic> {
        if self.maximum_operations == 0
            || self.maximum_operations > MAXIMUM_TRANSACTION_OPERATIONS
            || self.maximum_work == 0
            || self.maximum_work > MAXIMUM_TRANSACTION_WORK
            || self.maximum_affected_owners == 0
            || self.maximum_affected_owners > MAXIMUM_TRANSACTION_AFFECTED_OWNERS
        {
            return Err(transaction_error(
                DiagnosticClass::Resource,
                "semantic_transaction_budget",
                "transaction budgets are zero or exceed the current hard maxima",
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionRequest {
    pub contract_version: u16,
    pub graph_contract: String,
    pub repository_id: RepositoryId,
    pub base_revision: RevisionId,
    #[serde(default)]
    pub draft: Option<DraftId>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub preconditions: Vec<SemanticPrecondition>,
    pub operations: Vec<SemanticOperation>,
    #[serde(default)]
    pub budget: TransactionBudget,
    #[serde(default)]
    pub intent: Option<String>,
}

impl TransactionRequest {
    pub fn digest(&self) -> Result<TransactionDigest, Diagnostic> {
        self.validate_envelope()?;
        let bytes = serde_json::to_vec(self).map_err(transaction_json)?;
        Ok(TransactionDigest::of(&bytes))
    }

    pub fn validate_envelope(&self) -> Result<(), Diagnostic> {
        if self.contract_version != TRANSACTION_CONTRACT_VERSION
            || self.graph_contract != super::meaning::GRAPH_CONTRACT_IDENTITY
        {
            return Err(transaction_error(
                DiagnosticClass::Source,
                "semantic_transaction_contract",
                "transaction uses an unknown protocol or graph contract",
            ));
        }
        let budget = self.budget.validate()?;
        if (self.operations.is_empty() && self.draft.is_none())
            || self.operations.len() > budget.maximum_operations
        {
            return Err(transaction_error(
                DiagnosticClass::Resource,
                "semantic_transaction_operation_limit",
                "transaction operation count is zero or exceeds its declared budget",
            ));
        }
        if self.preconditions.len() > budget.maximum_operations {
            return Err(transaction_error(
                DiagnosticClass::Resource,
                "semantic_transaction_precondition_limit",
                "transaction precondition count exceeds its declared operation budget",
            ));
        }
        if self
            .intent
            .as_ref()
            .is_some_and(|value| value.len() > 4_096)
        {
            return Err(transaction_error(
                DiagnosticClass::Resource,
                "semantic_transaction_intent_limit",
                "nonsemantic intent exceeds 4096 bytes",
            ));
        }
        validate_idempotency(self.idempotency_key.as_deref())
    }
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "precondition", rename_all = "snake_case")]
pub enum SemanticPrecondition {
    RootDigest {
        equals: RootObjectDigest,
    },
    OwnerExists {
        owner: OwnerSelector,
    },
    OwnerAbsent {
        owner: OwnerSelector,
    },
    OwnerName {
        owner: OwnerSelector,
        equals: String,
    },
}

#[derive(Decode, Encode, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum OwnerSelector {
    Module(ModuleId),
    Declaration(DeclarationId),
    Target(TargetId),
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "operation_kind", rename_all = "snake_case")]
pub enum SemanticOperation {
    SetPackageMetadata {
        name: String,
    },
    AddDependency {
        binding: DependencyBinding,
    },
    ReplaceDependency {
        binding: DependencyBinding,
    },
    RemoveDependency {
        alias: String,
    },
    CreateModule {
        id: ModuleId,
        name: String,
    },
    RenameModule {
        module: ModuleId,
        new_name: String,
    },
    DeleteModule {
        module: ModuleId,
    },
    CreateDeclaration {
        module: ModuleId,
        identity: DeclarationIdentity,
        declaration: Declaration,
        #[serde(default)]
        exported: bool,
    },
    ReplaceDeclaration {
        declaration: DeclarationId,
        identity: DeclarationIdentity,
        value: Declaration,
    },
    RenameDeclaration {
        declaration: DeclarationId,
        new_name: String,
    },
    MoveDeclaration {
        declaration: DeclarationId,
        destination: ModuleId,
    },
    DeleteOwner {
        owner: OwnerSelector,
    },
    CloneOwner {
        source: DeclarationId,
        destination: ModuleId,
        identity: DeclarationIdentity,
        declaration: Declaration,
        #[serde(default)]
        exported: bool,
    },
    RestoreOwner {
        historical_revision: RevisionId,
        declaration: DeclarationId,
        destination: ModuleId,
        #[serde(default)]
        exported: bool,
    },
    AddRecordField {
        record: DeclarationId,
        id: FieldId,
        field: Field,
    },
    RenameRecordField {
        field: FieldId,
        new_name: String,
    },
    ChangeRecordFieldType {
        field: FieldId,
        ty: Type,
    },
    RemoveRecordField {
        field: FieldId,
    },
    AddVariantCase {
        variant: DeclarationId,
        id: CaseId,
        case: VariantCase,
    },
    RenameVariantCase {
        case: CaseId,
        new_name: String,
    },
    ChangeVariantPayload {
        case: CaseId,
        payload: Option<Type>,
    },
    RemoveVariantCase {
        case: CaseId,
    },
    AddInterfaceOperation {
        interface: DeclarationId,
        id: OperationId,
        parameter_ids: Vec<ParameterId>,
        operation_value: InterfaceOperation,
    },
    ChangeInterfaceOperation {
        operation: OperationId,
        parameter_ids: Vec<ParameterId>,
        operation_value: InterfaceOperation,
    },
    RemoveInterfaceOperation {
        operation: OperationId,
    },
    ChangeSignature {
        declaration: DeclarationId,
        parameters: Vec<Parameter>,
        result: Type,
        #[serde(default)]
        effect: Option<Effect>,
        members: Vec<MemberIdentity>,
    },
    ReplaceBody {
        declaration: DeclarationId,
        body: Expression,
        bindings: Vec<BindingIdentity>,
        expressions: Vec<ExpressionIdentity>,
    },
    ReplaceExpression {
        expression: ExpressionId,
        value: Expression,
        declaration_bindings: Vec<BindingIdentity>,
        declaration_expressions: Vec<ExpressionIdentity>,
    },
    RebindReference {
        expression: ExpressionId,
        reference: DeclarationReference,
    },
    RenameBinding {
        binding: BindingId,
        new_name: String,
    },
    ReplaceTestExpectation {
        test: DeclarationId,
        expected: Expression,
        bindings: Vec<BindingIdentity>,
        expressions: Vec<ExpressionIdentity>,
    },
    CreateTarget {
        target: TargetBinding,
    },
    DeleteTarget {
        target: TargetId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransactionMode {
    Plan,
    Validate,
    Apply,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Planned,
    Validated,
    AcceptedChange,
    Replayed,
    SemanticNoChange,
    StaleBase,
    PreconditionFailed,
    ForeignIdentity,
    InvalidGraph,
    ResourceExhausted,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionResult {
    pub contract_version: u16,
    pub graph_contract: &'static str,
    pub repository_id: RepositoryId,
    pub requested_base: RevisionId,
    pub observed_current: RevisionId,
    pub status: TransactionStatus,
    pub transaction: TransactionDigest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diff: Option<SemanticDiffDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predicted_revision: Option<RevisionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_revision: Option<RevisionId>,
    pub affected_owners: Vec<AffectedOwner>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<TransactionReceipt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

struct PreparedTransaction {
    root: Option<GraphRoot>,
    modules: Vec<MeaningModule>,
    validation: PreparedValidation,
    result_root: RootObjectDigest,
    affected_owners: Vec<AffectedOwner>,
}

pub fn execute_transaction(
    repository: &SemanticRepository,
    request: &TransactionRequest,
    mode: TransactionMode,
) -> Result<TransactionResult, Diagnostic> {
    request.validate_envelope()?;
    if request.draft.is_some() {
        return Err(transaction_error(
            DiagnosticClass::Source,
            "semantic_transaction_draft_route",
            "draft-bound requests must use the public draft workflow",
        ));
    }
    let transaction = request.digest()?;
    let current = repository.current_binding()?;
    let current_revision = current.head.revision;
    let repository_id = current.head.repository_id;
    let empty = |status, diagnostics| TransactionResult {
        contract_version: TRANSACTION_CONTRACT_VERSION,
        graph_contract: super::meaning::GRAPH_CONTRACT_IDENTITY,
        repository_id,
        requested_base: request.base_revision,
        observed_current: current_revision,
        status,
        transaction,
        semantic_diff: None,
        predicted_revision: None,
        published_revision: None,
        affected_owners: Vec::new(),
        receipt: None,
        diagnostics,
    };

    if request.repository_id != repository_id {
        return Ok(empty(
            TransactionStatus::ForeignIdentity,
            vec![transaction_error(
                DiagnosticClass::Source,
                "semantic_transaction_foreign_repository",
                "transaction repository identity does not match the opened authority",
            )],
        ));
    }
    if let Some(key) = request.idempotency_key.as_deref()
        && let Some(receipt) = repository.receipt_for_idempotency(key)?
    {
        if receipt.transaction != transaction {
            return Ok(empty(
                TransactionStatus::PreconditionFailed,
                vec![transaction_error(
                    DiagnosticClass::Semantic,
                    "semantic_idempotency_conflict",
                    "idempotency key already belongs to a different transaction",
                )],
            ));
        }
        return Ok(TransactionResult {
            published_revision: Some(receipt.result),
            receipt: Some(receipt.clone()),
            status: TransactionStatus::Replayed,
            semantic_diff: Some(receipt.semantic_diff),
            predicted_revision: Some(receipt.result),
            ..empty(TransactionStatus::Replayed, Vec::new())
        });
    }
    if request.base_revision != current_revision {
        return Ok(empty(TransactionStatus::StaleBase, Vec::new()));
    }
    let local = if request.preconditions.is_empty() {
        let prepared = match prepare_local_pure_body_transaction(repository, request, &current) {
            Ok(prepared) => prepared,
            Err(error) => {
                return Ok(empty(
                    transaction_failure_status(&error),
                    vec![without_source_location(error)],
                ));
            }
        };
        if prepared.is_some() {
            prepared
        } else {
            let prepared = match prepare_local_module_creation(repository, request, &current) {
                Ok(prepared) => prepared,
                Err(error) => {
                    return Ok(empty(
                        transaction_failure_status(&error),
                        vec![without_source_location(error)],
                    ));
                }
            };
            if prepared.is_some() {
                prepared
            } else {
                let prepared = match prepare_local_module_rename(repository, request, &current) {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        return Ok(empty(
                            transaction_failure_status(&error),
                            vec![without_source_location(error)],
                        ));
                    }
                };
                if prepared.is_some() {
                    prepared
                } else {
                    match prepare_local_declaration_rename(repository, request, &current) {
                        Ok(prepared) => prepared,
                        Err(error) => {
                            return Ok(empty(
                                transaction_failure_status(&error),
                                vec![without_source_location(error)],
                            ));
                        }
                    }
                }
            }
        }
    } else {
        None
    };
    let prepared = if let Some(prepared) = local {
        prepared
    } else {
        let reconstructed = repository.reconstruct_current()?;
        if reconstructed.current.head.revision != current_revision {
            return Ok(empty(TransactionStatus::StaleBase, Vec::new()));
        }
        for precondition in &request.preconditions {
            if let Err(error) = check_precondition(
                precondition,
                &reconstructed.current.root,
                &reconstructed.modules,
                reconstructed.current.record.core.root,
            ) {
                return Ok(empty(TransactionStatus::PreconditionFailed, vec![error]));
            }
        }
        let mut root = reconstructed.current.root.clone();
        let mut modules = reconstructed.modules.clone();
        let mut module_ids = modules
            .iter()
            .map(|module| module.module_id)
            .collect::<BTreeSet<_>>();
        let mut module_names = modules
            .iter()
            .map(|module| module.module.name.clone())
            .collect::<BTreeSet<_>>();
        let mut affected = BTreeSet::new();
        let mut work = 0usize;
        for operation in &request.operations {
            if let Err(error) = apply_operation(
                repository,
                operation,
                request.base_revision,
                &mut root,
                &mut modules,
                &mut module_ids,
                &mut module_names,
                &mut affected,
                &mut work,
                request.budget,
            ) {
                return Ok(empty(transaction_failure_status(&error), vec![error]));
            }
        }
        if affected.len() > request.budget.maximum_affected_owners {
            return Ok(empty(
                TransactionStatus::ResourceExhausted,
                vec![transaction_error(
                    DiagnosticClass::Resource,
                    "semantic_transaction_affected_limit",
                    "transaction exceeds its affected-owner budget",
                )],
            ));
        }
        let validation = match repository.canonicalize_proposal(
            request.base_revision,
            &reconstructed.current.root,
            &mut root,
            &mut modules,
        ) {
            Ok(prepared) => prepared,
            Err(error) => {
                return Ok(empty(
                    transaction_failure_status(&error),
                    vec![without_source_location(error)],
                ));
            }
        };
        let result_root = validation.result_root();
        PreparedTransaction {
            root: Some(root),
            modules,
            validation,
            result_root,
            affected_owners: affected.into_iter().collect(),
        }
    };
    if prepared.result_root == current.record.core.root {
        return Ok(empty(TransactionStatus::SemanticNoChange, Vec::new()));
    }
    let semantic_diff = semantic_diff_digest(current.record.core.root, prepared.result_root);
    let predicted_core = RevisionCore {
        contract_version: REVISION_CONTRACT_VERSION,
        graph_contract_version: super::meaning::GRAPH_CONTRACT_VERSION,
        repository_id,
        parents: vec![ParentRevision {
            revision: current_revision,
            record: current.head.record,
        }],
        root: prepared.result_root,
        semantic_certificate: prepared.validation.semantic_certificate(),
        semantic_diff,
        transaction,
    };
    let predicted_revision = predicted_core.revision_id()?;
    let affected_owners = prepared.affected_owners;
    if mode != TransactionMode::Apply {
        return Ok(TransactionResult {
            contract_version: TRANSACTION_CONTRACT_VERSION,
            graph_contract: super::meaning::GRAPH_CONTRACT_IDENTITY,
            repository_id,
            requested_base: request.base_revision,
            observed_current: current_revision,
            status: if mode == TransactionMode::Plan {
                TransactionStatus::Planned
            } else {
                TransactionStatus::Validated
            },
            transaction,
            semantic_diff: Some(semantic_diff),
            predicted_revision: Some(predicted_revision),
            published_revision: None,
            affected_owners,
            receipt: None,
            diagnostics: Vec::new(),
        });
    }
    let (outcome, receipt) = repository.publish(PublicationProposal {
        expected_base: request.base_revision,
        repository_id,
        root: prepared.root,
        modules: prepared.modules,
        transaction,
        idempotency_key: request.idempotency_key.clone(),
        semantic_diff,
        status: ReceiptStatus::AcceptedChange,
        affected_owners: affected_owners.clone(),
        intent: request.intent.clone(),
        dependency_artifacts: Vec::new(),
        prepared_validation: Some(prepared.validation),
    })?;
    match outcome {
        PublicationOutcome::Accepted { revision, .. } => Ok(TransactionResult {
            contract_version: TRANSACTION_CONTRACT_VERSION,
            graph_contract: super::meaning::GRAPH_CONTRACT_IDENTITY,
            repository_id,
            requested_base: request.base_revision,
            observed_current: current_revision,
            status: TransactionStatus::AcceptedChange,
            transaction,
            semantic_diff: Some(semantic_diff),
            predicted_revision: Some(predicted_revision),
            published_revision: Some(revision),
            affected_owners,
            receipt,
            diagnostics: Vec::new(),
        }),
        PublicationOutcome::SemanticNoChange { .. } => {
            Ok(empty(TransactionStatus::SemanticNoChange, Vec::new()))
        }
        PublicationOutcome::StaleBase { .. } => Ok(empty(TransactionStatus::StaleBase, Vec::new())),
    }
}

fn prepare_local_pure_body_transaction(
    repository: &SemanticRepository,
    request: &TransactionRequest,
    current: &CurrentBinding,
) -> Result<Option<PreparedTransaction>, Diagnostic> {
    let mut target_modules = BTreeSet::new();
    for operation in &request.operations {
        let SemanticOperation::ReplaceBody { declaration, .. } = operation else {
            return Ok(None);
        };
        let summary = match SemanticQueryIndex::owner_summary_revision(
            repository,
            current.head.revision,
            &declaration.to_string(),
        ) {
            Ok(summary) => summary,
            Err(_) => return Ok(None),
        };
        if summary.kind != OwnerKind::PureFunction {
            return Ok(None);
        }
        let Some(module_id) = summary.module_id else {
            return Ok(None);
        };
        target_modules.insert(module_id);
    }

    let mut pending = target_modules.clone();
    let mut references = BTreeMap::new();
    let mut modules = BTreeMap::new();
    let mut dependencies = BTreeMap::new();
    let mut dependency_lookups =
        BTreeMap::<super::package::PackageId, Option<DependencyBinding>>::new();
    let mut work = 0usize;
    while let Some(module_id) = pending.pop_first() {
        if modules.contains_key(&module_id) {
            continue;
        }
        consume_work(&mut work, 1, request.budget)?;
        let reference = repository
            .module_reference_by_id(current, module_id)?
            .ok_or_else(|| {
                transaction_error(
                    DiagnosticClass::Corrupt,
                    "semantic_local_module_binding",
                    "accepted persistent root lost a selected module binding",
                )
            })?;
        let module = repository.read_module(reference.object)?;
        for import in &module.module.imports {
            consume_work(&mut work, 1, request.budget)?;
            if import.target.package != current.stored_root.package_id {
                let dependency =
                    if let Some(cached) = dependency_lookups.get(&import.target.package) {
                        cached.clone()
                    } else {
                        let binding = repository
                            .dependency_binding_by_package(current, &import.target.package)?;
                        dependency_lookups.insert(import.target.package.clone(), binding.clone());
                        binding
                    };
                let binding = dependency.ok_or_else(|| {
                    transaction_error(
                        DiagnosticClass::Corrupt,
                        "semantic_local_import_package",
                        format!(
                            "accepted module '{}' imports missing dependency package '{}'",
                            module.module.name,
                            import.target.package.as_str()
                        ),
                    )
                })?;
                dependencies.insert(binding.alias.clone(), binding);
                continue;
            }
            let imported = repository
                .module_reference_by_id(current, import.target.module)?
                .ok_or_else(|| {
                    transaction_error(
                        DiagnosticClass::Corrupt,
                        "semantic_local_import_binding",
                        format!(
                            "accepted module '{}' imports missing local module '{}'",
                            module.module.name, import.target.module
                        ),
                    )
                })?;
            pending.insert(imported.id);
        }
        references.insert(module_id, reference);
        modules.insert(module_id, module);
    }

    for operation in &request.operations {
        let SemanticOperation::ReplaceBody { declaration, .. } = operation else {
            unreachable!("local preparation admitted only body replacements");
        };
        let Some((_identity, Declaration::Function(function))) = modules
            .values()
            .find_map(|module| module.declaration(*declaration))
        else {
            return Ok(None);
        };
        if function.effect != Effect::Pure {
            return Ok(None);
        }
    }

    let original_objects = references
        .iter()
        .map(|(id, reference)| (*id, reference.object))
        .collect::<BTreeMap<_, _>>();
    let mut root = GraphRoot {
        graph_contract_version: current.stored_root.graph_contract_version,
        repository_id: current.stored_root.repository_id,
        package_id: current.stored_root.package_id.clone(),
        package_name: current.stored_root.package_name.clone(),
        modules: references.into_values().collect(),
        dependencies: dependencies.into_values().collect(),
        targets: Vec::new(),
        tombstones: Vec::new(),
    };
    root.modules.sort();
    root.dependencies.sort();
    let mut modules = modules.into_values().collect::<Vec<_>>();
    modules.sort_by_key(|module| module.module_id);
    let mut module_ids = modules
        .iter()
        .map(|module| module.module_id)
        .collect::<BTreeSet<_>>();
    let mut module_names = modules
        .iter()
        .map(|module| module.module.name.clone())
        .collect::<BTreeSet<_>>();
    let mut affected = BTreeSet::new();
    for operation in &request.operations {
        apply_operation(
            repository,
            operation,
            request.base_revision,
            &mut root,
            &mut modules,
            &mut module_ids,
            &mut module_names,
            &mut affected,
            &mut work,
            request.budget,
        )?;
    }
    if affected.len() > request.budget.maximum_affected_owners {
        return Err(transaction_error(
            DiagnosticClass::Resource,
            "semantic_transaction_affected_limit",
            "transaction exceeds its affected-owner budget",
        ));
    }
    let validated = repository.canonicalize_slice(&mut root, &mut modules)?;
    let tombstone_upserts = root.tombstones.clone();
    let mut changed_modules = Vec::new();
    let mut module_upserts = Vec::new();
    for reference in &root.modules {
        let Some(original) = original_objects.get(&reference.id) else {
            return Ok(None);
        };
        if *original == reference.object {
            continue;
        }
        if !target_modules.contains(&reference.id) {
            return Ok(None);
        }
        let module = modules
            .iter()
            .find(|module| module.module_id == reference.id)
            .ok_or_else(|| {
                transaction_error(
                    DiagnosticClass::Infrastructure,
                    "semantic_local_changed_module",
                    "local canonicalization lost a changed module",
                )
            })?;
        changed_modules.push(module.clone());
        module_upserts.push(reference.clone());
    }
    let delta = StoredGraphRootDelta {
        module_upserts,
        tombstone_upserts,
        ..StoredGraphRootDelta::default()
    };
    let validation = repository.prepare_local_validation(
        current,
        delta,
        changed_modules,
        &validated,
        "incremental_pure_body_slice",
    )?;
    let result_root = validation.result_root();
    Ok(Some(PreparedTransaction {
        root: None,
        modules: Vec::new(),
        validation,
        result_root,
        affected_owners: affected.into_iter().collect(),
    }))
}

fn prepare_local_module_creation(
    repository: &SemanticRepository,
    request: &TransactionRequest,
    current: &CurrentBinding,
) -> Result<Option<PreparedTransaction>, Diagnostic> {
    let mut module_ids = BTreeSet::new();
    let mut module_names = BTreeSet::new();
    let mut modules = Vec::with_capacity(request.operations.len());
    let mut references = Vec::with_capacity(request.operations.len());
    let mut affected = Vec::with_capacity(request.operations.len());
    let mut work = 0usize;
    for operation in &request.operations {
        let SemanticOperation::CreateModule { id, name } = operation else {
            return Ok(None);
        };
        consume_work(&mut work, 1, request.budget)?;
        if !module_ids.insert(*id)
            || !module_names.insert(name.clone())
            || repository.module_reference_by_id(current, *id)?.is_some()
            || repository
                .module_reference_by_name(current, name)?
                .is_some()
        {
            return Err(operation_error(
                "create_module",
                "module identity or name exists",
            ));
        }
        if repository
            .tombstone_by_identity(current, &TombstoneIdentity::Module(*id))?
            .is_some()
        {
            return Err(operation_error(
                "create_module",
                "deleted module identity cannot be reused",
            ));
        }
        let module = MeaningModule {
            graph_contract_version: super::meaning::GRAPH_CONTRACT_VERSION,
            module_id: *id,
            module: Module {
                name: name.clone(),
                imports: Vec::new(),
                exports: Vec::new(),
                declarations: Vec::new(),
            },
            declarations: Vec::new(),
            relations: Vec::new(),
            documentation: Vec::new(),
            annotations: Vec::new(),
        };
        references.push(ModuleObjectRef {
            id: *id,
            name: name.clone(),
            object: module.digest()?,
        });
        modules.push(module);
        affected.push(AffectedOwner::Module(*id));
    }
    references.sort();
    modules.sort_by_key(|module| module.module_id);
    affected.sort();
    if affected.len() > request.budget.maximum_affected_owners {
        return Err(transaction_error(
            DiagnosticClass::Resource,
            "semantic_transaction_affected_limit",
            "transaction exceeds its affected-owner budget",
        ));
    }
    let mut root = GraphRoot {
        graph_contract_version: current.stored_root.graph_contract_version,
        repository_id: current.stored_root.repository_id,
        package_id: current.stored_root.package_id.clone(),
        package_name: current.stored_root.package_name.clone(),
        modules: references,
        dependencies: Vec::new(),
        targets: Vec::new(),
        tombstones: Vec::new(),
    };
    let validated = repository.canonicalize_slice(&mut root, &mut modules)?;
    let delta = StoredGraphRootDelta {
        module_upserts: root.modules,
        ..StoredGraphRootDelta::default()
    };
    let validation = repository.prepare_local_validation(
        current,
        delta,
        modules,
        &validated,
        "incremental_independent_module_create",
    )?;
    let result_root = validation.result_root();
    Ok(Some(PreparedTransaction {
        root: None,
        modules: Vec::new(),
        validation,
        result_root,
        affected_owners: affected,
    }))
}

fn prepare_local_module_rename(
    repository: &SemanticRepository,
    request: &TransactionRequest,
    current: &CurrentBinding,
) -> Result<Option<PreparedTransaction>, Diagnostic> {
    let mut requested = BTreeMap::<ModuleId, String>::new();
    for operation in &request.operations {
        let SemanticOperation::RenameModule { module, new_name } = operation else {
            return Ok(None);
        };
        if requested.insert(*module, new_name.clone()).is_some() {
            return Ok(None);
        }
    }

    let mut original_references = BTreeMap::new();
    for module in requested.keys() {
        let reference = repository
            .module_reference_by_id(current, *module)?
            .ok_or_else(|| operation_error("module_selection", "selected module is absent"))?;
        original_references.insert(*module, reference);
    }
    let final_names = requested.values().collect::<BTreeSet<_>>();
    if final_names.len() != requested.len() {
        return Err(operation_error(
            "rename_module",
            "two renamed modules cannot have the same destination name",
        ));
    }
    for (module, new_name) in &requested {
        if let Some(existing) = repository.module_reference_by_name(current, new_name)?
            && existing.id != *module
            && !requested.contains_key(&existing.id)
        {
            return Err(operation_error(
                "rename_module",
                "destination module name exists",
            ));
        }
    }

    // Validation follows only outgoing import dependencies of renamed modules. Importers are
    // deliberately absent: accepted imports bind stable package/module identities and therefore
    // do not change when presentation names change.
    let mut pending = requested.keys().copied().collect::<BTreeSet<_>>();
    let mut references = BTreeMap::new();
    let mut modules = BTreeMap::new();
    let mut dependencies = BTreeMap::new();
    let mut dependency_lookups =
        BTreeMap::<super::package::PackageId, Option<DependencyBinding>>::new();
    let mut work = 0usize;
    while let Some(module_id) = pending.pop_first() {
        if modules.contains_key(&module_id) {
            continue;
        }
        consume_work(&mut work, 1, request.budget)?;
        let reference = repository
            .module_reference_by_id(current, module_id)?
            .ok_or_else(|| {
                transaction_error(
                    DiagnosticClass::Corrupt,
                    "semantic_local_module_binding",
                    "accepted persistent root lost a selected module binding",
                )
            })?;
        let module = repository.read_module(reference.object)?;
        for import in &module.module.imports {
            consume_work(&mut work, 1, request.budget)?;
            if import.target.package != current.stored_root.package_id {
                let dependency =
                    if let Some(cached) = dependency_lookups.get(&import.target.package) {
                        cached.clone()
                    } else {
                        let binding = repository
                            .dependency_binding_by_package(current, &import.target.package)?;
                        dependency_lookups.insert(import.target.package.clone(), binding.clone());
                        binding
                    };
                let binding = dependency.ok_or_else(|| {
                    transaction_error(
                        DiagnosticClass::Corrupt,
                        "semantic_local_import_package",
                        format!(
                            "accepted module '{}' imports missing dependency package '{}'",
                            module.module.name,
                            import.target.package.as_str()
                        ),
                    )
                })?;
                dependencies.insert(binding.alias.clone(), binding);
            } else {
                let imported = repository
                    .module_reference_by_id(current, import.target.module)?
                    .ok_or_else(|| {
                        transaction_error(
                            DiagnosticClass::Corrupt,
                            "semantic_local_import_binding",
                            format!(
                                "accepted module '{}' imports missing local module '{}'",
                                module.module.name, import.target.module
                            ),
                        )
                    })?;
                pending.insert(imported.id);
            }
        }
        references.insert(module_id, reference);
        modules.insert(module_id, module);
    }

    let mut affected = Vec::new();
    for (module_id, new_name) in &requested {
        let module = modules.get_mut(module_id).ok_or_else(|| {
            transaction_error(
                DiagnosticClass::Infrastructure,
                "semantic_local_rename_module",
                "local rename preparation lost a selected module",
            )
        })?;
        if module.module.name != *new_name {
            module.module.name.clone_from(new_name);
            affected.push(AffectedOwner::Module(*module_id));
        }
    }
    affected.sort();
    if affected.len() > request.budget.maximum_affected_owners {
        return Err(transaction_error(
            DiagnosticClass::Resource,
            "semantic_transaction_affected_limit",
            "transaction exceeds its affected-owner budget",
        ));
    }

    let mut root = GraphRoot {
        graph_contract_version: current.stored_root.graph_contract_version,
        repository_id: current.stored_root.repository_id,
        package_id: current.stored_root.package_id.clone(),
        package_name: current.stored_root.package_name.clone(),
        modules: references.into_values().collect(),
        dependencies: dependencies.into_values().collect(),
        targets: Vec::new(),
        tombstones: Vec::new(),
    };
    root.modules.sort();
    root.dependencies.sort();
    let mut modules = modules.into_values().collect::<Vec<_>>();
    modules.sort_by_key(|module| module.module_id);
    let validated = repository.canonicalize_slice(&mut root, &mut modules)?;

    let mut module_removals = Vec::new();
    let mut module_upserts = Vec::new();
    let mut changed_modules = Vec::new();
    for (module_id, new_name) in &requested {
        let original = original_references.get(module_id).ok_or_else(|| {
            transaction_error(
                DiagnosticClass::Infrastructure,
                "semantic_local_rename_original",
                "local rename preparation lost an original module binding",
            )
        })?;
        if original.name == *new_name {
            continue;
        }
        let replacement = root
            .modules
            .iter()
            .find(|reference| reference.id == *module_id)
            .ok_or_else(|| {
                transaction_error(
                    DiagnosticClass::Infrastructure,
                    "semantic_local_rename_binding",
                    "local canonicalization lost a renamed module binding",
                )
            })?
            .clone();
        let changed = modules
            .iter()
            .find(|module| module.module_id == *module_id)
            .ok_or_else(|| {
                transaction_error(
                    DiagnosticClass::Infrastructure,
                    "semantic_local_rename_meaning",
                    "local canonicalization lost renamed module meaning",
                )
            })?
            .clone();
        module_removals.push(original.clone());
        module_upserts.push(replacement);
        changed_modules.push(changed);
    }
    let delta = StoredGraphRootDelta {
        module_removals,
        module_upserts,
        ..StoredGraphRootDelta::default()
    };
    let validation = repository.prepare_local_validation(
        current,
        delta,
        changed_modules,
        &validated,
        "incremental_module_rename",
    )?;
    let result_root = validation.result_root();
    Ok(Some(PreparedTransaction {
        root: None,
        modules: Vec::new(),
        validation,
        result_root,
        affected_owners: affected,
    }))
}

fn prepare_local_declaration_rename(
    repository: &SemanticRepository,
    request: &TransactionRequest,
    current: &CurrentBinding,
) -> Result<Option<PreparedTransaction>, Diagnostic> {
    let mut requested = BTreeMap::<DeclarationId, String>::new();
    let mut target_modules = BTreeSet::new();
    for operation in &request.operations {
        let SemanticOperation::RenameDeclaration {
            declaration,
            new_name,
        } = operation
        else {
            return Ok(None);
        };
        if requested.insert(*declaration, new_name.clone()).is_some() {
            return Ok(None);
        }
        let summary = match SemanticQueryIndex::owner_summary_revision(
            repository,
            current.head.revision,
            &declaration.to_string(),
        ) {
            Ok(summary) => summary,
            Err(_) => return Ok(None),
        };
        let module = summary.module_id.ok_or_else(|| {
            operation_error(
                "declaration_selection",
                "selected declaration has no owning module",
            )
        })?;
        target_modules.insert(module);
    }

    // Only the renamed owners and their outgoing import closure are needed. Canonical references
    // bind declaration identities, so importer modules have no semantic or physical update.
    let mut pending = target_modules.clone();
    let mut references = BTreeMap::new();
    let mut modules = BTreeMap::new();
    let mut dependencies = BTreeMap::new();
    let mut dependency_lookups =
        BTreeMap::<super::package::PackageId, Option<DependencyBinding>>::new();
    let mut work = 0usize;
    while let Some(module_id) = pending.pop_first() {
        if modules.contains_key(&module_id) {
            continue;
        }
        consume_work(&mut work, 1, request.budget)?;
        let reference = repository
            .module_reference_by_id(current, module_id)?
            .ok_or_else(|| {
                transaction_error(
                    DiagnosticClass::Corrupt,
                    "semantic_local_module_binding",
                    "accepted persistent root lost a selected module binding",
                )
            })?;
        let module = repository.read_module(reference.object)?;
        for import in &module.module.imports {
            consume_work(&mut work, 1, request.budget)?;
            if import.target.package != current.stored_root.package_id {
                let dependency =
                    if let Some(cached) = dependency_lookups.get(&import.target.package) {
                        cached.clone()
                    } else {
                        let binding = repository
                            .dependency_binding_by_package(current, &import.target.package)?;
                        dependency_lookups.insert(import.target.package.clone(), binding.clone());
                        binding
                    };
                let binding = dependency.ok_or_else(|| {
                    transaction_error(
                        DiagnosticClass::Corrupt,
                        "semantic_local_import_package",
                        format!(
                            "accepted module '{}' imports missing dependency package '{}'",
                            module.module.name,
                            import.target.package.as_str()
                        ),
                    )
                })?;
                dependencies.insert(binding.alias.clone(), binding);
            } else {
                let imported = repository
                    .module_reference_by_id(current, import.target.module)?
                    .ok_or_else(|| {
                        transaction_error(
                            DiagnosticClass::Corrupt,
                            "semantic_local_import_binding",
                            format!(
                                "accepted module '{}' imports missing local module '{}'",
                                module.module.name, import.target.module
                            ),
                        )
                    })?;
                pending.insert(imported.id);
            }
        }
        references.insert(module_id, reference);
        modules.insert(module_id, module);
    }

    let original_objects = references
        .iter()
        .map(|(id, reference)| (*id, reference.object))
        .collect::<BTreeMap<_, _>>();
    let mut root = GraphRoot {
        graph_contract_version: current.stored_root.graph_contract_version,
        repository_id: current.stored_root.repository_id,
        package_id: current.stored_root.package_id.clone(),
        package_name: current.stored_root.package_name.clone(),
        modules: references.into_values().collect(),
        dependencies: dependencies.into_values().collect(),
        targets: Vec::new(),
        tombstones: Vec::new(),
    };
    root.modules.sort();
    root.dependencies.sort();
    let mut modules = modules.into_values().collect::<Vec<_>>();
    modules.sort_by_key(|module| module.module_id);
    let mut module_ids = modules
        .iter()
        .map(|module| module.module_id)
        .collect::<BTreeSet<_>>();
    let mut module_names = modules
        .iter()
        .map(|module| module.module.name.clone())
        .collect::<BTreeSet<_>>();
    let mut affected = BTreeSet::new();
    for operation in &request.operations {
        apply_operation(
            repository,
            operation,
            request.base_revision,
            &mut root,
            &mut modules,
            &mut module_ids,
            &mut module_names,
            &mut affected,
            &mut work,
            request.budget,
        )?;
    }
    if affected.len() > request.budget.maximum_affected_owners {
        return Err(transaction_error(
            DiagnosticClass::Resource,
            "semantic_transaction_affected_limit",
            "transaction exceeds its affected-owner budget",
        ));
    }
    let validated = repository.canonicalize_slice(&mut root, &mut modules)?;

    let mut changed_modules = Vec::new();
    let mut module_upserts = Vec::new();
    for reference in &root.modules {
        let Some(original) = original_objects.get(&reference.id) else {
            return Ok(None);
        };
        if *original == reference.object {
            continue;
        }
        if !target_modules.contains(&reference.id) {
            return Ok(None);
        }
        let module = modules
            .iter()
            .find(|module| module.module_id == reference.id)
            .ok_or_else(|| {
                transaction_error(
                    DiagnosticClass::Infrastructure,
                    "semantic_local_changed_module",
                    "local canonicalization lost a changed module",
                )
            })?;
        changed_modules.push(module.clone());
        module_upserts.push(reference.clone());
    }
    let delta = StoredGraphRootDelta {
        module_upserts,
        ..StoredGraphRootDelta::default()
    };
    let validation = repository.prepare_local_validation(
        current,
        delta,
        changed_modules,
        &validated,
        "incremental_declaration_rename",
    )?;
    let result_root = validation.result_root();
    Ok(Some(PreparedTransaction {
        root: None,
        modules: Vec::new(),
        validation,
        result_root,
        affected_owners: affected.into_iter().collect(),
    }))
}

fn transaction_failure_status(error: &Diagnostic) -> TransactionStatus {
    if error.class == DiagnosticClass::Resource {
        TransactionStatus::ResourceExhausted
    } else {
        TransactionStatus::InvalidGraph
    }
}

fn check_precondition(
    precondition: &SemanticPrecondition,
    root: &GraphRoot,
    modules: &[MeaningModule],
    root_digest: RootObjectDigest,
) -> Result<(), Diagnostic> {
    match precondition {
        SemanticPrecondition::RootDigest { equals } if *equals != root_digest => {
            Err(transaction_error(
                DiagnosticClass::Semantic,
                "semantic_precondition_root",
                "root digest precondition failed",
            ))
        }
        SemanticPrecondition::OwnerExists { owner }
            if owner_name(*owner, root, modules).is_none() =>
        {
            Err(transaction_error(
                DiagnosticClass::Semantic,
                "semantic_precondition_owner_missing",
                "required semantic owner is absent",
            ))
        }
        SemanticPrecondition::OwnerAbsent { owner }
            if owner_name(*owner, root, modules).is_some() =>
        {
            Err(transaction_error(
                DiagnosticClass::Semantic,
                "semantic_precondition_owner_present",
                "semantic owner expected to be absent is present",
            ))
        }
        SemanticPrecondition::OwnerName { owner, equals }
            if owner_name(*owner, root, modules).as_deref() != Some(equals.as_str()) =>
        {
            Err(transaction_error(
                DiagnosticClass::Semantic,
                "semantic_precondition_owner_name",
                "semantic owner name precondition failed",
            ))
        }
        _ => Ok(()),
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_operation(
    repository: &SemanticRepository,
    operation: &SemanticOperation,
    base: RevisionId,
    root: &mut GraphRoot,
    modules: &mut Vec<MeaningModule>,
    module_ids: &mut BTreeSet<ModuleId>,
    module_names: &mut BTreeSet<String>,
    affected: &mut BTreeSet<AffectedOwner>,
    work: &mut usize,
    budget: TransactionBudget,
) -> Result<(), Diagnostic> {
    consume_work(work, 1, budget)?;
    match operation {
        SemanticOperation::SetPackageMetadata { name } => {
            root.package_name.clone_from(name);
            affected.insert(AffectedOwner::Package(root.package_id.clone()));
        }
        SemanticOperation::AddDependency { binding } => {
            if root
                .dependencies
                .iter()
                .any(|value| value.alias == binding.alias || value.package_id == binding.package_id)
            {
                return Err(operation_error(
                    "add_dependency",
                    "dependency alias or package identity already exists",
                ));
            }
            root.dependencies.push(binding.clone());
            root.dependencies.sort();
            affected.insert(AffectedOwner::Package(root.package_id.clone()));
        }
        SemanticOperation::ReplaceDependency { binding } => {
            if root
                .dependencies
                .iter()
                .any(|value| value.alias != binding.alias && value.package_id == binding.package_id)
            {
                return Err(operation_error(
                    "replace_dependency",
                    "replacement package identity is already bound by another alias",
                ));
            }
            let existing = root
                .dependencies
                .iter_mut()
                .find(|value| value.alias == binding.alias)
                .ok_or_else(|| {
                    operation_error("replace_dependency", "dependency alias is absent")
                })?;
            *existing = binding.clone();
            root.dependencies.sort();
            affected.insert(AffectedOwner::Package(root.package_id.clone()));
        }
        SemanticOperation::RemoveDependency { alias } => {
            let before = root.dependencies.len();
            root.dependencies.retain(|value| value.alias != *alias);
            if root.dependencies.len() == before {
                return Err(operation_error(
                    "remove_dependency",
                    "dependency alias is absent",
                ));
            }
            affected.insert(AffectedOwner::Package(root.package_id.clone()));
        }
        SemanticOperation::CreateModule { id, name } => {
            if module_ids.contains(id) || module_names.contains(name) {
                return Err(operation_error(
                    "create_module",
                    "module identity or name exists",
                ));
            }
            let module = MeaningModule {
                graph_contract_version: super::meaning::GRAPH_CONTRACT_VERSION,
                module_id: *id,
                module: Module {
                    name: name.clone(),
                    imports: Vec::new(),
                    exports: Vec::new(),
                    declarations: Vec::new(),
                },
                declarations: Vec::new(),
                relations: Vec::new(),
                documentation: Vec::new(),
                annotations: Vec::new(),
            };
            root.modules.push(ModuleObjectRef {
                id: *id,
                name: name.clone(),
                object: module.digest()?,
            });
            modules.push(module);
            module_ids.insert(*id);
            module_names.insert(name.clone());
            affected.insert(AffectedOwner::Module(*id));
        }
        SemanticOperation::RenameModule { module, new_name } => {
            let index = module_index(modules, *module)?;
            let old_name = modules[index].module.name.clone();
            if old_name == *new_name {
                return Ok(());
            }
            if module_names.contains(new_name) {
                return Err(operation_error(
                    "rename_module",
                    "destination module name exists",
                ));
            }
            modules[index].module.name.clone_from(new_name);
            module_names.remove(&old_name);
            module_names.insert(new_name.clone());
            affected.insert(AffectedOwner::Module(*module));
        }
        SemanticOperation::DeleteModule { module } => {
            let name = modules
                .iter()
                .find(|candidate| candidate.module_id == *module)
                .map(|candidate| candidate.module.name.clone())
                .ok_or_else(|| operation_error("module_selection", "selected module is absent"))?;
            delete_module(*module, base, root, modules, affected)?;
            module_ids.remove(module);
            module_names.remove(&name);
        }
        SemanticOperation::CreateDeclaration {
            module,
            identity,
            declaration,
            exported,
        } => insert_declaration(
            *module,
            identity.clone(),
            declaration.clone(),
            *exported,
            false,
            root,
            modules,
            affected,
        )?,
        SemanticOperation::CloneOwner {
            source,
            destination,
            identity,
            declaration,
            exported,
        } => {
            declaration_location(modules, *source)?;
            insert_declaration(
                *destination,
                identity.clone(),
                declaration.clone(),
                *exported,
                false,
                root,
                modules,
                affected,
            )?;
        }
        SemanticOperation::RestoreOwner {
            historical_revision,
            declaration,
            destination,
            exported,
        } => {
            let historical = repository.reconstruct_revision(*historical_revision)?;
            if historical.record.core.repository_id != root.repository_id {
                return Err(operation_error(
                    "restore_owner",
                    "historical revision belongs to a foreign repository",
                ));
            }
            let (historical_module, historical_index) =
                declaration_location(&historical.modules, *declaration)?;
            let identity =
                historical.modules[historical_module].declarations[historical_index].clone();
            let value =
                historical.modules[historical_module].module.declarations[historical_index].clone();
            insert_declaration(
                *destination,
                identity,
                value,
                *exported,
                true,
                root,
                modules,
                affected,
            )?;
        }
        SemanticOperation::ReplaceDeclaration {
            declaration,
            identity,
            value,
        } => {
            let (module_index, declaration_index) = declaration_location(modules, *declaration)?;
            if identity.id != *declaration {
                return Err(operation_error(
                    "replace_declaration",
                    "replacement must preserve declaration identity",
                ));
            }
            modules[module_index].declarations[declaration_index] = identity.clone();
            modules[module_index].module.declarations[declaration_index] = value.clone();
            affected.insert(AffectedOwner::Declaration(*declaration));
        }
        SemanticOperation::RenameDeclaration {
            declaration,
            new_name,
        } => rename_declaration(*declaration, new_name, modules, affected)?,
        SemanticOperation::MoveDeclaration {
            declaration,
            destination,
        } => move_declaration(*declaration, *destination, root, modules, affected)?,
        SemanticOperation::DeleteOwner { owner } => {
            let deleted_module = if let OwnerSelector::Module(module) = owner {
                Some((
                    *module,
                    modules
                        .iter()
                        .find(|candidate| candidate.module_id == *module)
                        .map(|candidate| candidate.module.name.clone())
                        .ok_or_else(|| {
                            operation_error("module_selection", "selected module is absent")
                        })?,
                ))
            } else {
                None
            };
            delete_owner(*owner, base, root, modules, affected)?;
            if let Some((module, name)) = deleted_module {
                module_ids.remove(&module);
                module_names.remove(&name);
            }
        }
        SemanticOperation::AddRecordField { record, id, field } => {
            let (module_index, declaration_index) = declaration_location(modules, *record)?;
            let declaration = &mut modules[module_index].module.declarations[declaration_index];
            let Declaration::Record(record_value) = declaration else {
                return Err(operation_error("add_record_field", "owner is not a record"));
            };
            record_value.fields.push(field.clone());
            modules[module_index].declarations[declaration_index]
                .members
                .push(MemberIdentity::Field {
                    id: *id,
                    name: field.name.clone(),
                });
            affected.insert(AffectedOwner::Declaration(*record));
        }
        SemanticOperation::RenameRecordField { field, new_name } => {
            rename_member(MemberKey::Field(*field), new_name, modules, affected)?;
        }
        SemanticOperation::ChangeRecordFieldType { field, ty } => {
            let (module_index, declaration_index, member_index) =
                member_location(modules, MemberKey::Field(*field))?;
            let field_index = member_kind_ordinal(
                &modules[module_index].declarations[declaration_index].members,
                member_index,
                |member| matches!(member, MemberIdentity::Field { .. }),
            );
            let Declaration::Record(record) =
                &mut modules[module_index].module.declarations[declaration_index]
            else {
                return Err(operation_error(
                    "change_record_field_type",
                    "owner is not a record",
                ));
            };
            record.fields[field_index].ty.clone_from(ty);
            affected.insert(AffectedOwner::Declaration(
                modules[module_index].declarations[declaration_index].id,
            ));
        }
        SemanticOperation::RemoveRecordField { field } => {
            remove_member(MemberKey::Field(*field), base, root, modules, affected)?;
        }
        SemanticOperation::AddVariantCase { variant, id, case } => {
            let (module_index, declaration_index) = declaration_location(modules, *variant)?;
            let Declaration::Variant(value) =
                &mut modules[module_index].module.declarations[declaration_index]
            else {
                return Err(operation_error(
                    "add_variant_case",
                    "owner is not a variant",
                ));
            };
            value.cases.push(case.clone());
            modules[module_index].declarations[declaration_index]
                .members
                .push(MemberIdentity::Case {
                    id: *id,
                    name: case.name.clone(),
                });
            affected.insert(AffectedOwner::Declaration(*variant));
        }
        SemanticOperation::RenameVariantCase { case, new_name } => {
            rename_member(MemberKey::Case(*case), new_name, modules, affected)?;
        }
        SemanticOperation::ChangeVariantPayload { case, payload } => {
            let (module_index, declaration_index, member_index) =
                member_location(modules, MemberKey::Case(*case))?;
            let case_index = member_kind_ordinal(
                &modules[module_index].declarations[declaration_index].members,
                member_index,
                |member| matches!(member, MemberIdentity::Case { .. }),
            );
            let Declaration::Variant(value) =
                &mut modules[module_index].module.declarations[declaration_index]
            else {
                return Err(operation_error(
                    "change_variant_payload",
                    "owner is not a variant",
                ));
            };
            value.cases[case_index].payload.clone_from(payload);
            affected.insert(AffectedOwner::Declaration(
                modules[module_index].declarations[declaration_index].id,
            ));
        }
        SemanticOperation::RemoveVariantCase { case } => {
            remove_member(MemberKey::Case(*case), base, root, modules, affected)?;
        }
        SemanticOperation::AddInterfaceOperation {
            interface,
            id,
            parameter_ids,
            operation_value,
        } => add_interface_operation(
            *interface,
            *id,
            parameter_ids,
            operation_value,
            modules,
            affected,
        )?,
        SemanticOperation::ChangeInterfaceOperation {
            operation,
            parameter_ids,
            operation_value,
        } => change_interface_operation(
            *operation,
            parameter_ids,
            operation_value,
            base,
            root,
            modules,
            affected,
        )?,
        SemanticOperation::RemoveInterfaceOperation { operation } => {
            remove_member(
                MemberKey::Operation(*operation),
                base,
                root,
                modules,
                affected,
            )?;
        }
        SemanticOperation::ChangeSignature {
            declaration,
            parameters,
            result,
            effect,
            members,
        } => change_signature(
            *declaration,
            parameters,
            result,
            effect.as_ref(),
            members,
            RewriteContext {
                base,
                root,
                modules,
                affected,
            },
        )?,
        SemanticOperation::ReplaceBody {
            declaration,
            body,
            bindings,
            expressions,
        } => replace_body(
            *declaration,
            body,
            bindings,
            expressions,
            RewriteContext {
                base,
                root,
                modules,
                affected,
            },
        )?,
        SemanticOperation::ReplaceExpression {
            expression,
            value,
            declaration_bindings,
            declaration_expressions,
        } => {
            let (before_module, before_declaration) =
                expression_declaration_location(modules, *expression)?;
            let before = modules[before_module].declarations[before_declaration].clone();
            let (module_index, declaration_index) =
                replace_expression(*expression, value.clone(), modules)?;
            modules[module_index].declarations[declaration_index].bindings =
                declaration_bindings.clone();
            modules[module_index].declarations[declaration_index].expressions =
                declaration_expressions.clone();
            tombstone_removed_nested_identities(
                root,
                &before,
                &modules[module_index].declarations[declaration_index],
                base,
            );
            affected.insert(AffectedOwner::Declaration(
                modules[module_index].declarations[declaration_index].id,
            ));
        }
        SemanticOperation::RebindReference {
            expression,
            reference,
        } => {
            let (module_index, declaration_index) =
                rebind_reference(*expression, reference, modules)?;
            affected.insert(AffectedOwner::Declaration(
                modules[module_index].declarations[declaration_index].id,
            ));
        }
        SemanticOperation::RenameBinding { binding, new_name } => {
            rename_binding(*binding, new_name, modules, affected)?;
        }
        SemanticOperation::ReplaceTestExpectation {
            test,
            expected,
            bindings,
            expressions,
        } => replace_test_expectation(
            *test,
            expected,
            bindings,
            expressions,
            RewriteContext {
                base,
                root,
                modules,
                affected,
            },
        )?,
        SemanticOperation::CreateTarget { target } => {
            if root.targets.iter().any(|value| value.id == target.id)
                || root.targets.iter().any(|value| value.name == target.name)
            {
                return Err(operation_error(
                    "create_target",
                    "target identity or name exists",
                ));
            }
            root.targets.push(target.clone());
            root.targets.sort();
            affected.insert(AffectedOwner::Target(target.id));
        }
        SemanticOperation::DeleteTarget { target } => {
            delete_target(*target, base, root, affected)?;
        }
    }
    Ok(())
}

// The remaining helpers deliberately operate on stable IDs and resolved relation owners. Names
// are updated only after an exact identity has been selected.

#[derive(Clone, Copy)]
enum MemberKey {
    Field(FieldId),
    Case(CaseId),
    Operation(OperationId),
}

fn module_index(modules: &[MeaningModule], id: ModuleId) -> Result<usize, Diagnostic> {
    modules
        .iter()
        .position(|module| module.module_id == id)
        .ok_or_else(|| operation_error("module_selection", "selected module is absent"))
}

fn declaration_location(
    modules: &[MeaningModule],
    id: DeclarationId,
) -> Result<(usize, usize), Diagnostic> {
    for (module_index, module) in modules.iter().enumerate() {
        if let Some(declaration_index) = module
            .declarations
            .iter()
            .position(|identity| identity.id == id)
        {
            return Ok((module_index, declaration_index));
        }
    }
    Err(operation_error(
        "declaration_selection",
        "selected declaration is absent",
    ))
}

fn member_location(
    modules: &[MeaningModule],
    key: MemberKey,
) -> Result<(usize, usize, usize), Diagnostic> {
    for (module_index, module) in modules.iter().enumerate() {
        for (declaration_index, declaration) in module.declarations.iter().enumerate() {
            if let Some(member_index) = declaration
                .members
                .iter()
                .position(|member| member_matches(member, key))
            {
                return Ok((module_index, declaration_index, member_index));
            }
        }
    }
    Err(operation_error(
        "member_selection",
        "selected member is absent or belongs to a foreign identity domain",
    ))
}

fn member_matches(member: &MemberIdentity, key: MemberKey) -> bool {
    match (member, key) {
        (MemberIdentity::Field { id, .. }, MemberKey::Field(expected)) => *id == expected,
        (MemberIdentity::Case { id, .. }, MemberKey::Case(expected)) => *id == expected,
        (MemberIdentity::Operation { id, .. }, MemberKey::Operation(expected)) => *id == expected,
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_declaration(
    module: ModuleId,
    identity: DeclarationIdentity,
    declaration: Declaration,
    exported: bool,
    restore: bool,
    root: &mut GraphRoot,
    modules: &mut [MeaningModule],
    affected: &mut BTreeSet<AffectedOwner>,
) -> Result<(), Diagnostic> {
    if identity.name != declaration.name() {
        return Err(operation_error(
            "create_declaration",
            "declaration identity and meaning names differ",
        ));
    }
    if declaration_location(modules, identity.id).is_ok() {
        return Err(operation_error(
            "create_declaration",
            "declaration identity already exists",
        ));
    }
    let index = module_index(modules, module)?;
    if modules[index]
        .module
        .declarations
        .iter()
        .any(|candidate| candidate.name() == declaration.name())
    {
        return Err(operation_error(
            "create_declaration",
            "declaration name already exists in the destination module",
        ));
    }
    if restore {
        if !root
            .tombstones
            .iter()
            .any(|tombstone| tombstone.identity == TombstoneIdentity::Declaration(identity.id))
        {
            return Err(operation_error(
                "restore_owner",
                "selected historical declaration is not deleted in the current revision",
            ));
        }
        remove_restored_tombstones(root, &identity);
    }
    modules[index].module.declarations.push(declaration);
    if exported {
        modules[index].module.exports.push(identity.id);
        modules[index].module.exports.sort();
        modules[index].module.exports.dedup();
    }
    affected.insert(AffectedOwner::Declaration(identity.id));
    affected.insert(AffectedOwner::Module(module));
    modules[index].declarations.push(identity);
    Ok(())
}

fn rename_declaration(
    declaration: DeclarationId,
    new_name: &str,
    modules: &mut [MeaningModule],
    affected: &mut BTreeSet<AffectedOwner>,
) -> Result<(), Diagnostic> {
    let (module_index, declaration_index) = declaration_location(modules, declaration)?;
    let old_name = modules[module_index].declarations[declaration_index]
        .name
        .clone();
    if old_name == new_name {
        return Ok(());
    }
    if modules[module_index]
        .declarations
        .iter()
        .any(|identity| identity.name == new_name)
    {
        return Err(operation_error(
            "rename_declaration",
            "destination declaration name exists",
        ));
    }
    let identity = &mut modules[module_index].declarations[declaration_index];
    identity.name = new_name.to_owned();
    set_declaration_name(
        &mut modules[module_index].module.declarations[declaration_index],
        new_name,
    );
    affected.insert(AffectedOwner::Declaration(declaration));
    Ok(())
}

fn move_declaration(
    declaration: DeclarationId,
    destination: ModuleId,
    root: &mut GraphRoot,
    modules: &mut [MeaningModule],
    affected: &mut BTreeSet<AffectedOwner>,
) -> Result<(), Diagnostic> {
    let (source_index, declaration_index) = declaration_location(modules, declaration)?;
    let destination_index = module_index(modules, destination)?;
    if source_index == destination_index {
        return Ok(());
    }
    let destination_name = modules[destination_index].module.name.clone();
    let declaration_name = modules[source_index].declarations[declaration_index]
        .name
        .clone();
    if modules[destination_index]
        .declarations
        .iter()
        .any(|value| value.name == declaration_name)
    {
        return Err(operation_error(
            "move_declaration",
            "destination module already owns that declaration name",
        ));
    }
    let source_module = modules[source_index].module_id;
    let referencing_modules = relation_modules_for_declaration(modules, declaration);
    for index in referencing_modules {
        if modules[index].module_id != destination {
            ensure_import(
                &mut modules[index].module,
                super::language::ModuleReference {
                    package: root.package_id.clone(),
                    module: destination,
                },
                &destination_name,
            )?;
        }
        replace_declaration_module_references_in_module(
            &mut modules[index].module,
            &root.package_id,
            source_module,
            declaration,
            destination,
        );
    }
    let was_exported = modules[source_index].module.exports.contains(&declaration);
    modules[source_index]
        .module
        .exports
        .retain(|value| *value != declaration);
    let identity = modules[source_index].declarations.remove(declaration_index);
    let value = modules[source_index]
        .module
        .declarations
        .remove(declaration_index);
    modules[destination_index].declarations.push(identity);
    modules[destination_index].module.declarations.push(value);
    if was_exported {
        modules[destination_index].module.exports.push(declaration);
        modules[destination_index].module.exports.sort();
        modules[destination_index].module.exports.dedup();
    }
    for target in &mut root.targets {
        if target.component == declaration {
            target.component_module = destination;
        }
    }
    affected.insert(AffectedOwner::Declaration(declaration));
    affected.insert(AffectedOwner::Module(modules[source_index].module_id));
    affected.insert(AffectedOwner::Module(destination));
    Ok(())
}

fn delete_owner(
    owner: OwnerSelector,
    base: RevisionId,
    root: &mut GraphRoot,
    modules: &mut Vec<MeaningModule>,
    affected: &mut BTreeSet<AffectedOwner>,
) -> Result<(), Diagnostic> {
    match owner {
        OwnerSelector::Module(module) => delete_module(module, base, root, modules, affected),
        OwnerSelector::Target(target) => delete_target(target, base, root, affected),
        OwnerSelector::Declaration(declaration) => {
            let (module_index, declaration_index) = declaration_location(modules, declaration)?;
            let owning_module = modules[module_index].module_id;
            if modules.iter().any(|module| {
                module.relations.iter().any(|relation| {
                    matches!(
                        &relation.target,
                        RelationTarget::Declaration(reference)
                            if reference.declaration == declaration
                    ) && !relation_source_belongs_to_declaration(
                        &relation.source,
                        &modules[module_index].declarations[declaration_index],
                    ) && !(relation.role == RelationRole::Export
                        && relation.source == RelationSource::Module(owning_module))
                })
            }) {
                return Err(operation_error(
                    "delete_owner",
                    "declaration still has incoming semantic references",
                ));
            }
            if root
                .targets
                .iter()
                .any(|target| target.component == declaration)
            {
                return Err(operation_error(
                    "delete_owner",
                    "declaration is selected by an executable target",
                ));
            }
            let identity = modules[module_index].declarations.remove(declaration_index);
            modules[module_index]
                .module
                .declarations
                .remove(declaration_index);
            modules[module_index]
                .module
                .exports
                .retain(|id| *id != declaration);
            tombstone_declaration(root, &identity, base);
            affected.insert(AffectedOwner::Declaration(declaration));
            affected.insert(AffectedOwner::Module(modules[module_index].module_id));
            Ok(())
        }
    }
}

fn delete_module(
    module: ModuleId,
    base: RevisionId,
    root: &mut GraphRoot,
    modules: &mut Vec<MeaningModule>,
    affected: &mut BTreeSet<AffectedOwner>,
) -> Result<(), Diagnostic> {
    let index = module_index(modules, module)?;
    if !modules[index].module.declarations.is_empty()
        || root
            .targets
            .iter()
            .any(|target| target.component_module == module)
    {
        return Err(operation_error(
            "delete_module",
            "module is not empty or is selected by a target",
        ));
    }
    if modules.iter().any(|candidate| {
        candidate.module_id != module
            && candidate.module.imports.iter().any(|import| {
                import.target.package == root.package_id && import.target.module == module
            })
    }) {
        return Err(operation_error(
            "delete_module",
            "module still has incoming imports",
        ));
    }
    let removed = modules.remove(index);
    root.modules.retain(|reference| reference.id != module);
    add_tombstone(
        root,
        TombstoneIdentity::Module(module),
        base,
        removed.module.name,
    );
    affected.insert(AffectedOwner::Module(module));
    Ok(())
}

fn delete_target(
    target: TargetId,
    base: RevisionId,
    root: &mut GraphRoot,
    affected: &mut BTreeSet<AffectedOwner>,
) -> Result<(), Diagnostic> {
    let index = root
        .targets
        .iter()
        .position(|value| value.id == target)
        .ok_or_else(|| operation_error("delete_target", "target is absent"))?;
    let removed = root.targets.remove(index);
    add_tombstone(root, TombstoneIdentity::Target(target), base, removed.name);
    affected.insert(AffectedOwner::Target(target));
    Ok(())
}

fn rename_member(
    key: MemberKey,
    new_name: &str,
    modules: &mut [MeaningModule],
    affected: &mut BTreeSet<AffectedOwner>,
) -> Result<(), Diagnostic> {
    let (module_index, declaration_index, member_index) = member_location(modules, key)?;
    let old_name = modules[module_index].declarations[declaration_index].members[member_index]
        .name()
        .to_owned();
    if old_name == new_name {
        return Ok(());
    }
    let expression_ids = relation_sources_for_member(modules, key);
    for expression_id in expression_ids {
        if let Ok((_, _, expression)) = expression_mut_by_id(modules, expression_id) {
            match (key, expression) {
                (MemberKey::Field(_), Expression::Field { field, .. }) if *field == old_name => {
                    *field = new_name.to_owned();
                }
                (MemberKey::Field(_), Expression::Record { fields, .. }) => {
                    for field in fields {
                        if field.name == old_name {
                            field.name = new_name.to_owned();
                        }
                    }
                }
                (MemberKey::Case(_), Expression::Variant { case, .. }) if *case == old_name => {
                    *case = new_name.to_owned();
                }
                (MemberKey::Case(_), Expression::Match { arms, .. }) => {
                    for arm in arms {
                        if arm.case == old_name {
                            arm.case = new_name.to_owned();
                        }
                    }
                }
                (MemberKey::Operation(_), Expression::Perform { operation, .. })
                    if *operation == old_name =>
                {
                    *operation = new_name.to_owned();
                }
                _ => {}
            }
        }
    }
    let members = modules[module_index].declarations[declaration_index]
        .members
        .clone();
    rename_member_definition(
        &mut modules[module_index].module.declarations[declaration_index],
        &members,
        member_index,
        new_name,
    )?;
    set_member_name(
        &mut modules[module_index].declarations[declaration_index].members[member_index],
        new_name,
    );
    affected.insert(AffectedOwner::Declaration(
        modules[module_index].declarations[declaration_index].id,
    ));
    Ok(())
}

fn remove_member(
    key: MemberKey,
    base: RevisionId,
    root: &mut GraphRoot,
    modules: &mut [MeaningModule],
    affected: &mut BTreeSet<AffectedOwner>,
) -> Result<(), Diagnostic> {
    if !relation_sources_for_member(modules, key).is_empty() {
        return Err(operation_error(
            "remove_member",
            "member still has semantic use sites; repair them in the same transaction",
        ));
    }
    let (module_index, declaration_index, member_index) = member_location(modules, key)?;
    let members = modules[module_index].declarations[declaration_index]
        .members
        .clone();
    let end = if matches!(key, MemberKey::Operation(_)) {
        following_parameter_end(&members, member_index)
    } else {
        member_index + 1
    };
    remove_member_definition(
        &mut modules[module_index].module.declarations[declaration_index],
        &members,
        member_index,
    )?;
    let removed = modules[module_index].declarations[declaration_index]
        .members
        .drain(member_index..end)
        .collect::<Vec<_>>();
    for identity in removed {
        let (tombstone, name) = member_tombstone(identity);
        add_tombstone(root, tombstone, base, name);
    }
    affected.insert(AffectedOwner::Declaration(
        modules[module_index].declarations[declaration_index].id,
    ));
    Ok(())
}

fn add_interface_operation(
    interface: DeclarationId,
    operation_id: OperationId,
    parameter_ids: &[ParameterId],
    operation: &InterfaceOperation,
    modules: &mut [MeaningModule],
    affected: &mut BTreeSet<AffectedOwner>,
) -> Result<(), Diagnostic> {
    if parameter_ids.len() != operation.parameters.len() {
        return Err(operation_error(
            "add_interface_operation",
            "parameter identity count does not match operation parameters",
        ));
    }
    let (module_index, declaration_index) = declaration_location(modules, interface)?;
    let Declaration::Interface(value) =
        &mut modules[module_index].module.declarations[declaration_index]
    else {
        return Err(operation_error(
            "add_interface_operation",
            "owner is not an interface",
        ));
    };
    value.operations.push(operation.clone());
    let members = &mut modules[module_index].declarations[declaration_index].members;
    members.push(MemberIdentity::Operation {
        id: operation_id,
        name: operation.name.clone(),
    });
    members.extend(
        parameter_ids
            .iter()
            .zip(&operation.parameters)
            .map(|(id, parameter)| MemberIdentity::Parameter {
                id: *id,
                name: parameter.name.clone(),
            }),
    );
    affected.insert(AffectedOwner::Declaration(interface));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn change_interface_operation(
    operation_id: OperationId,
    parameter_ids: &[ParameterId],
    operation: &InterfaceOperation,
    base: RevisionId,
    root: &mut GraphRoot,
    modules: &mut [MeaningModule],
    affected: &mut BTreeSet<AffectedOwner>,
) -> Result<(), Diagnostic> {
    if parameter_ids.len() != operation.parameters.len() {
        return Err(operation_error(
            "change_interface_operation",
            "parameter identity count does not match operation parameters",
        ));
    }
    let (module_index, declaration_index, member_index) =
        member_location(modules, MemberKey::Operation(operation_id))?;
    let before = modules[module_index].declarations[declaration_index].clone();
    let members = &mut modules[module_index].declarations[declaration_index].members;
    let end = following_parameter_end(members, member_index);
    members.splice(
        member_index..end,
        std::iter::once(MemberIdentity::Operation {
            id: operation_id,
            name: operation.name.clone(),
        })
        .chain(
            parameter_ids
                .iter()
                .zip(&operation.parameters)
                .map(|(id, parameter)| MemberIdentity::Parameter {
                    id: *id,
                    name: parameter.name.clone(),
                }),
        ),
    );
    let operation_index = member_kind_ordinal(members, member_index, |member| {
        matches!(member, MemberIdentity::Operation { .. })
    });
    let Declaration::Interface(value) =
        &mut modules[module_index].module.declarations[declaration_index]
    else {
        return Err(operation_error(
            "change_interface_operation",
            "owner is not an interface",
        ));
    };
    value.operations[operation_index] = operation.clone();
    tombstone_removed_nested_identities(
        root,
        &before,
        &modules[module_index].declarations[declaration_index],
        base,
    );
    affected.insert(AffectedOwner::Declaration(
        modules[module_index].declarations[declaration_index].id,
    ));
    Ok(())
}

struct RewriteContext<'a> {
    base: RevisionId,
    root: &'a mut GraphRoot,
    modules: &'a mut [MeaningModule],
    affected: &'a mut BTreeSet<AffectedOwner>,
}

fn change_signature(
    declaration: DeclarationId,
    parameters: &[Parameter],
    result: &Type,
    effect: Option<&Effect>,
    members: &[MemberIdentity],
    context: RewriteContext<'_>,
) -> Result<(), Diagnostic> {
    let RewriteContext {
        base,
        root,
        modules,
        affected,
    } = context;
    let (module_index, declaration_index) = declaration_location(modules, declaration)?;
    let before = modules[module_index].declarations[declaration_index].clone();
    match &mut modules[module_index].module.declarations[declaration_index] {
        Declaration::External(function) => {
            if effect.is_some() {
                return Err(operation_error(
                    "change_signature",
                    "external functions do not carry a task effect",
                ));
            }
            function.parameters = parameters.to_vec();
            function.result = result.clone();
        }
        Declaration::Function(function) => {
            function.parameters = parameters.to_vec();
            function.result = result.clone();
            if let Some(effect) = effect {
                function.effect = effect.clone();
            }
        }
        _ => {
            return Err(operation_error(
                "change_signature",
                "selected declaration is not a function",
            ));
        }
    }
    modules[module_index].declarations[declaration_index].members = members.to_vec();
    tombstone_removed_nested_identities(
        root,
        &before,
        &modules[module_index].declarations[declaration_index],
        base,
    );
    affected.insert(AffectedOwner::Declaration(declaration));
    Ok(())
}

fn replace_body(
    declaration: DeclarationId,
    body: &Expression,
    bindings: &[BindingIdentity],
    expressions: &[ExpressionIdentity],
    context: RewriteContext<'_>,
) -> Result<(), Diagnostic> {
    let RewriteContext {
        base,
        root,
        modules,
        affected,
    } = context;
    let (module_index, declaration_index) = declaration_location(modules, declaration)?;
    let before = modules[module_index].declarations[declaration_index].clone();
    match &mut modules[module_index].module.declarations[declaration_index] {
        Declaration::Function(function) => function.body = body.clone(),
        Declaration::Constant(constant) => constant.value = body.clone(),
        _ => {
            return Err(operation_error(
                "replace_body",
                "selected declaration has no replaceable function or constant body",
            ));
        }
    }
    modules[module_index].declarations[declaration_index].bindings = bindings.to_vec();
    modules[module_index].declarations[declaration_index].expressions = expressions.to_vec();
    tombstone_removed_nested_identities(
        root,
        &before,
        &modules[module_index].declarations[declaration_index],
        base,
    );
    affected.insert(AffectedOwner::Declaration(declaration));
    Ok(())
}

fn replace_expression(
    expression: ExpressionId,
    value: Expression,
    modules: &mut [MeaningModule],
) -> Result<(usize, usize), Diagnostic> {
    let (module_index, declaration_index, selected) = expression_mut_by_id(modules, expression)?;
    *selected = value;
    Ok((module_index, declaration_index))
}

fn rebind_reference(
    expression: ExpressionId,
    reference: &DeclarationReference,
    modules: &mut [MeaningModule],
) -> Result<(usize, usize), Diagnostic> {
    let (module_index, declaration_index, selected) = expression_mut_by_id(modules, expression)?;
    match selected {
        Expression::Call { function, .. } | Expression::FunctionRef { function, .. } => {
            function.clone_from(reference);
        }
        Expression::Constant(target, _) => target.clone_from(reference),
        Expression::Record { ty, .. } => *ty = Some(reference.clone()),
        Expression::Variant { ty, .. } => ty.clone_from(reference),
        _ => {
            return Err(operation_error(
                "rebind_reference",
                "selected expression has no rebindable semantic reference",
            ));
        }
    }
    Ok((module_index, declaration_index))
}

fn rename_binding(
    binding: BindingId,
    new_name: &str,
    modules: &mut [MeaningModule],
    affected: &mut BTreeSet<AffectedOwner>,
) -> Result<(), Diagnostic> {
    let mut location = None;
    for (module_index, module) in modules.iter().enumerate() {
        for (declaration_index, identity) in module.declarations.iter().enumerate() {
            if let Some(binding_index) = identity
                .bindings
                .iter()
                .position(|candidate| candidate.id == binding)
            {
                location = Some((module_index, declaration_index, binding_index));
                break;
            }
        }
    }
    let (module_index, declaration_index, binding_index) = location
        .ok_or_else(|| operation_error("rename_binding", "selected binding identity is absent"))?;
    let old_name = modules[module_index].declarations[declaration_index].bindings[binding_index]
        .name
        .clone();
    let source_ids = modules
        .iter()
        .flat_map(|module| &module.relations)
        .filter_map(|relation| match &relation.target {
            RelationTarget::Binding {
                binding: target, ..
            } if *target == binding => match relation.source {
                RelationSource::Expression(id) => Some(id),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    for expression in source_ids {
        if let Ok((_, _, Expression::Variable(name, _))) = expression_mut_by_id(modules, expression)
            && *name == old_name
        {
            *name = new_name.to_owned();
        }
    }
    let identity =
        modules[module_index].declarations[declaration_index].bindings[binding_index].clone();
    rename_binding_definition(
        &mut modules[module_index].module.declarations[declaration_index],
        &identity,
        new_name,
    )?;
    modules[module_index].declarations[declaration_index].bindings[binding_index].name =
        new_name.to_owned();
    affected.insert(AffectedOwner::Declaration(
        modules[module_index].declarations[declaration_index].id,
    ));
    Ok(())
}

fn replace_test_expectation(
    test: DeclarationId,
    expected: &Expression,
    bindings: &[BindingIdentity],
    expressions: &[ExpressionIdentity],
    context: RewriteContext<'_>,
) -> Result<(), Diagnostic> {
    let RewriteContext {
        base,
        root,
        modules,
        affected,
    } = context;
    let (module_index, declaration_index) = declaration_location(modules, test)?;
    let before = modules[module_index].declarations[declaration_index].clone();
    let Declaration::Test(value) =
        &mut modules[module_index].module.declarations[declaration_index]
    else {
        return Err(operation_error(
            "replace_test_expectation",
            "selected declaration is not a test",
        ));
    };
    value.expected = expected.clone();
    modules[module_index].declarations[declaration_index].bindings = bindings.to_vec();
    modules[module_index].declarations[declaration_index].expressions = expressions.to_vec();
    tombstone_removed_nested_identities(
        root,
        &before,
        &modules[module_index].declarations[declaration_index],
        base,
    );
    affected.insert(AffectedOwner::Declaration(test));
    Ok(())
}

fn relation_modules_for_declaration(
    modules: &[MeaningModule],
    declaration: DeclarationId,
) -> Vec<usize> {
    modules
        .iter()
        .enumerate()
        .filter(|(_, module)| {
            module.relations.iter().any(|relation| {
                matches!(
                    &relation.target,
                    RelationTarget::Declaration(reference)
                        if reference.declaration == declaration
                )
            })
        })
        .map(|(index, _)| index)
        .collect()
}

fn relation_sources_for_member(modules: &[MeaningModule], key: MemberKey) -> Vec<ExpressionId> {
    modules
        .iter()
        .flat_map(|module| &module.relations)
        .filter(|relation| match (&relation.target, key) {
            (RelationTarget::Field { field, .. }, MemberKey::Field(expected)) => *field == expected,
            (RelationTarget::Case { case, .. }, MemberKey::Case(expected)) => *case == expected,
            (RelationTarget::Operation { operation, .. }, MemberKey::Operation(expected)) => {
                *operation == expected
            }
            _ => false,
        })
        .filter_map(|relation| match relation.source {
            RelationSource::Expression(id) => Some(id),
            _ => None,
        })
        .collect()
}

fn ensure_import(
    module: &mut Module,
    target: super::language::ModuleReference,
    target_name: &str,
) -> Result<(), Diagnostic> {
    if module.imports.iter().any(|import| import.target == target) {
        return Ok(());
    }
    let base = target_name.rsplit('.').next().unwrap_or("moved").to_owned();
    let mut alias = base.clone();
    let mut ordinal = 2usize;
    while module.imports.iter().any(|import| import.alias == alias) {
        alias = format!("{base}-{ordinal}");
        ordinal = ordinal.checked_add(1).ok_or_else(|| {
            operation_error("move_declaration", "import alias allocation exhausted")
        })?;
    }
    module.imports.push(super::language::Import {
        alias: alias.clone(),
        target,
        span: semantic_span(),
    });
    module
        .imports
        .sort_by(|left, right| left.alias.cmp(&right.alias));
    Ok(())
}

fn replace_declaration_module_references_in_module(
    module: &mut Module,
    package: &super::package::PackageId,
    source_module: ModuleId,
    target_declaration: DeclarationId,
    destination_module: ModuleId,
) {
    for declaration in &mut module.declarations {
        replace_declaration_module_references_in_declaration(
            declaration,
            package,
            source_module,
            target_declaration,
            destination_module,
        );
    }
}

fn replace_declaration_module_references_in_declaration(
    declaration: &mut Declaration,
    package: &super::package::PackageId,
    source_module: ModuleId,
    target_declaration: DeclarationId,
    destination_module: ModuleId,
) {
    match declaration {
        Declaration::Record(record) => {
            for field in &mut record.fields {
                replace_declaration_module_references_in_type(
                    &mut field.ty,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Declaration::Variant(variant) => {
            for case in &mut variant.cases {
                if let Some(payload) = &mut case.payload {
                    replace_declaration_module_references_in_type(
                        payload,
                        package,
                        source_module,
                        target_declaration,
                        destination_module,
                    );
                }
            }
        }
        Declaration::Interface(interface) => {
            for operation in &mut interface.operations {
                for parameter in &mut operation.parameters {
                    replace_declaration_module_references_in_type(
                        &mut parameter.ty,
                        package,
                        source_module,
                        target_declaration,
                        destination_module,
                    );
                }
                replace_declaration_module_references_in_type(
                    &mut operation.result,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Declaration::External(function) => {
            for parameter in &mut function.parameters {
                replace_declaration_module_references_in_type(
                    &mut parameter.ty,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
            replace_declaration_module_references_in_type(
                &mut function.result,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
        }
        Declaration::Function(function) => {
            for parameter in &mut function.parameters {
                replace_declaration_module_references_in_type(
                    &mut parameter.ty,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
            replace_declaration_module_references_in_type(
                &mut function.result,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
            if let Effect::Task { capabilities } = &mut function.effect {
                for capability in capabilities {
                    replace_declaration_module_reference(
                        &mut capability.interface,
                        package,
                        source_module,
                        target_declaration,
                        destination_module,
                    );
                }
            }
            replace_declaration_module_references_in_expression(
                &mut function.body,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
        }
        Declaration::Constant(constant) => {
            replace_declaration_module_references_in_type(
                &mut constant.ty,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
            replace_declaration_module_references_in_expression(
                &mut constant.value,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
        }
        Declaration::Component(component) => {
            for requirement in &mut component.requirements {
                replace_declaration_module_reference(
                    &mut requirement.interface,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
            for port in &mut component.ports {
                replace_declaration_module_references_in_type(
                    &mut port.ty,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
                replace_declaration_module_references_in_expression(
                    &mut port.value,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Declaration::Test(test) => {
            replace_declaration_module_references_in_expression(
                &mut test.actual,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
            replace_declaration_module_references_in_expression(
                &mut test.expected,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
        }
    }
}

fn replace_declaration_module_references_in_type(
    ty: &mut Type,
    package: &super::package::PackageId,
    source_module: ModuleId,
    target_declaration: DeclarationId,
    destination_module: ModuleId,
) {
    match ty {
        Type::Named(value) => replace_declaration_module_reference(
            value,
            package,
            source_module,
            target_declaration,
            destination_module,
        ),
        Type::Record(fields) => {
            for field in fields {
                replace_declaration_module_references_in_type(
                    &mut field.ty,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Type::List(item) | Type::Option(item) | Type::Stream(item) => {
            replace_declaration_module_references_in_type(
                item,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
        }
        Type::Map(key, value) | Type::Result(key, value) => {
            replace_declaration_module_references_in_type(
                key,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
            replace_declaration_module_references_in_type(
                value,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
        }
        Type::Function(parameters, result) => {
            for parameter in parameters {
                replace_declaration_module_references_in_type(
                    parameter,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
            replace_declaration_module_references_in_type(
                result,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
        }
        Type::Unit
        | Type::Bool
        | Type::I64
        | Type::Bytes
        | Type::Text
        | Type::StaticText
        | Type::Parameter(_)
        | Type::Secret => {}
    }
}

fn replace_declaration_module_references_in_expression(
    expression: &mut Expression,
    package: &super::package::PackageId,
    source_module: ModuleId,
    target_declaration: DeclarationId,
    destination_module: ModuleId,
) {
    match expression {
        Expression::Constant(reference, _) => replace_declaration_module_reference(
            reference,
            package,
            source_module,
            target_declaration,
            destination_module,
        ),
        Expression::Call {
            function,
            type_arguments,
            arguments,
            ..
        } => {
            replace_declaration_module_reference(
                function,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
            for argument in type_arguments {
                replace_declaration_module_references_in_type(
                    argument,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
            for argument in arguments {
                replace_declaration_module_references_in_expression(
                    argument,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Expression::Invoke {
            callee, arguments, ..
        } => {
            replace_declaration_module_references_in_expression(
                callee,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
            for argument in arguments {
                replace_declaration_module_references_in_expression(
                    argument,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Expression::Record { ty, fields, .. } => {
            if let Some(ty) = ty {
                replace_declaration_module_reference(
                    ty,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
            for field in fields {
                replace_declaration_module_references_in_expression(
                    &mut field.value,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Expression::Variant { ty, payload, .. } => {
            replace_declaration_module_reference(
                ty,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
            if let Some(payload) = payload {
                replace_declaration_module_references_in_expression(
                    payload,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Expression::FunctionRef {
            function,
            type_arguments,
            ..
        } => {
            replace_declaration_module_reference(
                function,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
            for argument in type_arguments {
                replace_declaration_module_references_in_type(
                    argument,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Expression::If {
            condition,
            when_true,
            when_false,
            ..
        } => {
            for expression in [condition, when_true, when_false] {
                replace_declaration_module_references_in_expression(
                    expression,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Expression::Let { bindings, body, .. } => {
            for binding in bindings {
                replace_declaration_module_references_in_expression(
                    &mut binding.value,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
            replace_declaration_module_references_in_expression(
                body,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
        }
        Expression::Do { expressions, .. } => {
            for expression in expressions {
                replace_declaration_module_references_in_expression(
                    expression,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Expression::List {
            item_type, items, ..
        } => {
            replace_declaration_module_references_in_type(
                item_type,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
            for expression in items {
                replace_declaration_module_references_in_expression(
                    expression,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Expression::Field { value, .. } => replace_declaration_module_references_in_expression(
            value,
            package,
            source_module,
            target_declaration,
            destination_module,
        ),
        Expression::Map {
            key_type,
            value_type,
            entries,
            ..
        } => {
            replace_declaration_module_references_in_type(
                key_type,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
            replace_declaration_module_references_in_type(
                value_type,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
            for entry in entries {
                replace_declaration_module_references_in_expression(
                    &mut entry.key,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
                replace_declaration_module_references_in_expression(
                    &mut entry.value,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Expression::Match { value, arms, .. } => {
            replace_declaration_module_references_in_expression(
                value,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
            for arm in arms {
                replace_declaration_module_references_in_expression(
                    &mut arm.body,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Expression::Perform { arguments, .. } => {
            for argument in arguments {
                replace_declaration_module_references_in_expression(
                    argument,
                    package,
                    source_module,
                    target_declaration,
                    destination_module,
                );
            }
        }
        Expression::Transaction { body, .. } => {
            replace_declaration_module_references_in_expression(
                body,
                package,
                source_module,
                target_declaration,
                destination_module,
            );
        }
        Expression::Unit(_)
        | Expression::Bool(_, _)
        | Expression::I64(_, _)
        | Expression::Text(_, _)
        | Expression::StaticText(_, _)
        | Expression::Variable(_, _) => {}
    }
}

fn replace_declaration_module_reference(
    reference: &mut DeclarationReference,
    package: &super::package::PackageId,
    source_module: ModuleId,
    declaration: DeclarationId,
    destination_module: ModuleId,
) {
    if reference.package == *package
        && reference.module == source_module
        && reference.declaration == declaration
    {
        reference.module = destination_module;
    }
}

fn expression_mut_by_id(
    modules: &mut [MeaningModule],
    id: ExpressionId,
) -> Result<(usize, usize, &mut Expression), Diagnostic> {
    let mut location = None;
    for (module_index, module) in modules.iter().enumerate() {
        for (declaration_index, identity) in module.declarations.iter().enumerate() {
            if let Some(expression) = identity.expressions.iter().find(|value| value.id == id) {
                location = Some((module_index, declaration_index, expression.path.clone()));
                break;
            }
        }
    }
    let (module_index, declaration_index, path) = location.ok_or_else(|| {
        operation_error(
            "expression_selection",
            "selected expression identity is absent",
        )
    })?;
    let expression = expression_at_path_mut(
        &mut modules[module_index].module.declarations[declaration_index],
        &path,
    )?;
    Ok((module_index, declaration_index, expression))
}

fn expression_declaration_location(
    modules: &[MeaningModule],
    id: ExpressionId,
) -> Result<(usize, usize), Diagnostic> {
    for (module_index, module) in modules.iter().enumerate() {
        for (declaration_index, identity) in module.declarations.iter().enumerate() {
            if identity.expressions.iter().any(|value| value.id == id) {
                return Ok((module_index, declaration_index));
            }
        }
    }
    Err(operation_error(
        "expression_selection",
        "selected expression identity is absent",
    ))
}

fn expression_at_path_mut<'a>(
    declaration: &'a mut Declaration,
    path: &[u32],
) -> Result<&'a mut Expression, Diagnostic> {
    let (root, remaining) = path.split_first().ok_or_else(|| {
        operation_error("expression_path", "expression identity has an empty path")
    })?;
    let mut expression = match declaration {
        Declaration::Function(function) if *root == 0 => &mut function.body,
        Declaration::Constant(constant) if *root == 0 => &mut constant.value,
        Declaration::Component(component) => component
            .ports
            .get_mut(*root as usize)
            .map(|port| &mut port.value)
            .ok_or_else(|| operation_error("expression_path", "component port path is stale"))?,
        Declaration::Test(test) => match *root {
            0 => &mut test.actual,
            1 => &mut test.expected,
            _ => {
                return Err(operation_error(
                    "expression_path",
                    "test expression root path is stale",
                ));
            }
        },
        _ => {
            return Err(operation_error(
                "expression_path",
                "expression root path is stale for its declaration",
            ));
        }
    };
    for ordinal in remaining {
        expression = expression_child_mut(expression, *ordinal)?;
    }
    Ok(expression)
}

fn expression_child_mut(
    expression: &mut Expression,
    ordinal: u32,
) -> Result<&mut Expression, Diagnostic> {
    let index = ordinal as usize;
    match expression {
        Expression::If {
            condition,
            when_true,
            when_false,
            ..
        } => match index {
            0 => Ok(condition),
            1 => Ok(when_true),
            2 => Ok(when_false),
            _ => Err(stale_expression_path()),
        },
        Expression::Let { bindings, body, .. } => {
            if index < bindings.len() {
                Ok(&mut bindings[index].value)
            } else if index == bindings.len() {
                Ok(body)
            } else {
                Err(stale_expression_path())
            }
        }
        Expression::Do { expressions, .. } => {
            expressions.get_mut(index).ok_or_else(stale_expression_path)
        }
        Expression::Call { arguments, .. } | Expression::Perform { arguments, .. } => {
            arguments.get_mut(index).ok_or_else(stale_expression_path)
        }
        Expression::Invoke {
            callee, arguments, ..
        } => {
            if index == 0 {
                Ok(callee)
            } else {
                arguments
                    .get_mut(index - 1)
                    .ok_or_else(stale_expression_path)
            }
        }
        Expression::Record { fields, .. } => fields
            .get_mut(index)
            .map(|field| &mut field.value)
            .ok_or_else(stale_expression_path),
        Expression::Variant { payload, .. } if index == 0 => {
            payload.as_deref_mut().ok_or_else(stale_expression_path)
        }
        Expression::Field { value, .. } if index == 0 => Ok(value),
        Expression::List { items, .. } => items.get_mut(index).ok_or_else(stale_expression_path),
        Expression::Map { entries, .. } => {
            let entry = entries
                .get_mut(index / 2)
                .ok_or_else(stale_expression_path)?;
            if index.is_multiple_of(2) {
                Ok(&mut entry.key)
            } else {
                Ok(&mut entry.value)
            }
        }
        Expression::Match { value, arms, .. } => {
            if index == 0 {
                Ok(value)
            } else {
                arms.get_mut(index - 1)
                    .map(|arm| &mut arm.body)
                    .ok_or_else(stale_expression_path)
            }
        }
        Expression::Transaction { body, .. } if index == 0 => Ok(body),
        _ => Err(stale_expression_path()),
    }
}

fn rename_member_definition(
    declaration: &mut Declaration,
    members: &[MemberIdentity],
    member_index: usize,
    new_name: &str,
) -> Result<(), Diagnostic> {
    let ordinal = match &members[member_index] {
        MemberIdentity::Field { .. } => member_kind_ordinal(members, member_index, |member| {
            matches!(member, MemberIdentity::Field { .. })
        }),
        MemberIdentity::Case { .. } => member_kind_ordinal(members, member_index, |member| {
            matches!(member, MemberIdentity::Case { .. })
        }),
        MemberIdentity::Operation { .. } => member_kind_ordinal(members, member_index, |member| {
            matches!(member, MemberIdentity::Operation { .. })
        }),
        _ => {
            return Err(operation_error(
                "rename_member",
                "selected member kind is not renameable by this operation",
            ));
        }
    };
    match declaration {
        Declaration::Record(record) => record.fields[ordinal].name = new_name.to_owned(),
        Declaration::Variant(variant) => variant.cases[ordinal].name = new_name.to_owned(),
        Declaration::Interface(interface) => {
            interface.operations[ordinal].name = new_name.to_owned();
        }
        _ => {
            return Err(operation_error(
                "rename_member",
                "member definition kind does not match its declaration",
            ));
        }
    }
    Ok(())
}

fn remove_member_definition(
    declaration: &mut Declaration,
    members: &[MemberIdentity],
    member_index: usize,
) -> Result<(), Diagnostic> {
    let ordinal = match &members[member_index] {
        MemberIdentity::Field { .. } => member_kind_ordinal(members, member_index, |member| {
            matches!(member, MemberIdentity::Field { .. })
        }),
        MemberIdentity::Case { .. } => member_kind_ordinal(members, member_index, |member| {
            matches!(member, MemberIdentity::Case { .. })
        }),
        MemberIdentity::Operation { .. } => member_kind_ordinal(members, member_index, |member| {
            matches!(member, MemberIdentity::Operation { .. })
        }),
        _ => return Err(operation_error("remove_member", "unsupported member kind")),
    };
    match declaration {
        Declaration::Record(record) => {
            record.fields.remove(ordinal);
        }
        Declaration::Variant(variant) => {
            variant.cases.remove(ordinal);
        }
        Declaration::Interface(interface) => {
            interface.operations.remove(ordinal);
        }
        _ => {
            return Err(operation_error(
                "remove_member",
                "member definition kind does not match its declaration",
            ));
        }
    }
    Ok(())
}

fn member_kind_ordinal(
    members: &[MemberIdentity],
    member_index: usize,
    predicate: impl Fn(&MemberIdentity) -> bool,
) -> usize {
    members[..member_index]
        .iter()
        .filter(|member| predicate(member))
        .count()
}

fn following_parameter_end(members: &[MemberIdentity], operation_index: usize) -> usize {
    let mut end = operation_index + 1;
    while end < members.len() && matches!(members[end], MemberIdentity::Parameter { .. }) {
        end += 1;
    }
    end
}

fn set_member_name(member: &mut MemberIdentity, new_name: &str) {
    match member {
        MemberIdentity::Field { name, .. }
        | MemberIdentity::TypeParameter { name, .. }
        | MemberIdentity::Case { name, .. }
        | MemberIdentity::Operation { name, .. }
        | MemberIdentity::Parameter { name, .. }
        | MemberIdentity::TaskRequirement { name, .. }
        | MemberIdentity::ComponentRequirement { name, .. }
        | MemberIdentity::Port { name, .. } => *name = new_name.to_owned(),
    }
}

fn set_declaration_name(declaration: &mut Declaration, new_name: &str) {
    match declaration {
        Declaration::Record(value) => value.name = new_name.to_owned(),
        Declaration::Variant(value) => value.name = new_name.to_owned(),
        Declaration::Interface(value) => value.name = new_name.to_owned(),
        Declaration::External(value) => value.name = new_name.to_owned(),
        Declaration::Function(value) => value.name = new_name.to_owned(),
        Declaration::Constant(value) => value.name = new_name.to_owned(),
        Declaration::Component(value) => value.name = new_name.to_owned(),
        Declaration::Test(value) => value.name = new_name.to_owned(),
    }
}

fn rename_binding_definition(
    declaration: &mut Declaration,
    identity: &BindingIdentity,
    new_name: &str,
) -> Result<(), Diagnostic> {
    let expression = expression_at_path_mut(declaration, &identity.expression_path)?;
    match expression {
        Expression::Let { bindings, .. } => {
            let binding = bindings
                .get_mut(identity.slot as usize)
                .ok_or_else(stale_expression_path)?;
            binding.name = new_name.to_owned();
        }
        Expression::Match { arms, .. } => {
            let arm = arms
                .get_mut(identity.slot as usize)
                .ok_or_else(stale_expression_path)?;
            arm.binding = Some(new_name.to_owned());
        }
        Expression::Transaction { binding, .. } if identity.slot == 0 => {
            *binding = new_name.to_owned();
        }
        _ => return Err(stale_expression_path()),
    }
    Ok(())
}

fn relation_source_belongs_to_declaration(
    source: &RelationSource,
    declaration: &DeclarationIdentity,
) -> bool {
    match source {
        RelationSource::Declaration(id) => *id == declaration.id,
        RelationSource::Field(id) => declaration
            .members
            .iter()
            .any(|member| matches!(member, MemberIdentity::Field { id: value, .. } if value == id)),
        RelationSource::Case(id) => declaration
            .members
            .iter()
            .any(|member| matches!(member, MemberIdentity::Case { id: value, .. } if value == id)),
        RelationSource::Operation(id) => declaration.members.iter().any(
            |member| matches!(member, MemberIdentity::Operation { id: value, .. } if value == id),
        ),
        RelationSource::Parameter(id) => declaration.members.iter().any(
            |member| matches!(member, MemberIdentity::Parameter { id: value, .. } if value == id),
        ),
        RelationSource::Binding(id) => declaration.bindings.iter().any(|value| value.id == *id),
        RelationSource::Requirement(id) => declaration.members.iter().any(|member| {
            matches!(
                member,
                MemberIdentity::TaskRequirement { id: value, .. }
                    | MemberIdentity::ComponentRequirement { id: value, .. }
                    if value == id
            )
        }),
        RelationSource::Port(id) => declaration
            .members
            .iter()
            .any(|member| matches!(member, MemberIdentity::Port { id: value, .. } if value == id)),
        RelationSource::Expression(id) => {
            declaration.expressions.iter().any(|value| value.id == *id)
        }
        RelationSource::Module(_) | RelationSource::Target(_) => false,
    }
}

fn tombstone_declaration(root: &mut GraphRoot, identity: &DeclarationIdentity, base: RevisionId) {
    add_tombstone(
        root,
        TombstoneIdentity::Declaration(identity.id),
        base,
        identity.name.clone(),
    );
    for member in &identity.members {
        let (identity, name) = match member {
            MemberIdentity::TypeParameter { id, name } => {
                (TombstoneIdentity::TypeParameter(*id), name)
            }
            MemberIdentity::Field { id, name } => (TombstoneIdentity::Field(*id), name),
            MemberIdentity::Case { id, name } => (TombstoneIdentity::Case(*id), name),
            MemberIdentity::Operation { id, name } => (TombstoneIdentity::Operation(*id), name),
            MemberIdentity::Parameter { id, name } => (TombstoneIdentity::Parameter(*id), name),
            MemberIdentity::TaskRequirement { id, name }
            | MemberIdentity::ComponentRequirement { id, name } => {
                (TombstoneIdentity::Requirement(*id), name)
            }
            MemberIdentity::Port { id, name } => (TombstoneIdentity::Port(*id), name),
        };
        add_tombstone(root, identity, base, name.clone());
    }
    for binding in &identity.bindings {
        add_tombstone(
            root,
            TombstoneIdentity::Binding(binding.id),
            base,
            binding.name.clone(),
        );
    }
    for expression in &identity.expressions {
        add_tombstone(
            root,
            TombstoneIdentity::Expression(expression.id),
            base,
            format!("expression:{:?}", expression.kind).to_lowercase(),
        );
    }
}

fn member_tombstone(identity: MemberIdentity) -> (TombstoneIdentity, String) {
    match identity {
        MemberIdentity::TypeParameter { id, name } => (TombstoneIdentity::TypeParameter(id), name),
        MemberIdentity::Field { id, name } => (TombstoneIdentity::Field(id), name),
        MemberIdentity::Case { id, name } => (TombstoneIdentity::Case(id), name),
        MemberIdentity::Operation { id, name } => (TombstoneIdentity::Operation(id), name),
        MemberIdentity::Parameter { id, name } => (TombstoneIdentity::Parameter(id), name),
        MemberIdentity::TaskRequirement { id, name }
        | MemberIdentity::ComponentRequirement { id, name } => {
            (TombstoneIdentity::Requirement(id), name)
        }
        MemberIdentity::Port { id, name } => (TombstoneIdentity::Port(id), name),
    }
}

fn nested_tombstone_identities(
    declaration: &DeclarationIdentity,
) -> BTreeMap<TombstoneIdentity, String> {
    let mut identities = declaration
        .members
        .iter()
        .cloned()
        .map(member_tombstone)
        .collect::<BTreeMap<_, _>>();
    identities.extend(
        declaration
            .bindings
            .iter()
            .map(|binding| (TombstoneIdentity::Binding(binding.id), binding.name.clone())),
    );
    identities.extend(declaration.expressions.iter().map(|expression| {
        (
            TombstoneIdentity::Expression(expression.id),
            format!("expression:{:?}", expression.kind).to_lowercase(),
        )
    }));
    identities
}

fn tombstone_removed_nested_identities(
    root: &mut GraphRoot,
    before: &DeclarationIdentity,
    after: &DeclarationIdentity,
    base: RevisionId,
) {
    let after = nested_tombstone_identities(after)
        .into_keys()
        .collect::<BTreeSet<_>>();
    for (identity, name) in nested_tombstone_identities(before) {
        if !after.contains(&identity) {
            add_tombstone(root, identity, base, name);
        }
    }
}

fn add_tombstone(
    root: &mut GraphRoot,
    identity: TombstoneIdentity,
    base: RevisionId,
    name: String,
) {
    root.tombstones
        .retain(|tombstone| tombstone.identity != identity);
    root.tombstones.push(Tombstone {
        identity,
        last_live_revision: base,
        last_name: name,
    });
    root.tombstones.sort();
    root.tombstones.dedup();
}

fn remove_restored_tombstones(root: &mut GraphRoot, declaration: &DeclarationIdentity) {
    let mut restored = BTreeSet::new();
    restored.insert(TombstoneIdentity::Declaration(declaration.id));
    for member in &declaration.members {
        restored.insert(match member {
            MemberIdentity::TypeParameter { id, .. } => TombstoneIdentity::TypeParameter(*id),
            MemberIdentity::Field { id, .. } => TombstoneIdentity::Field(*id),
            MemberIdentity::Case { id, .. } => TombstoneIdentity::Case(*id),
            MemberIdentity::Operation { id, .. } => TombstoneIdentity::Operation(*id),
            MemberIdentity::Parameter { id, .. } => TombstoneIdentity::Parameter(*id),
            MemberIdentity::TaskRequirement { id, .. }
            | MemberIdentity::ComponentRequirement { id, .. } => {
                TombstoneIdentity::Requirement(*id)
            }
            MemberIdentity::Port { id, .. } => TombstoneIdentity::Port(*id),
        });
    }
    for binding in &declaration.bindings {
        restored.insert(TombstoneIdentity::Binding(binding.id));
    }
    for expression in &declaration.expressions {
        restored.insert(TombstoneIdentity::Expression(expression.id));
    }
    root.tombstones
        .retain(|tombstone| !restored.contains(&tombstone.identity));
}

fn owner_name(owner: OwnerSelector, root: &GraphRoot, modules: &[MeaningModule]) -> Option<String> {
    match owner {
        OwnerSelector::Module(id) => modules
            .iter()
            .find(|module| module.module_id == id)
            .map(|module| module.module.name.clone()),
        OwnerSelector::Declaration(id) => modules.iter().find_map(|module| {
            module
                .declarations
                .iter()
                .find(|declaration| declaration.id == id)
                .map(|declaration| declaration.name.clone())
        }),
        OwnerSelector::Target(id) => root
            .targets
            .iter()
            .find(|target| target.id == id)
            .map(|target| target.name.clone()),
    }
}

fn consume_work(
    work: &mut usize,
    amount: usize,
    budget: TransactionBudget,
) -> Result<(), Diagnostic> {
    *work = work.checked_add(amount).ok_or_else(work_exhausted)?;
    if *work > budget.maximum_work {
        return Err(work_exhausted());
    }
    Ok(())
}

fn validate_idempotency(value: Option<&str>) -> Result<(), Diagnostic> {
    if value.is_some_and(|value| {
        value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    }) {
        return Err(transaction_error(
            DiagnosticClass::Source,
            "semantic_transaction_idempotency",
            "idempotency key must contain 1 through 128 portable identifier bytes",
        ));
    }
    Ok(())
}

fn without_source_location(mut error: Diagnostic) -> Diagnostic {
    error.location = None;
    error
}

fn semantic_span() -> SourceSpan {
    SourceSpan {
        byte_start: 0,
        byte_end: 0,
        line: 1,
        column: 1,
    }
}

fn stale_expression_path() -> Diagnostic {
    operation_error(
        "expression_path",
        "semantic expression path is stale or structurally inconsistent",
    )
}

fn work_exhausted() -> Diagnostic {
    transaction_error(
        DiagnosticClass::Resource,
        "semantic_transaction_work_exhausted",
        "transaction exhausted its declared semantic work budget",
    )
}

fn operation_error(operation: &str, message: impl Into<String>) -> Diagnostic {
    transaction_error(
        DiagnosticClass::Semantic,
        &format!("semantic_{operation}"),
        message,
    )
}

fn transaction_json(error: serde_json::Error) -> Diagnostic {
    transaction_error(
        DiagnosticClass::Infrastructure,
        "semantic_transaction_encode",
        format!("transaction canonical encoding failed: {error}"),
    )
}

fn transaction_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::language::{Function, Import, RecordType};
    use crate::platform::{
        GRAPH_CONTRACT_VERSION, InitialPublication, PackageId, RequestIdentityAllocator,
    };

    struct Fixture {
        _temporary: tempfile::TempDir,
        repository: SemanticRepository,
        module: ModuleId,
        declaration: DeclarationId,
        field: FieldId,
    }

    fn fixture() -> Fixture {
        let temporary = tempfile::TempDir::new().expect("temporary semantic repository");
        let module_id = ModuleId::migrate(b"transaction-fixture", 1);
        let declaration_id = DeclarationId::migrate(b"transaction-fixture", 1);
        let field_id = FieldId::migrate(b"transaction-fixture", 1);
        let module = MeaningModule {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            module_id,
            module: Module {
                name: "sample".to_owned(),
                imports: Vec::new(),
                exports: vec![declaration_id],
                declarations: vec![Declaration::Record(RecordType {
                    name: "Item".to_owned(),
                    fields: vec![Field {
                        name: "name".to_owned(),
                        ty: Type::Text,
                        span: semantic_span(),
                    }],
                    span: semantic_span(),
                })],
            },
            declarations: vec![DeclarationIdentity {
                id: declaration_id,
                name: "Item".to_owned(),
                kind: super::super::meaning::DeclarationKind::Record,
                members: vec![MemberIdentity::Field {
                    id: field_id,
                    name: "name".to_owned(),
                }],
                bindings: Vec::new(),
                expressions: Vec::new(),
            }],
            relations: Vec::new(),
            documentation: Vec::new(),
            annotations: Vec::new(),
        };
        let root = GraphRoot {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(b"transaction-fixture", 1),
            package_id: PackageId::parse("10000000000000000000000000000001")
                .expect("package identity"),
            package_name: "fixture".to_owned(),
            modules: vec![ModuleObjectRef {
                id: module_id,
                name: "sample".to_owned(),
                object: module.digest().expect("module digest"),
            }],
            dependencies: Vec::new(),
            targets: Vec::new(),
            tombstones: Vec::new(),
        };
        let mut modules = vec![module];
        let mut root = root;
        super::super::semantic::canonicalize_graph_package(&mut root, &mut modules, &[])
            .expect("canonicalize fixture relations");
        let (repository, _) = SemanticRepository::initialize(
            temporary.path(),
            InitialPublication {
                root,
                modules,
                transaction: TransactionDigest::of(b"transaction fixture import"),
                semantic_diff: SemanticDiffDigest::of(b"transaction fixture initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
                status: ReceiptStatus::ImportAccepted,
            },
        )
        .expect("initialize semantic repository");
        Fixture {
            _temporary: temporary,
            repository,
            module: module_id,
            declaration: declaration_id,
            field: field_id,
        }
    }

    fn request(
        fixture: &Fixture,
        base_revision: RevisionId,
        idempotency_key: Option<&str>,
        operations: Vec<SemanticOperation>,
    ) -> TransactionRequest {
        TransactionRequest {
            contract_version: TRANSACTION_CONTRACT_VERSION,
            graph_contract: super::super::meaning::GRAPH_CONTRACT_IDENTITY.to_owned(),
            repository_id: fixture
                .repository
                .current()
                .expect("current repository")
                .head
                .repository_id,
            base_revision,
            draft: None,
            idempotency_key: idempotency_key.map(str::to_owned),
            preconditions: Vec::new(),
            operations,
            budget: TransactionBudget::default(),
            intent: None,
        }
    }

    #[test]
    fn pure_body_edit_validates_only_its_import_closure_and_matches_full_oracle() {
        let temporary = tempfile::TempDir::new().expect("temporary semantic repository");
        let package =
            PackageId::parse("20000000000000000000000000000002").expect("package identity");
        let mut helper = MeaningModule::create(
            Module {
                name: "helper".to_owned(),
                imports: Vec::new(),
                exports: Vec::new(),
                declarations: vec![
                    Declaration::Function(Function {
                        name: "one".to_owned(),
                        type_parameters: Vec::new(),
                        parameters: Vec::new(),
                        result: Type::I64,
                        effect: Effect::Pure,
                        body: Expression::I64(1, semantic_span()),
                        span: semantic_span(),
                    }),
                    Declaration::Function(Function {
                        name: "two".to_owned(),
                        type_parameters: Vec::new(),
                        parameters: Vec::new(),
                        result: Type::I64,
                        effect: Effect::Pure,
                        body: Expression::I64(2, semantic_span()),
                        span: semantic_span(),
                    }),
                ],
            },
            &mut RequestIdentityAllocator::new(b"local-body-helper".to_vec()),
        )
        .expect("helper module");
        let helper_module = helper.module_id;
        let helper_one = helper.declarations[0].id;
        let helper_two = helper.declarations[1].id;
        helper.module.exports = vec![helper_one, helper_two];
        helper.module.exports.sort();
        let mut target = MeaningModule::create(
            Module {
                name: "target".to_owned(),
                imports: vec![Import {
                    alias: "helper".to_owned(),
                    target: crate::platform::language::ModuleReference {
                        package: package.clone(),
                        module: helper.module_id,
                    },
                    span: semantic_span(),
                }],
                exports: Vec::new(),
                declarations: vec![Declaration::Function(Function {
                    name: "answer".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    result: Type::I64,
                    effect: Effect::Pure,
                    body: Expression::Call {
                        function: DeclarationReference {
                            package: package.clone(),
                            module: helper_module,
                            declaration: helper_one,
                        },
                        type_arguments: Vec::new(),
                        arguments: Vec::new(),
                        span: semantic_span(),
                    },
                    span: semantic_span(),
                })],
            },
            &mut RequestIdentityAllocator::new(b"local-body-target".to_vec()),
        )
        .expect("target module");
        let declaration = target.declarations[0].id;
        let expressions = target.declarations[0].expressions.clone();
        let bindings = target.declarations[0].bindings.clone();
        let mut modules = vec![target.clone(), helper];
        for ordinal in 0..64u64 {
            let name = format!("unrelated-{ordinal:02}");
            let mut seed = b"local-body-unrelated".to_vec();
            seed.extend_from_slice(&ordinal.to_be_bytes());
            modules.push(
                MeaningModule::create(
                    Module {
                        name,
                        imports: Vec::new(),
                        exports: Vec::new(),
                        declarations: Vec::new(),
                    },
                    &mut RequestIdentityAllocator::new(seed),
                )
                .expect("unrelated module"),
            );
        }
        let mut root = GraphRoot {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(b"local-body-repository", 1),
            package_id: package.clone(),
            package_name: "local-body".to_owned(),
            modules: modules
                .iter()
                .map(|module| {
                    Ok(ModuleObjectRef {
                        id: module.module_id,
                        name: module.module.name.clone(),
                        object: module.digest()?,
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()
                .expect("module references"),
            dependencies: Vec::new(),
            targets: Vec::new(),
            tombstones: Vec::new(),
        };
        root.modules.sort();
        crate::platform::semantic::canonicalize_graph_package(&mut root, &mut modules, &[])
            .expect("canonical fixture");
        target = modules
            .iter()
            .find(|module| module.module_id == target.module_id)
            .expect("canonical target")
            .clone();
        let (repository, _) = SemanticRepository::initialize(
            temporary.path(),
            InitialPublication {
                root,
                modules,
                transaction: TransactionDigest::of(b"local body import"),
                semantic_diff: SemanticDiffDigest::of(b"local body initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
                status: ReceiptStatus::ImportAccepted,
            },
        )
        .expect("initialize local body repository");
        let base = repository.reconstruct_current().expect("base");
        SemanticQueryIndex::owner_summary_revision(
            &repository,
            base.current.head.revision,
            &declaration.to_string(),
        )
        .expect("warm exact owner index");

        let operation = SemanticOperation::ReplaceBody {
            declaration,
            body: Expression::Call {
                function: DeclarationReference {
                    package: package.clone(),
                    module: helper_module,
                    declaration: helper_two,
                },
                type_arguments: Vec::new(),
                arguments: Vec::new(),
                span: semantic_span(),
            },
            bindings,
            expressions,
        };
        let mut oracle_root = base.current.root.clone();
        let mut oracle_modules = base.modules.clone();
        let mut module_ids = oracle_modules
            .iter()
            .map(|module| module.module_id)
            .collect::<BTreeSet<_>>();
        let mut module_names = oracle_modules
            .iter()
            .map(|module| module.module.name.clone())
            .collect::<BTreeSet<_>>();
        apply_operation(
            &repository,
            &operation,
            base.current.head.revision,
            &mut oracle_root,
            &mut oracle_modules,
            &mut module_ids,
            &mut module_names,
            &mut BTreeSet::new(),
            &mut 0,
            TransactionBudget::default(),
        )
        .expect("apply oracle operation");
        crate::platform::semantic::canonicalize_graph_package(
            &mut oracle_root,
            &mut oracle_modules,
            &[],
        )
        .expect("full oracle canonicalization");
        let expected_root = oracle_root.digest().expect("oracle root");

        let fixture = Fixture {
            _temporary: temporary,
            repository,
            module: target.module_id,
            declaration,
            field: FieldId::migrate(b"unused-local-body-field", 1),
        };
        let request = request(&fixture, base.current.head.revision, None, vec![operation]);
        let result = execute_transaction(&fixture.repository, &request, TransactionMode::Apply)
            .expect("local body apply");
        assert_eq!(
            result.status,
            TransactionStatus::AcceptedChange,
            "{:?}",
            result.diagnostics
        );
        let receipt = result.receipt.expect("local receipt");
        assert_eq!(receipt.validation.profile, "incremental_pure_body_slice");
        assert_eq!(receipt.validation.modules_checked, 2);
        assert_eq!(receipt.validation.declarations_checked, 3);
        let accepted = fixture
            .repository
            .reconstruct_current()
            .expect("accepted result");
        assert_eq!(accepted.current.record.core.root, expected_root);
        crate::platform::semantic::validate_graph_package(
            &accepted.current.root,
            accepted.modules,
            &[],
            Some(accepted.current.head.revision),
        )
        .expect("accepted result matches full semantic oracle");
    }

    #[test]
    fn module_rename_rewrites_only_the_stable_owner_and_matches_full_oracle() {
        let temporary = tempfile::TempDir::new().expect("temporary semantic repository");
        let package =
            PackageId::parse("30000000000000000000000000000003").expect("package identity");
        let helper = MeaningModule::create(
            Module {
                name: "helper".to_owned(),
                imports: Vec::new(),
                exports: Vec::new(),
                declarations: Vec::new(),
            },
            &mut RequestIdentityAllocator::new(b"local-rename-helper".to_vec()),
        )
        .expect("helper module");
        let helper_id = helper.module_id;
        let mut modules = vec![helper];
        for ordinal in 0..256u64 {
            let name = format!("importer-{ordinal:03}");
            let mut seed = b"local-rename-importer".to_vec();
            seed.extend_from_slice(&ordinal.to_be_bytes());
            modules.push(
                MeaningModule::create(
                    Module {
                        name,
                        imports: vec![Import {
                            alias: "helper".to_owned(),
                            target: crate::platform::language::ModuleReference {
                                package: package.clone(),
                                module: helper_id,
                            },
                            span: semantic_span(),
                        }],
                        exports: Vec::new(),
                        declarations: Vec::new(),
                    },
                    &mut RequestIdentityAllocator::new(seed),
                )
                .expect("importer module"),
            );
        }
        let mut root = GraphRoot {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(b"local-rename-repository", 1),
            package_id: package,
            package_name: "local-rename".to_owned(),
            modules: modules
                .iter()
                .map(|module| {
                    Ok(ModuleObjectRef {
                        id: module.module_id,
                        name: module.module.name.clone(),
                        object: module.digest()?,
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()
                .expect("module references"),
            dependencies: Vec::new(),
            targets: Vec::new(),
            tombstones: Vec::new(),
        };
        root.modules.sort();
        crate::platform::semantic::canonicalize_graph_package(&mut root, &mut modules, &[])
            .expect("canonical fixture");
        let (repository, _) = SemanticRepository::initialize(
            temporary.path(),
            InitialPublication {
                root,
                modules,
                transaction: TransactionDigest::of(b"local rename import"),
                semantic_diff: SemanticDiffDigest::of(b"local rename initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
                status: ReceiptStatus::ImportAccepted,
            },
        )
        .expect("initialize local rename repository");
        let base = repository.reconstruct_current().expect("base");
        let importer_objects = base
            .current
            .root
            .modules
            .iter()
            .filter(|reference| reference.id != helper_id)
            .map(|reference| (reference.id, reference.object))
            .collect::<BTreeMap<_, _>>();

        let operation = SemanticOperation::RenameModule {
            module: helper_id,
            new_name: "foundation".to_owned(),
        };
        let mut oracle_root = base.current.root.clone();
        let mut oracle_modules = base.modules.clone();
        let mut module_ids = oracle_modules
            .iter()
            .map(|module| module.module_id)
            .collect::<BTreeSet<_>>();
        let mut module_names = oracle_modules
            .iter()
            .map(|module| module.module.name.clone())
            .collect::<BTreeSet<_>>();
        apply_operation(
            &repository,
            &operation,
            base.current.head.revision,
            &mut oracle_root,
            &mut oracle_modules,
            &mut module_ids,
            &mut module_names,
            &mut BTreeSet::new(),
            &mut 0,
            TransactionBudget::default(),
        )
        .expect("apply oracle operation");
        crate::platform::semantic::canonicalize_graph_package(
            &mut oracle_root,
            &mut oracle_modules,
            &[],
        )
        .expect("full oracle canonicalization");
        let expected_root = oracle_root.digest().expect("oracle root");

        let fixture = Fixture {
            _temporary: temporary,
            repository,
            module: helper_id,
            declaration: DeclarationId::migrate(b"unused-local-rename-declaration", 1),
            field: FieldId::migrate(b"unused-local-rename-field", 1),
        };
        let request = request(&fixture, base.current.head.revision, None, vec![operation]);
        let result = execute_transaction(&fixture.repository, &request, TransactionMode::Apply)
            .expect("local rename apply");
        assert_eq!(
            result.status,
            TransactionStatus::AcceptedChange,
            "{:?}",
            result.diagnostics
        );
        let receipt = result.receipt.expect("local receipt");
        assert_eq!(receipt.validation.profile, "incremental_module_rename");
        assert_eq!(receipt.validation.modules_checked, 1);
        assert_eq!(receipt.validation.declarations_checked, 0);

        let accepted = fixture
            .repository
            .reconstruct_current()
            .expect("accepted result");
        assert_eq!(accepted.current.record.core.root, expected_root);
        assert!(
            accepted
                .current
                .root
                .modules
                .iter()
                .any(|reference| reference.id == helper_id && reference.name == "foundation")
        );
        for reference in accepted
            .current
            .root
            .modules
            .iter()
            .filter(|reference| reference.id != helper_id)
        {
            assert_eq!(importer_objects.get(&reference.id), Some(&reference.object));
        }
        assert!(
            accepted
                .modules
                .iter()
                .filter(|module| module.module_id != helper_id)
                .all(|module| module.module.imports
                    == vec![Import {
                        alias: "helper".to_owned(),
                        target: crate::platform::language::ModuleReference {
                            package: accepted.current.root.package_id.clone(),
                            module: helper_id,
                        },
                        span: semantic_span(),
                    }])
        );
        crate::platform::semantic::validate_graph_package(
            &accepted.current.root,
            accepted.modules,
            &[],
            Some(accepted.current.head.revision),
        )
        .expect("accepted result matches full semantic oracle");
    }

    #[test]
    fn declaration_rename_rewrites_no_importers_and_matches_full_oracle() {
        let temporary = tempfile::TempDir::new().expect("temporary semantic repository");
        let package =
            PackageId::parse("40000000000000000000000000000004").expect("package identity");
        let mut helper = MeaningModule::create(
            Module {
                name: "helper".to_owned(),
                imports: Vec::new(),
                exports: Vec::new(),
                declarations: vec![Declaration::Function(Function {
                    name: "answer".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    result: Type::I64,
                    effect: Effect::Pure,
                    body: Expression::I64(42, semantic_span()),
                    span: semantic_span(),
                })],
            },
            &mut RequestIdentityAllocator::new(b"local-declaration-rename-helper".to_vec()),
        )
        .expect("helper module");
        let helper_id = helper.module_id;
        let declaration = helper.declarations[0].id;
        helper.module.exports = vec![declaration];

        let mut modules = vec![helper];
        for ordinal in 0..256u64 {
            let name = format!("importer-{ordinal:03}");
            let mut seed = b"local-declaration-rename-importer".to_vec();
            seed.extend_from_slice(&ordinal.to_be_bytes());
            modules.push(
                MeaningModule::create(
                    Module {
                        name,
                        imports: vec![Import {
                            alias: "helper".to_owned(),
                            target: crate::platform::language::ModuleReference {
                                package: package.clone(),
                                module: helper_id,
                            },
                            span: semantic_span(),
                        }],
                        exports: Vec::new(),
                        declarations: vec![Declaration::Function(Function {
                            name: "use-answer".to_owned(),
                            type_parameters: Vec::new(),
                            parameters: Vec::new(),
                            result: Type::I64,
                            effect: Effect::Pure,
                            body: Expression::Call {
                                function: DeclarationReference {
                                    package: package.clone(),
                                    module: helper_id,
                                    declaration,
                                },
                                type_arguments: Vec::new(),
                                arguments: Vec::new(),
                                span: semantic_span(),
                            },
                            span: semantic_span(),
                        })],
                    },
                    &mut RequestIdentityAllocator::new(seed),
                )
                .expect("importer module"),
            );
        }
        let mut root = GraphRoot {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(b"local-declaration-rename-repository", 1),
            package_id: package,
            package_name: "local-declaration-rename".to_owned(),
            modules: modules
                .iter()
                .map(|module| {
                    Ok(ModuleObjectRef {
                        id: module.module_id,
                        name: module.module.name.clone(),
                        object: module.digest()?,
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()
                .expect("module references"),
            dependencies: Vec::new(),
            targets: Vec::new(),
            tombstones: Vec::new(),
        };
        root.modules.sort();
        crate::platform::semantic::canonicalize_graph_package(&mut root, &mut modules, &[])
            .expect("canonical fixture");
        let (repository, _) = SemanticRepository::initialize(
            temporary.path(),
            InitialPublication {
                root,
                modules,
                transaction: TransactionDigest::of(b"local declaration rename import"),
                semantic_diff: SemanticDiffDigest::of(b"local declaration rename initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
                status: ReceiptStatus::ImportAccepted,
            },
        )
        .expect("initialize local declaration rename repository");
        let base = repository.reconstruct_current().expect("base");
        let importer_objects = base
            .current
            .root
            .modules
            .iter()
            .filter(|reference| reference.id != helper_id)
            .map(|reference| (reference.id, reference.object))
            .collect::<BTreeMap<_, _>>();

        let operation = SemanticOperation::RenameDeclaration {
            declaration,
            new_name: "result".to_owned(),
        };
        let mut oracle_root = base.current.root.clone();
        let mut oracle_modules = base.modules.clone();
        let mut module_ids = oracle_modules
            .iter()
            .map(|module| module.module_id)
            .collect::<BTreeSet<_>>();
        let mut module_names = oracle_modules
            .iter()
            .map(|module| module.module.name.clone())
            .collect::<BTreeSet<_>>();
        apply_operation(
            &repository,
            &operation,
            base.current.head.revision,
            &mut oracle_root,
            &mut oracle_modules,
            &mut module_ids,
            &mut module_names,
            &mut BTreeSet::new(),
            &mut 0,
            TransactionBudget::default(),
        )
        .expect("apply oracle operation");
        crate::platform::semantic::canonicalize_graph_package(
            &mut oracle_root,
            &mut oracle_modules,
            &[],
        )
        .expect("full oracle canonicalization");
        let expected_root = oracle_root.digest().expect("oracle root");

        let fixture = Fixture {
            _temporary: temporary,
            repository,
            module: helper_id,
            declaration,
            field: FieldId::migrate(b"unused-local-declaration-rename-field", 1),
        };
        let request = request(&fixture, base.current.head.revision, None, vec![operation]);
        let result = execute_transaction(&fixture.repository, &request, TransactionMode::Apply)
            .expect("local declaration rename apply");
        assert_eq!(
            result.status,
            TransactionStatus::AcceptedChange,
            "{:?}",
            result.diagnostics
        );
        let receipt = result.receipt.expect("local receipt");
        assert_eq!(receipt.validation.profile, "incremental_declaration_rename");
        assert_eq!(receipt.validation.modules_checked, 1);
        assert_eq!(receipt.validation.declarations_checked, 1);

        let accepted = fixture
            .repository
            .reconstruct_current()
            .expect("accepted result");
        assert_eq!(accepted.current.record.core.root, expected_root);
        let renamed = accepted
            .modules
            .iter()
            .find(|module| module.module_id == helper_id)
            .and_then(|module| module.declarations.first())
            .expect("renamed declaration");
        assert_eq!(renamed.id, declaration);
        assert_eq!(renamed.name, "result");
        for reference in accepted
            .current
            .root
            .modules
            .iter()
            .filter(|reference| reference.id != helper_id)
        {
            assert_eq!(importer_objects.get(&reference.id), Some(&reference.object));
        }
        crate::platform::semantic::validate_graph_package(
            &accepted.current.root,
            accepted.modules,
            &[],
            Some(accepted.current.head.revision),
        )
        .expect("accepted result matches full semantic oracle");
    }

    #[test]
    fn plan_validate_apply_and_idempotent_replay_preserve_identity() {
        let fixture = fixture();
        let base = fixture.repository.current().expect("base").head.revision;
        let request = request(
            &fixture,
            base,
            Some("rename-item"),
            vec![SemanticOperation::RenameDeclaration {
                declaration: fixture.declaration,
                new_name: "Entry".to_owned(),
            }],
        );
        let planned = execute_transaction(&fixture.repository, &request, TransactionMode::Plan)
            .expect("plan");
        assert_eq!(planned.status, TransactionStatus::Planned);
        assert_eq!(
            fixture
                .repository
                .current()
                .expect("after plan")
                .head
                .revision,
            base
        );
        let validated =
            execute_transaction(&fixture.repository, &request, TransactionMode::Validate)
                .expect("validate");
        assert_eq!(validated.status, TransactionStatus::Validated);
        assert_eq!(planned.predicted_revision, validated.predicted_revision);
        assert_eq!(
            fixture
                .repository
                .current()
                .expect("after validate")
                .head
                .revision,
            base
        );

        let applied = execute_transaction(&fixture.repository, &request, TransactionMode::Apply)
            .expect("apply");
        assert_eq!(applied.status, TransactionStatus::AcceptedChange);
        assert_eq!(applied.predicted_revision, applied.published_revision);
        assert_eq!(
            applied
                .receipt
                .as_ref()
                .expect("prepared publication receipt")
                .validation
                .profile,
            "incremental_declaration_rename"
        );
        let current = fixture.repository.reconstruct_current().expect("current");
        assert_eq!(current.modules[0].declarations[0].id, fixture.declaration);
        assert_eq!(current.modules[0].declarations[0].name, "Entry");

        let replayed = execute_transaction(&fixture.repository, &request, TransactionMode::Apply)
            .expect("replay");
        assert_eq!(replayed.status, TransactionStatus::Replayed);
        assert_eq!(replayed.published_revision, applied.published_revision);
        assert_eq!(
            fixture.repository.history(None, 10).expect("history").len(),
            2
        );
    }

    #[test]
    fn stale_no_change_and_preconditions_publish_nothing() {
        let fixture = fixture();
        let base = fixture.repository.current().expect("base").head.revision;
        let stale = request(
            &fixture,
            RevisionId::from_digest([9; 32]),
            None,
            vec![SemanticOperation::SetPackageMetadata {
                name: "changed".to_owned(),
            }],
        );
        assert_eq!(
            execute_transaction(&fixture.repository, &stale, TransactionMode::Apply)
                .expect("stale")
                .status,
            TransactionStatus::StaleBase
        );
        let no_change = request(
            &fixture,
            base,
            None,
            vec![SemanticOperation::RenameDeclaration {
                declaration: fixture.declaration,
                new_name: "Item".to_owned(),
            }],
        );
        assert_eq!(
            execute_transaction(&fixture.repository, &no_change, TransactionMode::Apply)
                .expect("no change")
                .status,
            TransactionStatus::SemanticNoChange
        );
        let mut failed = request(
            &fixture,
            base,
            None,
            vec![SemanticOperation::SetPackageMetadata {
                name: "changed".to_owned(),
            }],
        );
        failed.preconditions.push(SemanticPrecondition::OwnerName {
            owner: OwnerSelector::Declaration(fixture.declaration),
            equals: "Wrong".to_owned(),
        });
        assert_eq!(
            execute_transaction(&fixture.repository, &failed, TransactionMode::Apply)
                .expect("precondition")
                .status,
            TransactionStatus::PreconditionFailed
        );
        assert_eq!(
            fixture
                .repository
                .current()
                .expect("unchanged")
                .head
                .revision,
            base
        );
    }

    #[test]
    fn deletion_prevents_identity_reuse_and_exact_history_can_restore() {
        let fixture = fixture();
        let historical = fixture
            .repository
            .current()
            .expect("historical")
            .head
            .revision;
        let deleted = execute_transaction(
            &fixture.repository,
            &request(
                &fixture,
                historical,
                None,
                vec![SemanticOperation::DeleteOwner {
                    owner: OwnerSelector::Declaration(fixture.declaration),
                }],
            ),
            TransactionMode::Apply,
        )
        .expect("delete");
        assert_eq!(deleted.status, TransactionStatus::AcceptedChange);
        let deleted_revision = deleted.published_revision.expect("deleted revision");

        let reused = execute_transaction(
            &fixture.repository,
            &request(
                &fixture,
                deleted_revision,
                None,
                vec![SemanticOperation::CreateDeclaration {
                    module: fixture.module,
                    identity: DeclarationIdentity {
                        id: fixture.declaration,
                        name: "Replacement".to_owned(),
                        kind: super::super::meaning::DeclarationKind::Record,
                        members: vec![MemberIdentity::Field {
                            id: fixture.field,
                            name: "name".to_owned(),
                        }],
                        bindings: Vec::new(),
                        expressions: Vec::new(),
                    },
                    declaration: Declaration::Record(RecordType {
                        name: "Replacement".to_owned(),
                        fields: vec![Field {
                            name: "name".to_owned(),
                            ty: Type::Text,
                            span: semantic_span(),
                        }],
                        span: semantic_span(),
                    }),
                    exported: false,
                }],
            ),
            TransactionMode::Apply,
        )
        .expect("identity reuse rejection");
        assert_eq!(reused.status, TransactionStatus::InvalidGraph);
        assert_eq!(
            fixture
                .repository
                .current()
                .expect("still deleted")
                .head
                .revision,
            deleted_revision
        );

        let restored = execute_transaction(
            &fixture.repository,
            &request(
                &fixture,
                deleted_revision,
                None,
                vec![SemanticOperation::RestoreOwner {
                    historical_revision: historical,
                    declaration: fixture.declaration,
                    destination: fixture.module,
                    exported: true,
                }],
            ),
            TransactionMode::Apply,
        )
        .expect("restore");
        assert_eq!(restored.status, TransactionStatus::AcceptedChange);
        let current = fixture
            .repository
            .reconstruct_current()
            .expect("restored current");
        assert_eq!(current.modules[0].declarations[0].id, fixture.declaration);
        assert_eq!(current.modules[0].declarations[0].name, "Item");
        assert!(current.current.root.tombstones.is_empty());
    }
}
