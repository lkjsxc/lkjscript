//! Exact canonical point reads used by change normalization.

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DependencyRecord, ExactOwnerKey, KernelSnapshot, OwnerKey, OwnerRecord, PackageId,
    PackageInterfaceRecord, RelationEdge, RelationEndpoint, RetirementRecord, SemanticRoot,
    TypeObject, TypeObjectDigest,
};
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use crate::platform::witness::{
    FullWitness, MAXIMUM_RELATION_PREFIX_ITEMS, MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS, NamespaceKey,
    OwnerSummary, OwnerSummaryDigest, OwnershipEntry, TestDependency, ValidationWitnessManifest,
};
use bincode::{Decode, Encode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Decode, Default, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize,
)]
#[schemars(rename = "lkjscript.CanonicalReadWorkV1")]
#[serde(deny_unknown_fields)]
pub struct CanonicalReadWork {
    pub point_reads: u64,
    pub map_pages_read: u64,
    pub map_entries_visited: u64,
    pub catalog_lookups: u64,
    pub objects_read: u64,
    pub bytes_read: u64,
    pub canonical_records_decoded: u64,
}

impl CanonicalReadWork {
    pub fn add(&mut self, other: Self) {
        self.point_reads = self.point_reads.saturating_add(other.point_reads);
        self.map_pages_read = self.map_pages_read.saturating_add(other.map_pages_read);
        self.map_entries_visited = self
            .map_entries_visited
            .saturating_add(other.map_entries_visited);
        self.catalog_lookups = self.catalog_lookups.saturating_add(other.catalog_lookups);
        self.objects_read = self.objects_read.saturating_add(other.objects_read);
        self.bytes_read = self.bytes_read.saturating_add(other.bytes_read);
        self.canonical_records_decoded = self
            .canonical_records_decoded
            .saturating_add(other.canonical_records_decoded);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalRead<T> {
    pub value: T,
    pub work: CanonicalReadWork,
}

impl<T> CanonicalRead<T> {
    fn memory(value: T) -> Self {
        Self {
            value,
            work: CanonicalReadWork {
                point_reads: 1,
                ..CanonicalReadWork::default()
            },
        }
    }
}

#[derive(
    Clone, Copy, Debug, Decode, Default, Deserialize, Encode, Eq, JsonSchema, PartialEq, Serialize,
)]
#[schemars(rename = "lkjscript.WitnessReadWorkV1")]
#[serde(deny_unknown_fields)]
pub struct WitnessReadWork {
    pub point_reads: u64,
    pub map_pages_read: u64,
    pub map_entries_visited: u64,
    pub catalog_lookups: u64,
    pub objects_read: u64,
    pub bytes_read: u64,
    pub witness_records_decoded: u64,
}

