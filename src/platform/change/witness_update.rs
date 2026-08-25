//! Exact path-copy updates for all six committed validation-witness maps.

use super::{DerivedDelta, SummaryDelta, TestDependencyDelta, WitnessBaseRead, WitnessReadWork};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::persistent_map::{
    BatchOutcome, MapAdmission, MapEdit, MapError, MapErrorClass, MapWork, MemoryPageStore,
    OverlayPageStore, PageStore, PersistentMap,
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

/// Independent logical-map and accepted-object read limits for one witness path-copy update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WitnessMapAdmission {
    pub map: MapAdmission,
    pub maximum_catalog_lookups: u64,
    pub maximum_objects: u64,
    pub maximum_bytes: u64,
}

impl WitnessMapAdmission {
    pub const fn unbounded() -> Self {
        Self {
            map: MapAdmission::unbounded(),
            maximum_catalog_lookups: u64::MAX,
            maximum_objects: u64::MAX,
            maximum_bytes: u64::MAX,
        }
    }
}

impl Default for WitnessMapAdmission {
    fn default() -> Self {
        Self::unbounded()
    }
}

#[derive(Clone, Debug)]
pub struct WitnessMapUpdate {
    pub roots: WitnessRoots,
    pub new_pages: MemoryPageStore,
    pub work: MapWork,
    pub read_work: WitnessReadWork,
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
        admission: WitnessMapAdmission,
    ) -> Result<WitnessMapUpdate, Diagnostic>;
}

impl WitnessMapBase for FullWitness {
    fn update_witness_maps(
        &self,
        derived: &DerivedDelta,
        summaries: &SummaryDelta,
        tests: &TestDependencyDelta,
        admission: WitnessMapAdmission,
    ) -> Result<WitnessMapUpdate, Diagnostic> {
        update_witness_maps(self, derived, summaries, tests, admission)
    }
}

pub fn update_witness_maps(
    base: &FullWitness,
    derived: &DerivedDelta,
    summaries: &SummaryDelta,
    tests: &TestDependencyDelta,
    admission: WitnessMapAdmission,
) -> Result<WitnessMapUpdate, Diagnostic> {
    update_witness_maps_from(
        &base.manifest,
        &base.pages,
        derived,
        summaries,
        tests,
        admission,
    )
}

/// Applies one exact derived delta to committed witness roots through a read-only base page
/// source. All produced pages remain isolated in the returned memory stage.
pub fn update_witness_maps_from<P: PageStore + ?Sized>(
    base: &ValidationWitnessManifest,
    pages: &P,
    derived: &DerivedDelta,
    summaries: &SummaryDelta,
    tests: &TestDependencyDelta,
    admission: WitnessMapAdmission,
) -> Result<WitnessMapUpdate, Diagnostic> {
    if !base.contract_is_current() {
        return Err(update_error(
            DiagnosticClass::Corrupt,
            "change_witness_contract",
            "base witness manifest is not current",
        ));
    }
    let mut store = OverlayPageStore::new(pages);
    let mut work = MapWork::with_admission(admission.map);
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

    let read_work = WitnessReadWork {
        map_pages_read: work.pages_read,
        map_entries_visited: work.entries_visited,
        bytes_read: work.bytes_read,
        ..WitnessReadWork::default()
    };
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
        read_work,
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
    mut edits: Vec<MapEdit>,
    store: &mut OverlayPageStore<'_, P>,
    work: &mut MapWork,
    counts: &mut WitnessEditCounts,
) -> Result<crate::platform::persistent_map::MapRoot, Diagnostic> {
    // Domain-level ordering is not necessarily canonical byte ordering. Namespace names, for
    // example, compare lexically in memory while their bounded map encoding prefixes byte length.
    edits.sort_unstable_by(|left, right| left.key.cmp(&right.key));
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
    let code = match error.code {
        "persistent_map_admission_pages_read" => "change_budget_witness_map_pages",
        "persistent_map_admission_bytes_read" => "change_budget_witness_bytes",
        "persistent_map_admission_entries_visited" => "change_budget_witness_map_entries",
        "persistent_map_admission_pages_encoded" => "change_budget_witness_map_pages_encoded",
        "persistent_map_admission_bytes_encoded" => "change_budget_witness_map_bytes_encoded",
        "object_read_catalog_lookups_exhausted" => "change_budget_witness_catalog_lookups",
        "object_read_objects_exhausted" => "change_budget_witness_objects",
        "object_read_bytes_exhausted" => "change_budget_witness_bytes",
        code => code,
    };
    Diagnostic::new(class, code, error.message)
}

