//! Read-only contributor observations outside the executable's public operation registry.

use super::change::ChangeBudget;
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::kernel::{
    BindingKind, DeclarationPayload, DeclarationReference, EncodedOwnerKey, ExpressionChildRole,
    ExpressionOperation, FunctionEffect, KernelSnapshot, LocalValueReference, Name,
    OperationReference, OwnerKey, OwnerKind, OwnerRecord, PackageInterfaceDeclarationPayload,
    PackageInterfaceRecord, ParameterParent, ParameterUse, RelationEndpoint, RequirementReference,
    TypeForm, TypeObjectDigest, encode_owner, extract_relations, infer_function_expression_type,
    validate_full,
};
use super::publication::GraphRepository;
use super::semantic_id::{DeclarationId, ExpressionId, encode_hex};
use super::storage::catalog::{CatalogHistory, CatalogWork};
use super::storage::directory::CatalogState;
use super::storage::object::{StoreError, StoreErrorClass};
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
const EXTRACTION_MOVED_DIGEST_DOMAIN: &str = "lkjscript.function-extraction.moved-owners.v1";

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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogWorkInventory {
    pub healthy_opens: u64,
    pub manifests_read: u64,
    pub manifest_bytes_read: u64,
    pub segment_metadata_read: u64,
    pub segment_metadata_bytes_read: u64,
    pub segment_lookups: u64,
    pub segment_blocks_read: u64,
    pub segment_block_bytes_read: u64,
    pub segment_entries_examined: u64,
    pub targeted_pack_footers_read: u64,
    pub targeted_pack_footer_bytes_read: u64,
    pub delta_segments_written: u64,
    pub merge_operations: u64,
    pub merge_entries_read: u64,
    pub merge_bytes_read: u64,
    pub segments_written: u64,
    pub segment_entries_written: u64,
    pub manifests_written: u64,
    pub obsolete_segments_removed: u64,
    pub full_rebuilds: u64,
    pub full_footer_scan_runs: u64,
    pub pack_footers_scanned: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogHistoryInventory {
    pub delta_segments: u64,
    pub merge_operations: u64,
    pub merge_entries_read: u64,
    pub merge_bytes_read: u64,
    pub segments_written: u64,
    pub segment_entries_written: u64,
    pub full_rebuilds: u64,
    pub full_footer_scan_runs: u64,
    pub pack_footers_scanned: u64,
}

/// Contributor-only bounded observation of derived catalog layout and an independent footer
/// reconstruction. It does not add a product operation or mutate semantic authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogInventory {
    pub identity: String,
    pub contract_version: u16,
    pub state: String,
    pub generation: u64,
    pub commitment: String,
    pub entries: u64,
    pub packs: u64,
    pub segments: u64,
    pub segment_bytes: u64,
    pub segment_metadata_bytes: u64,
    pub maximum_level: Option<u16>,
    pub maximum_live_segments: u64,
    pub maximum_lookup_segments: u64,
    pub block_entries: u64,
    pub history: CatalogHistoryInventory,
    pub work: CatalogWorkInventory,
    pub leftovers: Vec<String>,
    pub footer_oracle_packs: u64,
    pub footer_oracle_entries: u64,
    pub footer_oracle_bytes_read: u64,
    pub footer_oracle_duplicate_objects: u64,
    pub footer_oracle_commitment: String,
    pub footer_oracle_equal: bool,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionExtractionOracleCapture {
    pub source_kind: String,
    pub source: String,
    pub name: String,
    pub ty: String,
    pub use_mode: String,
    pub requirement: Option<String>,
    pub uses: Vec<String>,
}

/// Independent complete-authority derivation of one prospective extraction boundary.
///
/// This contributor-only path first reconstructs the definition with the full witness oracle,
/// then derives closure and capture membership from that independent preorder. It does not call
/// the production extraction traversal, candidate overlay, mapping, allocation, or plan encoder.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionExtractionOracle {
    pub repository: String,
    pub package: String,
    pub revision: String,
    pub function: String,
    pub selected_root: String,
    pub parent: String,
    pub base_body_records: u64,
    pub caller_body_records: u64,
    pub helper_body_records: u64,
    pub moved_digest: String,
    pub moved_owners: Vec<String>,
    pub preserved_owners: Vec<String>,
    pub changed_owners: Vec<String>,
    pub captures: Vec<FunctionExtractionOracleCapture>,
    pub result_type: String,
    pub effect: String,
    pub requirements: Vec<String>,
    pub affine_requirement: Option<String>,
    pub generated_owners: u64,
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

pub fn catalog_inventory(project: &Path) -> Result<CatalogInventory, Diagnostic> {
    let repository = GraphRepository::open(project)?;
    let store = repository.object_store()?;
    let observation = store.catalog_observation();
    let footer = store
        .verify_catalog_from_footers()
        .map_err(catalog_store_diagnostic)?;
    Ok(CatalogInventory {
        identity: observation.identity.to_owned(),
        contract_version: observation.contract_version,
        state: match observation.state {
            CatalogState::Loaded => "loaded",
            CatalogState::RebuiltPersisted => "rebuilt_persisted",
            CatalogState::IncrementalPersisted => "incremental_persisted",
        }
        .to_owned(),
        generation: observation.generation,
        commitment: observation.commitment.to_string(),
        entries: observation.entries,
        packs: observation.packs,
        segments: observation.segments as u64,
        segment_bytes: observation.segment_bytes,
        segment_metadata_bytes: observation.segment_metadata_bytes,
        maximum_level: observation.maximum_level,
        maximum_live_segments: observation.maximum_live_segments as u64,
        maximum_lookup_segments: observation.maximum_lookup_segments as u64,
        block_entries: observation.block_entries as u64,
        history: catalog_history_inventory(observation.history),
        work: catalog_work_inventory(observation.work),
        leftovers: observation.leftovers,
        footer_oracle_packs: footer.packs,
        footer_oracle_entries: footer.entries,
        footer_oracle_bytes_read: footer.footer_bytes_read,
        footer_oracle_duplicate_objects: footer.duplicate_objects,
        footer_oracle_commitment: footer.oracle_commitment.to_string(),
        footer_oracle_equal: footer.equal && footer.oracle_commitment == footer.manifest_commitment,
    })
}

fn catalog_history_inventory(value: CatalogHistory) -> CatalogHistoryInventory {
    CatalogHistoryInventory {
        delta_segments: value.delta_segments,
        merge_operations: value.merge_operations,
        merge_entries_read: value.merge_entries_read,
        merge_bytes_read: value.merge_bytes_read,
        segments_written: value.segments_written,
        segment_entries_written: value.segment_entries_written,
        full_rebuilds: value.full_rebuilds,
        full_footer_scan_runs: value.full_footer_scan_runs,
        pack_footers_scanned: value.pack_footers_scanned,
    }
}

fn catalog_work_inventory(value: CatalogWork) -> CatalogWorkInventory {
    CatalogWorkInventory {
        healthy_opens: value.healthy_opens,
        manifests_read: value.manifests_read,
        manifest_bytes_read: value.manifest_bytes_read,
        segment_metadata_read: value.segment_metadata_read,
        segment_metadata_bytes_read: value.segment_metadata_bytes_read,
        segment_lookups: value.segment_lookups,
        segment_blocks_read: value.segment_blocks_read,
        segment_block_bytes_read: value.segment_block_bytes_read,
        segment_entries_examined: value.segment_entries_examined,
        targeted_pack_footers_read: value.targeted_pack_footers_read,
        targeted_pack_footer_bytes_read: value.targeted_pack_footer_bytes_read,
        delta_segments_written: value.delta_segments_written,
        merge_operations: value.merge_operations,
        merge_entries_read: value.merge_entries_read,
        merge_bytes_read: value.merge_bytes_read,
        segments_written: value.segments_written,
        segment_entries_written: value.segment_entries_written,
        manifests_written: value.manifests_written,
        obsolete_segments_removed: value.obsolete_segments_removed,
        full_rebuilds: value.full_rebuilds,
        full_footer_scan_runs: value.full_footer_scan_runs,
        pack_footers_scanned: value.pack_footers_scanned,
    }
}

fn catalog_store_diagnostic(error: StoreError) -> Diagnostic {
    let class = match error.class {
        StoreErrorClass::Input => DiagnosticClass::Source,
        StoreErrorClass::Resource => DiagnosticClass::Resource,
        StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
        StoreErrorClass::Io => DiagnosticClass::Infrastructure,
    };
    Diagnostic::new(class, error.code, error.message)
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

/// Derive an extraction contract from complete accepted authority without using production
/// extraction planning.
pub fn function_extraction_oracle(
    project: &Path,
    function: &str,
    expression: &str,
) -> Result<FunctionExtractionOracle, Diagnostic> {
    let function = DeclarationId::parse(function)?;
    let expression = ExpressionId::parse(expression)?;
    let repository = GraphRepository::open(project)?;
    let view = repository.view_current()?;
    let read = view.reconstruct_full_oracle()?;
    let witness = rebuild_full_witness(&read.value).map_err(first_oracle_diagnostic)?;
    reconstruct_function_extraction(&read.value, &witness, view.revision(), function, expression)
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

#[derive(Clone)]
struct ExtractionOracleLocalUse {
    expression: ExpressionId,
    value: LocalValueReference,
    ordinal: usize,
    selected: bool,
}

#[derive(Clone)]
struct ExtractionOracleCapture {
    source: LocalValueReference,
    owner: OwnerKey,
    name: Name,
    ty: TypeObjectDigest,
    first_use: usize,
    uses: Vec<ExpressionId>,
    requirement: Option<RequirementReference>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ExtractionOracleResource {
    None,
    Direct(DeclarationReference),
    Contained,
}

fn reconstruct_function_extraction(
    snapshot: &KernelSnapshot,
    witness: &FullWitness,
    revision: super::semantic_id::RevisionId,
    function: DeclarationId,
    selected: ExpressionId,
) -> Result<FunctionExtractionOracle, Diagnostic> {
    let definition = reconstruct_function_definition(snapshot, witness, revision, function)?;
    let OwnerRecord::Declaration(declaration) = snapshot
        .owners
        .get(&OwnerKey::Declaration(function))
        .ok_or_else(|| {
            oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_extraction_function",
                "extraction oracle target disappeared from complete authority",
            )
        })?
    else {
        return Err(oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_extraction_function",
            "extraction oracle target is not a declaration",
        ));
    };
    let DeclarationPayload::Function(function_record) = &declaration.payload else {
        return Err(oracle_error(
            DiagnosticClass::Semantic,
            "contributor_extraction_function",
            "extraction oracle target is not a local function",
        ));
    };
    if !function_record.type_parameters.is_empty() {
        return Err(oracle_error(
            DiagnosticClass::Semantic,
            "contributor_extraction_generic",
            "extraction oracle does not admit a generic target",
        ));
    }
    let selected_text = selected.to_string();
    let selected_index = definition
        .body_preorder
        .iter()
        .position(|owner| owner.owner == selected_text)
        .ok_or_else(|| {
            oracle_error(
                DiagnosticClass::Semantic,
                "contributor_extraction_expression",
                "selected expression is outside the independently reconstructed function body",
            )
        })?;
    let selected_depth = definition.body_preorder[selected_index].depth;
    if selected_index == 0 || selected_depth == 0 {
        return Err(oracle_error(
            DiagnosticClass::Semantic,
            "contributor_extraction_whole_body",
            "selected expression is not a proper subtree",
        ));
    }
    oracle_require_acyclic_call_graph(
        snapshot,
        witness,
        revision,
        function,
        &mut BTreeSet::new(),
        &mut BTreeSet::new(),
        &mut 0_u64,
    )?;
    let selected_end = definition.body_preorder[selected_index + 1..]
        .iter()
        .position(|owner| owner.depth <= selected_depth)
        .map_or(definition.body_preorder.len(), |offset| {
            selected_index + 1 + offset
        });
    let body_owners = definition
        .body_preorder
        .iter()
        .map(|owner| owner.owner.parse::<OwnerKey>())
        .collect::<Result<Vec<_>, _>>()?;
    let selected_preorder = &body_owners[selected_index..selected_end];
    let selected_set = selected_preorder.iter().copied().collect::<BTreeSet<_>>();
    let defined_bindings = selected_set
        .iter()
        .filter(|owner| matches!(owner, OwnerKey::Binding(_)))
        .copied()
        .collect::<BTreeSet<_>>();
    let mut local_uses = Vec::new();
    for (ordinal, owner) in body_owners.iter().copied().enumerate() {
        let OwnerKey::Expression(expression) = owner else {
            continue;
        };
        let Some(OwnerRecord::Expression(record)) = snapshot.owners.get(&owner) else {
            return Err(oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_extraction_expression",
                "independent body preorder names a missing expression record",
            ));
        };
        if let ExpressionOperation::Local { value } = record.operation {
            local_uses.push(ExtractionOracleLocalUse {
                expression,
                value,
                ordinal,
                selected: ordinal >= selected_index && ordinal < selected_end,
            });
        }
    }
    if local_uses.iter().any(|local_use| {
        !local_use.selected && defined_bindings.contains(&extraction_local_owner(local_use.value))
    }) {
        return Err(oracle_error(
            DiagnosticClass::Semantic,
            "contributor_extraction_binding_escape",
            "a binding defined in the selected subtree escapes its boundary",
        ));
    }

    let mut grouped_uses = BTreeMap::<LocalValueReference, Vec<&ExtractionOracleLocalUse>>::new();
    for local_use in local_uses.iter().filter(|local_use| local_use.selected) {
        let owner = extraction_local_owner(local_use.value);
        if !selected_set.contains(&owner) {
            grouped_uses
                .entry(local_use.value)
                .or_default()
                .push(local_use);
        }
    }
    let function_parameters = function_record
        .parameters
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut captures = Vec::new();
    for (source, uses) in grouped_uses {
        let owner = extraction_local_owner(source);
        let (name, ty, parameter) = match source {
            LocalValueReference::FunctionParameter(parameter)
                if function_parameters.contains(&parameter) =>
            {
                let Some(OwnerRecord::Parameter(record)) =
                    snapshot.owners.get(&OwnerKey::Parameter(parameter))
                else {
                    return Err(oracle_error(
                        DiagnosticClass::Corrupt,
                        "contributor_extraction_capture",
                        "captured function parameter is missing",
                    ));
                };
                if record.parent != ParameterParent::Function(function) {
                    return Err(oracle_error(
                        DiagnosticClass::Corrupt,
                        "contributor_extraction_capture",
                        "captured function parameter belongs to another declaration",
                    ));
                }
                (record.name.clone(), record.ty, Some(record))
            }
            LocalValueReference::LexicalBinding(binding)
            | LocalValueReference::MatchPayload(binding) => {
                let Some(OwnerRecord::Binding(record)) =
                    snapshot.owners.get(&OwnerKey::Binding(binding))
                else {
                    return Err(oracle_error(
                        DiagnosticClass::Semantic,
                        "contributor_extraction_capture",
                        "free local binding is outside the target function",
                    ));
                };
                let expected = match source {
                    LocalValueReference::LexicalBinding(_) => BindingKind::Let,
                    LocalValueReference::MatchPayload(_) => BindingKind::MatchPayload,
                    _ => BindingKind::Transaction,
                };
                if record.kind != expected || !body_owners.contains(&owner) {
                    return Err(oracle_error(
                        DiagnosticClass::Semantic,
                        "contributor_extraction_capture",
                        "free local binding kind or function ownership disagrees",
                    ));
                }
                let ty = match (record.declared_type, record.value) {
                    (Some(ty), _) => ty,
                    (None, Some(value)) => oracle_infer_expression_type(
                        snapshot,
                        function,
                        value,
                        &function_record.effect,
                    )?,
                    (None, None) => {
                        return Err(oracle_error(
                            DiagnosticClass::Semantic,
                            "contributor_extraction_capture_type",
                            "free local binding has no declared or inferable exact type",
                        ));
                    }
                };
                (record.name.clone(), ty, None)
            }
            LocalValueReference::TransactionBinding(_) => {
                return Err(oracle_error(
                    DiagnosticClass::Semantic,
                    "contributor_extraction_transaction",
                    "transaction binding cannot cross an extracted function boundary",
                ));
            }
            LocalValueReference::OperationParameter(_)
            | LocalValueReference::FunctionParameter(_) => {
                return Err(oracle_error(
                    DiagnosticClass::Semantic,
                    "contributor_extraction_capture",
                    "free local reference is not owned by the target function",
                ));
            }
        };
        if oracle_type_contains_parameter(snapshot, ty, &mut BTreeSet::new())? {
            return Err(oracle_error(
                DiagnosticClass::Semantic,
                "contributor_extraction_free_type",
                "capture contains an unsupported free type parameter",
            ));
        }
        let requirement = match oracle_resource_class(
            snapshot,
            ty,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
        )? {
            ExtractionOracleResource::None => None,
            ExtractionOracleResource::Contained => {
                return Err(oracle_error(
                    DiagnosticClass::Semantic,
                    "contributor_extraction_resource_container",
                    "capture contains a resource in an unsupported shape",
                ));
            }
            ExtractionOracleResource::Direct(interface) => {
                if uses.len() != 1 {
                    return Err(oracle_error(
                        DiagnosticClass::Semantic,
                        "contributor_extraction_resource_use",
                        "free capability resource must have exactly one selected use",
                    ));
                }
                if local_uses.iter().any(|local_use| {
                    !local_use.selected
                        && local_use.value == source
                        && local_use.ordinal > selected_index
                }) {
                    return Err(oracle_error(
                        DiagnosticClass::Semantic,
                        "contributor_extraction_resource_post_use",
                        "free capability resource has a later caller use",
                    ));
                }
                let requirement = oracle_capture_requirement(
                    snapshot,
                    source,
                    parameter,
                    function_record,
                    interface,
                )?;
                if oracle_requirement(snapshot, requirement)?.interface != interface {
                    return Err(oracle_error(
                        DiagnosticClass::Semantic,
                        "contributor_extraction_resource_requirement",
                        "resource interface disagrees with its acquiring requirement",
                    ));
                }
                Some(requirement)
            }
        };
        captures.push(ExtractionOracleCapture {
            source,
            owner,
            name,
            ty,
            first_use: uses[0].ordinal,
            uses: uses.iter().map(|local_use| local_use.expression).collect(),
            requirement,
        });
    }
    if captures
        .iter()
        .filter(|capture| capture.requirement.is_some())
        .count()
        > 1
    {
        return Err(oracle_error(
            DiagnosticClass::Semantic,
            "contributor_extraction_multiple_resources",
            "selected subtree has more than one free capability resource",
        ));
    }
    captures.sort_by(|left, right| {
        left.first_use
            .cmp(&right.first_use)
            .then_with(|| EncodedOwnerKey::new(left.owner).cmp(&EncodedOwnerKey::new(right.owner)))
    });
    let affine_index = captures
        .iter()
        .position(|capture| capture.requirement.is_some());
    if let Some(index) = affine_index {
        let capture = captures.remove(index);
        captures.push(capture);
    }
    oracle_assign_capture_names(&mut captures)?;

    let result =
        oracle_infer_expression_type(snapshot, function, selected, &function_record.effect)?;
    if oracle_type_contains_parameter(snapshot, result, &mut BTreeSet::new())?
        || oracle_resource_class(snapshot, result, &mut BTreeSet::new(), &mut BTreeSet::new())?
            != ExtractionOracleResource::None
    {
        return Err(oracle_error(
            DiagnosticClass::Semantic,
            "contributor_extraction_result",
            "selected expression result is generic or resource-containing",
        ));
    }
    let mut required = BTreeSet::new();
    for owner in selected_preorder {
        let OwnerKey::Expression(_) = owner else {
            continue;
        };
        let Some(OwnerRecord::Expression(record)) = snapshot.owners.get(owner) else {
            return Err(oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_extraction_effect",
                "selected expression disappeared during independent effect analysis",
            ));
        };
        match &record.operation {
            ExpressionOperation::CapabilityCall { requirement, .. }
            | ExpressionOperation::Transaction { requirement, .. } => {
                required.insert(*requirement);
            }
            ExpressionOperation::Call {
                function: called, ..
            } => {
                if let FunctionEffect::Task { requirements } =
                    oracle_function_effect(snapshot, *called)?
                {
                    required.extend(requirements);
                }
            }
            ExpressionOperation::FunctionValue { .. } | ExpressionOperation::Invoke { .. } => {
                return Err(oracle_error(
                    DiagnosticClass::Semantic,
                    "contributor_extraction_closure",
                    "function values and indirect invocation are outside extraction",
                ));
            }
            _ => {}
        }
    }
    required.extend(captures.iter().filter_map(|capture| capture.requirement));
    let requirements = match &function_record.effect {
        FunctionEffect::Pure if required.is_empty() => Vec::new(),
        FunctionEffect::Pure => {
            return Err(oracle_error(
                DiagnosticClass::Semantic,
                "contributor_extraction_requirement",
                "pure caller cannot supply selected task requirements",
            ));
        }
        FunctionEffect::Task { requirements } => {
            if required
                .iter()
                .any(|requirement| !requirements.contains(requirement))
            {
                return Err(oracle_error(
                    DiagnosticClass::Semantic,
                    "contributor_extraction_requirement",
                    "selected subtree requires a task requirement absent from its caller",
                ));
            }
            requirements
                .iter()
                .copied()
                .filter(|requirement| required.contains(requirement))
                .collect()
        }
    };
    let effect = if requirements.is_empty() {
        "pure"
    } else {
        "task"
    };
    let mut moved = selected_preorder.to_vec();
    moved.sort_unstable_by_key(|owner| EncodedOwnerKey::new(*owner));
    let moved_set = moved.iter().copied().collect::<BTreeSet<_>>();
    let parent = definition.body_preorder[selected_index]
        .parent
        .parse::<OwnerKey>()?;
    let mut changed = captures
        .iter()
        .flat_map(|capture| capture.uses.iter().copied())
        .map(OwnerKey::Expression)
        .collect::<BTreeSet<_>>();
    changed.insert(parent);
    let preserved = moved_set.difference(&changed).copied().collect::<Vec<_>>();
    let mut changed = changed.into_iter().collect::<Vec<_>>();
    changed.sort_unstable_by_key(|owner| EncodedOwnerKey::new(*owner));
    let mut preserved = preserved;
    preserved.sort_unstable_by_key(|owner| EncodedOwnerKey::new(*owner));
    let moved_digest = oracle_moved_digest(snapshot, &moved)?;
    let base_body_records = u64::try_from(body_owners.len()).unwrap_or(u64::MAX);
    let helper_body_records = u64::try_from(moved.len()).unwrap_or(u64::MAX);
    let generated_owners = u64::try_from(captures.len())
        .unwrap_or(u64::MAX)
        .saturating_mul(2)
        .saturating_add(2);
    let caller_body_records = base_body_records
        .checked_sub(helper_body_records)
        .and_then(|count| count.checked_add(u64::try_from(captures.len()).unwrap_or(u64::MAX) + 1))
        .ok_or_else(|| {
            oracle_error(
                DiagnosticClass::Resource,
                "contributor_extraction_body_count",
                "post-extraction body count overflowed",
            )
        })?;
    Ok(FunctionExtractionOracle {
        repository: snapshot.root.repository_id.to_string(),
        package: snapshot.root.package_id.to_string(),
        revision: revision.to_string(),
        function: function.to_string(),
        selected_root: selected.to_string(),
        parent: parent.to_string(),
        base_body_records,
        caller_body_records,
        helper_body_records,
        moved_digest: format!("moved_{}", encode_hex(&moved_digest)),
        moved_owners: moved.iter().map(ToString::to_string).collect(),
        preserved_owners: preserved.iter().map(ToString::to_string).collect(),
        changed_owners: changed.iter().map(ToString::to_string).collect(),
        captures: captures
            .iter()
            .map(|capture| FunctionExtractionOracleCapture {
                source_kind: oracle_local_reference_kind(capture.source).to_owned(),
                source: capture.owner.to_string(),
                name: capture.name.to_string(),
                ty: capture.ty.to_string(),
                use_mode: if capture.requirement.is_some() {
                    "consume"
                } else {
                    "unrestricted"
                }
                .to_owned(),
                requirement: capture.requirement.map(oracle_requirement_reference),
                uses: capture.uses.iter().map(ToString::to_string).collect(),
            })
            .collect(),
        result_type: result.to_string(),
        effect: effect.to_owned(),
        requirements: requirements
            .iter()
            .copied()
            .map(oracle_requirement_reference)
            .collect(),
        affine_requirement: captures
            .last()
            .and_then(|capture| capture.requirement)
            .map(oracle_requirement_reference),
        generated_owners,
    })
}

