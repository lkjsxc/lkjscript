//! Declaration-local maintenance of graph-owned test dependency facts.

use super::relation_view::CandidateRelations;
use super::{BudgetCounter, BudgetDimension, WorkSaturation, minimum_limit, optional_usize};
use super::{
    CanonicalBaseRead, CanonicalDelta, ChangeBudget, DerivedDelta, KernelOverlay, TestAdmission,
    WitnessBaseRead, WitnessReadWork,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    ExactOwnerKey, OwnerKey, OwnerKind, OwnerRecord, PropagationClass, RelationEndpoint,
};
use crate::platform::witness::{
    OwnershipEntry, OwnershipParent, TestDependency,
    aggregation_children_with_bindings,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TestDeltaWork {
    pub ownership_steps: u64,
    pub owners_visited: u64,
    pub relation_edges_visited: u64,
    pub witness_reads: WitnessReadWork,
    pub saturation: WorkSaturation,
}

#[derive(Clone, Debug, Default)]
pub struct TestDependencyDelta {
    pub affected_tests: BTreeSet<OwnerKey>,
    pub removed: BTreeSet<TestDependency>,
    pub added: BTreeSet<TestDependency>,
    pub work: TestDeltaWork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TestDependencyAdmission {
    tests: TestAdmission,
    maximum_relation_edges: Option<u64>,
    maximum_relation_fanout: Option<u64>,
    maximum_witness_records: Option<u64>,
    maximum_dependency_changes: Option<u64>,
}

impl TestDependencyAdmission {
    pub(crate) const fn new(
        tests: TestAdmission,
        maximum_relation_edges: Option<u64>,
        maximum_relation_fanout: Option<u64>,
        maximum_witness_records: Option<u64>,
        maximum_dependency_changes: Option<u64>,
    ) -> Self {
        Self {
            tests,
            maximum_relation_edges,
            maximum_relation_fanout,
            maximum_witness_records,
            maximum_dependency_changes,
        }
    }
}

impl Default for TestDependencyAdmission {
    fn default() -> Self {
        let budget = ChangeBudget::default();
        Self::new(
            budget.tests,
            budget.impact.maximum_relation_edges,
            budget.impact.maximum_relation_fanout,
            budget.witness_reads.maximum_decoded_records,
            budget.witness_update.maximum_edits.map(|maximum| maximum / 2),
        )
    }
}

pub fn derive_test_dependency_delta<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    overlay: &KernelOverlay<'_, B>,
    canonical: &CanonicalDelta,
    derived: &DerivedDelta,
    base_witness: &W,
) -> Result<TestDependencyDelta, Diagnostic> {
    derive_test_dependency_delta_with_admission(
        overlay,
        canonical,
        derived,
        base_witness,
        TestDependencyAdmission::default(),
    )
}

pub(crate) fn derive_test_dependency_delta_with_admission<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    overlay: &KernelOverlay<'_, B>,
    canonical: &CanonicalDelta,
    derived: &DerivedDelta,
    base_witness: &W,
    admission: TestDependencyAdmission,
) -> Result<TestDependencyDelta, Diagnostic> {
    let mut witness =
        CandidateTestWitness::new(derived, base_witness, admission.maximum_witness_records);
    let mut work = TestDeltaWork::default();
    let mut seeds = canonical.owners.keys().copied().collect::<BTreeSet<_>>();
    seeds.extend(derived.ownership.iter().map(|edit| edit.key));
    seeds.extend(
        derived
            .relations
            .removed
            .iter()
            .chain(&derived.relations.added)
            .filter_map(|edge| local_owner(edge.source, overlay.package_id())),
    );
    let mut affected_tests = BTreeSet::new();
    for seed in seeds {
        for candidate in [false, true] {
            if let Some(declaration) =
                owning_declaration(seed, candidate, &mut witness, &mut work, admission.tests)?
                && witness.test_kind(declaration, overlay)?
            {
                admit_test(
                    &mut affected_tests,
                    declaration,
                    admission.tests.maximum_selected,
                )?;
            }
        }
        if witness.test_kind(seed, overlay)? {
            admit_test(&mut affected_tests, seed, admission.tests.maximum_selected)?;
        }
    }

    let mut relations = CandidateRelations::new(overlay.package_id(), derived, base_witness);
    let mut removed = BTreeSet::new();
    let mut added = BTreeSet::new();
    for test in &affected_tests {
        let before =
            witness.test_dependencies(*test, admission.tests.maximum_dependencies_per_test)?;
        let after = if overlay
            .owner(*test)?
            .is_some_and(|record| record.kind() == OwnerKind::Test)
        {
            candidate_test_dependencies(
                *test,
                overlay,
                &mut witness,
                &mut relations,
                &mut work,
                admission,
            )?
        } else {
            BTreeSet::new()
        };
        for dependency in before.difference(&after).copied() {
            admit_dependency_change(
                &mut removed,
                &added,
                dependency,
                admission.maximum_dependency_changes,
            )?;
        }
        for dependency in after.difference(&before).copied() {
            admit_dependency_change(
                &mut added,
                &removed,
                dependency,
                admission.maximum_dependency_changes,
            )?;
        }
    }
    work.witness_reads = witness.work();
    work.witness_reads.add(relations.work());
    Ok(TestDependencyDelta {
        affected_tests,
        removed,
        added,
        work,
    })
}