fn update_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::change::{DerivedValueEdit, OwnerSummaryEdit};
    use crate::platform::witness::rebuild_full_witness;

    fn two_map_delta(base: &FullWitness) -> (DerivedDelta, SummaryDelta) {
        let (&summary_owner, summary) = base.summaries.iter().next().expect("fixture summary");
        let before_digest = *base
            .entries
            .summaries
            .get(&summary_owner)
            .expect("fixture summary binding");
        let summaries = SummaryDelta {
            edits: vec![OwnerSummaryEdit {
                owner: summary_owner,
                before_digest: Some(before_digest),
                after_digest: None,
                before: Some(summary.clone()),
                after: None,
            }],
            ..SummaryDelta::default()
        };
        let (namespace, &owner) = base
            .entries
            .namespaces
            .iter()
            .next()
            .expect("fixture namespace");
        let derived = DerivedDelta {
            namespaces: vec![DerivedValueEdit {
                key: namespace.clone(),
                before: Some(owner),
                after: None,
            }],
            ..DerivedDelta::default()
        };
        (derived, summaries)
    }

    #[test]
    fn witness_path_copy_applies_one_admission_across_all_maps() {
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let base = rebuild_full_witness(&snapshot).expect("fixture witness");
        let (derived, summaries) = two_map_delta(&base);
        let tests = TestDependencyDelta::default();
        let baseline = update_witness_maps(
            &base,
            &derived,
            &summaries,
            &tests,
            WitnessMapAdmission::unbounded(),
        )
        .expect("baseline witness update");
        assert!(baseline.work.pages_read >= 2);
        assert!(baseline.work.bytes_read > 0);
        assert!(baseline.work.entries_visited >= 2);
        assert!(baseline.work.pages_encoded >= 2);
        assert!(baseline.work.bytes_encoded > 0);
        assert_eq!(baseline.read_work.map_pages_read, baseline.work.pages_read);
        assert_eq!(baseline.read_work.bytes_read, baseline.work.bytes_read);
        assert_eq!(
            baseline.read_work.map_entries_visited,
            baseline.work.entries_visited
        );
        assert_eq!(baseline.read_work.catalog_lookups, 0);
        assert_eq!(baseline.read_work.objects_read, 0);

        for (map, code) in [
            (
                MapAdmission {
                    maximum_pages_read: 0,
                    ..MapAdmission::unbounded()
                },
                "change_budget_witness_map_pages",
            ),
            (
                MapAdmission {
                    maximum_bytes_read: 0,
                    ..MapAdmission::unbounded()
                },
                "change_budget_witness_bytes",
            ),
            (
                MapAdmission {
                    maximum_entries_visited: 0,
                    ..MapAdmission::unbounded()
                },
                "change_budget_witness_map_entries",
            ),
            (
                MapAdmission {
                    maximum_pages_encoded: 1,
                    ..MapAdmission::unbounded()
                },
                "change_budget_witness_map_pages_encoded",
            ),
            (
                MapAdmission {
                    maximum_bytes_encoded: baseline.work.bytes_encoded - 1,
                    ..MapAdmission::unbounded()
                },
                "change_budget_witness_map_bytes_encoded",
            ),
        ] {
            let admission = WitnessMapAdmission {
                map,
                ..WitnessMapAdmission::unbounded()
            };
            let error = update_witness_maps(&base, &derived, &summaries, &tests, admission)
                .expect_err("exhausted witness map admission must reject");
            assert_eq!(error.code, code);
        }

        let exact = update_witness_maps(
            &base,
            &derived,
            &summaries,
            &tests,
            WitnessMapAdmission {
                map: MapAdmission {
                    maximum_pages_read: baseline.work.pages_read,
                    maximum_bytes_read: baseline.work.bytes_read,
                    maximum_entries_visited: baseline.work.entries_visited,
                    maximum_pages_encoded: baseline.work.pages_encoded,
                    maximum_bytes_encoded: baseline.work.bytes_encoded,
                },
                ..WitnessMapAdmission::unbounded()
            },
        )
        .expect("exact witness admission");
        assert_eq!(exact.roots, baseline.roots);
        assert_eq!(exact.work.pages_read, baseline.work.pages_read);
        assert_eq!(exact.work.bytes_read, baseline.work.bytes_read);
        assert_eq!(exact.work.entries_visited, baseline.work.entries_visited);
        assert_eq!(exact.work.pages_encoded, baseline.work.pages_encoded);
        assert_eq!(exact.work.bytes_encoded, baseline.work.bytes_encoded);
        assert_eq!(exact.read_work, baseline.read_work);
    }
}
