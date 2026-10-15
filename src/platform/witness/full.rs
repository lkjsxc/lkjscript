//! Deterministic full witness reconstruction and Merkle-map materialization.

use super::codec::{bind_witness_manifest, encode_owner_summary};
use super::entry::{
    NamespaceKey, OwnershipEntry, TestDependency, encode_ownership, forward_relation_key,
    owner_key_bytes, owner_value_bytes, reverse_relation_key, test_dependency_keys,
};
use super::ownership::{derive_namespaces, derive_ownership, derive_test_dependencies};
use super::summary::{OwnerSummary, SummaryBinding, ValidationWitnessManifest, WitnessRoots};
use super::summary_build::build_owner_summaries;
use super::{OwnerSummaryDigest, ValidationWitnessDigest, witness_error};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    FullValidationReport, KernelSnapshot, OwnerKey, RelationEdge, extract_relations, validate_full,
};
use crate::platform::persistent_map::{
    MapError, MapErrorClass, MapRoot, MapWork, MemoryPageStore, PersistentMap,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WitnessEntries {
    pub summaries: BTreeMap<OwnerKey, OwnerSummaryDigest>,
    pub namespaces: BTreeMap<NamespaceKey, OwnerKey>,
    pub ownership: BTreeMap<OwnerKey, OwnershipEntry>,
    pub relations: Vec<RelationEdge>,
    /// The same exact edges ordered by `(target, kind, source)` for bounded reverse impact reads.
    pub reverse_relations: Vec<RelationEdge>,
    pub test_dependencies: BTreeSet<TestDependency>,
    /// Prefix-oriented in-memory view matching the persisted forward test-dependency map.
    pub test_dependencies_by_test: BTreeMap<OwnerKey, BTreeSet<TestDependency>>,
}

#[derive(Clone, Debug)]
pub struct FullWitness {
    pub manifest: ValidationWitnessManifest,
    pub manifest_digest: ValidationWitnessDigest,
    pub manifest_bytes: Vec<u8>,
    pub summaries: BTreeMap<OwnerKey, OwnerSummary>,
    pub summary_objects: BTreeMap<OwnerSummaryDigest, Vec<u8>>,
    pub entries: WitnessEntries,
    pub pages: MemoryPageStore,
    pub report: WitnessBuildReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WitnessBuildReport {
    pub full_validation: FullValidationReport,
    pub owners_summarized: u64,
    pub namespace_entries: u64,
    pub ownership_entries: u64,
    pub relation_edges: u64,
    pub test_dependency_entries: u64,
    pub summary_objects: u64,
    pub map_pages: u64,
    pub map_bytes: u64,
    pub map_work: MapWork,
}

pub fn rebuild_full_witness(snapshot: &KernelSnapshot) -> Result<FullWitness, Vec<Diagnostic>> {
    let full_validation = validate_full(snapshot)?;
    build_validated_witness(snapshot, full_validation).map_err(|diagnostic| vec![diagnostic])
}

pub(crate) fn rebuild_full_witness_with_limit(
    snapshot: &KernelSnapshot,
    maximum_work: usize,
) -> Result<FullWitness, Vec<Diagnostic>> {
    rebuild_full_witness_checked(snapshot, maximum_work, &|| Ok(()))
}

#[cfg(test)]
std::thread_local! {
    static CHECKED_FULL_VALIDATIONS: std::cell::Cell<usize> = const {
        std::cell::Cell::new(0)
    };
}

#[cfg(test)]
pub(crate) fn checked_full_validation_invocations() -> usize {
    CHECKED_FULL_VALIDATIONS.with(std::cell::Cell::get)
}

pub(crate) fn rebuild_full_witness_checked(
    snapshot: &KernelSnapshot,
    maximum_work: usize,
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
) -> Result<FullWitness, Vec<Diagnostic>> {
    let validation =
        crate::platform::kernel::validate_full_checked(snapshot, maximum_work, &mut 0, checkpoint)?;
    #[cfg(test)]
    CHECKED_FULL_VALIDATIONS.with(|count| count.set(count.get().saturating_add(1)));
    let witness =
        build_validated_witness(snapshot, validation).map_err(|diagnostic| vec![diagnostic])?;
    checkpoint().map_err(|error| vec![error])?;
    Ok(witness)
}

/// A successful current full validation, scoped by the source reader's opaque exact binding.
/// Its report cannot be supplied by a caller or a historical producer certificate.
#[derive(Debug)]
pub(crate) struct CompletedSourceValidation {
    binding: crate::platform::package_transport::source::SourceValidationBinding,
    report: FullValidationReport,
    modeled_rebuild_work: u64,
}

impl CompletedSourceValidation {
    pub(crate) fn source_binding(
        &self,
    ) -> &crate::platform::package_transport::source::SourceValidationBinding {
        &self.binding
    }

    pub(crate) fn modeled_rebuild_work(&self) -> u64 {
        self.modeled_rebuild_work
    }
}

fn modeled_source_rebuild_work(witness: &FullWitness) -> Result<u64, Diagnostic> {
    let report = &witness.report;
    // Reserve current construction using complete accepted cardinalities and actual canonical
    // map work. This is a modeled cardinality/map-work reservation, not an exact observation of every
    // legacy summary/ownership traversal or host allocation. Existing current source/profile
    // and projection reservations remain; no witness bytes or variable records are cached.
    [
        (report.owners_summarized, 16),
        (report.full_validation.type_objects_checked, 8),
        (report.full_validation.expression_records_checked, 4),
        (report.full_validation.dependencies_checked, 4),
        (report.full_validation.retirements_checked, 4),
        (report.namespace_entries, 2),
        (report.ownership_entries, 2),
        (report.relation_edges, 8),
        (report.test_dependency_entries, 4),
        (report.summary_objects, 2),
        (report.map_pages, 4),
        (report.map_work.key_comparisons, 1),
        (report.map_work.entries_visited, 1),
    ]
    .into_iter()
    .try_fold(8_u64, |total, (count, weight)| {
        count.checked_mul(weight)?.checked_add(total)
    })
    .ok_or_else(|| {
        witness_error(
            crate::platform::diagnostic::DiagnosticClass::Resource,
            "witness_source_rebuild_work",
            "modeled current source witness reconstruction work overflowed",
        )
    })
}

pub(crate) fn rebuild_full_witness_checked_for_source(
    snapshot: &KernelSnapshot,
    maximum_work: usize,
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
    binding: crate::platform::package_transport::source::SourceValidationBinding,
) -> Result<(FullWitness, CompletedSourceValidation), Vec<Diagnostic>> {
    let witness = rebuild_full_witness_checked(snapshot, maximum_work, checkpoint)?;
    // Mint only after real full validation, canonical facts and the end checkpoint succeed.
    let completed = CompletedSourceValidation {
        binding,
        report: witness.report.full_validation.clone(),
        modeled_rebuild_work: modeled_source_rebuild_work(&witness).map_err(|error| vec![error])?,
    };
    Ok((witness, completed))
}

pub(crate) fn rebuild_full_witness_from_source_validation(
    snapshot: &KernelSnapshot,
    binding: &crate::platform::package_transport::source::SourceValidationBinding,
    completed: &CompletedSourceValidation,
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
) -> Result<FullWitness, Vec<Diagnostic>> {
    checkpoint().map_err(|error| vec![error])?;
    if &completed.binding != binding {
        return Err(vec![witness_error(
            crate::platform::diagnostic::DiagnosticClass::Corrupt,
            "witness_source_validation_binding",
            "a completed local validation cannot certify another exact source binding",
        )]);
    }
    // The source collector independently reconstructed all current local and attached foreign
    // inputs before this call. Rebuild canonical facts and current bytes, retaining the original
    // report as historical proof metadata; the collector charges current executed work separately.
    let witness =
        build_validated_witness(snapshot, completed.report.clone()).map_err(|error| vec![error])?;
    checkpoint().map_err(|error| vec![error])?;
    Ok(witness)
}

fn build_validated_witness(
    snapshot: &KernelSnapshot,
    full_validation: FullValidationReport,
) -> Result<FullWitness, Diagnostic> {
    let CanonicalWitnessFacts {
        summaries,
        summary_objects,
        entries,
        pages,
        map_work,
        roots,
    } = rebuild_canonical_facts(snapshot)?;
    let (semantic_root, _) = crate::platform::kernel::encode_root(&snapshot.root)?;
    let (manifest, manifest_digest, manifest_bytes) = bind_witness_manifest(
        snapshot.root.repository_id,
        snapshot.root.package_id,
        semantic_root,
        roots,
    )?;
    let report = WitnessBuildReport {
        full_validation,
        owners_summarized: summaries.len() as u64,
        namespace_entries: entries.namespaces.len() as u64,
        ownership_entries: entries.ownership.len() as u64,
        relation_edges: entries.relations.len() as u64,
        test_dependency_entries: entries.test_dependencies.len() as u64,
        summary_objects: summary_objects.len() as u64,
        map_pages: pages.object_count() as u64,
        map_bytes: pages.stored_bytes() as u64,
        map_work,
    };
    Ok(FullWitness {
        manifest,
        manifest_digest,
        manifest_bytes,
        summaries,
        summary_objects,
        entries,
        pages,
        report,
    })
}

/// Canonical projections without a semantic certificate. Independent semantic admission is
/// required before these derived facts can authorize a published witness.
#[derive(Debug)]
pub(crate) struct CanonicalWitnessFacts {
    pub summaries: BTreeMap<OwnerKey, OwnerSummary>,
    pub summary_objects: BTreeMap<OwnerSummaryDigest, Vec<u8>>,
    pub entries: WitnessEntries,
    pub pages: MemoryPageStore,
    pub map_work: MapWork,
    pub roots: WitnessRoots,
}

pub(crate) fn rebuild_canonical_facts(
    snapshot: &KernelSnapshot,
) -> Result<CanonicalWitnessFacts, Diagnostic> {
    let relations = extract_relations(
        snapshot.root.package_id,
        &snapshot.owners,
        &snapshot.types,
        &snapshot.dependencies,
    )?;
    let ownership = derive_ownership(snapshot)?;
    let namespaces = derive_namespaces(snapshot)?;
    let summaries = build_owner_summaries(snapshot, &ownership, &relations)?;

    let mut summary_objects = BTreeMap::new();
    let mut summary_bindings = BTreeMap::new();
    for (owner, summary) in &summaries {
        let (digest, bytes) = encode_owner_summary(summary)?;
        if let Some(previous) = summary_objects.insert(digest, bytes.clone())
            && previous != bytes
        {
            return Err(witness_error(
                DiagnosticClass::Corrupt,
                "witness_summary_collision",
                "one summary digest is bound to different canonical bytes",
            ));
        }
        summary_bindings.insert(*owner, digest);
    }

    let test_dependencies = derive_test_dependencies(snapshot, &ownership, &relations)?;
    let mut test_dependencies_by_test = BTreeMap::<OwnerKey, BTreeSet<TestDependency>>::new();
    for dependency in &test_dependencies {
        test_dependencies_by_test
            .entry(dependency.test)
            .or_default()
            .insert(*dependency);
    }
    let mut reverse_relations = relations.clone();
    reverse_relations.sort_unstable_by_key(|edge| (edge.target, edge.kind, edge.source));
    let entries = WitnessEntries {
        summaries: summary_bindings,
        namespaces,
        ownership,
        relations,
        reverse_relations,
        test_dependencies,
        test_dependencies_by_test,
    };

    let mut pages = MemoryPageStore::default();
    let mut map_work = MapWork::default();
    let roots = build_witness_maps(&entries, &summaries, &mut pages, &mut map_work)?;

    Ok(CanonicalWitnessFacts {
        summaries,
        summary_objects,
        entries,
        pages,
        map_work,
        roots,
    })
}

fn build_witness_maps(
    entries: &WitnessEntries,
    summaries: &BTreeMap<OwnerKey, OwnerSummary>,
    store: &mut MemoryPageStore,
    work: &mut MapWork,
) -> Result<WitnessRoots, Diagnostic> {
    let owner_summary_entries = entries
        .summaries
        .iter()
        .map(|(owner, digest)| {
            summaries
                .get(owner)
                .ok_or_else(|| {
                    witness_error(
                        DiagnosticClass::Corrupt,
                        "witness_summary_binding_missing",
                        "summary binding has no corresponding owner summary",
                    )
                })
                .map(|summary| {
                    (
                        owner_key_bytes(*owner),
                        SummaryBinding {
                            kind: summary.kind,
                            summary: *digest,
                        }
                        .encode(),
                    )
                })
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    let owner_summaries = build_map(owner_summary_entries, store, work)?;
    let namespaces = build_map(
        entries
            .namespaces
            .iter()
            .map(|(key, owner)| (key.encode(), owner_value_bytes(*owner)))
            .collect(),
        store,
        work,
    )?;
    let ownership = build_map(
        entries
            .ownership
            .iter()
            .map(|(owner, entry)| Ok((owner_key_bytes(*owner), encode_ownership(entry)?)))
            .collect::<Result<Vec<_>, Diagnostic>>()?,
        store,
        work,
    )?;
    let forward_relations = build_map(
        entries
            .relations
            .iter()
            .map(|edge| (forward_relation_key(*edge), Vec::new()))
            .collect(),
        store,
        work,
    )?;
    let reverse_relations = build_map(
        entries
            .relations
            .iter()
            .map(|edge| (reverse_relation_key(*edge), Vec::new()))
            .collect(),
        store,
        work,
    )?;
    let test_dependencies = build_map(
        entries
            .test_dependencies
            .iter()
            .flat_map(|dependency| test_dependency_keys(*dependency))
            .map(|key| (key, Vec::new()))
            .collect(),
        store,
        work,
    )?;
    Ok(WitnessRoots {
        owner_summaries,
        namespaces,
        ownership,
        forward_relations,
        reverse_relations,
        test_dependencies,
    })
}

fn build_map(
    mut entries: Vec<(Vec<u8>, Vec<u8>)>,
    store: &mut MemoryPageStore,
    work: &mut MapWork,
) -> Result<MapRoot, Diagnostic> {
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    if entries.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(witness_error(
            DiagnosticClass::Corrupt,
            "witness_map_duplicate",
            "derived witness map contains a duplicate canonical key",
        ));
    }
    PersistentMap::from_sorted(store, entries, work)
        .map(PersistentMap::root)
        .map_err(map_diagnostic)
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    let class = match error.class {
        MapErrorClass::Input => DiagnosticClass::Source,
        MapErrorClass::Resource => DiagnosticClass::Resource,
        MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
        MapErrorClass::Store => DiagnosticClass::Infrastructure,
    };
    Diagnostic::new(class, error.code, error.message)
}
