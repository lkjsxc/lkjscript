//! One deterministic reverse-impact plan from owner-summary dimension changes.

use super::relation_view::CandidateRelations;
use super::{
    CanonicalDelta, DerivedDelta, KernelOverlay, OwnerSummaryEdit, SummaryDelta, WitnessBaseRead,
    WitnessReadWork, derive_summary_delta, derive_summary_delta_for,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    ExactOwnerKey, OwnerKey, OwnerKind, PropagationClass, RelationEndpoint, RelationKind,
};
use crate::platform::witness::{OwnerSummary, OwnershipEntry, OwnershipParent};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SummaryDimensionChange {
    pub created: bool,
    pub deleted: bool,
    pub semantic_interface: bool,
    pub implementation: bool,
    pub type_digest: bool,
    pub effect: bool,
    pub capability: bool,
    pub relations: bool,
    pub presentation: bool,
    pub test: bool,
    pub validation_dependencies: bool,
}

impl SummaryDimensionChange {
    pub const fn executable(self) -> bool {
        self.created
            || self.deleted
            || self.semantic_interface
            || self.implementation
            || self.type_digest
            || self.effect
            || self.capability
            || self.relations
            || self.test
    }

    const fn affects_validation(self, propagation: PropagationClass) -> bool {
        if self.created || self.deleted {
            return !matches!(
                propagation,
                PropagationClass::Ownership | PropagationClass::Presentation
            );
        }
        match propagation {
            PropagationClass::Type | PropagationClass::Value => {
                self.semantic_interface || self.type_digest
            }
            PropagationClass::Behavior | PropagationClass::Capability => {
                self.semantic_interface || self.effect || self.capability
            }
            PropagationClass::Target => self.semantic_interface || self.implementation,
            PropagationClass::Test => self.implementation || self.test,
            PropagationClass::Package => {
                self.semantic_interface || self.implementation || self.type_digest
            }
            PropagationClass::Ownership | PropagationClass::Presentation => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ImpactReasonKind {
    DirectSemanticChange,
    ValidationDependency,
    TestBehavior,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ImpactReason {
    pub kind: ImpactReasonKind,
    pub source: OwnerKey,
    pub target: OwnerKey,
    pub relation: Option<RelationKind>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ImpactWork {
    pub summary_edits_examined: u64,
    pub reverse_edges_visited: u64,
    pub ownership_steps: u64,
    pub behavior_owners_visited: u64,
    pub witness_reads: WitnessReadWork,
}

#[derive(Clone, Debug, Default)]
pub struct ImpactPlan {
    pub structurally_checked: BTreeSet<OwnerKey>,
    pub semantically_checked: BTreeSet<OwnerKey>,
    pub summary_owners: BTreeSet<OwnerKey>,
    pub compiler_units: BTreeSet<OwnerKey>,
    pub tests: BTreeSet<OwnerKey>,
    pub reasons: Vec<ImpactReason>,
    pub work: ImpactWork,
}

#[derive(Clone, Debug)]
pub struct PlannedSummaries {
    pub initial: SummaryDelta,
    pub final_delta: SummaryDelta,
    pub plan: ImpactPlan,
}

pub fn plan_impact_and_summaries<W: WitnessBaseRead + ?Sized>(
    overlay: &KernelOverlay<'_>,
    canonical: &CanonicalDelta,
    derived: &DerivedDelta,
    base_witness: &W,
) -> Result<PlannedSummaries, Diagnostic> {
    let initial = derive_summary_delta(overlay, derived, base_witness)?;
    let mut plan = ImpactPlan {
        structurally_checked: canonical.owners.keys().copied().collect(),
        summary_owners: initial.selected.clone(),
        ..ImpactPlan::default()
    };
    let mut relations =
        CandidateRelations::new(overlay.base().root.package_id, derived, base_witness);
    let mut ownership = CandidateOwnership::new(derived, base_witness);
    let mut behavior = VecDeque::new();
    let mut behavior_seen = BTreeSet::new();
    let mut behavior_enqueued = BTreeSet::new();

    for edit in &initial.edits {
        plan.work.summary_edits_examined = plan.work.summary_edits_examined.saturating_add(1);
        let change = summary_dimension_change(edit);
        let kind = edit
            .after
            .as_ref()
            .or(edit.before.as_ref())
            .map(|summary| summary.kind);
        if change.executable() && kind.is_some_and(has_executable_meaning) {
            plan.semantically_checked.insert(edit.owner);
            add_owning_units(
                edit.owner,
                overlay,
                &mut ownership,
                &mut plan.compiler_units,
                &mut plan.work,
            )?;
            add_owning_units(
                edit.owner,
                overlay,
                &mut ownership,
                &mut plan.semantically_checked,
                &mut plan.work,
            )?;
            if behavior_enqueued.insert(edit.owner) {
                behavior.push_back(edit.owner);
            }
            plan.reasons.push(ImpactReason {
                kind: ImpactReasonKind::DirectSemanticChange,
                source: edit.owner,
                target: edit.owner,
                relation: None,
            });
        }
        for edge in relations.incoming(edit.owner)? {
            plan.work.reverse_edges_visited = plan.work.reverse_edges_visited.saturating_add(1);
            if !change.affects_validation(edge.kind.propagation()) {
                continue;
            }
            let Some(source) = local_owner(edge.source, overlay.base().root.package_id) else {
                continue;
            };
            add_summary_paths(
                source,
                &mut ownership,
                &mut plan.summary_owners,
                &mut plan.work,
            )?;
            add_owning_units(
                source,
                overlay,
                &mut ownership,
                &mut plan.semantically_checked,
                &mut plan.work,
            )?;
            add_owning_units(
                source,
                overlay,
                &mut ownership,
                &mut plan.compiler_units,
                &mut plan.work,
            )?;
            plan.reasons.push(ImpactReason {
                kind: ImpactReasonKind::ValidationDependency,
                source,
                target: edit.owner,
                relation: Some(edge.kind),
            });
        }
    }

    while let Some(target) = behavior.pop_front() {
        if !behavior_seen.insert(target) {
            continue;
        }
        plan.work.behavior_owners_visited = plan.work.behavior_owners_visited.saturating_add(1);
        for edge in relations.incoming(target)? {
            plan.work.reverse_edges_visited = plan.work.reverse_edges_visited.saturating_add(1);
            if matches!(
                edge.kind.propagation(),
                PropagationClass::Ownership | PropagationClass::Presentation
            ) {
                continue;
            }
            let Some(source) = local_owner(edge.source, overlay.base().root.package_id) else {
                continue;
            };
            let declarations = owning_units(source, overlay, &mut ownership, &mut plan.work)?;
            for declaration in declarations {
                if ownership.owner_kind(declaration, overlay)? == Some(OwnerKind::Test) {
                    plan.tests.insert(declaration);
                    plan.reasons.push(ImpactReason {
                        kind: ImpactReasonKind::TestBehavior,
                        source: declaration,
                        target,
                        relation: Some(edge.kind),
                    });
                } else if behavior_enqueued.insert(declaration) {
                    behavior.push_back(declaration);
                }
            }
        }
    }

    for owner in &plan.semantically_checked {
        if ownership.owner_kind(*owner, overlay)? == Some(OwnerKind::Test) {
            plan.tests.insert(*owner);
        }
    }
    plan.work.witness_reads = ownership.work();
    plan.work.witness_reads.add(relations.work());
    plan.reasons.sort_unstable();
    plan.reasons.dedup();
    let final_delta =
        derive_summary_delta_for(overlay, derived, base_witness, plan.summary_owners.clone())?;
    Ok(PlannedSummaries {
        initial,
        final_delta,
        plan,
    })
}

pub fn summary_dimension_change(edit: &OwnerSummaryEdit) -> SummaryDimensionChange {
    match (&edit.before, &edit.after) {
        (None, Some(_)) => SummaryDimensionChange {
            created: true,
            ..SummaryDimensionChange::default()
        },
        (Some(_), None) => SummaryDimensionChange {
            deleted: true,
            ..SummaryDimensionChange::default()
        },
        (Some(before), Some(after)) => compare_summaries(before, after),
        (None, None) => SummaryDimensionChange::default(),
    }
}

fn compare_summaries(before: &OwnerSummary, after: &OwnerSummary) -> SummaryDimensionChange {
    SummaryDimensionChange {
        semantic_interface: before.semantic_interface != after.semantic_interface,
        implementation: before.implementation != after.implementation,
        type_digest: before.type_digest != after.type_digest,
        effect: before.effect != after.effect,
        capability: before.capability != after.capability,
        relations: before.relations != after.relations,
        presentation: before.presentation != after.presentation,
        test: before.test != after.test,
        validation_dependencies: before.validation_dependencies != after.validation_dependencies,
        ..SummaryDimensionChange::default()
    }
}

struct CandidateOwnership<'a, W: ?Sized> {
    edits: BTreeMap<OwnerKey, Option<OwnershipEntry>>,
    base: &'a W,
    ownership: BTreeMap<OwnerKey, Option<OwnershipEntry>>,
    owner_kinds: BTreeMap<OwnerKey, Option<OwnerKind>>,
    work: WitnessReadWork,
}

impl<'a, W: WitnessBaseRead + ?Sized> CandidateOwnership<'a, W> {
    fn new(derived: &DerivedDelta, base: &'a W) -> Self {
        Self {
            edits: derived
                .ownership
                .iter()
                .map(|edit| (edit.key, edit.after))
                .collect(),
            base,
            ownership: BTreeMap::new(),
            owner_kinds: BTreeMap::new(),
            work: WitnessReadWork::default(),
        }
    }

    const fn work(&self) -> WitnessReadWork {
        self.work
    }

    fn candidate(&mut self, owner: OwnerKey) -> Result<Option<OwnershipEntry>, Diagnostic> {
        match self.edits.get(&owner) {
            Some(entry) => Ok(*entry),
            None => self.before(owner),
        }
    }

    fn before(&mut self, owner: OwnerKey) -> Result<Option<OwnershipEntry>, Diagnostic> {
        if !self.ownership.contains_key(&owner) {
            let read = self.base.read_ownership(owner)?;
            self.work.add(read.work);
            self.ownership.insert(owner, read.value);
        }
        Ok(self.ownership.get(&owner).copied().flatten())
    }

    fn owner_kind(
        &mut self,
        owner: OwnerKey,
        overlay: &KernelOverlay<'_>,
    ) -> Result<Option<OwnerKind>, Diagnostic> {
        if let Some(record) = overlay.owner(owner) {
            return Ok(Some(record.kind()));
        }
        if !self.owner_kinds.contains_key(&owner) {
            let read = self.base.read_owner_summary(owner)?;
            self.work.add(read.work);
            self.owner_kinds
                .insert(owner, read.value.map(|bound| bound.summary.kind));
        }
        Ok(self.owner_kinds.get(&owner).copied().flatten())
    }
}

fn add_summary_paths<W: WitnessBaseRead + ?Sized>(
    owner: OwnerKey,
    ownership: &mut CandidateOwnership<'_, W>,
    selected: &mut BTreeSet<OwnerKey>,
    work: &mut ImpactWork,
) -> Result<(), Diagnostic> {
    walk_aggregating_path(owner, |key| ownership.before(key), selected, work)?;
    walk_aggregating_path(owner, |key| ownership.candidate(key), selected, work)
}

fn walk_aggregating_path(
    owner: OwnerKey,
    mut lookup: impl FnMut(OwnerKey) -> Result<Option<OwnershipEntry>, Diagnostic>,
    selected: &mut BTreeSet<OwnerKey>,
    work: &mut ImpactWork,
) -> Result<(), Diagnostic> {
    let mut current = owner;
    let mut observed = BTreeSet::new();
    loop {
        work.ownership_steps = work.ownership_steps.saturating_add(1);
        if !observed.insert(current) {
            return Err(impact_error(
                "change_impact_ownership_cycle",
                "impact ownership path is cyclic",
            ));
        }
        selected.insert(current);
        let Some(entry) = lookup(current)? else {
            break;
        };
        if !entry.role.aggregates_into_parent() {
            break;
        }
        let OwnershipParent::Owner(parent) = entry.parent else {
            break;
        };
        current = parent;
    }
    Ok(())
}

fn add_owning_units<W: WitnessBaseRead + ?Sized>(
    owner: OwnerKey,
    overlay: &KernelOverlay<'_>,
    ownership: &mut CandidateOwnership<'_, W>,
    units: &mut BTreeSet<OwnerKey>,
    work: &mut ImpactWork,
) -> Result<(), Diagnostic> {
    units.extend(owning_units(owner, overlay, ownership, work)?);
    Ok(())
}

fn owning_units<W: WitnessBaseRead + ?Sized>(
    owner: OwnerKey,
    overlay: &KernelOverlay<'_>,
    ownership: &mut CandidateOwnership<'_, W>,
    work: &mut ImpactWork,
) -> Result<BTreeSet<OwnerKey>, Diagnostic> {
    let mut units = BTreeSet::new();
    for candidate in [false, true] {
        let mut current = owner;
        let mut observed = BTreeSet::new();
        loop {
            work.ownership_steps = work.ownership_steps.saturating_add(1);
            if !observed.insert(current) {
                return Err(impact_error(
                    "change_impact_unit_cycle",
                    "compiler-unit ownership path is cyclic",
                ));
            }
            if matches!(current, OwnerKey::Declaration(_) | OwnerKey::Target(_)) {
                units.insert(current);
                break;
            }
            let entry = if candidate {
                ownership.candidate(current)?
            } else {
                ownership.before(current)?
            };
            let Some(entry) = entry else {
                if overlay.owner(current).is_some_and(|record| {
                    matches!(record.kind(), OwnerKind::Port | OwnerKind::Target)
                }) {
                    units.insert(current);
                }
                break;
            };
            let OwnershipParent::Owner(parent) = entry.parent else {
                break;
            };
            current = parent;
        }
    }
    Ok(units)
}

const fn has_executable_meaning(kind: OwnerKind) -> bool {
    !matches!(
        kind,
        OwnerKind::Module | OwnerKind::Documentation | OwnerKind::Annotation
    )
}

fn local_owner(
    endpoint: RelationEndpoint,
    package: crate::platform::kernel::PackageId,
) -> Option<OwnerKey> {
    match endpoint {
        RelationEndpoint::Owner(ExactOwnerKey {
            package: source_package,
            owner,
        }) if source_package == package => Some(owner),
        RelationEndpoint::Owner(_) | RelationEndpoint::Package(_) => None,
    }
}

fn impact_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}