impl WitnessReadWork {
    pub fn add(&mut self, other: Self) {
        self.point_reads = self.point_reads.saturating_add(other.point_reads);
        self.map_pages_read = self.map_pages_read.saturating_add(other.map_pages_read);
        self.map_entries_visited = self
            .map_entries_visited
            .saturating_add(other.map_entries_visited);
        self.catalog_lookups = self.catalog_lookups.saturating_add(other.catalog_lookups);
        self.objects_read = self.objects_read.saturating_add(other.objects_read);
        self.bytes_read = self.bytes_read.saturating_add(other.bytes_read);
        self.witness_records_decoded = self
            .witness_records_decoded
            .saturating_add(other.witness_records_decoded);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WitnessRead<T> {
    pub value: T,
    pub work: WitnessReadWork,
}

impl<T> WitnessRead<T> {
    fn memory(value: T) -> Self {
        Self::memory_records(value, 0)
    }

    fn memory_records(value: T, witness_records_decoded: u64) -> Self {
        Self {
            value,
            work: WitnessReadWork {
                point_reads: 1,
                witness_records_decoded,
                ..WitnessReadWork::default()
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundOwnerSummary {
    pub digest: OwnerSummaryDigest,
    pub summary: OwnerSummary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WitnessRelationRead {
    pub edges: Vec<RelationEdge>,
    pub truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WitnessTestDependencyRead {
    pub dependencies: Vec<TestDependency>,
    pub truncated: bool,
}

/// Narrow accepted-authority surface required before high-level edits become an exact canonical
/// delta. Implementations must pin one immutable base for the lifetime of a normalization.
pub trait CanonicalBaseRead {
    fn semantic_root(&self) -> &SemanticRoot;

    fn repository_id(&self) -> RepositoryId;

    fn package_id(&self) -> PackageId;

    fn exact_revision(&self) -> Option<RevisionId>;

    fn owner_count(&self) -> u64;

    fn dependency_count(&self) -> u64;

    fn retirement_count(&self) -> u64;

    fn read_owner(&self, owner: OwnerKey)
    -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic>;

    fn read_type_object(
        &self,
        digest: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic>;

    fn read_package_interface_owner(
        &self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic>;

    fn read_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic>;

    fn read_retirement(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic>;
}

/// Exact derived-witness reads required to classify the local effects of canonical owner edits.
pub trait WitnessBaseRead {
    fn witness_manifest(&self) -> &ValidationWitnessManifest;

    fn witness_repository_id(&self) -> RepositoryId;

    fn witness_package_id(&self) -> PackageId;

    fn witness_contract_is_current(&self) -> bool;

    fn owner_summary_count(&self) -> u64;

    fn read_namespace(
        &self,
        key: &NamespaceKey,
    ) -> Result<WitnessRead<Option<OwnerKey>>, Diagnostic>;

    fn read_ownership(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<OwnershipEntry>>, Diagnostic>;

    fn contains_forward_relation(
        &self,
        edge: RelationEdge,
    ) -> Result<WitnessRead<bool>, Diagnostic>;

    fn read_owner_summary(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<BoundOwnerSummary>>, Diagnostic>;

    fn read_outgoing_relations(
        &self,
        owner: OwnerKey,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic>;

    fn read_incoming_relations(
        &self,
        owner: OwnerKey,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic>;

    fn read_incoming_package_relations(
        &self,
        package: PackageId,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic>;

    fn read_test_dependencies(
        &self,
        test: OwnerKey,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessTestDependencyRead>, Diagnostic>;
}

impl CanonicalBaseRead for KernelSnapshot {
    fn semantic_root(&self) -> &SemanticRoot {
        &self.root
    }

    fn repository_id(&self) -> RepositoryId {
        self.root.repository_id
    }

    fn package_id(&self) -> PackageId {
        self.root.package_id
    }

    fn exact_revision(&self) -> Option<RevisionId> {
        None
    }

    fn owner_count(&self) -> u64 {
        self.root.owners.entries()
    }

    fn dependency_count(&self) -> u64 {
        self.root.dependencies.entries()
    }

    fn retirement_count(&self) -> u64 {
        self.root.retirements.entries()
    }

    fn read_owner(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
        Ok(CanonicalRead::memory(self.owners.get(&owner).cloned()))
    }

    fn read_type_object(
        &self,
        digest: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        Ok(CanonicalRead::memory(
            self.types
                .get(&digest)
                .or_else(|| self.dependency_types.get(&digest))
                .cloned(),
        ))
    }

    fn read_package_interface_owner(
        &self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        Ok(CanonicalRead::memory(
            self.dependency_interfaces
                .get(&dependency.package_revision)
                .and_then(|owners| owners.get(&owner))
                .cloned(),
        ))
    }

    fn read_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        Ok(CanonicalRead::memory(
            self.dependencies.get(&package).cloned(),
        ))
    }

    fn read_retirement(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic> {
        Ok(CanonicalRead::memory(self.retirements.get(&owner).cloned()))
    }
}

impl WitnessBaseRead for FullWitness {
    fn witness_manifest(&self) -> &ValidationWitnessManifest {
        &self.manifest
    }

    fn witness_repository_id(&self) -> RepositoryId {
        self.manifest.repository_id
    }

    fn witness_package_id(&self) -> PackageId {
        self.manifest.package_id
    }

    fn witness_contract_is_current(&self) -> bool {
        self.manifest.contract_is_current()
    }

    fn owner_summary_count(&self) -> u64 {
        self.manifest.roots.owner_summaries.entries()
    }

    fn read_namespace(
        &self,
        key: &NamespaceKey,
    ) -> Result<WitnessRead<Option<OwnerKey>>, Diagnostic> {
        Ok(WitnessRead::memory(
            self.entries.namespaces.get(key).copied(),
        ))
    }

    fn read_ownership(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<OwnershipEntry>>, Diagnostic> {
        Ok(WitnessRead::memory(
            self.entries.ownership.get(&owner).copied(),
        ))
    }

    fn contains_forward_relation(
        &self,
        edge: RelationEdge,
    ) -> Result<WitnessRead<bool>, Diagnostic> {
        Ok(WitnessRead::memory(
            self.entries.relations.binary_search(&edge).is_ok(),
        ))
    }

    fn read_owner_summary(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<BoundOwnerSummary>>, Diagnostic> {
        let summary = self.summaries.get(&owner);
        let digest = self.entries.summaries.get(&owner).copied();
        match (summary, digest) {
            (Some(summary), Some(digest)) => {
                let (actual, _) = crate::platform::witness::encode_owner_summary(summary)?;
                if actual != digest {
                    return Err(base_read_error(
                        DiagnosticClass::Corrupt,
                        "change_summary_base_binding",
                        "base summary object disagrees with its witness binding",
                    ));
                }
                Ok(WitnessRead::memory_records(
                    Some(BoundOwnerSummary {
                        digest,
                        summary: summary.clone(),
                    }),
                    1,
                ))
            }
            (None, None) => Ok(WitnessRead::memory(None)),
            _ => Err(base_read_error(
                DiagnosticClass::Corrupt,
                "change_summary_base_missing",
                "base summary object and witness binding have different domains",
            )),
        }
    }

    fn read_outgoing_relations(
        &self,
        owner: OwnerKey,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        read_memory_relations(self, owner, maximum_items, false)
    }

    fn read_incoming_relations(
        &self,
        owner: OwnerKey,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        read_memory_relations(self, owner, maximum_items, true)
    }

    fn read_incoming_package_relations(
        &self,
        package: PackageId,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        if maximum_items == 0 || maximum_items > MAXIMUM_RELATION_PREFIX_ITEMS {
            return Err(base_read_error(
                DiagnosticClass::Resource,
                "change_relation_item_budget",
                "relation item budget is outside the current supported range",
            ));
        }
        let available = self
            .entries
            .reverse_relations
            .iter()
            .filter(|edge| {
                matches!(
                    edge.target,
                    RelationEndpoint::Owner(ExactOwnerKey {
                        package: target_package,
                        ..
                    }) if target_package == package
                )
            })
            .copied()
            .take(maximum_items.saturating_add(1))
            .collect::<Vec<_>>();
        let returned = available.len().min(maximum_items);
        Ok(WitnessRead::memory_records(
            WitnessRelationRead {
                edges: available[..returned].to_vec(),
                truncated: available.len() > maximum_items,
            },
            returned as u64,
        ))
    }

    fn read_test_dependencies(
        &self,
        test: OwnerKey,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessTestDependencyRead>, Diagnostic> {
        if maximum_items == 0 || maximum_items > MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS {
            return Err(base_read_error(
                DiagnosticClass::Resource,
                "change_test_dependency_item_budget",
                "test-dependency item budget is outside the current supported range",
            ));
        }
        if !matches!(test, OwnerKey::Declaration(_)) {
            return Err(base_read_error(
                DiagnosticClass::Source,
                "change_test_dependency_owner",
                "test-dependency lookup requires a declaration owner",
            ));
        }
        let dependencies = self
            .entries
            .test_dependencies_by_test
            .get(&test)
            .map(|entries| {
                entries
                    .iter()
                    .copied()
                    .take(maximum_items.saturating_add(1))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let returned = dependencies.len().min(maximum_items);
        Ok(WitnessRead::memory_records(
            WitnessTestDependencyRead {
                dependencies: dependencies[..returned].to_vec(),
                truncated: dependencies.len() > maximum_items,
            },
            returned as u64,
        ))
    }
}

fn read_memory_relations(
    witness: &FullWitness,
    owner: OwnerKey,
    maximum_items: usize,
    reverse: bool,
) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
    if maximum_items == 0 || maximum_items > MAXIMUM_RELATION_PREFIX_ITEMS {
        return Err(base_read_error(
            DiagnosticClass::Resource,
            "change_relation_item_budget",
            "relation item budget is outside the current supported range",
        ));
    }
    let endpoint = RelationEndpoint::Owner(ExactOwnerKey {
        package: witness.manifest.package_id,
        owner,
    });
    let relations = if reverse {
        &witness.entries.reverse_relations
    } else {
        &witness.entries.relations
    };
    let start = relations.partition_point(|edge| {
        if reverse {
            edge.target < endpoint
        } else {
            edge.source < endpoint
        }
    });
    let end = relations.partition_point(|edge| {
        if reverse {
            edge.target <= endpoint
        } else {
            edge.source <= endpoint
        }
    });
    let available = &relations[start..end];
    let returned = available.len().min(maximum_items);
    Ok(WitnessRead::memory_records(
        WitnessRelationRead {
            edges: available[..returned].to_vec(),
            truncated: available.len() > maximum_items,
        },
        returned as u64,
    ))
}

fn base_read_error(
    class: DiagnosticClass,
    code: &'static str,
    message: &'static str,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
