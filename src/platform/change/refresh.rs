//! Intent-read adapters used only during authored lowering, outside renewed validation.

/// Shape ceilings describe one semantic candidate; processing and retained-storage allowances
/// cover original proof, guards and renewed proof cumulatively.
pub(crate) fn remaining_refresh_budget(
    mut budget: ChangeBudget,
    prior: ChangeBudgetWork,
) -> Result<ChangeBudget, Diagnostic> {
    budget.check_observed(prior, "reviewed refresh prior preparation")?;
    budget.canonical_reads.maximum_point_reads -= prior.canonical_reads.point_reads;
    budget.canonical_reads.maximum_map_pages -= prior.canonical_reads.map_pages_read;
    budget.canonical_reads.maximum_map_entries -= prior.canonical_reads.map_entries_visited;
    budget.canonical_reads.maximum_catalog_lookups -= prior.canonical_reads.catalog_lookups;
    budget.canonical_reads.maximum_objects -= prior.canonical_reads.objects_read;
    budget.canonical_reads.maximum_bytes -= prior.canonical_reads.bytes_read;
    budget.canonical_reads.maximum_decoded_records -=
        prior.canonical_reads.canonical_records_decoded;
    budget.canonical_map_update.maximum_pages_encoded -= prior.canonical_map_update.pages_encoded;
    budget.canonical_map_update.maximum_bytes_encoded -= prior.canonical_map_update.bytes_encoded;
    budget.witness_reads.maximum_point_reads -= prior.witness_reads.point_reads;
    budget.witness_reads.maximum_map_pages -= prior.witness_reads.map_pages_read;
    budget.witness_reads.maximum_map_entries -= prior.witness_reads.map_entries_visited;
    budget.witness_reads.maximum_catalog_lookups -= prior.witness_reads.catalog_lookups;
    budget.witness_reads.maximum_objects -= prior.witness_reads.objects_read;
    budget.witness_reads.maximum_bytes -= prior.witness_reads.bytes_read;
    budget.witness_reads.maximum_decoded_records -= prior.witness_reads.witness_records_decoded;
    budget.impact.maximum_summary_owners -= prior.impact_summary_owners;
    budget.impact.maximum_summary_edits -= prior.impact_summary_edits;
    budget.impact.maximum_ownership_steps -= prior.impact_ownership_steps;
    budget.impact.maximum_behavior_owners -= prior.impact_behavior_owners;
    budget.impact.maximum_relation_edges -= prior.relation_edges;
    budget.validation.maximum_owner_records -= prior.validation.owner_records;
    budget.validation.maximum_ownership_entries -= prior.validation.ownership_entries;
    budget.validation.maximum_type_objects -= prior.validation.type_objects;
    budget.validation.maximum_expression_steps -= prior.validation.expression_steps;
    budget.validation.maximum_diagnostics -= prior.validation.diagnostics;
    budget.tests.maximum_ownership_steps -= prior.tests.ownership_steps;
    budget.tests.maximum_owners_visited -= prior.tests.owners_visited;
    budget.witness_update.maximum_edits -= prior.witness_update.edits;
    budget.witness_update.maximum_pages_encoded -= prior.witness_update.pages_encoded;
    budget.witness_update.maximum_bytes_encoded -= prior.witness_update.bytes_encoded;
    budget.staging.maximum_objects -= prior.staging.objects;
    budget.staging.maximum_bytes -= prior.staging.bytes;
    budget.staging.maximum_pages -= prior.staging.pages;
    Ok(budget)
}

