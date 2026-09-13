//! Full post-change proof when authentic historical acceptance is not a valid proof base.

use super::budget::ChangeBudgetMeter;
use super::derived::derive_local_delta_with_admission;
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{KernelSnapshot, OwnerKey, OwnerKind};
use crate::platform::witness::{CanonicalWitnessFacts, rebuild_full_witness_checked};
use std::collections::BTreeSet;

pub(crate) fn prepare_repair_analysis<W: WitnessBaseRead + CanonicalBaseRead + ?Sized>(
    base: &KernelSnapshot,
    facts: &CanonicalWitnessFacts,
    witness_base: &W,
    canonical: CanonicalDelta,
    budget: ChangeBudget,
    initial_work: ChangeBudgetWork,
) -> Result<PreparedChangeAnalysis, Vec<Diagnostic>> {
    let single = |error| vec![error];
    witness_base.validation_checkpoint().map_err(single)?;
    let mut meter = ChangeBudgetMeter::new(budget, initial_work).map_err(single)?;
    meter.observe_canonical(&canonical).map_err(single)?;
    let overlay = KernelOverlay::new(base, &canonical);
    let derived = derive_local_delta_with_admission(
        &overlay,
        &canonical,
        witness_base,
        budget.impact.maximum_ownership_steps,
        budget.impact.maximum_relation_edges,
    )
    .map_err(single)?;
    meter.observe_derived(&derived).map_err(single)?;
    // Logical materialization does not publish its placeholder map roots. The publication owner
    // stages the canonical delta and binds these freshly rebuilt witness maps to its actual root.
    let mut candidate = overlay.materialize_logical_oracle();
    candidate.dependency_interfaces.retain(|revision, _| {
        candidate
            .dependencies
            .values()
            .any(|dependency| dependency.package_revision == *revision)
    });
    // Type/blob objects are immutable storage content, but only live owner roots belong to
    // the logical candidate. Removing a body does not delete its historical stored objects.
    let maximum_work = usize::try_from(
        budget
            .validation
            .maximum_expression_steps
            .saturating_sub(initial_work.validation.expression_steps),
    )
    .unwrap_or(usize::MAX);
    let mut pruning_work = 0_usize;
    let mut pending_types: BTreeSet<_> = candidate
        .owners
        .values()
        .flat_map(crate::platform::kernel::OwnerRecord::type_roots)
        .collect();
    let mut live_types = BTreeSet::new();
    while let Some(ty) = pending_types.pop_first() {
        witness_base.validation_checkpoint().map_err(single)?;
        pruning_work = pruning_work
            .checked_add(1)
            .filter(|work| *work <= maximum_work)
            .ok_or_else(|| {
                vec![Diagnostic::new(
                    DiagnosticClass::Resource,
                    "change_budget_validation_expression_steps",
                    "complete repair type-root reconstruction exhausted validation work",
                )]
            })?;
        if live_types.insert(ty)
            && let Some(object) = candidate.types.get(&ty)
        {
            pending_types.extend(object.child_types());
        }
    }
    candidate.types.retain(|ty, _| live_types.contains(ty));
    let live_blobs: BTreeSet<_> = candidate
        .owners
        .values()
        .flat_map(crate::platform::kernel::OwnerRecord::blob_roots)
        .map(|(digest, _)| digest)
        .collect();
    candidate
        .blobs
        .retain(|digest, _| live_blobs.contains(digest));
    let full = rebuild_full_witness_checked(&candidate, maximum_work - pruning_work, &|| {
        witness_base.validation_checkpoint()
    })?;
    let report = full.report.full_validation.clone();
    let selected: BTreeSet<_> = facts
        .summaries
        .keys()
        .chain(full.summaries.keys())
        .copied()
        .collect();
    let mut final_delta = SummaryDelta {
        selected: selected.clone(),
        base_summaries_selected: facts.summaries.len() as u64,
        new_objects: full.summary_objects,
        ..SummaryDelta::default()
    };
    for owner in selected {
        let before = facts.summaries.get(&owner).cloned();
        let after = full.summaries.get(&owner).cloned();
        if before != after {
            final_delta.edits.push(OwnerSummaryEdit {
                owner,
                before_digest: facts.entries.summaries.get(&owner).copied(),
                after_digest: full.entries.summaries.get(&owner).copied(),
                before,
                after,
            });
        }
    }
    let owners: BTreeSet<_> = candidate.owners.keys().copied().collect();
    let test_owners: BTreeSet<_> = candidate
        .owners
        .iter()
        .filter_map(|(owner, record)| (record.kind() == OwnerKind::Test).then_some(*owner))
        .collect();
    let plan = ImpactPlan {
        structurally_checked: owners.clone(),
        semantically_checked: owners.clone(),
        summary_owners: final_delta.selected.clone(),
        compiler_units: owners
            .iter()
            .copied()
            .filter(|owner| matches!(owner, OwnerKey::Declaration(_) | OwnerKey::Target(_)))
            .collect(),
        tests: test_owners.clone(),
        reasons: Vec::new(),
        work: ImpactWork::default(),
    };
    let summaries = PlannedSummaries {
        initial: SummaryDelta::default(),
        final_delta,
        plan,
    };
    meter.observe_summaries(&summaries).map_err(single)?;
    meter.observe_impact(&summaries.plan).map_err(single)?;
    let mut validation_work = IncrementalValidationWork {
        owner_records_checked: report.owners_checked,
        ownership_entries_checked: full.entries.ownership.len() as u64,
        type_objects_checked: report.type_objects_checked,
        expression_work: report.work_consumed,
        witness_reads: WitnessReadWork::default(),
    };
    let http_routes = super::validate::validate_http_topology_frontier(
        &overlay,
        &summaries.plan,
        &canonical,
        witness_base,
        &mut validation_work,
        budget.validation,
    )
    .map_err(single)?;
    meter
        .observe_structural(&StructuralValidationReport {
            structurally_checked: owners.clone(),
            work: validation_work,
        })
        .map_err(single)?;
    meter
        .observe_expression_validation(report.work_consumed.saturating_add(pruning_work as u64), 0)
        .map_err(single)?;
    let tests = TestDependencyDelta {
        affected_tests: test_owners.clone(),
        removed: facts
            .entries
            .test_dependencies
            .difference(&full.entries.test_dependencies)
            .copied()
            .collect(),
        added: full
            .entries
            .test_dependencies
            .difference(&facts.entries.test_dependencies)
            .copied()
            .collect(),
        work: TestDeltaWork::default(),
    };
    meter.observe_tests(&tests).map_err(single)?;
    meter
        .preflight_witness_edits(&derived, &summaries.final_delta, &tests)
        .map_err(single)?;
    let witness = WitnessMapUpdate {
        roots: full.manifest.roots,
        new_pages: full.pages,
        work: full.report.map_work,
        read_work: WitnessReadWork::default(),
        edits: WitnessEditCounts {
            inserted: full
                .report
                .namespace_entries
                .saturating_add(full.report.ownership_entries)
                .saturating_add(full.report.owners_summarized)
                .saturating_add(full.report.relation_edges.saturating_mul(2))
                .saturating_add(full.report.test_dependency_entries),
            ..WitnessEditCounts::default()
        },
    };
    meter.observe_witness(&witness).map_err(single)?;
    meter
        .observe_canonical_reads(overlay.work())
        .map_err(single)?;
    let validation = IncrementalValidationReport {
        profile: "full_candidate_repair",
        canonical_owners_changed: canonical.owners.len() as u64,
        structurally_checked: owners.clone(),
        semantically_checked: owners,
        summaries_reused: 0,
        tests_selected: test_owners.len() as u64,
        http_routes,
        work: validation_work,
    };
    Ok(PreparedChangeAnalysis {
        canonical_read_work: overlay.work(),
        witness_read_work: WitnessReadWork::default(),
        canonical,
        derived,
        summaries,
        tests,
        witness,
        validation,
        budget_work: meter.finish(),
        budget,
        full_candidate: Some(report),
    })
}
