//! Read-only contributor observations outside the executable's public operation registry.

use super::change::ChangeBudget;
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::kernel::{
    BindingKind, DeclarationPayload, ExpressionChildRole, ExpressionOperation, FunctionEffect,
    KernelSnapshot, OperationReference, OwnerKey, OwnerKind, OwnerRecord, PackageInterfaceRecord,
    ParameterUse, RelationEndpoint, encode_owner, extract_relations, validate_full,
};
use super::publication::GraphRepository;
use super::semantic_id::DeclarationId;
use super::witness::{
    BindingContainerRole, ExpressionRootRole, FullWitness, OwnershipEntry, OwnershipParent,
    OwnershipRole, rebuild_full_witness,
};
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const OWNER_DIGEST_DOMAIN: &str = "lkjscript.contributor.owner-identities.v1";
const RELATION_DIGEST_DOMAIN: &str = "lkjscript.contributor.relations.v1";
const DEFINITION_OWNER_DIGEST_DOMAIN: &str = "lkjscript.contributor.function-definition.owners.v1";
const DEFINITION_FACT_DIGEST_DOMAIN: &str = "lkjscript.contributor.function-definition.facts.v1";
const DEFINITION_RELATION_DIGEST_DOMAIN: &str =
    "lkjscript.contributor.function-definition.relations.v1";

/// Bounded typed reconstruction of one accepted current semantic revision.
///
/// This operation opens a revision-pinned `GraphRepository` view and calls its complete typed
/// oracle. It does not call a public result formatter, mutate authority, or read compiler caches.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticInventory {
    pub revision: String,
    pub owners: u64,
    pub modules: u64,
    pub functions: u64,
    pub relations: u64,
    pub types: u64,
    pub dependencies: u64,
    pub retirements: u64,
    pub owner_kinds: BTreeMap<String, u64>,
    pub owner_identity_digest: String,
    pub relation_digest: String,
    pub validation_owner_records: u64,
    pub validation_type_objects: u64,
    pub validation_expression_records: u64,
    pub validation_relation_edges: u64,
    pub validation_work: u64,
    pub map_pages_read: u64,
    pub map_bytes_read: u64,
    pub store_objects_read: u64,
    pub store_bytes_read: u64,
}

/// One typed owner in the implementation-disjoint function-definition reconstruction.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionDefinitionOracleOwner {
    pub owner: String,
    pub parent: String,
    pub role: String,
    pub ordinal: u32,
    pub depth: u64,
    pub kind: String,
    pub form: String,
    pub name: Option<String>,
    pub record: String,
    pub summary: String,
    pub type_roots: Vec<String>,
    pub expression_roots: Vec<String>,
    pub blob_roots: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionDefinitionOracleCapability {
    pub expression: String,
    pub requirement: String,
    pub operation: String,
    pub arguments: u64,
    pub parameter_uses: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionDefinitionOracleMatch {
    pub expression: String,
    pub cases: Vec<String>,
}

/// One semantic relation whose source is inside the reconstructed definition closure.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionDefinitionOracleRelation {
    pub source: String,
    pub kind: String,
    pub target: String,
}

/// Independent reconstruction of one accepted local function definition.
///
/// This contributor-only oracle uses the complete typed authority and a fresh witness rebuild. It
/// shares no traversal, order, paging, rendering, continuation, or expected-result helper with the
/// production inspection path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionDefinitionOracle {
    pub repository: String,
    pub package: String,
    pub revision: String,
    pub function: String,
    pub kind: String,
    pub name: String,
    pub effect: String,
    pub result_type: String,
    pub type_parameters: u64,
    pub parameters: u64,
    pub requirements: u64,
    pub body_root: String,
    pub contract_owners: Vec<FunctionDefinitionOracleOwner>,
    pub body_preorder: Vec<FunctionDefinitionOracleOwner>,
    pub relations: Vec<FunctionDefinitionOracleRelation>,
    pub capability_calls: Vec<FunctionDefinitionOracleCapability>,
    pub matches: Vec<FunctionDefinitionOracleMatch>,
    pub structural_edges: u64,
    pub maximum_depth: u64,
    pub owner_order_digest: String,
    pub fact_digest: String,
    pub relation_digest: String,
    pub validator: String,
    pub certificate: String,
}

/// The operation admission used when the current compact public request omits a custom budget.
pub fn compact_change_default_maximum_operations() -> u64 {
    ChangeBudget::default().authored.maximum_operations
}

