//! Exact canonical point reads used by change normalization.

use crate::platform::diagnostic::Diagnostic;
use crate::platform::kernel::{
    DependencyRecord, KernelSnapshot, OwnerKey, OwnerRecord, PackageId, RelationEdge,
    RetirementRecord, TypeObject, TypeObjectDigest,
};
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use crate::platform::witness::{FullWitness, NamespaceKey, OwnershipEntry};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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
        Self {
            value,
            work: WitnessReadWork {
                point_reads: 1,
                ..WitnessReadWork::default()
            },
        }
    }
}

/// Narrow accepted-authority surface required before high-level edits become an exact canonical
/// delta. Implementations must pin one immutable base for the lifetime of a normalization.
pub trait CanonicalBaseRead {
    fn repository_id(&self) -> RepositoryId;

    fn package_id(&self) -> PackageId;

    fn exact_revision(&self) -> Option<RevisionId>;

    fn read_owner(&self, owner: OwnerKey)
    -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic>;

    fn read_type_object(
        &self,
        digest: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic>;

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
    fn witness_repository_id(&self) -> RepositoryId;

    fn witness_package_id(&self) -> PackageId;

    fn witness_contract_is_current(&self) -> bool;

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
}

impl CanonicalBaseRead for KernelSnapshot {
    fn repository_id(&self) -> RepositoryId {
        self.root.repository_id
    }

    fn package_id(&self) -> PackageId {
        self.root.package_id
    }

    fn exact_revision(&self) -> Option<RevisionId> {
        None
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
        Ok(CanonicalRead::memory(self.types.get(&digest).cloned()))
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
    fn witness_repository_id(&self) -> RepositoryId {
        self.manifest.repository_id
    }

    fn witness_package_id(&self) -> PackageId {
        self.manifest.package_id
    }

    fn witness_contract_is_current(&self) -> bool {
        self.manifest.contract_is_current()
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
}
