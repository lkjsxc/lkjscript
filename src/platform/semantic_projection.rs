//! Deterministic, non-authoritative review projection of accepted semantic meaning.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::meaning::GRAPH_CONTRACT_IDENTITY;
#[cfg(test)]
use super::meaning::MeaningModule;
use super::repository::{RevisionSnapshot, SemanticRepository};
use super::semantic_id::{RepositoryId, RevisionId};
use serde::Serialize;
use serde_json::Value as JsonValue;

pub const REVIEW_PROJECTION_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_REVIEW_PROJECTION_BYTES: usize = 128 * 1_048_576;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewProjectionReceipt {
    pub contract_version: u16,
    pub repository_id: RepositoryId,
    pub revision: RevisionId,
    pub digest: String,
    pub bytes: usize,
    pub modules: usize,
    pub declarations: usize,
    pub relations: usize,
    pub authoritative: bool,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ReviewProjection {
    contract_version: u16,
    graph_contract: &'static str,
    authority: &'static str,
    repository_id: RepositoryId,
    revision: RevisionId,
    parents: Vec<RevisionId>,
    package_id: super::package::PackageId,
    package_name: String,
    dependencies: JsonValue,
    targets: JsonValue,
    tombstones: JsonValue,
    modules: Vec<JsonValue>,
}

pub fn render_review_projection(
    repository: &SemanticRepository,
    revision: Option<RevisionId>,
) -> Result<(Vec<u8>, ReviewProjectionReceipt), Diagnostic> {
    let revision = revision.unwrap_or(repository.current()?.head.revision);
    let snapshot = repository.reconstruct_revision(revision)?;
    render_snapshot(snapshot)
}

fn render_snapshot(
    mut snapshot: RevisionSnapshot,
) -> Result<(Vec<u8>, ReviewProjectionReceipt), Diagnostic> {
    snapshot.modules.sort_by_key(|module| module.module_id);
    let module_count = snapshot.modules.len();
    let declaration_count = snapshot
        .modules
        .iter()
        .try_fold(0usize, |count, module| {
            count.checked_add(module.declarations.len())
        })
        .ok_or_else(projection_count_overflow)?;
    let relation_count = snapshot
        .modules
        .iter()
        .try_fold(0usize, |count, module| {
            count.checked_add(module.relations.len())
        })
        .ok_or_else(projection_count_overflow)?;
    let modules = snapshot
        .modules
        .iter()
        .map(semantic_json)
        .collect::<Result<Vec<_>, _>>()?;
    let projection = ReviewProjection {
        contract_version: REVIEW_PROJECTION_CONTRACT_VERSION,
        graph_contract: GRAPH_CONTRACT_IDENTITY,
        authority: "non_authoritative_review_projection",
        repository_id: snapshot.record.core.repository_id,
        revision: snapshot.record.revision,
        parents: snapshot
            .record
            .core
            .parents
            .iter()
            .map(|parent| parent.revision)
            .collect(),
        package_id: snapshot.root.package_id,
        package_name: snapshot.root.package_name,
        dependencies: semantic_json(&snapshot.root.dependencies)?,
        targets: semantic_json(&snapshot.root.targets)?,
        tombstones: semantic_json(&snapshot.root.tombstones)?,
        modules,
    };
    let mut bytes = serde_json::to_vec_pretty(&projection).map_err(projection_json)?;
    bytes.push(b'\n');
    if bytes.len() > MAXIMUM_REVIEW_PROJECTION_BYTES {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "semantic_review_projection_limit",
            format!(
                "review projection has {} bytes; the hard limit is {MAXIMUM_REVIEW_PROJECTION_BYTES}",
                bytes.len()
            ),
        ));
    }
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.semantic-review-projection.v1");
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(&bytes);
    let receipt = ReviewProjectionReceipt {
        contract_version: REVIEW_PROJECTION_CONTRACT_VERSION,
        repository_id: snapshot.record.core.repository_id,
        revision: snapshot.record.revision,
        digest: hasher.finalize().to_hex().to_string(),
        bytes: bytes.len(),
        modules: module_count,
        declarations: declaration_count,
        relations: relation_count,
        authoritative: false,
    };
    Ok((bytes, receipt))
}

fn semantic_json(value: &impl Serialize) -> Result<JsonValue, Diagnostic> {
    let mut value = serde_json::to_value(value).map_err(projection_json)?;
    remove_source_coordinates(&mut value);
    Ok(value)
}

fn remove_source_coordinates(value: &mut JsonValue) {
    match value {
        JsonValue::Object(values) => {
            values.remove("span");
            for value in values.values_mut() {
                remove_source_coordinates(value);
            }
        }
        JsonValue::Array(values) => {
            values.retain(|value| !is_source_span(value));
            for value in values {
                remove_source_coordinates(value);
            }
        }
        _ => {}
    }
}

fn is_source_span(value: &JsonValue) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.len() == 4
        && object.contains_key("byte_start")
        && object.contains_key("byte_end")
        && object.contains_key("line")
        && object.contains_key("column")
}

fn projection_json(error: serde_json::Error) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Infrastructure,
        "semantic_review_projection_json",
        format!("review projection encoding failed: {error}"),
    )
}

fn projection_count_overflow() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "semantic_review_projection_count",
        "review projection item count overflowed",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{
        GraphRoot, InitialPublication, MigrationIdentityAllocator, ModuleObjectRef, PackageId,
        RepositoryId, SemanticDiffDigest, SourceLimits, TransactionDigest, parse_module,
        parse_source,
    };

    #[test]
    fn review_projection_is_deterministic_span_free_and_non_authoritative() {
        let temporary = tempfile::TempDir::new().expect("temporary project");
        let document = parse_source(
            "fixture.lkj",
            b"(module sample (record Item (name Text)))\n",
            SourceLimits::default(),
        )
        .expect("source oracle");
        let module = parse_module(&document).expect("module oracle");
        let mut allocator = MigrationIdentityAllocator::new(b"review-projection".to_vec());
        let meaning = MeaningModule::import(module, &mut allocator).expect("meaning");
        let root = GraphRoot {
            graph_contract_version: super::super::meaning::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(b"review-projection", 1),
            package_id: PackageId::parse("10000000000000000000000000000001").expect("package"),
            package_name: "fixture".to_owned(),
            modules: vec![ModuleObjectRef {
                id: meaning.module_id,
                name: meaning.module.name.clone(),
                object: meaning.digest().expect("module digest"),
            }],
            dependencies: Vec::new(),
            targets: Vec::new(),
            tombstones: Vec::new(),
        };
        let (repository, _) = SemanticRepository::initialize(
            temporary.path(),
            InitialPublication {
                root,
                modules: vec![meaning],
                transaction: TransactionDigest::of(b"review-projection-import"),
                semantic_diff: SemanticDiffDigest::of(b"review-projection-initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
            },
        )
        .expect("initialize");
        let (first, first_receipt) =
            render_review_projection(&repository, None).expect("first projection");
        let (second, second_receipt) =
            render_review_projection(&repository, None).expect("second projection");
        assert_eq!(first, second);
        assert_eq!(first_receipt, second_receipt);
        assert!(!first_receipt.authoritative);
        let text = String::from_utf8(first).expect("UTF-8 review projection");
        assert!(!text.contains("byte_start"));
        assert!(!text.contains("\"span\""));
        assert!(text.contains("non_authoritative_review_projection"));
    }
}