pub fn semantic_inventory(project: &Path) -> Result<SemanticInventory, Diagnostic> {
    let repository = GraphRepository::open(project)?;
    let view = repository.view_current()?;
    let read = view.reconstruct_full_oracle()?;
    let validation = validate_full(&read.value).map_err(|diagnostics| {
        diagnostics.into_iter().next().unwrap_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Corrupt,
                "contributor_validation_empty",
                "typed repository validation failed without a diagnostic",
            )
        })
    })?;
    let relations = extract_relations(
        read.value.root.package_id,
        &read.value.owners,
        &read.value.types,
        &read.value.dependencies,
    )?;
    let mut owner_kinds = BTreeMap::new();
    for record in read.value.owners.values() {
        let count = owner_kinds
            .entry(record.kind().name().to_owned())
            .or_insert(0_u64);
        *count = count.saturating_add(1);
    }
    let modules = owner_kinds
        .get(OwnerKind::Module.name())
        .copied()
        .unwrap_or(0);
    let functions = owner_kinds
        .get(OwnerKind::PureFunction.name())
        .copied()
        .unwrap_or(0)
        .saturating_add(
            owner_kinds
                .get(OwnerKind::TaskFunction.name())
                .copied()
                .unwrap_or(0),
        );
    let mut owner_hasher = Hasher::new_derive_key(OWNER_DIGEST_DOMAIN);
    for owner in read.value.owners.keys() {
        let value = owner.to_string();
        owner_hasher.update(&(value.len() as u64).to_be_bytes());
        owner_hasher.update(value.as_bytes());
    }
    owner_hasher.update(&(read.value.owners.len() as u64).to_be_bytes());
    let mut relation_hasher = Hasher::new_derive_key(RELATION_DIGEST_DOMAIN);
    for relation in &relations {
        hash_endpoint(&mut relation_hasher, relation.source);
        relation_hasher.update(&[relation.kind.tag()]);
        hash_endpoint(&mut relation_hasher, relation.target);
    }
    relation_hasher.update(&(relations.len() as u64).to_be_bytes());
    Ok(SemanticInventory {
        revision: view.revision().to_string(),
        owners: read.value.owners.len() as u64,
        modules,
        functions,
        relations: relations.len() as u64,
        types: read.value.types.len() as u64,
        dependencies: read.value.dependencies.len() as u64,
        retirements: read.value.retirements.len() as u64,
        owner_kinds,
        owner_identity_digest: format!(
            "semantic_owner_identities_{}",
            owner_hasher.finalize().to_hex()
        ),
        relation_digest: format!("semantic_relations_{}", relation_hasher.finalize().to_hex()),
        validation_owner_records: validation.owners_checked,
        validation_type_objects: validation.type_objects_checked,
        validation_expression_records: validation.expression_records_checked,
        validation_relation_edges: validation.relation_edges,
        validation_work: validation.work_consumed,
        map_pages_read: read.work.map.pages_read,
        map_bytes_read: read.work.map.bytes_read,
        store_objects_read: read.work.store.objects_read,
        store_bytes_read: read.work.store.bytes_read,
    })
}

/// Reconstruct one complete accepted function from typed authority for independent verification.
pub fn function_definition_oracle(
    project: &Path,
    function: &str,
) -> Result<FunctionDefinitionOracle, Diagnostic> {
    let function = DeclarationId::parse(function)?;
    let repository = GraphRepository::open(project)?;
    let view = repository.view_current()?;
    let read = view.reconstruct_full_oracle()?;
    let witness = rebuild_full_witness(&read.value).map_err(first_oracle_diagnostic)?;
    reconstruct_function_definition(&read.value, &witness, view.revision(), function)
}

/// Select the largest live local function by body-owner count, with exact identity as the stable
/// tie-breaker, using the same complete typed-authority oracle input.
pub fn largest_function_definition_oracle(
    project: &Path,
) -> Result<FunctionDefinitionOracle, Diagnostic> {
    let repository = GraphRepository::open(project)?;
    let view = repository.view_current()?;
    let read = view.reconstruct_full_oracle()?;
    let witness = rebuild_full_witness(&read.value).map_err(first_oracle_diagnostic)?;
    let functions = read
        .value
        .owners
        .iter()
        .filter_map(|(owner, record)| {
            let OwnerKey::Declaration(declaration) = owner else {
                return None;
            };
            matches!(
                record,
                OwnerRecord::Declaration(value)
                    if matches!(&value.payload, DeclarationPayload::Function(_))
            )
            .then_some(*declaration)
        })
        .collect::<Vec<_>>();
    let mut largest: Option<FunctionDefinitionOracle> = None;
    for function in functions {
        let candidate =
            reconstruct_function_definition(&read.value, &witness, view.revision(), function)?;
        let replace = largest.as_ref().is_none_or(|current| {
            (candidate.body_preorder.len(), candidate.function.as_str())
                > (current.body_preorder.len(), current.function.as_str())
        });
        if replace {
            largest = Some(candidate);
        }
    }
    largest.ok_or_else(|| {
        oracle_error(
            DiagnosticClass::Semantic,
            "contributor_definition_functions_empty",
            "accepted typed authority contains no local function definitions",
        )
    })
}