fn oracle_require_acyclic_call_graph(
    snapshot: &KernelSnapshot,
    witness: &FullWitness,
    revision: super::semantic_id::RevisionId,
    function: DeclarationId,
    visiting: &mut BTreeSet<DeclarationId>,
    complete: &mut BTreeSet<DeclarationId>,
    work: &mut u64,
) -> Result<(), Diagnostic> {
    if complete.contains(&function) {
        return Ok(());
    }
    if !visiting.insert(function) {
        return Err(oracle_error(
            DiagnosticClass::Semantic,
            "contributor_extraction_recursive",
            "extraction oracle does not admit a target with a recursive local call cycle",
        ));
    }
    let declaration = match snapshot.owners.get(&OwnerKey::Declaration(function)) {
        Some(OwnerRecord::Declaration(declaration)) => declaration,
        _ => {
            return Err(oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_extraction_call_graph",
                "local call-graph declaration is missing or bound to another owner kind",
            ));
        }
    };
    if matches!(&declaration.payload, DeclarationPayload::External(_)) {
        visiting.remove(&function);
        complete.insert(function);
        return Ok(());
    }
    if !matches!(&declaration.payload, DeclarationPayload::Function(_)) {
        return Err(oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_extraction_call_graph",
            "local direct call names a declaration that is not callable",
        ));
    }
    let definition = reconstruct_function_definition(snapshot, witness, revision, function)?;
    *work = work
        .checked_add(u64::try_from(definition.body_preorder.len()).unwrap_or(u64::MAX))
        .ok_or_else(|| {
            oracle_error(
                DiagnosticClass::Resource,
                "contributor_extraction_call_graph_work",
                "independent local call-graph work overflowed",
            )
        })?;
    if *work > u64::try_from(super::kernel::contract::MAXIMUM_VALIDATION_WORK).unwrap_or(u64::MAX) {
        return Err(oracle_error(
            DiagnosticClass::Resource,
            "contributor_extraction_call_graph_work",
            "independent local call-graph work exceeded its finite validation boundary",
        ));
    }
    let mut called = BTreeSet::new();
    for owner in &definition.body_preorder {
        let owner = owner.owner.parse::<OwnerKey>()?;
        let Some(OwnerRecord::Expression(expression)) = snapshot.owners.get(&owner) else {
            continue;
        };
        if let ExpressionOperation::Call { function, .. } = &expression.operation
            && function.package == snapshot.root.package_id
        {
            called.insert(function.declaration);
        }
    }
    for called in called {
        oracle_require_acyclic_call_graph(
            snapshot, witness, revision, called, visiting, complete, work,
        )?;
    }
    visiting.remove(&function);
    complete.insert(function);
    Ok(())
}

