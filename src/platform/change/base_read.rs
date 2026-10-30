//! Exact canonical point reads used by change normalization.

use super::budget::{
    BudgetCounter, BudgetDimension, CanonicalReadAdmission, WitnessReadAdmission, WorkSaturation,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DependencyRecord, ExactOwnerKey, KernelSnapshot, OwnerKey, OwnerRecord, PackageId,
    PackageInterfaceRecord, RelationEdge, RelationEndpoint, RelationKind, RetirementRecord,
    SemanticRoot, TypeObject, TypeObjectDigest,
};
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use crate::platform::witness::{
    FullWitness, NamespaceKey, OwnerSummary, OwnerSummaryDigest, OwnershipEntry, TestDependency,
    ValidationWitnessManifest,
};
use bincode::{Decode, Encode};
use std::cell::Cell;

const READ_ADMISSION_UNSUPPORTED: &str = "base_read_admission_unsupported";
const READ_ADMISSION_DECODED_RECORDS: &str = "base_read_admission_decoded_records";

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct CanonicalReadWork {
    pub point_reads: u64,
    pub map_pages_read: u64,
    pub map_entries_visited: u64,
    pub catalog_lookups: u64,
    pub objects_read: u64,
    pub bytes_read: u64,
    pub canonical_records_decoded: u64,
    pub saturation: WorkSaturation,
}

impl CanonicalReadWork {
    pub fn add(&mut self, other: Self) {
        self.saturation.observe(
            &mut self.point_reads,
            BudgetDimension::CanonicalPointReads,
            BudgetCounter {
                consumed: other.point_reads,
                saturated: other
                    .saturation
                    .contains(BudgetDimension::CanonicalPointReads),
            },
        );
        self.saturation.observe(
            &mut self.map_pages_read,
            BudgetDimension::CanonicalMapPages,
            BudgetCounter {
                consumed: other.map_pages_read,
                saturated: other
                    .saturation
                    .contains(BudgetDimension::CanonicalMapPages),
            },
        );
        self.saturation.observe(
            &mut self.map_entries_visited,
            BudgetDimension::CanonicalMapEntries,
            BudgetCounter {
                consumed: other.map_entries_visited,
                saturated: other
                    .saturation
                    .contains(BudgetDimension::CanonicalMapEntries),
            },
        );
        self.saturation.observe(
            &mut self.catalog_lookups,
            BudgetDimension::CanonicalCatalogLookups,
            BudgetCounter {
                consumed: other.catalog_lookups,
                saturated: other
                    .saturation
                    .contains(BudgetDimension::CanonicalCatalogLookups),
            },
        );
        self.saturation.observe(
            &mut self.objects_read,
            BudgetDimension::CanonicalObjects,
            BudgetCounter {
                consumed: other.objects_read,
                saturated: other.saturation.contains(BudgetDimension::CanonicalObjects),
            },
        );
        self.saturation.observe(
            &mut self.bytes_read,
            BudgetDimension::CanonicalBytes,
            BudgetCounter {
                consumed: other.bytes_read,
                saturated: other.saturation.contains(BudgetDimension::CanonicalBytes),
            },
        );
        self.saturation.observe(
            &mut self.canonical_records_decoded,
            BudgetDimension::CanonicalDecodedRecords,
            BudgetCounter {
                consumed: other.canonical_records_decoded,
                saturated: other
                    .saturation
                    .contains(BudgetDimension::CanonicalDecodedRecords),
            },
        );
    }

    pub fn saturated(self) -> bool {
        self.saturation.any()
    }