fn reconstruct_function_definition(
    snapshot: &KernelSnapshot,
    witness: &FullWitness,
    revision: super::semantic_id::RevisionId,
    function: DeclarationId,
) -> Result<FunctionDefinitionOracle, Diagnostic> {
    let function_owner = OwnerKey::Declaration(function);
    let root = snapshot.owners.get(&function_owner).ok_or_else(|| {
        oracle_error(
            DiagnosticClass::Semantic,
            "contributor_definition_missing",
            format!("function '{function}' is absent from the accepted typed authority"),
        )
    })?;
    let OwnerRecord::Declaration(declaration) = root else {
        return Err(oracle_error(
            DiagnosticClass::Semantic,
            "contributor_definition_kind",
            format!("owner '{function}' is not a declaration"),
        ));
    };
    let DeclarationPayload::Function(function_record) = &declaration.payload else {
        return Err(oracle_error(
            DiagnosticClass::Semantic,
            "contributor_definition_kind",
            format!("declaration '{function}' is not a local function with a body"),
        ));
    };
    let package = snapshot.root.package_id;
    let mut seen = BTreeSet::new();
    let mut contract_owners = Vec::new();
    contract_owners.push(oracle_owner(
        snapshot,
        witness,
        &mut seen,
        function_owner,
        OwnerKey::Module(declaration.module),
        "module_declaration",
        0,
        0,
        declaration.expected_kind().name(),
        OwnershipEntry::new(
            OwnershipParent::Owner(OwnerKey::Module(declaration.module)),
            OwnershipRole::ModuleDeclaration,
        ),
    )?);
    for (ordinal, type_parameter) in function_record.type_parameters.iter().copied().enumerate() {
        let ordinal = oracle_ordinal(ordinal)?;
        contract_owners.push(oracle_owner(
            snapshot,
            witness,
            &mut seen,
            OwnerKey::TypeParameter(type_parameter),
            function_owner,
            "function_type_parameter",
            ordinal,
            0,
            "type_parameter",
            OwnershipEntry::new(
                OwnershipParent::Owner(function_owner),
                OwnershipRole::DeclarationTypeParameter,
            ),
        )?);
    }
    for (ordinal, parameter) in function_record.parameters.iter().copied().enumerate() {
        let ordinal = oracle_ordinal(ordinal)?;
        contract_owners.push(oracle_owner(
            snapshot,
            witness,
            &mut seen,
            OwnerKey::Parameter(parameter),
            function_owner,
            "function_parameter",
            ordinal,
            0,
            "parameter",
            OwnershipEntry::new(
                OwnershipParent::Owner(function_owner),
                OwnershipRole::DeclarationParameter,
            ),
        )?);
    }
    let requirements = match &function_record.effect {
        FunctionEffect::Pure => &[][..],
        FunctionEffect::Task { requirements } => requirements.as_slice(),
    };
    for (ordinal, requirement) in requirements.iter().copied().enumerate() {
        if requirement.package != package {
            continue;
        }
        let owner = OwnerKey::Requirement(requirement.requirement);
        let expected = OwnershipEntry::new(
            OwnershipParent::Owner(function_owner),
            OwnershipRole::DeclarationRequirement,
        );
        if witness.entries.ownership.get(&owner) != Some(&expected) {
            continue;
        }
        contract_owners.push(oracle_owner(
            snapshot,
            witness,
            &mut seen,
            owner,
            function_owner,
            "function_requirement",
            oracle_ordinal(ordinal)?,
            0,
            "requirement",
            expected,
        )?);
    }

    let mut walker = DefinitionOracleWalker {
        snapshot,
        witness,
        seen,
        body: Vec::new(),
        capability_calls: Vec::new(),
        matches: Vec::new(),
        maximum_depth: 0,
    };
    walker.visit_expression(
        function_record.body,
        function_owner,
        "function_body",
        0,
        0,
        OwnershipEntry::new(
            OwnershipParent::Owner(function_owner),
            OwnershipRole::ExpressionRoot(ExpressionRootRole::FunctionBody),
        ),
    )?;
    let maximum_depth = walker.maximum_depth;
    let capability_calls = walker.capability_calls;
    let matches = walker.matches;
    let body_preorder = walker.body;
    let closure = walker.seen;
    let relations = witness
        .entries
        .relations
        .iter()
        .filter_map(|edge| {
            let RelationEndpoint::Owner(source) = edge.source else {
                return None;
            };
            if source.package != package || !closure.contains(&source.owner) {
                return None;
            }
            Some(FunctionDefinitionOracleRelation {
                source: oracle_endpoint(edge.source),
                kind: edge.kind.name().to_owned(),
                target: oracle_endpoint(edge.target),
            })
        })
        .collect::<Vec<_>>();
    let mut owner_hasher = Hasher::new_derive_key(DEFINITION_OWNER_DIGEST_DOMAIN);
    let mut fact_hasher = Hasher::new_derive_key(DEFINITION_FACT_DIGEST_DOMAIN);
    for owner in contract_owners.iter().chain(&body_preorder) {
        hash_oracle_owner(&mut owner_hasher, owner, false);
        hash_oracle_owner(&mut fact_hasher, owner, true);
    }
    owner_hasher.update(&((contract_owners.len() + body_preorder.len()) as u64).to_be_bytes());
    fact_hasher.update(&((contract_owners.len() + body_preorder.len()) as u64).to_be_bytes());
    let mut relation_hasher = Hasher::new_derive_key(DEFINITION_RELATION_DIGEST_DOMAIN);
    for relation in &relations {
        hash_string(&mut relation_hasher, &relation.source);
        hash_string(&mut relation_hasher, &relation.kind);
        hash_string(&mut relation_hasher, &relation.target);
    }
    relation_hasher.update(&(relations.len() as u64).to_be_bytes());
    let effect = match &function_record.effect {
        FunctionEffect::Pure => "pure",
        FunctionEffect::Task { .. } => "task",
    };
    Ok(FunctionDefinitionOracle {
        repository: snapshot.root.repository_id.to_string(),
        package: package.to_string(),
        revision: revision.to_string(),
        function: function.to_string(),
        kind: declaration.expected_kind().name().to_owned(),
        name: declaration.name.as_str().to_owned(),
        effect: effect.to_owned(),
        result_type: function_record.result.to_string(),
        type_parameters: function_record.type_parameters.len() as u64,
        parameters: function_record.parameters.len() as u64,
        requirements: requirements.len() as u64,
        body_root: function_record.body.to_string(),
        structural_edges: (contract_owners.len() + body_preorder.len()) as u64,
        maximum_depth,
        owner_order_digest: format!(
            "definition_oracle_owners_{}",
            owner_hasher.finalize().to_hex()
        ),
        fact_digest: format!(
            "definition_oracle_facts_{}",
            fact_hasher.finalize().to_hex()
        ),
        relation_digest: format!(
            "definition_oracle_relations_{}",
            relation_hasher.finalize().to_hex()
        ),
        validator: witness.manifest.validator_contract.to_string(),
        certificate: witness.manifest.certificate.to_string(),
        contract_owners,
        body_preorder,
        relations,
        capability_calls,
        matches,
    })
}