fn oracle_infer_expression_type(
    snapshot: &KernelSnapshot,
    function: DeclarationId,
    expression: ExpressionId,
    effect: &FunctionEffect,
) -> Result<TypeObjectDigest, Diagnostic> {
    let mut work = 0_usize;
    infer_function_expression_type(
        snapshot,
        function,
        expression,
        effect,
        &mut work,
        super::kernel::contract::MAXIMUM_VALIDATION_WORK,
    )
    .map_err(|diagnostic| {
        oracle_error(
            diagnostic.class,
            "contributor_extraction_type",
            format!(
                "independent extraction type inference failed: {}",
                diagnostic.message
            ),
        )
    })
}

fn extraction_local_owner(value: LocalValueReference) -> OwnerKey {
    match value {
        LocalValueReference::FunctionParameter(parameter)
        | LocalValueReference::OperationParameter(parameter) => OwnerKey::Parameter(parameter),
        LocalValueReference::LexicalBinding(binding)
        | LocalValueReference::MatchPayload(binding)
        | LocalValueReference::TransactionBinding(binding) => OwnerKey::Binding(binding),
    }
}

fn oracle_local_reference_kind(value: LocalValueReference) -> &'static str {
    match value {
        LocalValueReference::FunctionParameter(_) => "function-parameter",
        LocalValueReference::OperationParameter(_) => "operation-parameter",
        LocalValueReference::LexicalBinding(_) => "lexical-binding",
        LocalValueReference::MatchPayload(_) => "match-payload",
        LocalValueReference::TransactionBinding(_) => "transaction-binding",
    }
}