fn candidate_test_dependencies<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    test: OwnerKey,
    overlay: &KernelOverlay<'_, B>,
    witness: &mut CandidateTestWitness<'_, W>,
    relations: &mut CandidateRelations<'_, W>,
    work: &mut TestDeltaWork,
    admission: TestDependencyAdmission,
) -> Result<BTreeSet<TestDependency>, Diagnostic> {
    let mut dependencies = BTreeSet::new();
    let mut pending = VecDeque::from([test]);
    let mut visited = BTreeSet::new();
    while let Some(owner) = pending.pop_front() {
        if visited.contains(&owner) {
            continue;
        }
        work.saturation.charge(&mut work.owners_visited, BudgetDimension::TestOwnersVisited,
            BudgetCounter::exact(1), admission.tests.maximum_owners_visited, "test dependency traversal")?;
        visited.insert(owner);
        let record = overlay.owner(owner)?.ok_or_else(|| {
            test_error(
                "change_test_owner_missing",
                format!("test semantic closure references missing owner {owner:?}"),
            )
        })?;
        pending.extend(
            aggregation_children_with_bindings(&record, |binding| {
                match overlay.owner(OwnerKey::Binding(binding))? {
                    Some(OwnerRecord::Binding(record)) => Ok(Some(record)),
                    None => Ok(None),
                    Some(_) => Err(Diagnostic::new(
                        DiagnosticClass::Corrupt,
                        "change_test_binding_record",
                        "test closure binding identity selects a foreign candidate record",
                    )),
                }
            })?
            .into_iter()
            .filter(|(role, _)| role.aggregates_into_parent())
            .map(|(_, child)| child),
        );
        let remaining_relations = work.saturation
            .counter(BudgetDimension::RelationEdges, work.relation_edges_visited)
            .remaining(admission.maximum_relation_edges, "test dependency traversal", "relation_edges")?;
        let relation_read = relations.outgoing(
            owner,
            remaining_relations,
            admission.maximum_relation_fanout,
        )?;
        work.saturation.observe(&mut work.relation_edges_visited, BudgetDimension::RelationEdges, relation_read.edges_examined);
        for edge in relation_read.edges {
            if matches!(
                edge.kind.propagation(),
                PropagationClass::Ownership
                    | PropagationClass::Presentation
                    | PropagationClass::Test
            ) {
                continue;
            }
            if let Some(target) = local_owner(edge.target, overlay.package_id())
                && owning_declaration(target, true, witness, work, admission.tests)? == Some(test)
            {
                continue;
            }
            admit_test_dependency(
                &mut dependencies,
                TestDependency {
                    test,
                    kind: edge.kind,
                    target: edge.target,
                },
                admission.tests.maximum_dependencies_per_test,
            )?;
        }
    }
    Ok(dependencies)
}

struct CandidateTestWitness<'a, W: ?Sized> {
    edits: BTreeMap<OwnerKey, Option<OwnershipEntry>>,
    base: &'a W,
    ownership: BTreeMap<OwnerKey, Option<OwnershipEntry>>,
    kinds: BTreeMap<OwnerKey, Option<OwnerKind>>,
    dependencies: BTreeMap<OwnerKey, BTreeSet<TestDependency>>,
    work: WitnessReadWork,
    maximum_witness_records: Option<u64>,
}

