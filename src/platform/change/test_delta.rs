//! Declaration-local maintenance of graph-owned test dependency facts.

use super::relation_view::CandidateRelations;
use super::{
    CanonicalBaseRead, CanonicalDelta, ChangeBudget, DerivedDelta, KernelOverlay, TestAdmission,
    WitnessBaseRead, WitnessReadWork,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    ExactOwnerKey, OwnerKey, OwnerKind, PropagationClass, RelationEndpoint,
};
use crate::platform::witness::{
    MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS, OwnershipEntry, OwnershipParent, TestDependency,
    aggregation_children,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TestDeltaWork {
    pub ownership_steps: u64,
    pub owners_visited: u64,
    pub relation_edges_visited: u64,
    pub witness_reads: WitnessReadWork,
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
    maximum_relation_edges: u64,
    maximum_relation_fanout: u64,
    maximum_witness_records: u64,
    maximum_dependency_changes: u64,
}

impl TestDependencyAdmission {
    pub(crate) const fn new(
        tests: TestAdmission,
        maximum_relation_edges: u64,
        maximum_relation_fanout: u64,
        maximum_witness_records: u64,
        maximum_dependency_changes: u64,
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
            budget.witness_update.maximum_edits / 2,
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
        if work.owners_visited >= admission.tests.maximum_owners_visited {
            return Err(test_resource_error(
                "change_budget_test_owners_visited",
                format!(
                    "test dependency traversal exceeds the declared {}-owner budget",
                    admission.tests.maximum_owners_visited
                ),
            ));
        }
        visited.insert(owner);
        work.owners_visited = work.owners_visited.saturating_add(1);
        let record = overlay.owner(owner)?.ok_or_else(|| {
            test_error(
                "change_test_owner_missing",
                format!("test semantic closure references missing owner {owner:?}"),
            )
        })?;
        pending.extend(
            aggregation_children(&record)?
                .into_iter()
                .filter(|(role, _)| role.aggregates_into_parent())
                .map(|(_, child)| child),
        );
        let remaining_relations = admission
            .maximum_relation_edges
            .checked_sub(work.relation_edges_visited)
            .ok_or_else(|| {
                test_resource_error(
                    "change_budget_relation_edges",
                    "test dependency relation traversal exceeded its edge budget",
                )
            })?;
        let relation_read = relations.outgoing(
            owner,
            remaining_relations,
            admission.maximum_relation_fanout,
        )?;
        work.relation_edges_visited = work
            .relation_edges_visited
            .saturating_add(relation_read.edges_examined);
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
    maximum_witness_records: u64,
}

impl<'a, W: WitnessBaseRead + ?Sized> CandidateTestWitness<'a, W> {
    fn new(derived: &DerivedDelta, base: &'a W, maximum_witness_records: u64) -> Self {
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
        maximum_per_test: u64,
    ) -> Result<BTreeSet<TestDependency>, Diagnostic> {
        if let Some(cached) = self.dependencies.get(&test) {
            return Ok(cached.clone());
        }
        let remaining_records = self
            .maximum_witness_records
            .checked_sub(self.work.witness_records_decoded)
            .ok_or_else(|| {
                test_resource_error(
                    "change_budget_witness_decoded_records",
                    "test dependency reads exhausted the remaining witness-record budget",
                )
            })?;
        let maximum_items = maximum_per_test
            .min(remaining_records)
            .min(MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS as u64);
        if maximum_items == 0 {
            let (code, message) = if remaining_records == 0 {
                (
                    "change_budget_witness_decoded_records",
                    "test dependency read has no remaining witness-record budget",
                )
            } else {
                (
                    "change_budget_test_dependencies_per_test",
                    "test dependency read has a zero per-test dependency budget",
                )
            };
            return Err(test_resource_error(code, message));
        }
        let read = self.base.read_test_dependencies(
            test,
            usize::try_from(maximum_items).unwrap_or(MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS),
        )?;
        self.work.add(read.work);
        if read.value.truncated {
            let (code, limit) = if remaining_records < maximum_per_test {
                ("change_budget_witness_decoded_records", remaining_records)
            } else {
                ("change_budget_test_dependencies_per_test", maximum_per_test)
            };
            return Err(test_resource_error(
                code,
                format!("test dependency prefix exceeds the declared {limit}-record budget"),
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
    maximum: u64,
) -> Result<(), Diagnostic> {
    if destination.contains(&dependency) {
        return Ok(());
    }
    let observed = u64::try_from(destination.len())
        .unwrap_or(u64::MAX)
        .saturating_add(u64::try_from(other.len()).unwrap_or(u64::MAX));
    if observed >= maximum {
        return Err(test_resource_error(
            "change_budget_witness_edits",
            format!(
                "test dependency delta exceeds the remaining {maximum}-dependency-change witness budget"
            ),
        ));
    }
    destination.insert(dependency);
    Ok(())
}

fn admit_test_dependency(
    dependencies: &mut BTreeSet<TestDependency>,
    dependency: TestDependency,
    maximum: u64,
) -> Result<(), Diagnostic> {
    if dependencies.contains(&dependency) {
        return Ok(());
    }
    if u64::try_from(dependencies.len()).unwrap_or(u64::MAX) >= maximum {
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
        if work.ownership_steps >= admission.maximum_ownership_steps {
            return Err(test_resource_error(
                "change_budget_test_ownership_steps",
                format!(
                    "test ownership traversal exceeds the declared {}-step budget",
                    admission.maximum_ownership_steps
                ),
            ));
        }
        work.ownership_steps = work.ownership_steps.saturating_add(1);
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
    maximum: u64,
) -> Result<(), Diagnostic> {
    if selected.contains(&test) {
        return Ok(());
    }
    if u64::try_from(selected.len()).unwrap_or(u64::MAX) >= maximum {
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
            admit_test_dependency(&mut dependencies, dependency, 0)
                .expect_err("zero admission must reject before insertion")
                .code,
            "change_budget_test_dependencies_per_test"
        );
        assert!(dependencies.is_empty());

        admit_test_dependency(&mut dependencies, dependency, 1)
            .expect("one dependency fits one exact admission");
        admit_test_dependency(&mut dependencies, dependency, 1)
            .expect("an equal dependency consumes no additional admission");
        let second = TestDependency {
            target: RelationEndpoint::Owner(ExactOwnerKey {
                package,
                owner: OwnerKey::Declaration(DeclarationId::migrate(b"test-dependency-budget", 3)),
            }),
            ..dependency
        };
        assert_eq!(
            admit_test_dependency(&mut dependencies, second, 1)
                .expect_err("a second dependency must reject before insertion")
                .code,
            "change_budget_test_dependencies_per_test"
        );
        assert_eq!(dependencies.len(), 1);
    }
}