    pub(crate) fn observation(self, dimension: BudgetDimension, consumed: u64) -> BudgetCounter {
        BudgetCounter {
            consumed,
            saturated: self.saturation.contains(dimension),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalRead<T> {
    pub value: T,
    pub work: CanonicalReadWork,
}

/// One independently admitted exact dependency interface, shared by all selectors in a request.
#[derive(Clone, Debug)]
pub struct CanonicalReferenceInterface {
    pub revision: crate::platform::package_transport::PackageRevision,
    pub owners: std::collections::BTreeMap<OwnerKey, PackageInterfaceRecord>,
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

#[derive(Clone, Copy, Debug, Decode, Default, Encode, Eq, PartialEq)]
pub struct WitnessReadWork {
    pub point_reads: u64,
    pub map_pages_read: u64,
    pub map_entries_visited: u64,
    pub catalog_lookups: u64,
    pub objects_read: u64,
    pub bytes_read: u64,
    pub witness_records_decoded: u64,
    pub saturation: WorkSaturation,
}

impl WitnessReadWork {
    pub fn add(&mut self, other: Self) {
        self.saturation.observe(
            &mut self.point_reads,
            BudgetDimension::WitnessPointReads,
            BudgetCounter {
                consumed: other.point_reads,
                saturated: other
                    .saturation
                    .contains(BudgetDimension::WitnessPointReads),
            },
        );
        self.saturation.observe(
            &mut self.map_pages_read,
            BudgetDimension::WitnessMapPages,
            BudgetCounter {
                consumed: other.map_pages_read,
                saturated: other.saturation.contains(BudgetDimension::WitnessMapPages),
            },
        );
        self.saturation.observe(
            &mut self.map_entries_visited,
            BudgetDimension::WitnessMapEntries,
            BudgetCounter {
                consumed: other.map_entries_visited,
                saturated: other
                    .saturation
                    .contains(BudgetDimension::WitnessMapEntries),
            },
        );
        self.saturation.observe(
            &mut self.catalog_lookups,
            BudgetDimension::WitnessCatalogLookups,
            BudgetCounter {
                consumed: other.catalog_lookups,
                saturated: other
                    .saturation
                    .contains(BudgetDimension::WitnessCatalogLookups),
            },
        );
        self.saturation.observe(
            &mut self.objects_read,
            BudgetDimension::WitnessObjects,
            BudgetCounter {
                consumed: other.objects_read,
                saturated: other.saturation.contains(BudgetDimension::WitnessObjects),
            },
        );
        self.saturation.observe(
            &mut self.bytes_read,
            BudgetDimension::WitnessBytes,
            BudgetCounter {
                consumed: other.bytes_read,
                saturated: other.saturation.contains(BudgetDimension::WitnessBytes),
            },
        );
        self.saturation.observe(
            &mut self.witness_records_decoded,
            BudgetDimension::WitnessDecodedRecords,
            BudgetCounter {
                consumed: other.witness_records_decoded,
                saturated: other
                    .saturation
                    .contains(BudgetDimension::WitnessDecodedRecords),
            },
        );
    }

    pub fn saturated(self) -> bool {
        self.saturation.any()
    }

    pub(crate) fn observation(self, dimension: BudgetDimension, consumed: u64) -> BudgetCounter {
        BudgetCounter {
            consumed,
            saturated: self.saturation.contains(dimension),
        }
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
    /// Interrupt in-memory analysis as well as physical reads at the owning operation's boundary.
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        Ok(())
    }
    fn read_reference_interface(
        &self,
        dependency: &DependencyRecord,
    ) -> Result<CanonicalRead<CanonicalReferenceInterface>, Diagnostic> {
        self.read_reference_interface_admitted(dependency, CanonicalReadAdmission::unbounded())
    }

    fn read_reference_interface_admitted(
        &self,
        _dependency: &DependencyRecord,
        _admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<CanonicalReferenceInterface>, Diagnostic> {
        Err(admission_unsupported("exact reference interface"))
    }

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

    /// Exact direct roots; consumers must apply proposal additions and removals before use.
    fn read_dependencies(&self) -> Result<CanonicalRead<Vec<DependencyRecord>>, Diagnostic> {
        self.read_dependencies_admitted(CanonicalReadAdmission::unbounded())
    }

    fn read_dependencies_admitted(
        &self,
        _admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Vec<DependencyRecord>>, Diagnostic> {
        Err(admission_unsupported("direct dependency inventory"))
    }

    /// Metadata authority under one exact direct root, without public import authority.
    fn read_representation_dependency(
        &self,
        root: &DependencyRecord,
        package: PackageId,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        self.read_representation_dependency_admitted(
            root,
            package,
            CanonicalReadAdmission::unbounded(),
        )
    }

    fn read_representation_dependency_admitted(
        &self,
        _root: &DependencyRecord,
        _package: PackageId,
        _admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        Err(admission_unsupported("private representation dependency"))
    }

    fn read_retirement(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic>;

    /// Performs an admitted owner read. The default fails before invoking an unbounded reader.
    fn read_owner_admitted(
        &self,
        _owner: OwnerKey,
        _admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
        Err(admission_unsupported("canonical owner"))
    }

    /// Performs an admitted type-object read. The default fails closed.
    fn read_type_object_admitted(
        &self,
        _digest: TypeObjectDigest,
        _admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        Err(admission_unsupported("canonical type object"))
    }

    /// Performs an admitted package-interface owner read. The default fails closed.
    fn read_package_interface_owner_admitted(
        &self,
        _dependency: &DependencyRecord,
        _owner: OwnerKey,
        _admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        Err(admission_unsupported("package-interface owner"))
    }

    /// Performs an admitted dependency read. The default fails closed.
    fn read_dependency_admitted(
        &self,
        _package: PackageId,
        _admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        Err(admission_unsupported("canonical dependency"))
    }

    /// Performs an admitted retirement read. The default fails closed.
    fn read_retirement_admitted(
        &self,
        _owner: OwnerKey,
        _admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic> {
        Err(admission_unsupported("canonical retirement"))
    }
}

/// Exact derived-witness reads required to classify the local effects of canonical owner edits.
pub trait WitnessBaseRead {
    fn witness_manifest(&self) -> &ValidationWitnessManifest;

    fn witness_repository_id(&self) -> RepositoryId;

    fn witness_package_id(&self) -> PackageId;

    fn witness_contract_is_current(&self) -> bool;

    /// Canonical namespace/ownership/summary projections may be rebuilt for repair even when
    /// semantic admission fails. This grants no incremental-validation or execution proof.
    fn canonical_facts_are_current(&self) -> bool {
        self.witness_contract_is_current()
    }

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
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic>;

    fn read_incoming_relations(
        &self,
        owner: OwnerKey,
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic>;

    fn read_incoming_relations_of_kind(
        &self,
        owner: OwnerKey,
        kind: RelationKind,
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic>;

    fn read_incoming_package_relations(
        &self,
        package: PackageId,
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic>;

    fn read_test_dependencies(
        &self,
        test: OwnerKey,
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessTestDependencyRead>, Diagnostic>;

    /// Performs an admitted namespace read. The default fails before an unbounded read.
    fn read_namespace_admitted(
        &self,
        _key: &NamespaceKey,
        _admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<Option<OwnerKey>>, Diagnostic> {
        Err(admission_unsupported("witness namespace"))
    }

    /// Performs an admitted ownership read. The default fails closed.
    fn read_ownership_admitted(
        &self,
        _owner: OwnerKey,
        _admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<Option<OwnershipEntry>>, Diagnostic> {
        Err(admission_unsupported("witness ownership"))
    }

    /// Performs an admitted forward-relation membership read. The default fails closed.
    fn contains_forward_relation_admitted(
        &self,
        _edge: RelationEdge,
        _admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<bool>, Diagnostic> {
        Err(admission_unsupported("witness forward relation"))
    }

    /// Performs an admitted owner-summary read. The default fails closed.
    fn read_owner_summary_admitted(
        &self,
        _owner: OwnerKey,
        _admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<Option<BoundOwnerSummary>>, Diagnostic> {
        Err(admission_unsupported("witness owner summary"))
    }

    /// Performs an admitted outgoing-relation prefix read. The default fails closed.
    fn read_outgoing_relations_admitted(
        &self,
        _owner: OwnerKey,
        _maximum_items: Option<usize>,
        _admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        Err(admission_unsupported("witness outgoing relations"))
    }

    /// Performs an admitted incoming-relation prefix read. The default fails closed.
    fn read_incoming_relations_admitted(
        &self,
        _owner: OwnerKey,
        _maximum_items: Option<usize>,
        _admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        Err(admission_unsupported("witness incoming relations"))
    }

    /// Performs an admitted incoming-relation prefix read within one exact relation kind.
    /// The default fails closed rather than reading an unrelated unbounded prefix.
    fn read_incoming_relations_of_kind_admitted(
        &self,
        _owner: OwnerKey,
        _kind: RelationKind,
        _maximum_items: Option<usize>,
        _admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        Err(admission_unsupported("witness incoming relations by kind"))
    }

    /// Performs an admitted foreign-package relation prefix read. The default fails closed.
    fn read_incoming_package_relations_admitted(
        &self,
        _package: PackageId,
        _maximum_items: Option<usize>,
        _admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        Err(admission_unsupported("witness package relations"))
    }

    /// Performs an admitted test-dependency prefix read. The default fails closed.
    fn read_test_dependencies_admitted(
        &self,
        _test: OwnerKey,
        _maximum_items: Option<usize>,
        _admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessTestDependencyRead>, Diagnostic> {
        Err(admission_unsupported("witness test dependencies"))
    }
}

/// Cumulative canonical-read admission over one immutable base.
///
/// The wrapper owns only request-local permission and observations. The wrapped source remains
/// the exact authority and must enforce the supplied operation-local limits before lower-level
/// map, object, byte, or decoder work.
pub(crate) struct BudgetedCanonicalBase<'a, B: ?Sized> {
    base: &'a B,
    admission: CanonicalReadAdmission,
    work: Cell<CanonicalReadWork>,
}

impl<'a, B: CanonicalBaseRead + ?Sized> BudgetedCanonicalBase<'a, B> {
    pub(crate) fn new(
        base: &'a B,
        admission: CanonicalReadAdmission,
        initial_work: CanonicalReadWork,
    ) -> Result<Self, Diagnostic> {
        admission.remaining_after(initial_work, "canonical base-read admission")?;
        Ok(Self {
            base,
            admission,
            work: Cell::new(initial_work),
        })
    }

    pub(crate) fn work(&self) -> CanonicalReadWork {
        self.work.get()
    }

    pub(crate) fn remaining(&self) -> Result<CanonicalReadAdmission, Diagnostic> {
        self.admission
            .remaining_after(self.work(), "canonical base-read admission")
    }

    /// Runs one source-specific logical point read through this cumulative admission.
    pub(crate) fn read_admitted<T>(
        &self,
        read: impl FnOnce(CanonicalReadAdmission) -> Result<CanonicalRead<T>, Diagnostic>,
    ) -> Result<CanonicalRead<T>, Diagnostic> {
        let remaining = self.remaining()?;
        remaining.check_work(
            CanonicalReadWork {
                point_reads: 1,
                ..CanonicalReadWork::default()
            },
            "canonical point-read admission",
        )?;
        let result = read(remaining).map_err(map_canonical_admission_diagnostic)?;
        if result.work.point_reads != 1 {
            return Err(base_read_error(
                DiagnosticClass::Corrupt,
                "change_canonical_read_point_work",
                "admitted canonical source did not report exactly one logical point read",
            ));
        }
        remaining
            .check_work(result.work, "canonical admitted source")
            .map_err(map_canonical_admission_diagnostic)?;
        let mut work = self.work();
        work.add(result.work);
        self.admission
            .check_work(work, "canonical cumulative base reads")?;
        self.work.set(work);
        Ok(result)
    }
}

impl<B: CanonicalBaseRead + ?Sized> CanonicalBaseRead for BudgetedCanonicalBase<'_, B> {
    fn read_dependencies(&self) -> Result<CanonicalRead<Vec<DependencyRecord>>, Diagnostic> {
        self.read_admitted(|admission| self.base.read_dependencies_admitted(admission))
    }

    fn read_representation_dependency(
        &self,
        root: &DependencyRecord,
        package: PackageId,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        self.read_admitted(|admission| {
            self.base
                .read_representation_dependency_admitted(root, package, admission)
        })
    }

    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        self.base.validation_checkpoint()
    }
    fn read_reference_interface(
        &self,
        dependency: &DependencyRecord,
    ) -> Result<CanonicalRead<CanonicalReferenceInterface>, Diagnostic> {
        self.read_admitted(|admission| {
            self.base
                .read_reference_interface_admitted(dependency, admission)
        })
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
        self.read_admitted(|admission| self.base.read_owner_admitted(owner, admission))
    }

    fn read_type_object(
        &self,
        digest: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        self.read_admitted(|admission| self.base.read_type_object_admitted(digest, admission))
    }

    fn read_package_interface_owner(
        &self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        self.read_admitted(|admission| {
            self.base
                .read_package_interface_owner_admitted(dependency, owner, admission)
        })
    }

    fn read_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        self.read_admitted(|admission| self.base.read_dependency_admitted(package, admission))
    }

    fn read_retirement(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic> {
        self.read_admitted(|admission| self.base.read_retirement_admitted(owner, admission))
    }
}

/// Cumulative validation-witness read admission over one immutable witness base.
pub(crate) struct BudgetedWitnessBase<'a, W: ?Sized> {
    base: &'a W,
    admission: WitnessReadAdmission,
    work: Cell<WitnessReadWork>,
}

impl<'a, W: WitnessBaseRead + ?Sized> BudgetedWitnessBase<'a, W> {
    pub(crate) fn new(
        base: &'a W,
        admission: WitnessReadAdmission,
        initial_work: WitnessReadWork,
    ) -> Result<Self, Diagnostic> {
        admission.remaining_after(initial_work, "witness base-read admission")?;
        Ok(Self {
            base,
            admission,
            work: Cell::new(initial_work),
        })
    }

    pub(crate) fn work(&self) -> WitnessReadWork {
        self.work.get()
    }

    pub(crate) fn remaining(&self) -> Result<WitnessReadAdmission, Diagnostic> {
        self.admission
            .remaining_after(self.work(), "witness base-read admission")
    }

    fn admitted<T>(
        &self,
        read: impl FnOnce(WitnessReadAdmission) -> Result<WitnessRead<T>, Diagnostic>,
    ) -> Result<WitnessRead<T>, Diagnostic> {
        let remaining = self.remaining()?;
        remaining.check_work(
            WitnessReadWork {
                point_reads: 1,
                ..WitnessReadWork::default()
            },
            "witness point-read admission",
        )?;
        let result = read(remaining).map_err(map_witness_admission_diagnostic)?;
        if result.work.point_reads != 1 {
            return Err(base_read_error(
                DiagnosticClass::Corrupt,
                "change_witness_read_point_work",
                "admitted witness source did not report exactly one logical point read",
            ));
        }
        remaining
            .check_work(result.work, "witness admitted source")
            .map_err(map_witness_admission_diagnostic)?;
        let mut work = self.work();
        work.add(result.work);
        self.admission
            .check_work(work, "witness cumulative base reads")?;
        self.work.set(work);
        Ok(result)
    }
}

impl<W: WitnessBaseRead + ?Sized> WitnessBaseRead for BudgetedWitnessBase<'_, W> {
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
        self.admitted(|admission| self.base.read_namespace_admitted(key, admission))
    }

    fn read_ownership(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<OwnershipEntry>>, Diagnostic> {
        self.admitted(|admission| self.base.read_ownership_admitted(owner, admission))
    }

    fn contains_forward_relation(
        &self,
        edge: RelationEdge,
    ) -> Result<WitnessRead<bool>, Diagnostic> {
        self.admitted(|admission| {
            self.base
                .contains_forward_relation_admitted(edge, admission)
        })
    }

    fn read_owner_summary(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<BoundOwnerSummary>>, Diagnostic> {
        self.admitted(|admission| self.base.read_owner_summary_admitted(owner, admission))
    }

    fn read_outgoing_relations(
        &self,
        owner: OwnerKey,
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        self.admitted(|admission| {
            self.base
                .read_outgoing_relations_admitted(owner, maximum_items, admission)
        })
    }

    fn read_incoming_relations(
        &self,
        owner: OwnerKey,
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        self.admitted(|admission| {
            self.base
                .read_incoming_relations_admitted(owner, maximum_items, admission)
        })
    }

    fn read_incoming_relations_of_kind(
        &self,
        owner: OwnerKey,
        kind: RelationKind,
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        self.admitted(|admission| {
            self.base.read_incoming_relations_of_kind_admitted(
                owner,
                kind,
                maximum_items,
                admission,
            )
        })
    }

    fn read_incoming_package_relations(
        &self,
        package: PackageId,
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        self.admitted(|admission| {
            self.base
                .read_incoming_package_relations_admitted(package, maximum_items, admission)
        })
    }

    fn read_test_dependencies(
        &self,
        test: OwnerKey,
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessTestDependencyRead>, Diagnostic> {
        self.admitted(|admission| {
            self.base
                .read_test_dependencies_admitted(test, maximum_items, admission)
        })
    }
}

impl CanonicalBaseRead for KernelSnapshot {
    fn read_dependencies_admitted(
        &self,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Vec<DependencyRecord>>, Diagnostic> {
        let work = CanonicalReadWork {
            point_reads: 1,
            canonical_records_decoded: u64::try_from(self.dependencies.len()).unwrap_or(u64::MAX),
            ..CanonicalReadWork::default()
        };
        admission.check_work(work, "in-memory direct dependency inventory")?;
        Ok(CanonicalRead {
            value: self.dependencies.values().cloned().collect(),
            work,
        })
    }

    fn read_representation_dependency_admitted(
        &self,
        root: &DependencyRecord,
        package: PackageId,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        canonical_memory_read(admission, || {
            let selected = self
                .dependencies
                .get(&root.package)
                .or_else(|| self.representation_dependencies.get(&root.package))?;
            if selected.package_revision != root.package_revision
                || selected.semantic_revision != root.semantic_revision
            {
                return None;
            }
            self.dependencies
                .get(&package)
                .or_else(|| self.representation_dependencies.get(&package))
                .cloned()
                .map(|mut dependency| {
                    dependency.graph_contract_version =
                        crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
                    dependency
                })
        })
    }

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
        self.read_owner_admitted(owner, CanonicalReadAdmission::unbounded())
    }

    fn read_owner_admitted(
        &self,
        owner: OwnerKey,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
        canonical_memory_read(admission, || self.owners.get(&owner).cloned())
    }

    fn read_type_object(
        &self,
        digest: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        self.read_type_object_admitted(digest, CanonicalReadAdmission::unbounded())
    }

    fn read_type_object_admitted(
        &self,
        digest: TypeObjectDigest,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        canonical_memory_read(admission, || {
            self.types
                .get(&digest)
                .or_else(|| self.dependency_types.get(&digest))
                .cloned()
        })
    }

    fn read_package_interface_owner(
        &self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        self.read_package_interface_owner_admitted(
            dependency,
            owner,
            CanonicalReadAdmission::unbounded(),
        )
    }

    fn read_package_interface_owner_admitted(
        &self,
        dependency: &DependencyRecord,
        owner: OwnerKey,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        canonical_memory_read(admission, || {
            self.dependency_interfaces
                .get(&dependency.package_revision)
                .and_then(|owners| owners.get(&owner))
                .cloned()
        })
    }

    fn read_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        self.read_dependency_admitted(package, CanonicalReadAdmission::unbounded())
    }

    fn read_dependency_admitted(
        &self,
        package: PackageId,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
        canonical_memory_read(admission, || self.dependencies.get(&package).cloned())
    }

    fn read_retirement(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic> {
        self.read_retirement_admitted(owner, CanonicalReadAdmission::unbounded())
    }

    fn read_retirement_admitted(
        &self,
        owner: OwnerKey,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic> {
        canonical_memory_read(admission, || self.retirements.get(&owner).cloned())
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
        self.read_namespace_admitted(key, WitnessReadAdmission::unbounded())
    }

    fn read_namespace_admitted(
        &self,
        key: &NamespaceKey,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<Option<OwnerKey>>, Diagnostic> {
        witness_memory_read(admission, 0, || self.entries.namespaces.get(key).copied())
    }

    fn read_ownership(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<OwnershipEntry>>, Diagnostic> {
        self.read_ownership_admitted(owner, WitnessReadAdmission::unbounded())
    }

    fn read_ownership_admitted(
        &self,
        owner: OwnerKey,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<Option<OwnershipEntry>>, Diagnostic> {
        witness_memory_read(admission, 0, || self.entries.ownership.get(&owner).copied())
    }

    fn contains_forward_relation(
        &self,
        edge: RelationEdge,
    ) -> Result<WitnessRead<bool>, Diagnostic> {
        self.contains_forward_relation_admitted(edge, WitnessReadAdmission::unbounded())
    }

    fn contains_forward_relation_admitted(
        &self,
        edge: RelationEdge,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<bool>, Diagnostic> {
        witness_memory_read(admission, 0, || {
            self.entries.relations.binary_search(&edge).is_ok()
        })
    }

    fn read_owner_summary(
        &self,
        owner: OwnerKey,
    ) -> Result<WitnessRead<Option<BoundOwnerSummary>>, Diagnostic> {
        self.read_owner_summary_admitted(owner, WitnessReadAdmission::unbounded())
    }

    fn read_owner_summary_admitted(
        &self,
        owner: OwnerKey,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<Option<BoundOwnerSummary>>, Diagnostic> {
        admit_witness_point(admission)?;
        let summary = self.summaries.get(&owner);
        let digest = self.entries.summaries.get(&owner).copied();
        match (summary, digest) {
            (Some(summary), Some(digest)) => {
                admission.check_work(
                    WitnessReadWork {
                        point_reads: 1,
                        witness_records_decoded: 1,
                        ..WitnessReadWork::default()
                    },
                    "in-memory witness summary read",
                )?;
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
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        self.read_outgoing_relations_admitted(
            owner,
            maximum_items,
            WitnessReadAdmission::unbounded(),
        )
    }

    fn read_outgoing_relations_admitted(
        &self,
        owner: OwnerKey,
        maximum_items: Option<usize>,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        read_memory_relations(self, owner, None, maximum_items, false, admission)
    }

    fn read_incoming_relations(
        &self,
        owner: OwnerKey,
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        self.read_incoming_relations_admitted(
            owner,
            maximum_items,
            WitnessReadAdmission::unbounded(),
        )
    }

    fn read_incoming_relations_admitted(
        &self,
        owner: OwnerKey,
        maximum_items: Option<usize>,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        read_memory_relations(self, owner, None, maximum_items, true, admission)
    }

    fn read_incoming_relations_of_kind(
        &self,
        owner: OwnerKey,
        kind: RelationKind,
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        self.read_incoming_relations_of_kind_admitted(
            owner,
            kind,
            maximum_items,
            WitnessReadAdmission::unbounded(),
        )
    }

    fn read_incoming_relations_of_kind_admitted(
        &self,
        owner: OwnerKey,
        kind: RelationKind,
        maximum_items: Option<usize>,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        read_memory_relations(self, owner, Some(kind), maximum_items, true, admission)
    }

    fn read_incoming_package_relations(
        &self,
        package: PackageId,
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        self.read_incoming_package_relations_admitted(
            package,
            maximum_items,
            WitnessReadAdmission::unbounded(),
        )
    }

    fn read_incoming_package_relations_admitted(
        &self,
        package: PackageId,
        maximum_items: Option<usize>,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
        admit_witness_point(admission)?;
        let mut edges = Vec::with_capacity(maximum_items.map_or(64, |maximum| maximum.min(64)));
        let mut truncated = false;
        for edge in self.entries.reverse_relations.iter().filter(|edge| {
            matches!(
                edge.target,
                RelationEndpoint::Owner(ExactOwnerKey {
                    package: target_package,
                    ..
                }) if target_package == package
            )
        }) {
            if maximum_items.is_some_and(|maximum| edges.len() >= maximum) {
                truncated = true;
                break;
            }
            let decoded = u64::try_from(edges.len())
                .unwrap_or(u64::MAX)
                .saturating_add(1);
            admission.check_work(
                WitnessReadWork {
                    point_reads: 1,
                    witness_records_decoded: decoded,
                    ..WitnessReadWork::default()
                },
                "in-memory witness package-relation read",
            )?;
            edges.push(*edge);
        }
        let returned = u64::try_from(edges.len()).unwrap_or(u64::MAX);
        Ok(WitnessRead::memory_records(
            WitnessRelationRead { edges, truncated },
            returned,
        ))
    }

    fn read_test_dependencies(
        &self,
        test: OwnerKey,
        maximum_items: Option<usize>,
    ) -> Result<WitnessRead<WitnessTestDependencyRead>, Diagnostic> {
        self.read_test_dependencies_admitted(test, maximum_items, WitnessReadAdmission::unbounded())
    }

    fn read_test_dependencies_admitted(
        &self,
        test: OwnerKey,
        maximum_items: Option<usize>,
        admission: WitnessReadAdmission,
    ) -> Result<WitnessRead<WitnessTestDependencyRead>, Diagnostic> {
        admit_witness_point(admission)?;
        if !matches!(test, OwnerKey::Declaration(_)) {
            return Err(base_read_error(
                DiagnosticClass::Source,
                "change_test_dependency_owner",
                "test-dependency lookup requires a declaration owner",
            ));
        }
        let available = self.entries.test_dependencies_by_test.get(&test);
        let available_count = available.map_or(0, |entries| entries.len());
        let returned =
            maximum_items.map_or(available_count, |maximum| available_count.min(maximum));
        admission.check_work(
            WitnessReadWork {
                point_reads: 1,
                witness_records_decoded: u64::try_from(returned).unwrap_or(u64::MAX),
                ..WitnessReadWork::default()
            },
            "in-memory witness test-dependency read",
        )?;
        Ok(WitnessRead::memory_records(
            WitnessTestDependencyRead {
                dependencies: available
                    .into_iter()
                    .flat_map(|entries| entries.iter().copied())
                    .take(returned)
                    .collect(),
                truncated: maximum_items.is_some_and(|maximum| available_count > maximum),
            },
            u64::try_from(returned).unwrap_or(u64::MAX),
        ))
    }
}

fn read_memory_relations(
    witness: &FullWitness,
    owner: OwnerKey,
    kind: Option<RelationKind>,
    maximum_items: Option<usize>,
    reverse: bool,
    admission: WitnessReadAdmission,
) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
    admit_witness_point(admission)?;
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
    let matching = if let Some(kind) = kind {
        let start = available.partition_point(|edge| edge.kind < kind);
        let end = available.partition_point(|edge| edge.kind <= kind);
        &available[start..end]
    } else {
        available
    };
    let returned = maximum_items.map_or(matching.len(), |maximum| matching.len().min(maximum));
    admission.check_work(
        WitnessReadWork {
            point_reads: 1,
            witness_records_decoded: u64::try_from(returned).unwrap_or(u64::MAX),
            ..WitnessReadWork::default()
        },
        "in-memory witness relation read",
    )?;
    Ok(WitnessRead::memory_records(
        WitnessRelationRead {
            edges: matching[..returned].to_vec(),
            truncated: maximum_items.is_some_and(|maximum| matching.len() > maximum),
        },
        returned as u64,
    ))
}

fn canonical_memory_read<T>(
    admission: CanonicalReadAdmission,
    value: impl FnOnce() -> T,
) -> Result<CanonicalRead<T>, Diagnostic> {
    let work = CanonicalReadWork {
        point_reads: 1,
        ..CanonicalReadWork::default()
    };
    admission.check_work(work, "in-memory canonical point read")?;
    Ok(CanonicalRead {
        value: value(),
        work,
    })
}

fn witness_memory_read<T>(
    admission: WitnessReadAdmission,
    witness_records_decoded: u64,
    value: impl FnOnce() -> T,
) -> Result<WitnessRead<T>, Diagnostic> {
    let work = WitnessReadWork {
        point_reads: 1,
        witness_records_decoded,
        ..WitnessReadWork::default()
    };
    admission.check_work(work, "in-memory witness point read")?;
    Ok(WitnessRead {
        value: value(),
        work,
    })
}

fn admit_witness_point(admission: WitnessReadAdmission) -> Result<(), Diagnostic> {
    admission.check_work(
        WitnessReadWork {
            point_reads: 1,
            ..WitnessReadWork::default()
        },
        "in-memory witness point read",
    )
}

fn map_canonical_admission_diagnostic(mut diagnostic: Diagnostic) -> Diagnostic {
    let mapped = match diagnostic.code.as_str() {
        "persistent_map_admission_pages_read" => Some("change_budget_canonical_map_pages"),
        "persistent_map_admission_entries_visited" => Some("change_budget_canonical_map_entries"),
        "persistent_map_admission_bytes_read" | "object_read_bytes_exhausted" => {
            Some("change_budget_canonical_bytes")
        }
        "object_read_catalog_lookups_exhausted" => Some("change_budget_canonical_catalog_lookups"),
        "object_read_objects_exhausted" => Some("change_budget_canonical_objects"),
        READ_ADMISSION_DECODED_RECORDS => Some("change_budget_canonical_decoded_records"),
        _ => None,
    };
    if let Some(code) = mapped {
        diagnostic.code = code.to_owned();
    }
    diagnostic
}

fn map_witness_admission_diagnostic(mut diagnostic: Diagnostic) -> Diagnostic {
    let mapped = match diagnostic.code.as_str() {
        "persistent_map_admission_pages_read" => Some("change_budget_witness_map_pages"),
        "persistent_map_admission_entries_visited" => Some("change_budget_witness_map_entries"),
        "persistent_map_admission_bytes_read" | "object_read_bytes_exhausted" => {
            Some("change_budget_witness_bytes")
        }
        "object_read_catalog_lookups_exhausted" => Some("change_budget_witness_catalog_lookups"),
        "object_read_objects_exhausted" => Some("change_budget_witness_objects"),
        READ_ADMISSION_DECODED_RECORDS => Some("change_budget_witness_decoded_records"),
        _ => None,
    };
    if let Some(code) = mapped {
        diagnostic.code = code.to_owned();
    }
    diagnostic
}

fn admission_unsupported(kind: &'static str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Infrastructure,
        READ_ADMISSION_UNSUPPORTED,
        format!("{kind} source does not implement pre-consumption read admission"),
    )
}

fn base_read_error(
    class: DiagnosticClass,
    code: &'static str,
    message: &'static str,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::witness::rebuild_full_witness;

    struct CountingCanonical<'a> {
        base: &'a KernelSnapshot,
        admitted_calls: Cell<u64>,
        raw_type_calls: Cell<u64>,
        inventory_records_returned: Cell<u64>,
        last_admission: Cell<Option<CanonicalReadAdmission>>,
        failure: Option<&'static str>,
    }

    impl CountingCanonical<'_> {
        fn admitted_calls(&self) -> u64 {
            self.admitted_calls.get()
        }
    }

    impl CanonicalBaseRead for CountingCanonical<'_> {
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
            self.base.read_owner(owner)
        }

        fn read_type_object(
            &self,
            digest: TypeObjectDigest,
        ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
            self.raw_type_calls
                .set(self.raw_type_calls.get().saturating_add(1));
            self.base.read_type_object(digest)
        }

        fn read_package_interface_owner(
            &self,
            dependency: &DependencyRecord,
            owner: OwnerKey,
        ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
            self.base.read_package_interface_owner(dependency, owner)
        }

        fn read_dependency(
            &self,
            package: PackageId,
        ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
            self.base.read_dependency(package)
        }

        fn read_dependencies_admitted(
            &self,
            admission: CanonicalReadAdmission,
        ) -> Result<CanonicalRead<Vec<DependencyRecord>>, Diagnostic> {
            self.admitted_calls.set(self.admitted_calls.get() + 1);
            self.last_admission.set(Some(admission));
            let result = self.base.read_dependencies_admitted(admission)?;
            self.inventory_records_returned
                .set(self.inventory_records_returned.get() + result.value.len() as u64);
            Ok(result)
        }

        fn read_representation_dependency_admitted(
            &self,
            root: &DependencyRecord,
            package: PackageId,
            admission: CanonicalReadAdmission,
        ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
            self.admitted_calls.set(self.admitted_calls.get() + 1);
            self.last_admission.set(Some(admission));
            self.base
                .read_representation_dependency_admitted(root, package, admission)
        }

        fn read_retirement(
            &self,
            owner: OwnerKey,
        ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic> {
            self.base.read_retirement(owner)
        }

        fn read_owner_admitted(
            &self,
            owner: OwnerKey,
            admission: CanonicalReadAdmission,
        ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
            self.admitted_calls
                .set(self.admitted_calls.get().saturating_add(1));
            self.last_admission.set(Some(admission));
            if let Some(code) = self.failure {
                return Err(Diagnostic::new(
                    DiagnosticClass::Resource,
                    code,
                    "injected admitted canonical failure",
                ));
            }
            self.base.read_owner_admitted(owner, admission)
        }
    }

    struct CountingWitness<'a> {
        base: &'a FullWitness,
        admitted_calls: Cell<u64>,
        raw_ownership_calls: Cell<u64>,
        last_admission: Cell<Option<WitnessReadAdmission>>,
        failure: Option<&'static str>,
    }

    impl CountingWitness<'_> {
        fn admitted_calls(&self) -> u64 {
            self.admitted_calls.get()
        }
    }

    impl WitnessBaseRead for CountingWitness<'_> {
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

        fn owner_summary_count(&self) -> u64 {
            self.base.owner_summary_count()
        }

        fn read_namespace(
            &self,
            key: &NamespaceKey,
        ) -> Result<WitnessRead<Option<OwnerKey>>, Diagnostic> {
            self.base.read_namespace(key)
        }

        fn read_ownership(
            &self,
            owner: OwnerKey,
        ) -> Result<WitnessRead<Option<OwnershipEntry>>, Diagnostic> {
            self.raw_ownership_calls
                .set(self.raw_ownership_calls.get().saturating_add(1));
            self.base.read_ownership(owner)
        }

        fn contains_forward_relation(
            &self,
            edge: RelationEdge,
        ) -> Result<WitnessRead<bool>, Diagnostic> {
            self.base.contains_forward_relation(edge)
        }

        fn read_owner_summary(
            &self,
            owner: OwnerKey,
        ) -> Result<WitnessRead<Option<BoundOwnerSummary>>, Diagnostic> {
            self.base.read_owner_summary(owner)
        }

        fn read_outgoing_relations(
            &self,
            owner: OwnerKey,
            maximum_items: Option<usize>,
        ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
            self.base.read_outgoing_relations(owner, maximum_items)
        }

        fn read_incoming_relations(
            &self,
            owner: OwnerKey,
            maximum_items: Option<usize>,
        ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
            self.base.read_incoming_relations(owner, maximum_items)
        }

        fn read_incoming_relations_of_kind(
            &self,
            owner: OwnerKey,
            kind: RelationKind,
            maximum_items: Option<usize>,
        ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
            self.base
                .read_incoming_relations_of_kind(owner, kind, maximum_items)
        }

        fn read_incoming_package_relations(
            &self,
            package: PackageId,
            maximum_items: Option<usize>,
        ) -> Result<WitnessRead<WitnessRelationRead>, Diagnostic> {
            self.base
                .read_incoming_package_relations(package, maximum_items)
        }

        fn read_test_dependencies(
            &self,
            test: OwnerKey,
            maximum_items: Option<usize>,
        ) -> Result<WitnessRead<WitnessTestDependencyRead>, Diagnostic> {
            self.base.read_test_dependencies(test, maximum_items)
        }

        fn read_namespace_admitted(
            &self,
            key: &NamespaceKey,
            admission: WitnessReadAdmission,
        ) -> Result<WitnessRead<Option<OwnerKey>>, Diagnostic> {
            self.admitted_calls
                .set(self.admitted_calls.get().saturating_add(1));
            self.last_admission.set(Some(admission));
            if let Some(code) = self.failure {
                return Err(Diagnostic::new(
                    DiagnosticClass::Resource,
                    code,
                    "injected admitted witness failure",
                ));
            }
            self.base.read_namespace_admitted(key, admission)
        }
    }

    fn canonical_source<'a>(
        base: &'a KernelSnapshot,
        failure: Option<&'static str>,
    ) -> CountingCanonical<'a> {
        CountingCanonical {
            base,
            admitted_calls: Cell::new(0),
            raw_type_calls: Cell::new(0),
            inventory_records_returned: Cell::new(0),
            last_admission: Cell::new(None),
            failure,
        }
    }

    fn representation_snapshot() -> (KernelSnapshot, DependencyRecord, DependencyRecord) {
        let mut base = crate::platform::kernel::tests::witness_snapshot();
        base.dependencies.clear();
        base.representation_dependencies.clear();
        let dependency = |ordinal: u8| DependencyRecord {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            package: PackageId::migrate(b"representation-base-read", u64::from(ordinal)),
            semantic_revision: RevisionId::from_digest([ordinal; 32]),
            package_revision: crate::platform::kernel::PackageRevisionDigest::from_bytes(
                [ordinal; 32],
            ),
        };
        let direct = dependency(1);
        let sibling = dependency(2);
        let private = dependency(3);
        base.dependencies.insert(direct.package, direct.clone());
        base.dependencies.insert(sibling.package, sibling);
        base.representation_dependencies
            .insert(private.package, private.clone());
        (base, direct, private)
    }

    #[test]
    fn direct_inventory_does_not_grant_private_transitive_import_authority() {
        let (base, direct, private) = representation_snapshot();
        let inventory = base.read_dependencies().unwrap();
        assert_eq!(inventory.value.len(), 2);
        assert!(inventory.value.contains(&direct));
        assert!(!inventory.value.contains(&private));
        assert_eq!(inventory.work.canonical_records_decoded, 2);
        assert!(
            base.read_dependency(private.package)
                .unwrap()
                .value
                .is_none()
        );
        assert_eq!(
            base.read_representation_dependency(&direct, private.package)
                .unwrap()
                .value,
            Some(private.clone())
        );

        // Exercise each independently authenticated identity dimension.
        let mut stale = direct.clone();
        stale.package_revision =
            crate::platform::kernel::PackageRevisionDigest::from_bytes([99; 32]);
        assert!(
            base.read_representation_dependency(&stale, private.package)
                .unwrap()
                .value
                .is_none()
        );
        let mut stale = direct.clone();
        stale.semantic_revision = RevisionId::from_digest([99; 32]);
        assert!(
            base.read_representation_dependency(&stale, private.package)
                .unwrap()
                .value
                .is_none()
        );
        let mut absent = direct;
        absent.package = PackageId::migrate(b"representation-base-read", 99);
        assert!(
            base.read_representation_dependency(&absent, private.package)
                .unwrap()
                .value
                .is_none()
        );
    }

    #[test]
    fn representation_reads_obey_zero_and_cumulative_point_admission_before_source() {
        let (base, direct, private) = representation_snapshot();
        let source = canonical_source(&base, None);
        let zero = BudgetedCanonicalBase::new(
            &source,
            CanonicalReadAdmission {
                maximum_point_reads: Some(0),
                ..CanonicalReadAdmission::unbounded()
            },
            CanonicalReadWork::default(),
        )
        .unwrap();
        assert_eq!(
            zero.read_dependencies().unwrap_err().code,
            "change_budget_canonical_point_reads"
        );
        assert_eq!(
            zero.read_representation_dependency(&direct, private.package)
                .unwrap_err()
                .code,
            "change_budget_canonical_point_reads"
        );
        assert_eq!(source.admitted_calls(), 0);
        assert_eq!(zero.work(), CanonicalReadWork::default());

        let admitted = BudgetedCanonicalBase::new(
            &source,
            CanonicalReadAdmission {
                maximum_point_reads: Some(2),
                ..CanonicalReadAdmission::unbounded()
            },
            CanonicalReadWork {
                point_reads: 1,
                ..CanonicalReadWork::default()
            },
        )
        .unwrap();
        assert_eq!(
            admitted
                .read_representation_dependency(&direct, private.package)
                .unwrap()
                .value,
            Some(private)
        );
        assert_eq!(source.admitted_calls(), 1);
        assert_eq!(admitted.work().point_reads, 2);
        assert_eq!(
            admitted.read_dependencies().unwrap_err().code,
            "change_budget_canonical_point_reads"
        );
        assert_eq!(source.admitted_calls(), 1);
    }

    #[test]
    fn direct_inventory_admits_complete_record_count_before_returning_any_records() {
        let (base, _, _) = representation_snapshot();
        let source = canonical_source(&base, None);
        let zero = BudgetedCanonicalBase::new(
            &source,
            CanonicalReadAdmission {
                maximum_decoded_records: Some(0),
                ..CanonicalReadAdmission::unbounded()
            },
            CanonicalReadWork::default(),
        )
        .unwrap();
        assert_eq!(
            zero.read_dependencies().unwrap_err().code,
            "change_budget_canonical_decoded_records"
        );
        assert_eq!(source.inventory_records_returned.get(), 0);
        assert_eq!(zero.work(), CanonicalReadWork::default());

        let source = canonical_source(&base, None);
        let admitted = BudgetedCanonicalBase::new(
            &source,
            CanonicalReadAdmission {
                maximum_decoded_records: Some(3),
                ..CanonicalReadAdmission::unbounded()
            },
            CanonicalReadWork {
                canonical_records_decoded: 1,
                ..CanonicalReadWork::default()
            },
        )
        .unwrap();
        assert_eq!(admitted.read_dependencies().unwrap().value.len(), 2);
        assert_eq!(admitted.work().canonical_records_decoded, 3);
        assert_eq!(source.inventory_records_returned.get(), 2);
        let work = admitted.work();
        assert_eq!(
            admitted.read_dependencies().unwrap_err().code,
            "change_budget_canonical_decoded_records"
        );
        assert_eq!(source.inventory_records_returned.get(), 2);
        assert_eq!(admitted.work(), work);
        assert_eq!(
            source.last_admission.get().unwrap().maximum_decoded_records,
            Some(0)
        );
    }

    fn witness_source<'a>(
        base: &'a FullWitness,
        failure: Option<&'static str>,
    ) -> CountingWitness<'a> {
        CountingWitness {
            base,
            admitted_calls: Cell::new(0),
            raw_ownership_calls: Cell::new(0),
            last_admission: Cell::new(None),
            failure,
        }
    }

    #[test]
    fn canonical_zero_point_rejects_before_source_and_cumulative_work_includes_initial() {
        let base = crate::platform::kernel::tests::witness_snapshot();
        let owner = *base.owners.keys().next().expect("fixture owner");

        let zero_source = canonical_source(&base, None);
        let zero = BudgetedCanonicalBase::new(
            &zero_source,
            CanonicalReadAdmission {
                maximum_point_reads: Some(0),
                ..CanonicalReadAdmission::unbounded()
            },
            CanonicalReadWork::default(),
        )
        .expect("valid zero-point admission");
        let error = zero
            .read_owner(owner)
            .expect_err("zero point budget must reject");
        assert_eq!(error.code, "change_budget_canonical_point_reads");
        assert_eq!(zero_source.admitted_calls(), 0);
        assert_eq!(zero.work(), CanonicalReadWork::default());

        let source = canonical_source(&base, None);
        let initial = CanonicalReadWork {
            point_reads: 1,
            map_pages_read: 2,
            bytes_read: 3,
            ..CanonicalReadWork::default()
        };
        let admitted = BudgetedCanonicalBase::new(
            &source,
            CanonicalReadAdmission {
                maximum_point_reads: Some(2),
                maximum_map_pages: Some(7),
                maximum_bytes: Some(11),
                ..CanonicalReadAdmission::unbounded()
            },
            initial,
        )
        .expect("valid cumulative admission");
        admitted.read_owner(owner).expect("one remaining read");
        assert_eq!(admitted.work().point_reads, 2);
        assert_eq!(source.admitted_calls(), 1);
        let source_admission = source.last_admission.get().expect("source admission");
        assert_eq!(source_admission.maximum_point_reads, Some(1));
        assert_eq!(source_admission.maximum_map_pages, Some(5));
        assert_eq!(source_admission.maximum_bytes, Some(8));
        let error = admitted
            .read_owner(owner)
            .expect_err("cumulative point budget must reject a second source call");
        assert_eq!(error.code, "change_budget_canonical_point_reads");
        assert_eq!(source.admitted_calls(), 1);
        assert_eq!(admitted.work().point_reads, 2);
    }

    #[test]
    fn canonical_default_admitted_source_fails_without_calling_raw_reader() {
        let base = crate::platform::kernel::tests::witness_snapshot();
        let digest = *base.types.keys().next().expect("fixture type");
        let source = canonical_source(&base, None);
        let admitted = BudgetedCanonicalBase::new(
            &source,
            CanonicalReadAdmission::unbounded(),
            CanonicalReadWork::default(),
        )
        .expect("unbounded wrapper");
        let error = admitted
            .read_type_object(digest)
            .expect_err("missing admitted source implementation must fail closed");
        assert_eq!(error.code, READ_ADMISSION_UNSUPPORTED);
        assert_eq!(source.raw_type_calls.get(), 0);
    }

    #[test]
    fn canonical_wrapper_maps_generic_lower_read_exhaustion() {
        let base = crate::platform::kernel::tests::witness_snapshot();
        let owner = *base.owners.keys().next().expect("fixture owner");
        for (generic, expected) in [
            (
                "persistent_map_admission_pages_read",
                "change_budget_canonical_map_pages",
            ),
            (
                "persistent_map_admission_entries_visited",
                "change_budget_canonical_map_entries",
            ),
            (
                "persistent_map_admission_bytes_read",
                "change_budget_canonical_bytes",
            ),
            (
                "object_read_catalog_lookups_exhausted",
                "change_budget_canonical_catalog_lookups",
            ),
            (
                "object_read_objects_exhausted",
                "change_budget_canonical_objects",
            ),
            (
                "object_read_bytes_exhausted",
                "change_budget_canonical_bytes",
            ),
            (
                READ_ADMISSION_DECODED_RECORDS,
                "change_budget_canonical_decoded_records",
            ),
        ] {
            let source = canonical_source(&base, Some(generic));
            let admitted = BudgetedCanonicalBase::new(
                &source,
                CanonicalReadAdmission::unbounded(),
                CanonicalReadWork::default(),
            )
            .expect("unbounded wrapper");
            let error = admitted
                .read_owner(owner)
                .expect_err("injected lower admission failure");
            assert_eq!(error.code, expected);
            assert_eq!(source.admitted_calls(), 1);
            assert_eq!(admitted.work(), CanonicalReadWork::default());
        }
    }

    #[test]
    fn witness_zero_point_rejects_before_source_and_cumulative_work_includes_initial() {
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let base = rebuild_full_witness(&snapshot).expect("fixture witness");
        let key = base
            .entries
            .namespaces
            .keys()
            .next()
            .expect("fixture namespace")
            .clone();

        let zero_source = witness_source(&base, None);
        let zero = BudgetedWitnessBase::new(
            &zero_source,
            WitnessReadAdmission {
                maximum_point_reads: Some(0),
                ..WitnessReadAdmission::unbounded()
            },
            WitnessReadWork::default(),
        )
        .expect("valid zero-point admission");
        let error = zero
            .read_namespace(&key)
            .expect_err("zero point budget must reject");
        assert_eq!(error.code, "change_budget_witness_point_reads");
        assert_eq!(zero_source.admitted_calls(), 0);
        assert_eq!(zero.work(), WitnessReadWork::default());

        let source = witness_source(&base, None);
        let initial = WitnessReadWork {
            point_reads: 1,
            map_entries_visited: 2,
            bytes_read: 3,
            ..WitnessReadWork::default()
        };
        let admitted = BudgetedWitnessBase::new(
            &source,
            WitnessReadAdmission {
                maximum_point_reads: Some(2),
                maximum_map_entries: Some(7),
                maximum_bytes: Some(11),
                ..WitnessReadAdmission::unbounded()
            },
            initial,
        )
        .expect("valid cumulative admission");
        admitted.read_namespace(&key).expect("one remaining read");
        assert_eq!(admitted.work().point_reads, 2);
        assert_eq!(source.admitted_calls(), 1);
        let source_admission = source.last_admission.get().expect("source admission");
        assert_eq!(source_admission.maximum_point_reads, Some(1));
        assert_eq!(source_admission.maximum_map_entries, Some(5));
        assert_eq!(source_admission.maximum_bytes, Some(8));
        let error = admitted
            .read_namespace(&key)
            .expect_err("cumulative point budget must reject a second source call");
        assert_eq!(error.code, "change_budget_witness_point_reads");
        assert_eq!(source.admitted_calls(), 1);
        assert_eq!(admitted.work().point_reads, 2);
    }

    #[test]
    fn witness_default_admitted_source_fails_without_calling_raw_reader() {
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let base = rebuild_full_witness(&snapshot).expect("fixture witness");
        let owner = *base.entries.ownership.keys().next().expect("fixture owner");
        let source = witness_source(&base, None);
        let admitted = BudgetedWitnessBase::new(
            &source,
            WitnessReadAdmission::unbounded(),
            WitnessReadWork::default(),
        )
        .expect("unbounded wrapper");
        let error = admitted
            .read_ownership(owner)
            .expect_err("missing admitted source implementation must fail closed");
        assert_eq!(error.code, READ_ADMISSION_UNSUPPORTED);
        assert_eq!(source.raw_ownership_calls.get(), 0);
    }

    #[test]
    fn witness_wrapper_maps_generic_lower_read_exhaustion() {
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let base = rebuild_full_witness(&snapshot).expect("fixture witness");
        let key = base
            .entries
            .namespaces
            .keys()
            .next()
            .expect("fixture namespace")
            .clone();
        for (generic, expected) in [
            (
                "persistent_map_admission_pages_read",
                "change_budget_witness_map_pages",
            ),
            (
                "persistent_map_admission_entries_visited",
                "change_budget_witness_map_entries",
            ),
            (
                "persistent_map_admission_bytes_read",
                "change_budget_witness_bytes",
            ),
            (
                "object_read_catalog_lookups_exhausted",
                "change_budget_witness_catalog_lookups",
            ),
            (
                "object_read_objects_exhausted",
                "change_budget_witness_objects",
            ),
            ("object_read_bytes_exhausted", "change_budget_witness_bytes"),
            (
                READ_ADMISSION_DECODED_RECORDS,
                "change_budget_witness_decoded_records",
            ),
        ] {
            let source = witness_source(&base, Some(generic));
            let admitted = BudgetedWitnessBase::new(
                &source,
                WitnessReadAdmission::unbounded(),
                WitnessReadWork::default(),
            )
            .expect("unbounded wrapper");
            let error = admitted
                .read_namespace(&key)
                .expect_err("injected lower admission failure");
            assert_eq!(error.code, expected);
            assert_eq!(source.admitted_calls(), 1);
            assert_eq!(admitted.work(), WitnessReadWork::default());
        }
    }

    #[test]
    fn in_memory_witness_decoded_record_limit_is_checked_before_clone() {
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let base = rebuild_full_witness(&snapshot).expect("fixture witness");
        let owner = *base.summaries.keys().next().expect("fixture summary");
        let admitted = BudgetedWitnessBase::new(
            &base,
            WitnessReadAdmission {
                maximum_decoded_records: Some(0),
                ..WitnessReadAdmission::unbounded()
            },
            WitnessReadWork::default(),
        )
        .expect("valid decoded-record admission");
        let error = admitted
            .read_owner_summary(owner)
            .expect_err("summary clone must not exceed decoded-record admission");
        assert_eq!(error.code, "change_budget_witness_decoded_records");
        assert_eq!(admitted.work(), WitnessReadWork::default());
    }
}
