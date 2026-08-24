//! Declaration-local maintenance of graph-owned test dependency facts.

use super::relation_view::CandidateRelations;
use super::{
    CanonicalBaseRead, CanonicalDelta, DerivedDelta, KernelOverlay, WitnessBaseRead,
    WitnessReadWork,
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

pub fn derive_test_dependency_delta<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    overlay: &KernelOverlay<'_, B>,
    canonical: &CanonicalDelta,
    derived: &DerivedDelta,
    base_witness: &W,
) -> Result<TestDependencyDelta, Diagnostic> {
    let mut witness = CandidateTestWitness::new(derived, base_witness);
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
            if let Some(declaration) = owning_declaration(seed, candidate, &mut witness, &mut work)?
                && witness.test_kind(declaration, overlay)?
            {
                affected_tests.insert(declaration);
            }
        }
        if witness.test_kind(seed, overlay)? {
            affected_tests.insert(seed);
        }
    }

    let mut relations = CandidateRelations::new(overlay.package_id(), derived, base_witness);
    let mut removed = BTreeSet::new();
    let mut added = BTreeSet::new();
    for test in &affected_tests {
        let before = witness.test_dependencies(*test)?;
        let after = if overlay
            .owner(*test)?
            .is_some_and(|record| record.kind() == OwnerKind::Test)
        {
            candidate_test_dependencies(*test, overlay, &mut witness, &mut relations, &mut work)?
        } else {
            BTreeSet::new()
        };
        removed.extend(before.difference(&after).copied());
        added.extend(after.difference(&before).copied());
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
) -> Result<BTreeSet<TestDependency>, Diagnostic> {
    let mut dependencies = BTreeSet::new();
    let mut pending = VecDeque::from([test]);
    let mut visited = BTreeSet::new();
    while let Some(owner) = pending.pop_front() {
        if !visited.insert(owner) {
            continue;
        }
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
        for edge in relations.outgoing(owner)? {
            work.relation_edges_visited = work.relation_edges_visited.saturating_add(1);
            if matches!(
                edge.kind.propagation(),
                PropagationClass::Ownership
                    | PropagationClass::Presentation
                    | PropagationClass::Test
            ) {
                continue;
            }
            if let Some(target) = local_owner(edge.target, overlay.package_id())
                && owning_declaration(target, true, witness, work)? == Some(test)
            {
                continue;
            }
            dependencies.insert(TestDependency {
                test,
                kind: edge.kind,
                target: edge.target,
            });
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
}

impl<'a, W: WitnessBaseRead + ?Sized> CandidateTestWitness<'a, W> {
    fn new(derived: &DerivedDelta, base: &'a W) -> Self {
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
    ) -> Result<BTreeSet<TestDependency>, Diagnostic> {
        if let Some(cached) = self.dependencies.get(&test) {
            return Ok(cached.clone());
        }
        let read = self
            .base
            .read_test_dependencies(test, MAXIMUM_TEST_DEPENDENCY_PREFIX_ITEMS)?;
        self.work.add(read.work);
        if read.value.truncated {
            return Err(test_resource_error(
                "change_test_dependency_budget",
                "test dependency prefix exceeds the current per-test work budget",
            ));
        }
        let dependencies = read.value.dependencies.into_iter().collect::<BTreeSet<_>>();
        self.dependencies.insert(test, dependencies.clone());
        Ok(dependencies)
    }
}

fn owning_declaration<W: WitnessBaseRead + ?Sized>(
    owner: OwnerKey,
    candidate: bool,
    witness: &mut CandidateTestWitness<'_, W>,
    work: &mut TestDeltaWork,
) -> Result<Option<OwnerKey>, Diagnostic> {
    let mut current = owner;
    let mut observed = BTreeSet::new();
    loop {
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