pub(crate) fn include_prior_refresh_work(
    mut current: ChangeBudgetWork,
    prior: ChangeBudgetWork,
) -> Result<ChangeBudgetWork, Diagnostic> {
    current.canonical_reads.point_reads = add_observed(
        current.canonical_reads.point_reads,
        prior.canonical_reads.point_reads,
    )?;
    current.canonical_reads.map_pages_read = add_observed(
        current.canonical_reads.map_pages_read,
        prior.canonical_reads.map_pages_read,
    )?;
    current.canonical_reads.map_entries_visited = add_observed(
        current.canonical_reads.map_entries_visited,
        prior.canonical_reads.map_entries_visited,
    )?;
    current.canonical_reads.catalog_lookups = add_observed(
        current.canonical_reads.catalog_lookups,
        prior.canonical_reads.catalog_lookups,
    )?;
    current.canonical_reads.objects_read = add_observed(
        current.canonical_reads.objects_read,
        prior.canonical_reads.objects_read,
    )?;
    current.canonical_reads.bytes_read = add_observed(
        current.canonical_reads.bytes_read,
        prior.canonical_reads.bytes_read,
    )?;
    current.canonical_reads.canonical_records_decoded = add_observed(
        current.canonical_reads.canonical_records_decoded,
        prior.canonical_reads.canonical_records_decoded,
    )?;
    current.canonical_map_update.pages_encoded = add_observed(
        current.canonical_map_update.pages_encoded,
        prior.canonical_map_update.pages_encoded,
    )?;
    current.canonical_map_update.bytes_encoded = add_observed(
        current.canonical_map_update.bytes_encoded,
        prior.canonical_map_update.bytes_encoded,
    )?;
    current.witness_reads.point_reads = add_observed(
        current.witness_reads.point_reads,
        prior.witness_reads.point_reads,
    )?;
    current.witness_reads.map_pages_read = add_observed(
        current.witness_reads.map_pages_read,
        prior.witness_reads.map_pages_read,
    )?;
    current.witness_reads.map_entries_visited = add_observed(
        current.witness_reads.map_entries_visited,
        prior.witness_reads.map_entries_visited,
    )?;
    current.witness_reads.catalog_lookups = add_observed(
        current.witness_reads.catalog_lookups,
        prior.witness_reads.catalog_lookups,
    )?;
    current.witness_reads.objects_read = add_observed(
        current.witness_reads.objects_read,
        prior.witness_reads.objects_read,
    )?;
    current.witness_reads.bytes_read = add_observed(
        current.witness_reads.bytes_read,
        prior.witness_reads.bytes_read,
    )?;
    current.witness_reads.witness_records_decoded = add_observed(
        current.witness_reads.witness_records_decoded,
        prior.witness_reads.witness_records_decoded,
    )?;
    current.impact_summary_owners =
        add_observed(current.impact_summary_owners, prior.impact_summary_owners)?;
    current.impact_summary_edits =
        add_observed(current.impact_summary_edits, prior.impact_summary_edits)?;
    current.impact_ownership_steps =
        add_observed(current.impact_ownership_steps, prior.impact_ownership_steps)?;
    current.impact_behavior_owners =
        add_observed(current.impact_behavior_owners, prior.impact_behavior_owners)?;
    current.relation_edges = add_observed(current.relation_edges, prior.relation_edges)?;
    current.validation.owner_records = add_observed(
        current.validation.owner_records,
        prior.validation.owner_records,
    )?;
    current.validation.ownership_entries = add_observed(
        current.validation.ownership_entries,
        prior.validation.ownership_entries,
    )?;
    current.validation.type_objects = add_observed(
        current.validation.type_objects,
        prior.validation.type_objects,
    )?;
    current.validation.expression_steps = add_observed(
        current.validation.expression_steps,
        prior.validation.expression_steps,
    )?;
    current.validation.diagnostics =
        add_observed(current.validation.diagnostics, prior.validation.diagnostics)?;
    current.tests.ownership_steps =
        add_observed(current.tests.ownership_steps, prior.tests.ownership_steps)?;
    current.tests.owners_visited =
        add_observed(current.tests.owners_visited, prior.tests.owners_visited)?;
    current.witness_update.edits =
        add_observed(current.witness_update.edits, prior.witness_update.edits)?;
    current.witness_update.pages_encoded = add_observed(
        current.witness_update.pages_encoded,
        prior.witness_update.pages_encoded,
    )?;
    current.witness_update.bytes_encoded = add_observed(
        current.witness_update.bytes_encoded,
        prior.witness_update.bytes_encoded,
    )?;
    current.staging.objects = add_observed(current.staging.objects, prior.staging.objects)?;
    current.staging.bytes = add_observed(current.staging.bytes, prior.staging.bytes)?;
    current.staging.pages = add_observed(current.staging.pages, prior.staging.pages)?;
    Ok(current)
}

