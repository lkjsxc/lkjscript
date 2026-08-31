//! Read-only contributor observations outside the executable's public operation registry.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::kernel::{OwnerKind, RelationEndpoint, extract_relations, validate_full};
use super::publication::GraphRepository;
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

const OWNER_DIGEST_DOMAIN: &str = "lkjscript.contributor.owner-identities.v1";
const RELATION_DIGEST_DOMAIN: &str = "lkjscript.contributor.relations.v1";

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
    use super::semantic_inventory;
    use std::path::Path;

    #[test]
    fn maintained_standard_inventory_is_typed_and_read_only() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard");
        let before = std::fs::read(project.join("HEAD")).expect("standard HEAD before oracle");
        let inventory = semantic_inventory(&project).expect("standard semantic inventory");
        assert_eq!(inventory.owners, 409);
        assert_eq!(inventory.modules, 12);
        assert!(inventory.functions > 0);
        assert!(inventory.relations > 0);
        assert_eq!(
            std::fs::read(project.join("HEAD")).expect("standard HEAD after oracle"),
            before
        );
    }
}
