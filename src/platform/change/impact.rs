//! One deterministic reverse-impact plan from owner-summary dimension changes.

use super::relation_view::CandidateRelations;
use super::summary_delta::derive_summary_delta_for_with_relation_limit;
use super::{
    CanonicalBaseRead, CanonicalDelta, DerivedDelta, ImpactAdmission, KernelOverlay,
    OwnerSummaryEdit, SummaryDelta, WitnessBaseRead, WitnessReadWork,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    EncodedOwnerKey, ExactOwnerKey, OwnerKey, OwnerKind, PropagationClass, RelationEndpoint,
    RelationKind,
};
use crate::platform::witness::{OwnerSummary, OwnershipEntry, OwnershipParent};
use std::cmp::Ordering;
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
    DependencyBinding,
    ValidationDependency,
    TestBehavior,
}

impl ImpactReasonKind {
    pub const ALL: [Self; 4] = [
        Self::DirectSemanticChange,
        Self::DependencyBinding,
        Self::ValidationDependency,
        Self::TestBehavior,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::DirectSemanticChange => "direct_semantic_change",
            Self::DependencyBinding => "dependency_binding",
            Self::ValidationDependency => "validation_dependency",
            Self::TestBehavior => "test_behavior",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == value)
    }

    const fn order(self) -> u8 {
        match self {
            Self::DirectSemanticChange => 1,
            Self::DependencyBinding => 2,
            Self::ValidationDependency => 3,
            Self::TestBehavior => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ImpactReason {
    pub kind: ImpactReasonKind,
    pub source: OwnerKey,
    pub target: OwnerKey,
    pub relation: Option<RelationKind>,
}

impl ImpactReason {
    pub fn logical_cmp(&self, other: &Self) -> Ordering {
        self.kind
            .order()
            .cmp(&other.kind.order())
            .then_with(|| {
                EncodedOwnerKey::new(self.source).cmp(&EncodedOwnerKey::new(other.source))
            })
            .then_with(|| {
                EncodedOwnerKey::new(self.target).cmp(&EncodedOwnerKey::new(other.target))
            })
            .then_with(|| relation_order(self.relation).cmp(&relation_order(other.relation)))
    }
}

fn relation_order(relation: Option<RelationKind>) -> usize {
    relation.map_or(0, |relation| {
        RelationKind::ALL
            .iter()
            .position(|candidate| *candidate == relation)
            .map_or(usize::MAX, |index| index.saturating_add(1))
    })
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ImpactWork {
    pub summary_owners_examined: u64,
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

pub fn plan_impact_and_summaries<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    overlay: &KernelOverlay<'_, B>,
    canonical: &CanonicalDelta,
    derived: &DerivedDelta,
    base_witness: &W,
) -> Result<PlannedSummaries, Diagnostic> {
    plan_impact_and_summaries_with_admission(
        overlay,
        canonical,
        derived,
        base_witness,
        ImpactAdmission::default(),
    )
}

pub(crate) fn plan_impact_and_summaries_with_admission<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    overlay: &KernelOverlay<'_, B>,
    canonical: &CanonicalDelta,
    derived: &DerivedDelta,
    base_witness: &W,
    admission: ImpactAdmission,
) -> Result<PlannedSummaries, Diagnostic> {
    admit_count(
        "change_budget_impact_summary_owners",
        "summary owners",
        u64::try_from(derived.summary_candidates.len()).unwrap_or(u64::MAX),
        admission.maximum_summary_owners,
    )?;
    let initial = derive_summary_delta_for_with_relation_limit(
        overlay,
        derived,
        base_witness,
        derived.summary_candidates.clone(),
        admission.maximum_relation_edges,
        admission.maximum_relation_fanout,
    )?;
    let mut admitted = BTreeSet::new();
    for owner in canonical.owners.keys().chain(&initial.selected).copied() {
        admit_affected(&mut admitted, owner, admission.maximum_affected_owners)?;
    }
    let mut plan = ImpactPlan {
        structurally_checked: canonical.owners.keys().copied().collect(),
        summary_owners: initial.selected.clone(),
        work: ImpactWork {
            summary_owners_examined: u64::try_from(initial.selected.len()).unwrap_or(u64::MAX),
            ..ImpactWork::default()
        },
        ..ImpactPlan::default()
    };
    let package = overlay.package_id();
    let mut relations = CandidateRelations::new(package, derived, base_witness);
    let mut ownership = CandidateOwnership::new(derived, base_witness);
    let mut behavior = VecDeque::new();
    let mut behavior_seen = BTreeSet::new();
    let mut behavior_enqueued = BTreeSet::new();

    for edit in &initial.edits {
        admit_summary_edit(&mut plan.work, admission.maximum_summary_edits)?;
        let change = summary_dimension_change(edit);
        let kind = edit
            .after
            .as_ref()
            .or(edit.before.as_ref())
            .map(|summary| summary.kind);
        if change.executable() && kind.is_some_and(has_executable_meaning) {
            admit_affected(&mut admitted, edit.owner, admission.maximum_affected_owners)?;
            plan.semantically_checked.insert(edit.owner);
            add_owning_units(
                edit.owner,
                overlay,
                &mut ownership,
                &mut plan.compiler_units,
                &mut plan.work,
                &mut admitted,
                OwningUnitAdmission {
                    maximum_affected: admission.maximum_affected_owners,
                    maximum_ownership_steps: admission.maximum_ownership_steps,
                },
            )?;
            add_owning_units(
                edit.owner,
                overlay,
                &mut ownership,
                &mut plan.semantically_checked,
                &mut plan.work,
                &mut admitted,
                OwningUnitAdmission {
                    maximum_affected: admission.maximum_affected_owners,
                    maximum_ownership_steps: admission.maximum_ownership_steps,
                },
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
        let relation_read = relations.incoming(
            edit.owner,
            remaining_relations(
                admission.maximum_relation_edges,
                initial
                    .relation_edges_read
                    .saturating_add(plan.work.reverse_edges_visited),
            )?,
            admission.maximum_relation_fanout,
        )?;
        plan.work.reverse_edges_visited = plan
            .work
            .reverse_edges_visited
            .saturating_add(relation_read.edges_examined);
        for edge in relation_read.edges {
            if !change.affects_validation(edge.kind.propagation()) {
                continue;
            }
            let Some(source) = local_owner(edge.source, package) else {
                continue;
            };
            add_summary_paths(
                source,
                &mut ownership,
                &mut plan.summary_owners,
                &mut plan.work,
                &mut admitted,
                admission.maximum_affected_owners,
                admission.maximum_ownership_steps,
            )?;
            add_owning_units(
                source,
                overlay,
                &mut ownership,
                &mut plan.semantically_checked,
                &mut plan.work,
                &mut admitted,
                OwningUnitAdmission {
                    maximum_affected: admission.maximum_affected_owners,
                    maximum_ownership_steps: admission.maximum_ownership_steps,
                },
            )?;
            add_owning_units(
                source,
                overlay,
                &mut ownership,
                &mut plan.compiler_units,
                &mut plan.work,
                &mut admitted,
                OwningUnitAdmission {
                    maximum_affected: admission.maximum_affected_owners,
                    maximum_ownership_steps: admission.maximum_ownership_steps,
                },
            )?;
            plan.reasons.push(ImpactReason {
                kind: ImpactReasonKind::ValidationDependency,
                source,
                target: edit.owner,
                relation: Some(edge.kind),
            });
        }
    }

    for (dependency, edit) in &canonical.dependencies {
        if edit.before.is_none() {
            continue;
        }
        let relation_read = relations.incoming_package(
            *dependency,
            remaining_relations(
                admission.maximum_relation_edges,
                initial
                    .relation_edges_read
                    .saturating_add(plan.work.reverse_edges_visited),
            )?,
            admission.maximum_relation_fanout,
        )?;
        plan.work.reverse_edges_visited = plan
            .work
            .reverse_edges_visited
            .saturating_add(relation_read.edges_examined);
        for edge in relation_read.edges {
            let Some(source) = local_owner(edge.source, package) else {
                continue;
            };
            add_summary_paths(
                source,
                &mut ownership,
                &mut plan.summary_owners,
                &mut plan.work,
                &mut admitted,
                admission.maximum_affected_owners,
                admission.maximum_ownership_steps,
            )?;
            for declaration in owning_units(
                source,
                overlay,
                &mut ownership,
                &mut plan.work,
                admission.maximum_ownership_steps,
            )? {
                admit_affected(
                    &mut admitted,
                    declaration,
                    admission.maximum_affected_owners,
                )?;
                plan.semantically_checked.insert(declaration);
                plan.compiler_units.insert(declaration);
                if behavior_enqueued.insert(declaration) {
                    behavior.push_back(declaration);
                }
            }
            plan.reasons.push(ImpactReason {
                kind: ImpactReasonKind::DependencyBinding,
                source,
                target: source,
                relation: Some(edge.kind),
            });
        }
    }

    while let Some(target) = behavior.pop_front() {
        if behavior_seen.contains(&target) {
            continue;
        }
        admit_behavior_owner(&mut plan.work, admission.maximum_behavior_owners)?;
        behavior_seen.insert(target);
        let relation_read = relations.incoming(
            target,
            remaining_relations(
                admission.maximum_relation_edges,
                initial
                    .relation_edges_read
                    .saturating_add(plan.work.reverse_edges_visited),
            )?,
            admission.maximum_relation_fanout,
        )?;
        plan.work.reverse_edges_visited = plan
            .work
            .reverse_edges_visited
            .saturating_add(relation_read.edges_examined);
        for edge in relation_read.edges {
            if matches!(
                edge.kind.propagation(),
                PropagationClass::Ownership | PropagationClass::Presentation
            ) {
                continue;
            }
            let Some(source) = local_owner(edge.source, package) else {
                continue;
            };
            let declarations = owning_units(
                source,
                overlay,
                &mut ownership,
                &mut plan.work,
                admission.maximum_ownership_steps,
            )?;
            for declaration in declarations {
                if ownership.owner_kind(declaration, overlay)? == Some(OwnerKind::Test) {
                    admit_affected(
                        &mut admitted,
                        declaration,
                        admission.maximum_affected_owners,
                    )?;
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
            admit_affected(&mut admitted, *owner, admission.maximum_affected_owners)?;
            plan.tests.insert(*owner);
        }
    }
    plan.work.witness_reads = ownership.work();
    plan.work.witness_reads.add(relations.work());
    plan.reasons.sort_unstable_by(ImpactReason::logical_cmp);
    plan.reasons
        .dedup_by(|left, right| left.logical_cmp(right).is_eq());
    let final_relation_limit = remaining_relations(
        admission.maximum_relation_edges,
        initial
            .relation_edges_read
            .saturating_add(plan.work.reverse_edges_visited),
    )?;
    let final_summary_owners = u64::try_from(plan.summary_owners.len()).unwrap_or(u64::MAX);
    let total_summary_owners = plan
        .work
        .summary_owners_examined
        .checked_add(final_summary_owners)
        .ok_or_else(|| {
            impact_resource_error(
                "change_budget_impact_summary_owners",
                "summary-owner observation overflowed",
            )
        })?;
    admit_count(
        "change_budget_impact_summary_owners",
        "summary owners",
        total_summary_owners,
        admission.maximum_summary_owners,
    )?;
    plan.work.summary_owners_examined = total_summary_owners;
    let final_delta = derive_summary_delta_for_with_relation_limit(
        overlay,
        derived,
        base_witness,
        plan.summary_owners.clone(),
        final_relation_limit,
        admission.maximum_relation_fanout,
    )?;
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

    fn owner_kind<B: CanonicalBaseRead + ?Sized>(
        &mut self,
        owner: OwnerKey,
        overlay: &KernelOverlay<'_, B>,
    ) -> Result<Option<OwnerKind>, Diagnostic> {
        if let Some(record) = overlay.owner(owner)? {
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
    admitted: &mut BTreeSet<OwnerKey>,
    maximum_affected: u64,
    maximum_ownership_steps: u64,
) -> Result<(), Diagnostic> {
    walk_aggregating_path(
        owner,
        |key| ownership.before(key),
        selected,
        work,
        admitted,
        maximum_affected,
        maximum_ownership_steps,
    )?;
    walk_aggregating_path(
        owner,
        |key| ownership.candidate(key),
        selected,
        work,
        admitted,
        maximum_affected,
        maximum_ownership_steps,
    )
}

fn walk_aggregating_path(
    owner: OwnerKey,
    mut lookup: impl FnMut(OwnerKey) -> Result<Option<OwnershipEntry>, Diagnostic>,
    selected: &mut BTreeSet<OwnerKey>,
    work: &mut ImpactWork,
    admitted: &mut BTreeSet<OwnerKey>,
    maximum_affected: u64,
    maximum_ownership_steps: u64,
) -> Result<(), Diagnostic> {
    let mut current = owner;
    let mut observed = BTreeSet::new();
    loop {
        if !observed.insert(current) {
            return Err(impact_error(
                "change_impact_ownership_cycle",
                "impact ownership path is cyclic",
            ));
        }
        admit_ownership_step(work, maximum_ownership_steps)?;
        admit_affected(admitted, current, maximum_affected)?;
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

#[derive(Clone, Copy)]
struct OwningUnitAdmission {
    maximum_affected: u64,
    maximum_ownership_steps: u64,
}

fn add_owning_units<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    owner: OwnerKey,
    overlay: &KernelOverlay<'_, B>,
    ownership: &mut CandidateOwnership<'_, W>,
    units: &mut BTreeSet<OwnerKey>,
    work: &mut ImpactWork,
    admitted: &mut BTreeSet<OwnerKey>,
    admission: OwningUnitAdmission,
) -> Result<(), Diagnostic> {
    for unit in owning_units(
        owner,
        overlay,
        ownership,
        work,
        admission.maximum_ownership_steps,
    )? {
        admit_affected(admitted, unit, admission.maximum_affected)?;
        units.insert(unit);
    }
    Ok(())
}

fn admit_affected(
    admitted: &mut BTreeSet<OwnerKey>,
    owner: OwnerKey,
    maximum: u64,
) -> Result<(), Diagnostic> {
    if admitted.contains(&owner) {
        return Ok(());
    }
    if u64::try_from(admitted.len()).unwrap_or(u64::MAX) >= maximum {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "change_budget_affected_frontier_owners",
            format!("affected frontier exceeds the declared {maximum}-owner budget"),
        ));
    }
    admitted.insert(owner);
    Ok(())
}

fn admit_summary_edit(work: &mut ImpactWork, maximum: u64) -> Result<(), Diagnostic> {
    if work.summary_edits_examined >= maximum {
        return Err(impact_resource_error(
            "change_budget_impact_summary_edits",
            format!("impact planning exceeds the declared {maximum}-summary-edit budget"),
        ));
    }
    work.summary_edits_examined = work.summary_edits_examined.saturating_add(1);
    Ok(())
}

fn admit_behavior_owner(work: &mut ImpactWork, maximum: u64) -> Result<(), Diagnostic> {
    if work.behavior_owners_visited >= maximum {
        return Err(impact_resource_error(
            "change_budget_impact_behavior_owners",
            format!("impact planning exceeds the declared {maximum}-behavior-owner budget"),
        ));
    }
    work.behavior_owners_visited = work.behavior_owners_visited.saturating_add(1);
    Ok(())
}

fn admit_count(
    code: &'static str,
    unit: &'static str,
    observed: u64,
    maximum: u64,
) -> Result<(), Diagnostic> {
    if observed > maximum {
        return Err(impact_resource_error(
            code,
            format!("impact planning requires {observed} {unit}, exceeding the declared {maximum}"),
        ));
    }
    Ok(())
}

fn remaining_relations(maximum: u64, observed: u64) -> Result<u64, Diagnostic> {
    maximum.checked_sub(observed).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "change_budget_relation_edges",
            format!(
                "relation traversal observed {observed} edges, exceeding the declared {maximum}-edge budget"
            ),
        )
    })
}

fn admit_ownership_step(work: &mut ImpactWork, maximum: u64) -> Result<(), Diagnostic> {
    if work.ownership_steps >= maximum {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "change_budget_impact_ownership_steps",
            format!("impact ownership traversal exceeds the declared {maximum}-step budget"),
        ));
    }
    work.ownership_steps = work.ownership_steps.saturating_add(1);
    Ok(())
}

fn owning_units<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    owner: OwnerKey,
    overlay: &KernelOverlay<'_, B>,
    ownership: &mut CandidateOwnership<'_, W>,
    work: &mut ImpactWork,
    maximum_ownership_steps: u64,
) -> Result<BTreeSet<OwnerKey>, Diagnostic> {
    let mut units = BTreeSet::new();
    for candidate in [false, true] {
        let mut current = owner;
        let mut observed = BTreeSet::new();
        loop {
            if !observed.insert(current) {
                return Err(impact_error(
                    "change_impact_unit_cycle",
                    "compiler-unit ownership path is cyclic",
                ));
            }
            admit_ownership_step(work, maximum_ownership_steps)?;
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
                if overlay.owner(current)?.is_some_and(|record| {
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

fn impact_resource_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Resource, code, message)
}