fn oracle_assign_capture_names(captures: &mut [ExtractionOracleCapture]) -> Result<(), Diagnostic> {
    let mut names = BTreeSet::new();
    for capture in captures {
        if names.insert(capture.name.clone()) {
            continue;
        }
        let suffix = encode_hex(&EncodedOwnerKey::new(capture.owner).bytes());
        let prefix_length = capture.name.as_str().len().min(
            super::kernel::contract::MAXIMUM_NAME_BYTES
                .saturating_sub(suffix.len().saturating_add(1)),
        );
        let prefix = &capture.name.as_str()[..prefix_length];
        let resolved = Name::new(format!("{prefix}-{suffix}"))?;
        if !names.insert(resolved.clone()) {
            return Err(oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_extraction_capture_name",
                "identity-derived capture name is not unique",
            ));
        }
        capture.name = resolved;
    }
    Ok(())
}

fn oracle_moved_digest(
    snapshot: &KernelSnapshot,
    owners: &[OwnerKey],
) -> Result<[u8; 32], Diagnostic> {
    let mut hasher = Hasher::new_derive_key(EXTRACTION_MOVED_DIGEST_DOMAIN);
    hasher.update(
        &u64::try_from(owners.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for owner in owners {
        let record = snapshot.owners.get(owner).ok_or_else(|| {
            oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_extraction_moved_digest",
                "moved owner disappeared during independent digest derivation",
            )
        })?;
        let (_, bytes) = encode_owner(record)?;
        hasher.update(&EncodedOwnerKey::new(*owner).bytes());
        hasher.update(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
        hasher.update(&bytes);
    }
    Ok(*hasher.finalize().as_bytes())
}

fn oracle_requirement_reference(requirement: RequirementReference) -> String {
    format!("{}/{}", requirement.package, requirement.requirement)
}

fn oracle_type_object(
    snapshot: &KernelSnapshot,
    digest: TypeObjectDigest,
) -> Result<&super::kernel::TypeObject, Diagnostic> {
    snapshot
        .types
        .get(&digest)
        .or_else(|| snapshot.dependency_types.get(&digest))
        .ok_or_else(|| {
            oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_extraction_type_missing",
                format!("type object '{digest}' is missing from complete authority"),
            )
        })
}

fn oracle_type_contains_parameter(
    snapshot: &KernelSnapshot,
    digest: TypeObjectDigest,
    active: &mut BTreeSet<TypeObjectDigest>,
) -> Result<bool, Diagnostic> {
    if !active.insert(digest) {
        return Ok(false);
    }
    let object = oracle_type_object(snapshot, digest)?;
    let mut contains = matches!(object.form, TypeForm::TypeParameter { .. });
    if !contains {
        for child in object.child_types() {
            if oracle_type_contains_parameter(snapshot, child, active)? {
                contains = true;
                break;
            }
        }
    }
    active.remove(&digest);
    Ok(contains)
}

fn oracle_resource_class(
    snapshot: &KernelSnapshot,
    digest: TypeObjectDigest,
    active_types: &mut BTreeSet<TypeObjectDigest>,
    active_declarations: &mut BTreeSet<(super::kernel::PackageId, DeclarationId)>,
) -> Result<ExtractionOracleResource, Diagnostic> {
    if !active_types.insert(digest) {
        return Ok(ExtractionOracleResource::None);
    }
    let object = oracle_type_object(snapshot, digest)?;
    let class = match object.form {
        TypeForm::CapabilityResource { interface } => ExtractionOracleResource::Direct(interface),
        TypeForm::Named { declaration } => {
            let key = (declaration.package, declaration.declaration);
            if !active_declarations.insert(key) {
                ExtractionOracleResource::None
            } else {
                let mut contained = false;
                for member in oracle_named_member_types(snapshot, declaration)? {
                    if oracle_resource_class(snapshot, member, active_types, active_declarations)?
                        != ExtractionOracleResource::None
                    {
                        contained = true;
                        break;
                    }
                }
                active_declarations.remove(&key);
                if contained {
                    ExtractionOracleResource::Contained
                } else {
                    ExtractionOracleResource::None
                }
            }
        }
        _ => {
            let mut contained = false;
            for child in object.child_types() {
                if oracle_resource_class(snapshot, child, active_types, active_declarations)?
                    != ExtractionOracleResource::None
                {
                    contained = true;
                    break;
                }
            }
            if contained {
                ExtractionOracleResource::Contained
            } else {
                ExtractionOracleResource::None
            }
        }
    };
    active_types.remove(&digest);
    Ok(class)
}

fn oracle_named_member_types(
    snapshot: &KernelSnapshot,
    reference: DeclarationReference,
) -> Result<Vec<TypeObjectDigest>, Diagnostic> {
    if reference.package == snapshot.root.package_id {
        let Some(OwnerRecord::Declaration(declaration)) = snapshot
            .owners
            .get(&OwnerKey::Declaration(reference.declaration))
        else {
            return Err(oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_extraction_named_type",
                "local named type declaration is missing",
            ));
        };
        return match &declaration.payload {
            DeclarationPayload::Record { fields } => fields
                .iter()
                .map(
                    |field| match snapshot.owners.get(&OwnerKey::Field(*field)) {
                        Some(OwnerRecord::Field(record)) => Ok(record.ty),
                        _ => Err(oracle_error(
                            DiagnosticClass::Corrupt,
                            "contributor_extraction_named_member",
                            "local named record field is missing",
                        )),
                    },
                )
                .collect(),
            DeclarationPayload::Variant { cases } => cases
                .iter()
                .map(|case| match snapshot.owners.get(&OwnerKey::Case(*case)) {
                    Some(OwnerRecord::Case(record)) => Ok(record.payload),
                    _ => Err(oracle_error(
                        DiagnosticClass::Corrupt,
                        "contributor_extraction_named_member",
                        "local named variant case is missing",
                    )),
                })
                .collect::<Result<Vec<_>, _>>()
                .map(|values| values.into_iter().flatten().collect()),
            _ => Ok(Vec::new()),
        };
    }
    let record = oracle_dependency_owner(
        snapshot,
        reference.package,
        OwnerKey::Declaration(reference.declaration),
    )?;
    match record {
        PackageInterfaceRecord::Declaration(declaration) => match declaration.payload {
            PackageInterfaceDeclarationPayload::Record { fields } => fields
                .into_iter()
                .map(|field| {
                    match oracle_dependency_owner(
                        snapshot,
                        reference.package,
                        OwnerKey::Field(field),
                    )? {
                        PackageInterfaceRecord::Field(record) => Ok(record.ty),
                        _ => Err(oracle_error(
                            DiagnosticClass::Corrupt,
                            "contributor_extraction_named_member",
                            "dependency named record field is missing",
                        )),
                    }
                })
                .collect(),
            PackageInterfaceDeclarationPayload::Variant { cases } => cases
                .into_iter()
                .map(|case| {
                    match oracle_dependency_owner(
                        snapshot,
                        reference.package,
                        OwnerKey::Case(case),
                    )? {
                        PackageInterfaceRecord::Case(record) => Ok(record.payload),
                        _ => Err(oracle_error(
                            DiagnosticClass::Corrupt,
                            "contributor_extraction_named_member",
                            "dependency named variant case is missing",
                        )),
                    }
                })
                .collect::<Result<Vec<_>, _>>()
                .map(|values| values.into_iter().flatten().collect()),
            _ => Ok(Vec::new()),
        },
        _ => Err(oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_extraction_named_type",
            "dependency named type declaration is missing",
        )),
    }
}