struct DefinitionOracleWalker<'a> {
    snapshot: &'a KernelSnapshot,
    witness: &'a FullWitness,
    seen: BTreeSet<OwnerKey>,
    body: Vec<FunctionDefinitionOracleOwner>,
    capability_calls: Vec<FunctionDefinitionOracleCapability>,
    matches: Vec<FunctionDefinitionOracleMatch>,
    maximum_depth: u64,
}

impl DefinitionOracleWalker<'_> {
    fn visit_expression(
        &mut self,
        expression: super::semantic_id::ExpressionId,
        parent: OwnerKey,
        role: &'static str,
        ordinal: u32,
        depth: u64,
        expected: OwnershipEntry,
    ) -> Result<(), Diagnostic> {
        let owner = OwnerKey::Expression(expression);
        let observation = oracle_owner(
            self.snapshot,
            self.witness,
            &mut self.seen,
            owner,
            parent,
            role,
            ordinal,
            depth,
            "expression",
            expected,
        )?;
        self.maximum_depth = self.maximum_depth.max(depth);
        self.body.push(observation);
        let record = self.snapshot.owners.get(&owner).cloned().ok_or_else(|| {
            oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_definition_owner",
                format!("body expression '{owner}' is absent"),
            )
        })?;
        let OwnerRecord::Expression(record) = record else {
            return Err(oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_definition_owner",
                format!("body owner '{owner}' is not an expression"),
            ));
        };
        match &record.operation {
            ExpressionOperation::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => self
                .capability_calls
                .push(FunctionDefinitionOracleCapability {
                    expression: owner.to_string(),
                    requirement: format!("{}/{}", requirement.package, requirement.requirement),
                    operation: format!("{}/{}", operation.package, operation.operation),
                    arguments: arguments.len() as u64,
                    parameter_uses: oracle_operation_parameter_uses(self.snapshot, *operation)?,
                }),
            ExpressionOperation::Match { arms, .. } => {
                self.matches.push(FunctionDefinitionOracleMatch {
                    expression: owner.to_string(),
                    cases: arms
                        .iter()
                        .map(|arm| format!("{}/{}", arm.case.package, arm.case.case))
                        .collect(),
                });
            }
            _ => {}
        }
        let child_depth = depth.checked_add(1).ok_or_else(|| {
            oracle_error(
                DiagnosticClass::Resource,
                "contributor_definition_depth",
                "oracle body depth overflowed",
            )
        })?;
        match &record.operation {
            ExpressionOperation::Let { bindings, body } => {
                for (index, binding) in bindings.iter().copied().enumerate() {
                    self.visit_binding(
                        binding,
                        owner,
                        "let_binding",
                        oracle_ordinal(index)?,
                        child_depth,
                        (BindingKind::Let, BindingContainerRole::Let),
                    )?;
                }
                self.visit_expression_child(
                    *body,
                    owner,
                    "let_body",
                    0,
                    child_depth,
                    ExpressionChildRole::LetBody,
                )?;
            }
            ExpressionOperation::Match { value, arms } => {
                self.visit_expression_child(
                    *value,
                    owner,
                    "match_value",
                    0,
                    child_depth,
                    ExpressionChildRole::MatchValue,
                )?;
                for (index, arm) in arms.iter().enumerate() {
                    let ordinal = oracle_ordinal(index)?;
                    if let Some(binding) = arm.payload_binding {
                        self.visit_binding(
                            binding,
                            owner,
                            "match_payload",
                            ordinal,
                            child_depth,
                            (
                                BindingKind::MatchPayload,
                                BindingContainerRole::MatchPayload,
                            ),
                        )?;
                    }
                    self.visit_expression_child(
                        arm.body,
                        owner,
                        "match_arm",
                        ordinal,
                        child_depth,
                        ExpressionChildRole::MatchArmBody,
                    )?;
                }
            }
            ExpressionOperation::Transaction { binding, body, .. } => {
                self.visit_binding(
                    *binding,
                    owner,
                    "transaction_binding",
                    0,
                    child_depth,
                    (BindingKind::Transaction, BindingContainerRole::Transaction),
                )?;
                self.visit_expression_child(
                    *body,
                    owner,
                    "transaction_body",
                    0,
                    child_depth,
                    ExpressionChildRole::TransactionBody,
                )?;
            }
            _ => {
                for child in record.children() {
                    self.visit_expression_child(
                        child.expression,
                        owner,
                        oracle_child_role(child.role),
                        child.ordinal,
                        child_depth,
                        child.role,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn visit_expression_child(
        &mut self,
        expression: super::semantic_id::ExpressionId,
        parent: OwnerKey,
        role: &'static str,
        ordinal: u32,
        depth: u64,
        child_role: ExpressionChildRole,
    ) -> Result<(), Diagnostic> {
        self.visit_expression(
            expression,
            parent,
            role,
            ordinal,
            depth,
            OwnershipEntry::new(
                OwnershipParent::Owner(parent),
                OwnershipRole::ExpressionChild {
                    role: child_role,
                    ordinal,
                },
            ),
        )
    }

    fn visit_binding(
        &mut self,
        binding: super::semantic_id::BindingId,
        parent: OwnerKey,
        role: &'static str,
        ordinal: u32,
        depth: u64,
        expected: (BindingKind, BindingContainerRole),
    ) -> Result<(), Diagnostic> {
        let (expected_kind, container) = expected;
        let owner = OwnerKey::Binding(binding);
        let observation = oracle_owner(
            self.snapshot,
            self.witness,
            &mut self.seen,
            owner,
            parent,
            role,
            ordinal,
            depth,
            "binding",
            OwnershipEntry::new(
                OwnershipParent::Owner(parent),
                OwnershipRole::ExpressionBinding {
                    role: container,
                    ordinal,
                },
            ),
        )?;
        self.maximum_depth = self.maximum_depth.max(depth);
        self.body.push(observation);
        let record = self.snapshot.owners.get(&owner).cloned().ok_or_else(|| {
            oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_definition_owner",
                format!("body binding '{owner}' is absent"),
            )
        })?;
        let OwnerRecord::Binding(record) = record else {
            return Err(oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_definition_owner",
                format!("body owner '{owner}' is not a binding"),
            ));
        };
        if record.kind != expected_kind {
            return Err(oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_definition_binding",
                format!("binding '{owner}' has an unexpected lexical kind"),
            ));
        }
        if let Some(value) = record.value {
            let child_depth = depth.checked_add(1).ok_or_else(|| {
                oracle_error(
                    DiagnosticClass::Resource,
                    "contributor_definition_depth",
                    "oracle body depth overflowed",
                )
            })?;
            self.visit_expression(
                value,
                owner,
                "binding_value",
                0,
                child_depth,
                OwnershipEntry::new(
                    OwnershipParent::Owner(owner),
                    OwnershipRole::ExpressionRoot(ExpressionRootRole::BindingValue),
                ),
            )?;
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn oracle_owner(
    snapshot: &KernelSnapshot,
    witness: &FullWitness,
    seen: &mut BTreeSet<OwnerKey>,
    owner: OwnerKey,
    parent: OwnerKey,
    role: &str,
    ordinal: u32,
    depth: u64,
    form: &str,
    expected: OwnershipEntry,
) -> Result<FunctionDefinitionOracleOwner, Diagnostic> {
    if !seen.insert(owner) {
        return Err(oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_definition_shared",
            format!("definition owner '{owner}' is structurally shared or cyclic"),
        ));
    }
    if witness.entries.ownership.get(&owner) != Some(&expected) {
        return Err(oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_definition_ownership",
            format!("definition owner '{owner}' has unexpected rebuilt ownership"),
        ));
    }
    let record = snapshot.owners.get(&owner).ok_or_else(|| {
        oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_definition_owner",
            format!("definition owner '{owner}' is absent"),
        )
    })?;
    if record.owner() != owner {
        return Err(oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_definition_owner",
            format!("definition owner '{owner}' disagrees with its record identity"),
        ));
    }
    let (record_digest, _) = encode_owner(record)?;
    let summary_digest = witness.entries.summaries.get(&owner).ok_or_else(|| {
        oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_definition_summary",
            format!("definition owner '{owner}' has no rebuilt summary binding"),
        )
    })?;
    let summary = witness.summaries.get(&owner).ok_or_else(|| {
        oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_definition_summary",
            format!("definition owner '{owner}' has no rebuilt typed summary"),
        )
    })?;
    if summary.record != record_digest || summary.kind != record.kind() || summary.owner != owner {
        return Err(oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_definition_summary",
            format!("definition owner '{owner}' disagrees with its rebuilt summary"),
        ));
    }
    Ok(FunctionDefinitionOracleOwner {
        owner: owner.to_string(),
        parent: parent.to_string(),
        role: role.to_owned(),
        ordinal,
        depth,
        kind: record.kind().name().to_owned(),
        form: if form == "expression" {
            let OwnerRecord::Expression(expression) = record else {
                return Err(oracle_error(
                    DiagnosticClass::Corrupt,
                    "contributor_definition_owner",
                    format!("definition owner '{owner}' is not an expression"),
                ));
            };
            oracle_expression_form(&expression.operation).to_owned()
        } else if form == "binding" {
            let OwnerRecord::Binding(binding) = record else {
                return Err(oracle_error(
                    DiagnosticClass::Corrupt,
                    "contributor_definition_owner",
                    format!("definition owner '{owner}' is not a binding"),
                ));
            };
            format!("binding:{}", oracle_binding_kind(binding.kind))
        } else {
            form.to_owned()
        },
        name: record.name().map(|value| value.as_str().to_owned()),
        record: record_digest.to_string(),
        summary: summary_digest.to_string(),
        type_roots: record
            .type_roots()
            .into_iter()
            .map(|value| value.to_string())
            .collect(),
        expression_roots: record
            .expression_roots()
            .into_iter()
            .map(|value| value.to_string())
            .collect(),
        blob_roots: record
            .blob_roots()
            .into_iter()
            .map(|(digest, bytes)| format!("{digest}:{bytes}"))
            .collect(),
    })
}