use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};

fn add_observed(current: u64, prior: u64) -> Result<u64, Diagnostic> {
    current.checked_add(prior).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "change_refresh_preparation_capacity",
            "aggregate refresh preparation observation overflowed",
        )
    })
}
use crate::platform::kernel::{
    DependencyRecord, OwnerKey, OwnerRecord, PackageId, PackageInterfaceRecord, RelationEdge,
    RelationKind, RetirementRecord, SemanticRoot, TypeObject, TypeObjectDigest,
};
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use crate::platform::witness::{NamespaceKey, OwnershipEntry, ValidationWitnessManifest};
use std::cell::RefCell;

pub(crate) struct RecordedCanonicalBase<'a, B: ?Sized> {
    pub base: &'a B,
    pub reads: &'a RefCell<AuthoredReadFootprint>,
}
pub(crate) struct RecordedWitnessBase<'a, W: ?Sized> {
    pub base: &'a W,
    pub reads: &'a RefCell<AuthoredReadFootprint>,
}

impl<B: CanonicalBaseRead + ?Sized> CanonicalBaseRead for RecordedCanonicalBase<'_, B> {
    fn accepted_retry_encoding(&self) -> Option<u16> {
        self.base.accepted_retry_encoding()
    }
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        self.base.validation_checkpoint()
    }
    fn semantic_root(&self) -> &SemanticRoot {
        self.base.semantic_root()
    }
    fn repository_id(&self) -> RepositoryId {
        self.base.repository_id()
    }
    fn package_id(&self) -> PackageId {
        self.base.package_id()
    }
    fn exact_revision(&self) -> Option<RevisionId> {
        self.base.exact_revision()
    }
    fn owner_count(&self) -> u64 {
        self.base.owner_count()
    }
    fn dependency_count(&self) -> u64 {
        self.base.dependency_count()
    }
    fn retirement_count(&self) -> u64 {
        self.base.retirement_count()
    }
    fn read_owner(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
        let read = self.base.read_owner(owner)?;
        self.reads
            .borrow_mut()
            .record_owner(owner, read.value.as_ref())?;
        Ok(read)
    }
    fn read_type_object(
        &self,
        digest: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        let read = self.base.read_type_object(digest)?;
        Ok(read)
    }
    fn read_package_interface_owner(
        &self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        let read = self.base.read_package_interface_owner(dependency, owner)?;
        self.reads
            .borrow_mut()
            .record_interface_owner(dependency, owner, read.value.as_ref())?;
        Ok(read)
    }
    fn read_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        let read = self.base.read_dependency(package)?;
        self.reads
            .borrow_mut()
            .record_dependency(package, read.value.as_ref())?;
        Ok(read)
    }
    fn read_retirement(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic> {
        let read = self.base.read_retirement(owner)?;
        self.reads
            .borrow_mut()
            .record_retirement(owner, read.value.as_ref())?;
        Ok(read)
    }
    fn read_reference_interface(
        &self,
        dependency: &DependencyRecord,
    ) -> Result<CanonicalRead<CanonicalReferenceInterface>, Diagnostic> {
        let read = self.base.read_reference_interface(dependency)?;
        self.reads
            .borrow_mut()
            .record_interface(dependency, read.value.revision.interface.bytes())?;
        Ok(read)
    }
}