fn oracle_dependency_owner(
    snapshot: &KernelSnapshot,
    package: super::kernel::PackageId,
    owner: OwnerKey,
) -> Result<PackageInterfaceRecord, Diagnostic> {
    let dependency = snapshot.dependencies.get(&package).ok_or_else(|| {
        oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_extraction_dependency",
            format!("package '{package}' is not an exact dependency"),
        )
    })?;
    snapshot
        .dependency_interfaces
        .get(&dependency.package_revision)
        .and_then(|interface| interface.get(&owner))
        .cloned()
        .ok_or_else(|| {
            oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_extraction_dependency_owner",
                format!("dependency owner '{package}/{owner}' is missing"),
            )
        })
}

fn oracle_capture_requirement(
    snapshot: &KernelSnapshot,
    source: LocalValueReference,
    parameter: Option<&super::kernel::ParameterRecord>,
    function: &super::kernel::FunctionDeclaration,
    interface: DeclarationReference,
) -> Result<RequirementReference, Diagnostic> {
    if let Some(parameter) = parameter {
        if parameter.use_mode != ParameterUse::Consume {
            return Err(oracle_error(
                DiagnosticClass::Semantic,
                "contributor_extraction_resource_source",
                "captured capability parameter is not consume-only",
            ));
        }
        return parameter.resource_requirement.ok_or_else(|| {
            oracle_error(
                DiagnosticClass::Semantic,
                "contributor_extraction_resource_source",
                "captured capability parameter lacks an exact requirement",
            )
        });
    }
    let binding = match source {
        LocalValueReference::LexicalBinding(binding)
        | LocalValueReference::MatchPayload(binding) => binding,
        _ => {
            return Err(oracle_error(
                DiagnosticClass::Semantic,
                "contributor_extraction_resource_source",
                "captured capability binding has unsupported provenance",
            ));
        }
    };
    let Some(OwnerRecord::Binding(binding)) = snapshot.owners.get(&OwnerKey::Binding(binding))
    else {
        return Err(oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_extraction_resource_source",
            "captured capability binding is missing",
        ));
    };
    if matches!(source, LocalValueReference::MatchPayload(_)) {
        let FunctionEffect::Task { requirements } = &function.effect else {
            return Err(oracle_error(
                DiagnosticClass::Semantic,
                "contributor_extraction_resource_source",
                "captured match payload has no task requirement provenance",
            ));
        };
        let mut candidates = Vec::new();
        for requirement in requirements {
            if oracle_requirement(snapshot, *requirement)?.interface == interface {
                candidates.push(*requirement);
            }
        }
        return match candidates.as_slice() {
            [requirement] => Ok(*requirement),
            [] => Err(oracle_error(
                DiagnosticClass::Semantic,
                "contributor_extraction_resource_source",
                "captured match payload has no exact caller requirement for its resource interface",
            )),
            _ => Err(oracle_error(
                DiagnosticClass::Semantic,
                "contributor_extraction_resource_ambiguity",
                "captured match payload has more than one caller requirement for its resource interface",
            )),
        };
    }
    let Some(value) = binding.value else {
        return Err(oracle_error(
            DiagnosticClass::Semantic,
            "contributor_extraction_resource_source",
            "captured capability binding has no acquiring value",
        ));
    };
    match snapshot.owners.get(&OwnerKey::Expression(value)) {
        Some(OwnerRecord::Expression(record)) => match record.operation {
            ExpressionOperation::CapabilityCall { requirement, .. } => Ok(requirement),
            _ => Err(oracle_error(
                DiagnosticClass::Semantic,
                "contributor_extraction_resource_source",
                "captured capability binding is not acquired by one capability call",
            )),
        },
        _ => Err(oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_extraction_resource_source",
            "captured capability acquiring expression is missing",
        )),
    }
}