fn oracle_operation_parameter_uses(
    snapshot: &KernelSnapshot,
    operation: OperationReference,
) -> Result<Vec<String>, Diagnostic> {
    let parameters = if operation.package == snapshot.root.package_id {
        let owner = OwnerKey::Operation(operation.operation);
        let Some(OwnerRecord::Operation(record)) = snapshot.owners.get(&owner) else {
            return Err(oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_definition_operation",
                format!("local capability operation '{owner}' is absent"),
            ));
        };
        record.parameters.clone()
    } else {
        let dependency = snapshot
            .dependencies
            .get(&operation.package)
            .ok_or_else(|| {
                oracle_error(
                    DiagnosticClass::Corrupt,
                    "contributor_definition_operation",
                    format!(
                        "capability operation package '{}' is not an exact dependency",
                        operation.package
                    ),
                )
            })?;
        let interface = snapshot
            .dependency_interfaces
            .get(&dependency.package_revision)
            .ok_or_else(|| {
                oracle_error(
                    DiagnosticClass::Corrupt,
                    "contributor_definition_operation",
                    "capability operation dependency interface is absent",
                )
            })?;
        let Some(PackageInterfaceRecord::Operation(record)) =
            interface.get(&OwnerKey::Operation(operation.operation))
        else {
            return Err(oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_definition_operation",
                format!(
                    "capability operation '{}' is absent from its exact dependency interface",
                    operation.operation
                ),
            ));
        };
        record.parameters.clone()
    };
    parameters
        .into_iter()
        .map(|parameter| {
            let record = if operation.package == snapshot.root.package_id {
                let Some(OwnerRecord::Parameter(record)) =
                    snapshot.owners.get(&OwnerKey::Parameter(parameter))
                else {
                    return Err(oracle_error(
                        DiagnosticClass::Corrupt,
                        "contributor_definition_operation_parameter",
                        format!("local operation parameter '{parameter}' is absent"),
                    ));
                };
                record
            } else {
                let dependency = snapshot.dependencies.get(&operation.package).ok_or_else(|| {
                    oracle_error(
                        DiagnosticClass::Corrupt,
                        "contributor_definition_operation_parameter",
                        "operation dependency disappeared during oracle reconstruction",
                    )
                })?;
                let interface = snapshot
                    .dependency_interfaces
                    .get(&dependency.package_revision)
                    .ok_or_else(|| {
                        oracle_error(
                            DiagnosticClass::Corrupt,
                            "contributor_definition_operation_parameter",
                            "operation dependency interface disappeared during oracle reconstruction",
                        )
                    })?;
                let Some(PackageInterfaceRecord::Parameter(record)) =
                    interface.get(&OwnerKey::Parameter(parameter))
                else {
                    return Err(oracle_error(
                        DiagnosticClass::Corrupt,
                        "contributor_definition_operation_parameter",
                        format!(
                            "operation parameter '{parameter}' is absent from its exact dependency interface"
                        ),
                    ));
                };
                record
            };
            Ok(match record.use_mode {
                ParameterUse::Unrestricted => "unrestricted",
                ParameterUse::Borrow => "borrow",
                ParameterUse::Consume => "consume",
            }
            .to_owned())
        })
        .collect()
}