impl<'a, W: WitnessBaseRead + ?Sized> CandidateTestWitness<'a, W> {
    fn new(derived: &DerivedDelta, base: &'a W, maximum_witness_records: Option<u64>) -> Self {
        Self {
            edits: derived
                .ownership
                .iter()
                .map(|edit| (edit.key, edit.after))
                .collect(),
            base,
            ownership: BTreeMap::new(),
            kinds: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            work: WitnessReadWork::default(),
            maximum_witness_records,
        }
    }

    const fn work(&self) -> WitnessReadWork {
        self.work
    }

    fn ownership(
        &mut self,
        owner: OwnerKey,
        candidate: bool,
    ) -> Result<Option<OwnershipEntry>, Diagnostic> {
        if candidate && let Some(entry) = self.edits.get(&owner) {
            return Ok(*entry);
        }
        if let Some(cached) = self.ownership.get(&owner) {
            return Ok(*cached);
        }
        let read = self.base.read_ownership(owner)?;
        self.work.add(read.work);
        self.ownership.insert(owner, read.value);
        Ok(read.value)
    }

    fn test_kind<B: CanonicalBaseRead + ?Sized>(
        &mut self,
        owner: OwnerKey,
        overlay: &KernelOverlay<'_, B>,
    ) -> Result<bool, Diagnostic> {
        if let Some(record) = overlay.owner(owner)? {
            return Ok(record.kind() == OwnerKind::Test);
        }
        let kind = if let Some(cached) = self.kinds.get(&owner) {
            *cached
        } else {
            let read = self.base.read_owner_summary(owner)?;
            self.work.add(read.work);
            let kind = read.value.map(|bound| bound.summary.kind);
            self.kinds.insert(owner, kind);
            kind
        };
        Ok(kind == Some(OwnerKind::Test))
    }

    fn test_dependencies(
        &mut self,
        test: OwnerKey,
        maximum_per_test: Option<u64>,
    ) -> Result<BTreeSet<TestDependency>, Diagnostic> {
        if let Some(cached) = self.dependencies.get(&test) {
            return Ok(cached.clone());
        }
        let remaining_records = self.work.observation(BudgetDimension::WitnessDecodedRecords,
            self.work.witness_records_decoded).remaining(self.maximum_witness_records,
                "test dependency reads", "witness_decoded_records")?;
        let maximum_items = minimum_limit(maximum_per_test, remaining_records);
        let read = self.base.read_test_dependencies(test,
            optional_usize(maximum_items, "test_dependencies_per_test")?)?;
        self.work.add(read.work);
        if read.value.truncated {
            let (code, limit) = if remaining_records.is_some_and(|records| maximum_per_test.is_none_or(|per_test| records < per_test)) {
                ("change_budget_witness_decoded_records", remaining_records)
            } else {
                ("change_budget_test_dependencies_per_test", maximum_per_test)
            };
            return Err(test_resource_error(
                code,
                format!("test dependency prefix exceeds the declared {limit:?}-record budget"),
            ));
        }
        let dependencies = read.value.dependencies.into_iter().collect::<BTreeSet<_>>();
        self.dependencies.insert(test, dependencies.clone());
        Ok(dependencies)
    }
}

fn admit_dependency_change(
    destination: &mut BTreeSet<TestDependency>,
    other: &BTreeSet<TestDependency>,
    dependency: TestDependency,
    maximum: Option<u64>,
) -> Result<(), Diagnostic> {
    if destination.contains(&dependency) {
        return Ok(());
    }
    let mut observed = BudgetCounter::exact(destination.len() as u64);
    observed.observe(BudgetCounter::exact(other.len() as u64));
    observed.charge(BudgetCounter::exact(1), maximum, "test dependency delta", "witness_edits")?;
    destination.insert(dependency);
    Ok(())
}