fn oracle_requirement(
    snapshot: &KernelSnapshot,
    reference: RequirementReference,
) -> Result<super::kernel::RequirementRecord, Diagnostic> {
    if reference.package == snapshot.root.package_id {
        return match snapshot
            .owners
            .get(&OwnerKey::Requirement(reference.requirement))
        {
            Some(OwnerRecord::Requirement(record)) => Ok(record.clone()),
            _ => Err(oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_extraction_requirement",
                "local requirement record is missing",
            )),
        };
    }
    match oracle_dependency_owner(
        snapshot,
        reference.package,
        OwnerKey::Requirement(reference.requirement),
    )? {
        PackageInterfaceRecord::Requirement(record) => Ok(record),
        _ => Err(oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_extraction_requirement",
            "dependency requirement record is missing",
        )),
    }
}

fn oracle_function_effect(
    snapshot: &KernelSnapshot,
    reference: DeclarationReference,
) -> Result<FunctionEffect, Diagnostic> {
    if reference.package == snapshot.root.package_id {
        return match snapshot
            .owners
            .get(&OwnerKey::Declaration(reference.declaration))
        {
            Some(OwnerRecord::Declaration(declaration)) => match &declaration.payload {
                DeclarationPayload::Function(function) => Ok(function.effect.clone()),
                DeclarationPayload::External(_) => Ok(FunctionEffect::Pure),
                _ => Err(oracle_error(
                    DiagnosticClass::Semantic,
                    "contributor_extraction_call",
                    "local direct call names a non-callable declaration",
                )),
            },
            _ => Err(oracle_error(
                DiagnosticClass::Corrupt,
                "contributor_extraction_call",
                "local direct-call declaration is missing",
            )),
        };
    }
    match oracle_dependency_owner(
        snapshot,
        reference.package,
        OwnerKey::Declaration(reference.declaration),
    )? {
        PackageInterfaceRecord::Declaration(declaration) => match declaration.payload {
            PackageInterfaceDeclarationPayload::Function(function) => Ok(function.effect),
            PackageInterfaceDeclarationPayload::External(_) => Ok(FunctionEffect::Pure),
            _ => Err(oracle_error(
                DiagnosticClass::Semantic,
                "contributor_extraction_call",
                "dependency direct call names a non-callable declaration",
            )),
        },
        _ => Err(oracle_error(
            DiagnosticClass::Corrupt,
            "contributor_extraction_call",
            "dependency direct-call declaration is missing",
        )),
    }
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
        GraphRepository, catalog_inventory, compact_change_default_maximum_operations,
        function_definition_oracle, function_extraction_oracle, largest_function_definition_oracle,
        semantic_inventory,
    };
    use crate::platform::kernel::{DeclarationPayload, ExpressionOperation, OwnerKey, OwnerRecord};
    use std::path::Path;

    #[test]
    fn maintained_standard_inventory_is_typed_and_read_only() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard");
        let before = std::fs::read(project.join("HEAD")).expect("standard HEAD before oracle");
        let inventory = semantic_inventory(&project).expect("standard semantic inventory");
        assert_eq!(inventory.owners, 550);
        assert_eq!(inventory.modules, 13);
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
    fn catalog_inventory_binds_incremental_work_and_independent_footer_oracle() {
        let temporary = tempfile::tempdir().expect("temporary catalog inventory parent");
        let project = temporary.path().join("meaning");
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let created = GraphRepository::create(&project, &snapshot, None)
            .expect("create catalog inventory repository");
        let before = std::fs::read(project.join("HEAD")).expect("HEAD before catalog inventory");
        let inventory = catalog_inventory(&project).expect("catalog inventory");
        assert_eq!(inventory.identity, "lkjscript-object-catalog-2");
        assert_eq!(inventory.contract_version, 2);
        assert_eq!(inventory.state, "loaded");
        assert!(inventory.entries > 0);
        assert!(inventory.segments <= inventory.maximum_live_segments);
        assert_eq!(inventory.maximum_lookup_segments, inventory.segments);
        assert_eq!(inventory.history.full_rebuilds, 0);
        assert_eq!(inventory.work.full_rebuilds, 0);
        assert!(inventory.footer_oracle_equal);
        assert_eq!(inventory.footer_oracle_commitment, inventory.commitment);
        assert_eq!(
            std::fs::read(project.join("HEAD")).expect("HEAD after catalog inventory"),
            before
        );
        assert_eq!(
            created.current.head.revision.to_string(),
            semantic_inventory(&project).unwrap().revision
        );
    }

    #[test]
    fn function_extraction_oracle_derives_a_proper_capture_boundary_read_only() {
        let temporary = tempfile::tempdir().expect("temporary extraction oracle repository");
        let project = temporary.path().join("meaning");
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let created = GraphRepository::create(&project, &snapshot, None)
            .expect("create extraction oracle repository");
        let (function, selected, body_root) = snapshot
            .owners
            .iter()
            .find_map(|(owner, record)| {
                let OwnerKey::Declaration(function) = owner else {
                    return None;
                };
                let OwnerRecord::Declaration(declaration) = record else {
                    return None;
                };
                let DeclarationPayload::Function(body) = &declaration.payload else {
                    return None;
                };
                if declaration.name.as_str() != "with_binding" {
                    return None;
                }
                let Some(OwnerRecord::Expression(expression)) =
                    snapshot.owners.get(&OwnerKey::Expression(body.body))
                else {
                    return None;
                };
                let ExpressionOperation::Let { body, .. } = expression.operation else {
                    return None;
                };
                Some((*function, body, expression.id))
            })
            .expect("with_binding extraction boundary");
        let before = std::fs::read(project.join("HEAD")).expect("oracle HEAD before");
        let oracle =
            function_extraction_oracle(&project, &function.to_string(), &selected.to_string())
                .expect("derive extraction oracle");
        assert_eq!(oracle.revision, created.current.head.revision.to_string());
        assert_eq!(oracle.base_body_records, 4);
        assert_eq!(oracle.caller_body_records, 5);
        assert_eq!(oracle.helper_body_records, 1);
        assert_eq!(oracle.moved_owners, vec![selected.to_string()]);
        assert!(oracle.preserved_owners.is_empty());
        assert_eq!(oracle.changed_owners.len(), 2);
        assert_eq!(oracle.captures.len(), 1);
        assert_eq!(oracle.captures[0].source_kind, "lexical-binding");
        assert_eq!(oracle.captures[0].name, "local");
        assert_eq!(oracle.captures[0].uses, vec![selected.to_string()]);
        assert_eq!(oracle.captures[0].use_mode, "unrestricted");
        assert_eq!(oracle.effect, "pure");
        assert!(oracle.requirements.is_empty());
        assert_eq!(oracle.generated_owners, 4);
        let whole_oracle =
            function_extraction_oracle(&project, &function.to_string(), &body_root.to_string())
                .expect_err("oracle rejects the complete function body");
        assert_eq!(whole_oracle.code, "contributor_extraction_whole_body");
        let repository = GraphRepository::open(&project).expect("open extraction repository");
        let whole_request = crate::platform::change::AuthoredChangeSet {
            base: created.current.head.revision,
            preconditions: Vec::new(),
            changes: vec![crate::platform::change::AuthoredChange::ExtractFunction {
                symbol: "$whole-helper".to_owned(),
                function: crate::platform::change::DeclarationSelector::Id {
                    declaration: function,
                },
                expression: body_root,
                name: crate::platform::kernel::Name::new("whole-helper")
                    .expect("rejected helper name"),
            }],
            budget: crate::platform::change::ChangeBudget::default(),
        };
        let whole_production = repository
            .prepare_authored_change(
                &whole_request,
                crate::platform::publication::PublicationOptions::default(),
            )
            .expect_err("production rejects the complete function body");
        assert_eq!(whole_production[0].code, "change_extract_whole_body");
        assert_eq!(
            std::fs::read(project.join("HEAD")).expect("oracle HEAD after"),
            before
        );
    }

    #[test]
    fn recursive_extraction_boundary_is_rejected_by_production_and_disjoint_oracle() {
        let temporary = tempfile::tempdir().expect("temporary recursive extraction repository");
        let project = temporary.path().join("meaning");
        let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
        let (function, selected) = snapshot
            .owners
            .iter()
            .find_map(|(owner, record)| {
                let OwnerKey::Declaration(function) = owner else {
                    return None;
                };
                let OwnerRecord::Declaration(declaration) = record else {
                    return None;
                };
                let DeclarationPayload::Function(body) = &declaration.payload else {
                    return None;
                };
                if declaration.name.as_str() != "with_binding" {
                    return None;
                }
                let Some(OwnerRecord::Expression(expression)) =
                    snapshot.owners.get(&OwnerKey::Expression(body.body))
                else {
                    return None;
                };
                let ExpressionOperation::Let { body, .. } = expression.operation else {
                    return None;
                };
                Some((*function, body))
            })
            .expect("recursive extraction fixture");
        let package = snapshot.root.package_id;
        let Some(OwnerRecord::Expression(expression)) =
            snapshot.owners.get_mut(&OwnerKey::Expression(selected))
        else {
            panic!("recursive selected expression")
        };
        expression.operation = ExpressionOperation::Call {
            function: crate::platform::kernel::DeclarationReference {
                package,
                declaration: function,
            },
            type_arguments: Vec::new(),
            arguments: Vec::new(),
        };
        let created = GraphRepository::create(&project, &snapshot, None)
            .expect("create valid recursive extraction repository");
        let before = std::fs::read(project.join("HEAD")).expect("recursive HEAD before");
        let oracle =
            function_extraction_oracle(&project, &function.to_string(), &selected.to_string())
                .expect_err("disjoint oracle rejects recursive target");
        assert_eq!(oracle.code, "contributor_extraction_recursive");
        let request = crate::platform::change::AuthoredChangeSet {
            base: created.current.head.revision,
            preconditions: Vec::new(),
            changes: vec![crate::platform::change::AuthoredChange::ExtractFunction {
                symbol: "$recursive-helper".to_owned(),
                function: crate::platform::change::DeclarationSelector::Id {
                    declaration: function,
                },
                expression: selected,
                name: crate::platform::kernel::Name::new("recursive-helper")
                    .expect("recursive helper name"),
            }],
            budget: crate::platform::change::ChangeBudget::default(),
        };
        let production = created
            .repository
            .prepare_authored_change(
                &request,
                crate::platform::publication::PublicationOptions::default(),
            )
            .expect_err("production rejects recursive target");
        assert_eq!(production[0].code, "change_extract_recursive_target");
        assert_eq!(
            std::fs::read(project.join("HEAD")).expect("recursive HEAD after"),
            before
        );
    }

    #[test]
    fn maintained_affine_extraction_plan_matches_disjoint_oracle_and_is_read_only() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal");
        let before = std::fs::read(project.join("HEAD")).expect("lkjournal HEAD before oracle");
        let function = "decl_a914bb78de075ff44a857ac028d704f3"
            .parse::<crate::platform::semantic_id::DeclarationId>()
            .expect("maintained worker function identity");
        let selected = "expr_9c71a3f66a11506528fec58a584c31a6"
            .parse::<crate::platform::semantic_id::ExpressionId>()
            .expect("maintained handoff expression identity");
        let oracle =
            function_extraction_oracle(&project, &function.to_string(), &selected.to_string())
                .expect("derive affine extraction oracle");
        let repository = GraphRepository::open(&project).expect("open maintained repository");
        let view = repository.view_current().expect("open maintained revision");
        let request = crate::platform::change::AuthoredChangeSet {
            base: view.revision(),
            preconditions: Vec::new(),
            changes: vec![crate::platform::change::AuthoredChange::ExtractFunction {
                symbol: "$affine-review-helper".to_owned(),
                function: crate::platform::change::DeclarationSelector::Id {
                    declaration: function,
                },
                expression: selected,
                name: crate::platform::kernel::Name::new("process-acquired-lease-review")
                    .expect("review helper name"),
            }],
            budget: crate::platform::change::ChangeBudget::default(),
        };
        let mut prepared = view
            .prepare_authored_change(
                &request,
                crate::platform::publication::PublicationOptions::default(),
            )
            .expect("prepare maintained affine extraction");
        let definition_digest =
            crate::platform::cli::function_definition_digest_for_extraction(&view, function)
                .expect("bind maintained public definition");
        prepared
            .logical_plan
            .extraction
            .as_mut()
            .expect("affine extraction evidence")
            .base_definition = Some(definition_digest);
        let extraction = prepared
            .logical_plan
            .extraction
            .as_ref()
            .expect("affine extraction evidence");
        assert_eq!(oracle.base_body_records, 15);
        assert_eq!(oracle.caller_body_records, 15);
        assert_eq!(oracle.helper_body_records, 3);
        assert_eq!(extraction.caller_body_records, oracle.caller_body_records);
        assert_eq!(extraction.helper_body_records, oracle.helper_body_records);
        assert_eq!(
            format!(
                "moved_{}",
                crate::platform::semantic_id::encode_hex(&extraction.moved_digest)
            ),
            oracle.moved_digest
        );
        assert_eq!(
            extraction
                .moved_owners
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            oracle.moved_owners
        );
        assert_eq!(
            extraction
                .preserved_owners
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            oracle.preserved_owners
        );
        assert_eq!(
            extraction
                .changed_owners
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            oracle.changed_owners
        );
        assert_eq!(extraction.captures.len(), 2);
        for (capture, expected) in extraction.captures.iter().zip(&oracle.captures) {
            assert_eq!(
                super::extraction_local_owner(capture.source).to_string(),
                expected.source
            );
            assert_eq!(capture.name.as_str(), expected.name);
            assert_eq!(capture.ty.to_string(), expected.ty);
            assert_eq!(
                match capture.use_mode {
                    crate::platform::kernel::ParameterUse::Unrestricted => "unrestricted",
                    crate::platform::kernel::ParameterUse::Borrow => "borrow",
                    crate::platform::kernel::ParameterUse::Consume => "consume",
                },
                expected.use_mode
            );
            assert_eq!(
                capture
                    .resource_requirement
                    .map(super::oracle_requirement_reference),
                expected.requirement
            );
            assert_eq!(
                capture
                    .rewritten_uses
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
                expected.uses
            );
        }
        assert_eq!(
            extraction.captures[1].use_mode,
            crate::platform::kernel::ParameterUse::Consume
        );
        let crate::platform::kernel::FunctionEffect::Task { requirements } = &extraction.effect
        else {
            panic!("affine extraction helper must be task effect")
        };
        assert_eq!(
            requirements
                .iter()
                .copied()
                .map(super::oracle_requirement_reference)
                .collect::<Vec<_>>(),
            oracle.requirements
        );
        assert_eq!(
            extraction.generated_owners.len(),
            usize::try_from(oracle.generated_owners).expect("generated owner count")
        );
        assert!(extraction.base_definition.is_some());
        assert_eq!(
            std::fs::read(project.join("HEAD")).expect("lkjournal HEAD after oracle"),
            before
        );
    }

    #[test]
    fn maintained_resource_boundaries_are_rejected_by_production_and_disjoint_oracle() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal");
        let before = std::fs::read(project.join("HEAD")).expect("lkjournal HEAD before rejection");
        let function = "decl_a914bb78de075ff44a857ac028d704f3"
            .parse::<crate::platform::semantic_id::DeclarationId>()
            .expect("maintained worker function identity");
        let repository = GraphRepository::open(&project).expect("open maintained repository");
        let view = repository.view_current().expect("open maintained revision");
        for (label, expression, oracle_code, production_code) in [
            (
                "resource-result",
                "expr_76ab37fa588f1e250b5b2044bfb15645",
                "contributor_extraction_result",
                "change_extract_resource_result",
            ),
            (
                "resource-container",
                "expr_041383ce5f6ff91ad343b3bad7954b61",
                "contributor_extraction_resource_container",
                "change_extract_resource_container",
            ),
        ] {
            let selected = expression
                .parse::<crate::platform::semantic_id::ExpressionId>()
                .expect("maintained rejected expression identity");
            let oracle =
                function_extraction_oracle(&project, &function.to_string(), &selected.to_string())
                    .expect_err("disjoint oracle must reject resource boundary");
            assert_eq!(
                oracle.class,
                crate::platform::diagnostic::DiagnosticClass::Semantic,
                "{label}"
            );
            assert_eq!(oracle.code, oracle_code, "{label}");
            let request = crate::platform::change::AuthoredChangeSet {
                base: view.revision(),
                preconditions: Vec::new(),
                changes: vec![crate::platform::change::AuthoredChange::ExtractFunction {
                    symbol: ["$", label, "-helper"].concat(),
                    function: crate::platform::change::DeclarationSelector::Id {
                        declaration: function,
                    },
                    expression: selected,
                    name: crate::platform::kernel::Name::new(format!("{label}-helper"))
                        .expect("rejected helper name"),
                }],
                budget: crate::platform::change::ChangeBudget::default(),
            };
            let production = view
                .prepare_authored_change(
                    &request,
                    crate::platform::publication::PublicationOptions::default(),
                )
                .expect_err("production planner must reject resource boundary");
            assert_eq!(
                production[0].class,
                crate::platform::diagnostic::DiagnosticClass::Semantic,
                "{label}"
            );
            assert_eq!(production[0].code, production_code, "{label}");
        }
        assert_eq!(
            std::fs::read(project.join("HEAD")).expect("lkjournal HEAD after rejection"),
            before
        );
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
        assert_eq!(largest.function, "decl_97e3d3c28142723096e5b121c0205ef2");
        assert_eq!(largest.body_preorder.len(), 148);
    }
}
