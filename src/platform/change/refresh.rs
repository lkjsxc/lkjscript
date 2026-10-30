//! Intent-read adapters used only during authored lowering, outside renewed validation.

/// Shape ceilings describe one semantic candidate; processing and retained-storage allowances
/// cover original proof, guards and renewed proof cumulatively.
const REFRESH_WORK_DIMENSIONS: &[BudgetDimension] = &[
    BudgetDimension::CanonicalPointReads,
    BudgetDimension::CanonicalMapPages,
    BudgetDimension::CanonicalMapEntries,
    BudgetDimension::CanonicalCatalogLookups,
    BudgetDimension::CanonicalObjects,
    BudgetDimension::CanonicalBytes,
    BudgetDimension::CanonicalDecodedRecords,
    BudgetDimension::CanonicalMapPagesEncoded,
    BudgetDimension::CanonicalMapBytesEncoded,
    BudgetDimension::WitnessPointReads,
    BudgetDimension::WitnessMapPages,
    BudgetDimension::WitnessMapEntries,
    BudgetDimension::WitnessCatalogLookups,
    BudgetDimension::WitnessObjects,
    BudgetDimension::WitnessBytes,
    BudgetDimension::WitnessDecodedRecords,
    BudgetDimension::ImpactSummaryOwners,
    BudgetDimension::ImpactSummaryEdits,
    BudgetDimension::ImpactOwnershipSteps,
    BudgetDimension::ImpactBehaviorOwners,
    BudgetDimension::RelationEdges,
    BudgetDimension::ValidationOwnerRecords,
    BudgetDimension::ValidationOwnershipEntries,
    BudgetDimension::ValidationTypeObjects,
    BudgetDimension::ValidationExpressionSteps,
    BudgetDimension::ValidationDiagnostics,
    BudgetDimension::TestOwnershipSteps,
    BudgetDimension::TestOwnersVisited,
    BudgetDimension::WitnessEdits,
    BudgetDimension::WitnessMapPagesEncoded,
    BudgetDimension::WitnessMapBytesEncoded,
    BudgetDimension::StagedObjects,
    BudgetDimension::StagedBytes,
    BudgetDimension::StagedPages,
];

pub(crate) fn remaining_refresh_budget(
    mut budget: ChangeBudget,
    prior: ChangeBudgetWork,
) -> Result<ChangeBudget, Diagnostic> {
    budget.check_observed(prior, "reviewed refresh prior preparation")?;
    for dimension in REFRESH_WORK_DIMENSIONS {
        if let Some(observed) = prior.observation(*dimension) {
            let remaining = observed.remaining(
                budget.maximum(*dimension),
                "reviewed refresh prior preparation",
                dimension.name(),
            )?;
            budget.set_policy_limit(dimension.policy_name(), remaining)?;
        }
    }
    Ok(budget)
}

pub(crate) fn include_prior_refresh_work(
    mut current: ChangeBudgetWork,
    prior: ChangeBudgetWork,
) -> Result<ChangeBudgetWork, Diagnostic> {
    for dimension in REFRESH_WORK_DIMENSIONS {
        if let Some(observed) = prior.observation(*dimension) {
            current.charge(*dimension, observed, None, "reviewed refresh observation")?;
        }
    }
    Ok(current)
}

use super::*;
use crate::platform::diagnostic::Diagnostic;

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
    fn read_dependencies(&self) -> Result<CanonicalRead<Vec<DependencyRecord>>, Diagnostic> {
        let read = self.base.read_dependencies()?;
        self.reads.borrow_mut().record_dependencies(&read.value)?;
        Ok(read)
    }
    fn read_representation_dependency(
        &self,
        root: &DependencyRecord,
        package: PackageId,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        let read = self.base.read_representation_dependency(root, package)?;
        self.reads.borrow_mut().record_representation_dependency(
            root,
            package,
            read.value.as_ref(),
        )?;
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
        maximum_items: Option<usize>,
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
        maximum_items: Option<usize>,
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
        maximum_items: Option<usize>,
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
        maximum_items: Option<usize>,
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
        maximum_items: Option<usize>,
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