fn oracle_expression_form(operation: &ExpressionOperation) -> &'static str {
    match operation {
        ExpressionOperation::Unit {} => "unit",
        ExpressionOperation::Bool { .. } => "bool",
        ExpressionOperation::I64 { .. } => "i64",
        ExpressionOperation::Text { .. } => "text",
        ExpressionOperation::StaticText { .. } => "static_text",
        ExpressionOperation::Local { .. } => "local",
        ExpressionOperation::Constant { .. } => "constant",
        ExpressionOperation::If { .. } => "if",
        ExpressionOperation::Let { .. } => "let",
        ExpressionOperation::Sequence { .. } => "sequence",
        ExpressionOperation::Call { .. } => "call",
        ExpressionOperation::FunctionValue { .. } => "function_value",
        ExpressionOperation::Invoke { .. } => "invoke",
        ExpressionOperation::Record { .. } => "record",
        ExpressionOperation::Variant { .. } => "variant",
        ExpressionOperation::Field { .. } => "field",
        ExpressionOperation::List { .. } => "list",
        ExpressionOperation::Map { .. } => "map",
        ExpressionOperation::Match { .. } => "match",
        ExpressionOperation::CapabilityCall { .. } => "capability_call",
        ExpressionOperation::Transaction { .. } => "transaction",
    }
}

