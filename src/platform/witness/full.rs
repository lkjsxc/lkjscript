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
    let validation = crate::platform::kernel::validate_full_with_limit(snapshot, maximum_work)?;
    build_validated_witness(snapshot, validation).map_err(|diagnostic| vec![diagnostic])
}

fn build_validated_witness(
    snapshot: &KernelSnapshot,
    full_validation: FullValidationReport,
) -> Result<FullWitness, Diagnostic> {
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
