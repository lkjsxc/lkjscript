//! Exact path-copy updates for all six committed validation-witness maps.

use super::{DerivedDelta, SummaryDelta, TestDependencyDelta, WitnessBaseRead};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::persistent_map::{
    BatchOutcome, MapEdit, MapError, MapErrorClass, MapWork, MemoryPageStore, OverlayPageStore,
    PageStore, PersistentMap,
};
use crate::platform::witness::{
    FullWitness, SummaryBinding, ValidationWitnessManifest, WitnessRoots, encode_ownership,
    forward_relation_key, owner_key_bytes, owner_value_bytes, reverse_relation_key,
    test_dependency_keys,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WitnessEditCounts {
    pub inserted: u64,
    pub replaced: u64,
    pub removed: u64,
    pub unchanged: u64,
}

#[derive(Clone, Debug)]
pub struct WitnessMapUpdate {
    pub roots: WitnessRoots,
    pub new_pages: MemoryPageStore,
    pub work: MapWork,
    pub edits: WitnessEditCounts,
}

/// Exact base witness capable of path-copying its committed maps into an isolated candidate
/// stage. Implementations may read pages from memory or immutable packed repository authority.
pub trait WitnessMapBase: WitnessBaseRead {
    fn update_witness_maps(
        &self,
        derived: &DerivedDelta,
        summaries: &SummaryDelta,
        tests: &TestDependencyDelta,
    ) -> Result<WitnessMapUpdate, Diagnostic>;
}

impl WitnessMapBase for FullWitness {
    fn update_witness_maps(
        &self,
        derived: &DerivedDelta,
        summaries: &SummaryDelta,
        tests: &TestDependencyDelta,
    ) -> Result<WitnessMapUpdate, Diagnostic> {
        update_witness_maps(self, derived, summaries, tests)
    }
}

pub fn update_witness_maps(
    base: &FullWitness,
    derived: &DerivedDelta,
    summaries: &SummaryDelta,
    tests: &TestDependencyDelta,
) -> Result<WitnessMapUpdate, Diagnostic> {
    update_witness_maps_from(&base.manifest, &base.pages, derived, summaries, tests)
}

/// Applies one exact derived delta to committed witness roots through a read-only base page
/// source. All produced pages remain isolated in the returned memory stage.
pub fn update_witness_maps_from<P: PageStore + ?Sized>(
    base: &ValidationWitnessManifest,
    pages: &P,
    derived: &DerivedDelta,
    summaries: &SummaryDelta,
    tests: &TestDependencyDelta,
) -> Result<WitnessMapUpdate, Diagnostic> {
    if !base.contract_is_current() {
        return Err(update_error(
            DiagnosticClass::Corrupt,
            "change_witness_contract",
            "base witness manifest is not current",
        ));
    }
    let mut store = OverlayPageStore::new(pages);
    let mut work = MapWork::default();
    let mut counts = WitnessEditCounts::default();

    let owner_summaries = apply_map(
        base.roots.owner_summaries,
        summary_edits(summaries),
        &mut store,
        &mut work,
        &mut counts,
    )?;
    let namespaces = apply_map(
        base.roots.namespaces,
        derived
            .namespaces
            .iter()
            .map(|edit| MapEdit {
                key: edit.key.encode(),
                before: edit.before.map(owner_value_bytes),
                after: edit.after.map(owner_value_bytes),
            })
            .collect(),
        &mut store,
        &mut work,
        &mut counts,
    )?;
    let ownership = apply_map(
        base.roots.ownership,
        derived
            .ownership
            .iter()
            .map(|edit| {
                Ok(MapEdit {
                    key: owner_key_bytes(edit.key),
                    before: edit.before.as_ref().map(encode_ownership).transpose()?,
                    after: edit.after.as_ref().map(encode_ownership).transpose()?,
                })
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?,
        &mut store,
        &mut work,
        &mut counts,
    )?;
    let forward_relations = apply_map(
        base.roots.forward_relations,
        relation_edits(derived, forward_relation_key),
        &mut store,
        &mut work,
        &mut counts,
    )?;
    let reverse_relations = apply_map(
        base.roots.reverse_relations,
        relation_edits(derived, reverse_relation_key),
        &mut store,
        &mut work,
        &mut counts,
    )?;
    let test_dependencies = apply_map(
        base.roots.test_dependencies,
        test_edits(tests)?,
        &mut store,
        &mut work,
        &mut counts,
    )?;

    Ok(WitnessMapUpdate {
        roots: WitnessRoots {
            owner_summaries,
            namespaces,
            ownership,
            forward_relations,
            reverse_relations,
            test_dependencies,
        },
        new_pages: store.into_pages(),
        work,
        edits: counts,
    })
}

fn summary_edits(summaries: &SummaryDelta) -> Vec<MapEdit> {
    summaries
        .edits
        .iter()
        .map(|edit| MapEdit {
            key: owner_key_bytes(edit.owner),
            before: edit
                .before
                .as_ref()
                .zip(edit.before_digest)
                .map(|(summary, digest)| {
                    SummaryBinding {
                        kind: summary.kind,
                        summary: digest,
                    }
                    .encode()
                }),
            after: edit
                .after
                .as_ref()
                .zip(edit.after_digest)
                .map(|(summary, digest)| {
                    SummaryBinding {
                        kind: summary.kind,
                        summary: digest,
                    }
                    .encode()
                }),
        })
        .collect()
}

fn relation_edits(
    derived: &DerivedDelta,
    key: impl Fn(crate::platform::kernel::RelationEdge) -> Vec<u8>,
) -> Vec<MapEdit> {
    let mut edits = derived
        .relations
        .removed
        .iter()
        .map(|edge| MapEdit {
            key: key(*edge),
            before: Some(Vec::new()),
            after: None,
        })
        .chain(derived.relations.added.iter().map(|edge| MapEdit {
            key: key(*edge),
            before: None,
            after: Some(Vec::new()),
        }))
        .collect::<Vec<_>>();
    edits.sort_unstable_by(|left, right| left.key.cmp(&right.key));
    edits
}

fn test_edits(tests: &TestDependencyDelta) -> Result<Vec<MapEdit>, Diagnostic> {
    let mut edits = BTreeMap::<Vec<u8>, MapEdit>::new();
    for dependency in &tests.removed {
        for key in test_dependency_keys(*dependency) {
            if edits
                .insert(
                    key.clone(),
                    MapEdit {
                        key,
                        before: Some(Vec::new()),
                        after: None,
                    },
                )
                .is_some()
            {
                return Err(update_error(
                    DiagnosticClass::Corrupt,
                    "change_test_map_duplicate",
                    "test dependency removal generated a duplicate witness key",
                ));
            }
        }
    }
    for dependency in &tests.added {
        for key in test_dependency_keys(*dependency) {
            if edits
                .insert(
                    key.clone(),
                    MapEdit {
                        key,
                        before: None,
                        after: Some(Vec::new()),
                    },
                )
                .is_some()
            {
                return Err(update_error(
                    DiagnosticClass::Corrupt,
                    "change_test_map_duplicate",
                    "test dependency insertion generated a duplicate witness key",
                ));
            }
        }
    }
    Ok(edits.into_values().collect())
}

fn apply_map<P: PageStore + ?Sized>(
    root: crate::platform::persistent_map::MapRoot,
    edits: Vec<MapEdit>,
    store: &mut OverlayPageStore<'_, P>,
    work: &mut MapWork,
    counts: &mut WitnessEditCounts,
) -> Result<crate::platform::persistent_map::MapRoot, Diagnostic> {
    let (map, outcome) = PersistentMap::from_root(root)
        .apply_sorted_edits(store, &edits, work)
        .map_err(map_diagnostic)?;
    add_counts(counts, outcome);
    Ok(map.root())
}

fn add_counts(counts: &mut WitnessEditCounts, outcome: BatchOutcome) {
    counts.inserted = counts.inserted.saturating_add(outcome.inserted);
    counts.replaced = counts.replaced.saturating_add(outcome.replaced);
    counts.removed = counts.removed.saturating_add(outcome.removed);
    counts.unchanged = counts.unchanged.saturating_add(outcome.unchanged);
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    let class = match error.class {
        MapErrorClass::Input => DiagnosticClass::Semantic,
        MapErrorClass::Resource => DiagnosticClass::Resource,
        MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
        MapErrorClass::Store => DiagnosticClass::Infrastructure,
    };
    Diagnostic::new(class, error.code, error.message)
}

fn update_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
