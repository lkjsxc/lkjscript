//! Declaration-local maintenance of graph-owned test dependency facts.

use super::relation_view::CandidateRelations;
use super::{CanonicalDelta, DerivedDelta, KernelOverlay};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    ExactOwnerKey, OwnerKey, OwnerKind, PropagationClass, RelationEndpoint,
};
use crate::platform::witness::{
    FullWitness, OwnershipEntry, OwnershipParent, TestDependency, aggregation_children,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TestDeltaWork {
    pub ownership_steps: u64,
    pub owners_visited: u64,
    pub relation_edges_visited: u64,
}

#[derive(Clone, Debug, Default)]
pub struct TestDependencyDelta {
    pub affected_tests: BTreeSet<OwnerKey>,
    pub removed: BTreeSet<TestDependency>,
    pub added: BTreeSet<TestDependency>,
    pub work: TestDeltaWork,
}

pub fn derive_test_dependency_delta(
    overlay: &KernelOverlay<'_>,
    canonical: &CanonicalDelta,
    derived: &DerivedDelta,
    base_witness: &FullWitness,
) -> Result<TestDependencyDelta, Diagnostic> {
    let ownership = CandidateTestOwnership::new(derived, base_witness);
    let mut work = TestDeltaWork::default();
    let mut seeds = canonical.owners.keys().copied().collect::<BTreeSet<_>>();
    seeds.extend(derived.ownership.iter().map(|edit| edit.key));
    seeds.extend(
        derived
            .relations
            .removed
            .iter()
            .chain(&derived.relations.added)
            .filter_map(|edge| local_owner(edge.source, overlay.base().root.package_id)),
    );
    let mut affected_tests = BTreeSet::new();
    for seed in seeds {
        for candidate in [false, true] {
            if let Some(declaration) = owning_declaration(seed, candidate, &ownership, &mut work)?
                && test_kind(declaration, overlay, base_witness)
            {
                affected_tests.insert(declaration);
            }
        }
        if test_kind(seed, overlay, base_witness) {
            affected_tests.insert(seed);
        }
    }

    let relations = CandidateRelations::new(overlay.base().root.package_id, derived, base_witness);
    let mut removed = BTreeSet::new();
    let mut added = BTreeSet::new();
    for test in &affected_tests {
        let before = base_witness
            .entries
            .test_dependencies_by_test
            .get(test)
            .cloned()
            .unwrap_or_default();
        let after = if overlay
            .owner(*test)
            .is_some_and(|record| record.kind() == OwnerKind::Test)
        {
            candidate_test_dependencies(*test, overlay, &ownership, &relations, &mut work)?
        } else {
            BTreeSet::new()
        };
        removed.extend(before.difference(&after).copied());
        added.extend(after.difference(&before).copied());
    }
    Ok(TestDependencyDelta {
        affected_tests,
        removed,
        added,
        work,
    })
}

fn candidate_test_dependencies(
    test: OwnerKey,
    overlay: &KernelOverlay<'_>,
    ownership: &CandidateTestOwnership<'_>,
    relations: &CandidateRelations<'_>,
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
        let record = overlay.owner(owner).ok_or_else(|| {
            test_error(
                "change_test_owner_missing",
                format!("test semantic closure references missing owner {owner:?}"),
            )
        })?;
        pending.extend(
            aggregation_children(record)?
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
            if let Some(target) = local_owner(edge.target, overlay.base().root.package_id)
                && owning_declaration(target, true, ownership, work)? == Some(test)
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

struct CandidateTestOwnership<'a> {
    edits: BTreeMap<OwnerKey, Option<OwnershipEntry>>,
    base: &'a FullWitness,
}

impl<'a> CandidateTestOwnership<'a> {
    fn new(derived: &DerivedDelta, base: &'a FullWitness) -> Self {
        Self {
            edits: derived
                .ownership
                .iter()
                .map(|edit| (edit.key, edit.after))
                .collect(),
            base,
        }
    }

    fn get(&self, owner: OwnerKey, candidate: bool) -> Option<OwnershipEntry> {
        if candidate {
            match self.edits.get(&owner) {
                Some(entry) => *entry,
                None => self.base.entries.ownership.get(&owner).copied(),
            }
        } else {
            self.base.entries.ownership.get(&owner).copied()
        }
    }
}

fn owning_declaration(
    owner: OwnerKey,
    candidate: bool,
    ownership: &CandidateTestOwnership<'_>,
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
        let Some(entry) = ownership.get(current, candidate) else {
            return Ok(None);
        };
        match entry.parent {
            OwnershipParent::Package => return Ok(None),
            OwnershipParent::Owner(parent) => current = parent,
        }
    }
}

fn test_kind(owner: OwnerKey, overlay: &KernelOverlay<'_>, base: &FullWitness) -> bool {
    overlay
        .owner(owner)
        .map(crate::platform::kernel::OwnerRecord::kind)
        .or_else(|| base.summaries.get(&owner).map(|summary| summary.kind))
        == Some(OwnerKind::Test)
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