fn admit_test_dependency(
    dependencies: &mut BTreeSet<TestDependency>,
    dependency: TestDependency,
    maximum: Option<u64>,
) -> Result<(), Diagnostic> {
    if dependencies.contains(&dependency) {
        return Ok(());
    }
    if let Some(maximum) = maximum && dependencies.len() as u64 >= maximum {
        return Err(test_resource_error(
            "change_budget_test_dependencies_per_test",
            format!(
                "candidate test dependency closure exceeds the declared {maximum}-dependency per-test budget"
            ),
        ));
    }
    dependencies.insert(dependency);
    Ok(())
}

fn owning_declaration<W: WitnessBaseRead + ?Sized>(
    owner: OwnerKey,
    candidate: bool,
    witness: &mut CandidateTestWitness<'_, W>,
    work: &mut TestDeltaWork,
    admission: TestAdmission,
) -> Result<Option<OwnerKey>, Diagnostic> {
    let mut current = owner;
    let mut observed = BTreeSet::new();
    loop {
        work.saturation.charge(&mut work.ownership_steps, BudgetDimension::TestOwnershipSteps,
            BudgetCounter::exact(1), admission.maximum_ownership_steps, "test ownership traversal")?;
        if matches!(current, OwnerKey::Declaration(_)) {
            return Ok(Some(current));
        }
        if !observed.insert(current) {
            return Err(test_error(
                "change_test_ownership_cycle",
                "test dependency ownership path is cyclic",
            ));
        }
        let Some(entry) = witness.ownership(current, candidate)? else {
            return Ok(None);
        };
        match entry.parent {
            OwnershipParent::Package => return Ok(None),
            OwnershipParent::Owner(parent) => current = parent,
        }
    }
}

fn admit_test(
    selected: &mut BTreeSet<OwnerKey>,
    test: OwnerKey,
    maximum: Option<u64>,
) -> Result<(), Diagnostic> {
    if selected.contains(&test) {
        return Ok(());
    }
    if let Some(maximum) = maximum && selected.len() as u64 >= maximum {
        return Err(test_resource_error(
            "change_budget_tests_selected",
            format!("test selection exceeds the declared {maximum}-test budget"),
        ));
    }
    selected.insert(test);
    Ok(())
}

fn local_owner(
    endpoint: RelationEndpoint,
    package: crate::platform::kernel::PackageId,
) -> Option<OwnerKey> {
    match endpoint {
        RelationEndpoint::Owner(ExactOwnerKey {
            package: endpoint_package,
            owner,
        }) if endpoint_package == package => Some(owner),
        RelationEndpoint::Owner(_) | RelationEndpoint::Package(_) => None,
    }
}

fn test_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}

fn test_resource_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Resource, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::kernel::{ExactOwnerKey, RelationKind};
    use crate::platform::semantic_id::DeclarationId;

    #[test]
    fn candidate_test_dependencies_admit_before_insertion() {
        let package = crate::platform::kernel::PackageId::migrate(b"test-dependency-budget", 0);
        let test = OwnerKey::Declaration(DeclarationId::migrate(b"test-dependency-budget", 1));
        let dependency = TestDependency {
            test,
            kind: RelationKind::FunctionCall,
            target: RelationEndpoint::Owner(ExactOwnerKey {
                package,
                owner: OwnerKey::Declaration(DeclarationId::migrate(b"test-dependency-budget", 2)),
            }),
        };

        let mut dependencies = BTreeSet::new();
        assert_eq!(
            admit_test_dependency(&mut dependencies, dependency, Some(0))
                .expect_err("zero admission must reject before insertion")
                .code,
            "change_budget_test_dependencies_per_test"
        );
        assert!(dependencies.is_empty());

        admit_test_dependency(&mut dependencies, dependency, Some(1))
            .expect("one dependency fits one exact admission");
        admit_test_dependency(&mut dependencies, dependency, Some(1))
            .expect("an equal dependency consumes no additional admission");
        let second = TestDependency {
            target: RelationEndpoint::Owner(ExactOwnerKey {
                package,
                owner: OwnerKey::Declaration(DeclarationId::migrate(b"test-dependency-budget", 3)),
            }),
            ..dependency
        };
        assert_eq!(
            admit_test_dependency(&mut dependencies, second, Some(1))
                .expect_err("a second dependency must reject before insertion")
                .code,
            "change_budget_test_dependencies_per_test"
        );
        assert_eq!(dependencies.len(), 1);
    }
}