fn oracle_binding_kind(kind: BindingKind) -> &'static str {
    match kind {
        BindingKind::Let => "let",
        BindingKind::MatchPayload => "match_payload",
        BindingKind::Transaction => "transaction",
    }
}

fn oracle_child_role(role: ExpressionChildRole) -> &'static str {
    match role {
        ExpressionChildRole::Condition => "condition",
        ExpressionChildRole::TrueBranch => "true_branch",
        ExpressionChildRole::FalseBranch => "false_branch",
        ExpressionChildRole::LetBody => "let_body",
        ExpressionChildRole::SequenceItem => "sequence_item",
        ExpressionChildRole::CallArgument => "call_argument",
        ExpressionChildRole::InvokeCallee => "invoke_callee",
        ExpressionChildRole::InvokeArgument => "invoke_argument",
        ExpressionChildRole::RecordField => "record_field",
        ExpressionChildRole::VariantPayload => "variant_payload",
        ExpressionChildRole::FieldValue => "field_value",
        ExpressionChildRole::ListItem => "list_item",
        ExpressionChildRole::MapKey => "map_key",
        ExpressionChildRole::MapValue => "map_value",
        ExpressionChildRole::MatchValue => "match_value",
        ExpressionChildRole::MatchArmBody => "match_arm",
        ExpressionChildRole::CapabilityArgument => "capability_argument",
        ExpressionChildRole::TransactionBody => "transaction_body",
    }
}

fn oracle_endpoint(endpoint: RelationEndpoint) -> String {
    match endpoint {
        RelationEndpoint::Package(package) => format!("package:{package}"),
        RelationEndpoint::Owner(owner) => format!("owner:{}/{}", owner.package, owner.owner),
    }
}

fn hash_oracle_owner(hasher: &mut Hasher, owner: &FunctionDefinitionOracleOwner, facts: bool) {
    hash_string(hasher, &owner.owner);
    hash_string(hasher, &owner.parent);
    hash_string(hasher, &owner.role);
    hasher.update(&owner.ordinal.to_be_bytes());
    hasher.update(&owner.depth.to_be_bytes());
    hash_string(hasher, &owner.kind);
    hash_string(hasher, &owner.form);
    hash_string(hasher, owner.name.as_deref().unwrap_or(""));
    if facts {
        hash_string(hasher, &owner.record);
        hash_string(hasher, &owner.summary);
    }
}