impl<W: WitnessBaseRead + ?Sized> WitnessBaseRead for RecordedWitnessBase<'_, W> {
    fn witness_manifest(&self) -> &ValidationWitnessManifest {
        self.base.witness_manifest()
    }
    fn witness_repository_id(&self) -> RepositoryId {
        self.base.witness_repository_id()
    }
    fn witness_package_id(&self) -> PackageId {
        self.base.witness_package_id()
    }
    fn witness_contract_is_current(&self) -> bool {
        self.base.witness_contract_is_current()
    }
    fn canonical_facts_are_current(&self) -> bool {
        self.base.canonical_facts_are_current()
    }
    fn owner_summary_count(&self) -> u64 {
        self.base.owner_summary_count()
    }
    fn read_namespace(
        &self,
        key: &NamespaceKey,
    ) -> Result<WitnessRead<Option<OwnerKey>>, Diagnostic> {
        let read = self.base.read_namespace(key)?;
        self.reads.borrow_mut().record_namespace(key, read.value)?;
        Ok(read)
    }
    fn read_ownership(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<OwnershipEntry>>, Diagnostic> {
        let read = self.base.read_ownership(owner)?;
        self.reads
            .borrow_mut()
            .record_ownership(owner, read.value.as_ref())?;
        Ok(read)
    }
    fn contains_forward_relation(
        &self,
        edge: RelationEdge,
    ) -> Result<WitnessRead<bool>, Diagnostic> {
        let read = self.base.contains_forward_relation(edge)?;
        self.reads.borrow_mut().record_relation(edge, read.value)?;
        Ok(read)
    }
    fn read_owner_summary(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<BoundOwnerSummary>>, Diagnostic> {
        let read = self.base.read_owner_summary(owner)?;
        self.reads
            .borrow_mut()
            .record_summary(owner, read.value.as_ref())?;
        Ok(read)
    }
    fn read_outgoing_relations(
        &self,
        owner: OwnerKey,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        let read = self.base.read_outgoing_relations(owner, maximum_items)?;
        if !read.value.truncated {
            self.reads.borrow_mut().record_relations(
                owner,
                None,
                false,
                maximum_items,
                &read.value.edges,
                false,
            )?;
        }
        Ok(read)
    }
    fn read_incoming_relations(
        &self,
        owner: OwnerKey,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        let read = self.base.read_incoming_relations(owner, maximum_items)?;
        if !read.value.truncated {
            self.reads.borrow_mut().record_relations(
                owner,
                None,
                true,
                maximum_items,
                &read.value.edges,
                false,
            )?;
        }
        Ok(read)
    }
    fn read_incoming_relations_of_kind(
        &self,
        owner: OwnerKey,
        kind: RelationKind,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        let read = self
            .base
            .read_incoming_relations_of_kind(owner, kind, maximum_items)?;
        if !read.value.truncated {
            self.reads.borrow_mut().record_relations(
                owner,
                Some(kind),
                true,
                maximum_items,
                &read.value.edges,
                false,
            )?;
        }
        Ok(read)
    }
    fn read_incoming_package_relations(
        &self,
        package: PackageId,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        let read = self
            .base
            .read_incoming_package_relations(package, maximum_items)?;
        if !read.value.truncated {
            self.reads.borrow_mut().record_package_relations(
                package,
                maximum_items,
                &read.value.edges,
                false,
            )?;
        }
        Ok(read)
    }
    fn read_test_dependencies(
        &self,
        test: OwnerKey,
        maximum_items: usize,
    ) -> Result<WitnessRead<WitnessTestDependencyRead>, Diagnostic> {
        let read = self.base.read_test_dependencies(test, maximum_items)?;
        if !read.value.truncated {
            self.reads.borrow_mut().record_test_dependencies(
                test,
                maximum_items,
                &read.value.dependencies,
                false,
            )?;
        }
        Ok(read)
    }
}