fn hash_string(hasher: &mut Hasher, value: &str) {
    hasher.update(&(value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
}

fn oracle_ordinal(value: usize) -> Result<u32, Diagnostic> {
    u32::try_from(value).map_err(|_| {
        oracle_error(
            DiagnosticClass::Resource,
            "contributor_definition_ordinal",
            "definition ordinal cannot be represented",
        )
    })
}

fn first_oracle_diagnostic(diagnostics: Vec<Diagnostic>) -> Diagnostic {
    diagnostics.into_iter().next().unwrap_or_else(|| {
        oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_definition_oracle_empty",
            "full witness reconstruction failed without a diagnostic",
        )
    })
}

fn oracle_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

fn hash_endpoint(hasher: &mut Hasher, endpoint: RelationEndpoint) {
    let value = match endpoint {
        RelationEndpoint::Package(package) => format!("package:{package}"),
        RelationEndpoint::Owner(owner) => format!("owner:{}/{}", owner.package, owner.owner),
    };
    hasher.update(&(value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::{
        GraphRepository, compact_change_default_maximum_operations, function_definition_oracle,
        largest_function_definition_oracle, semantic_inventory,
    };
    use std::path::Path;

    #[test]
    fn maintained_standard_inventory_is_typed_and_read_only() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard");
        let before = std::fs::read(project.join("HEAD")).expect("standard HEAD before oracle");
        let inventory = semantic_inventory(&project).expect("standard semantic inventory");
        assert_eq!(inventory.owners, 431);
        assert_eq!(inventory.modules, 12);
        assert!(inventory.functions > 0);
        assert!(inventory.relations > 0);
        assert_eq!(
            std::fs::read(project.join("HEAD")).expect("standard HEAD after oracle"),
            before
        );
    }

    #[test]
    fn compact_change_default_is_the_current_batch_authority() {
        assert_eq!(compact_change_default_maximum_operations(), 1_000);
    }

    #[test]
    fn maintained_affine_worker_oracle_is_typed_disjoint_and_read_only() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal");
        let before = std::fs::read(project.join("HEAD")).expect("lkjournal HEAD before oracle");
        let oracle = function_definition_oracle(&project, "decl_a914bb78de075ff44a857ac028d704f3")
            .expect("maintained worker definition oracle");
        assert_eq!(oracle.kind, "task_function");
        assert_eq!(oracle.name, "run");
        assert_eq!(oracle.contract_owners.len(), 3);
        assert_eq!(oracle.body_preorder.len(), 15);
        assert_eq!(oracle.structural_edges, 18);
        assert_eq!(oracle.maximum_depth, 3);
        assert!(oracle.body_preorder.iter().all(|owner| {
            owner.name.as_deref() != Some("lease-info")
                && owner.name.as_deref() != Some("renewed-lease")
        }));
        assert_eq!(oracle.matches.len(), 1);
        assert!(oracle.relations.iter().any(|relation| {
            relation.kind == "function_call"
                && relation
                    .target
                    .ends_with("decl_7f443401f4946c55fa239c5430e8ad93")
        }));
        let operation_uses = oracle
            .capability_calls
            .iter()
            .map(|call| (call.operation.as_str(), call.parameter_uses.as_slice()))
            .collect::<Vec<_>>();
        assert!(
            operation_uses.iter().any(|(operation, _)| {
                operation.ends_with("op_23bc0c498113c09a2ff0a4cf9c0a37ab")
            })
        );
        for operation in [
            "op_1a5491eb1c3ef3d15ec28268b6f04afc",
            "op_f593ba236055aa1afa6c02eaf0db6a64",
            "op_679b43bb7dc0b298a7706d4e8a7bef23",
            "op_242e065f9738b454e2328ed0e558e6a0",
        ] {
            assert!(
                operation_uses
                    .iter()
                    .all(|(observed, _)| !observed.ends_with(operation)),
                "entry retained transferred operation {operation}"
            );
        }

        let helper = function_definition_oracle(&project, "decl_7f443401f4946c55fa239c5430e8ad93")
            .expect("maintained worker helper definition oracle");
        assert_eq!(helper.kind, "task_function");
        assert_eq!(helper.name, "process-lease");
        assert_eq!(helper.parameters, 2);
        assert_eq!(helper.requirements, 1);
        assert_eq!(helper.contract_owners.len(), 3);
        assert_eq!(helper.body_preorder.len(), 36);
        assert_eq!(helper.structural_edges, 39);
        assert_eq!(helper.maximum_depth, 6);
        assert!(
            helper
                .body_preorder
                .iter()
                .any(|owner| owner.name.as_deref() == Some("lease-info"))
        );
        assert!(
            helper
                .body_preorder
                .iter()
                .any(|owner| owner.name.as_deref() == Some("renewed-lease"))
        );
        assert_eq!(helper.matches.len(), 1);
        assert!(helper.relations.iter().any(|relation| {
            relation.kind == "parameter_requirement"
                && relation
                    .target
                    .ends_with("req_0cebded5cb056cda5484e39aa40594ad")
        }));
        let helper_operation_uses = helper
            .capability_calls
            .iter()
            .map(|call| (call.operation.as_str(), call.parameter_uses.as_slice()))
            .collect::<Vec<_>>();
        assert!(helper_operation_uses.iter().any(|(operation, uses)| {
            operation.ends_with("op_1a5491eb1c3ef3d15ec28268b6f04afc")
                && uses.contains(&"borrow".to_owned())
        }));
        for operation in [
            "op_f593ba236055aa1afa6c02eaf0db6a64",
            "op_679b43bb7dc0b298a7706d4e8a7bef23",
            "op_242e065f9738b454e2328ed0e558e6a0",
        ] {
            assert!(helper_operation_uses.iter().any(|(observed, uses)| {
                observed.ends_with(operation) && uses.contains(&"consume".to_owned())
            }));
        }
        let repository = GraphRepository::open(&project).expect("maintained worker repository");
        let view = repository.view_current().expect("maintained worker view");
        let complete = view
            .reconstruct_full_oracle()
            .expect("maintained complete authority");
        let rebuilt = crate::platform::witness::rebuild_full_witness(&complete.value)
            .expect("maintained complete witness");
        let mut summary_mismatches = Vec::new();
        for owner in complete.value.owners.keys().copied() {
            let bound = view
                .definition_reader()
                .summary(owner)
                .expect("bound maintained summary read")
                .expect("bound maintained summary");
            if bound.summary != rebuilt.summaries[&owner]
                || bound.digest != rebuilt.entries.summaries[&owner]
            {
                summary_mismatches.push(owner);
            }
        }
        assert!(
            summary_mismatches.is_empty(),
            "incremental and complete summaries disagree for {summary_mismatches:?}"
        );
        assert_eq!(
            std::fs::read(project.join("HEAD")).expect("lkjournal HEAD after oracle"),
            before
        );
        let largest =
            largest_function_definition_oracle(&project).expect("largest maintained definition");
        assert_eq!(largest.function, "decl_0693166bd7c29bee83d2ead289148f65");
        assert_eq!(largest.body_preorder.len(), 192);
    }
}
