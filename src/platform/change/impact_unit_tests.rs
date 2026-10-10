//! Isolated ownership projection laws; these synthetic read maps are not admitted programs.
use super::*;
use crate::platform::change::DerivedValueEdit;
use crate::platform::semantic_id::{DeclarationId, ExpressionId, HttpRouteId, PortId, TargetId};
use crate::platform::witness::{OwnershipRole, rebuild_full_witness};

type Parents = BTreeMap<OwnerKey, OwnershipEntry>;

fn edge(parent: OwnerKey, role: OwnershipRole) -> OwnershipEntry {
    OwnershipEntry::new(OwnershipParent::Owner(parent), role)
}

fn delta(before: &Parents, after: &Parents) -> DerivedDelta {
    let keys: BTreeSet<_> = before.keys().chain(after.keys()).copied().collect();
    DerivedDelta {
        ownership: keys
            .into_iter()
            .filter_map(|key| {
                let before = before.get(&key).copied();
                let after = after.get(&key).copied();
                (before != after).then_some(DerivedValueEdit { key, before, after })
            })
            .collect(),
        ..DerivedDelta::default()
    }
}

// Independent forward closure from each unit; never follows a queried child's parent chain.
fn oracle(query: OwnerKey, roots: &[OwnerKey], snapshots: &[&Parents]) -> BTreeSet<OwnerKey> {
    roots
        .iter()
        .copied()
        .filter(|root| {
            snapshots.iter().any(|parents| {
                let mut descendants = BTreeSet::from([*root]);
                loop {
                    let before = descendants.len();
                    for (child, entry) in *parents {
                        if let OwnershipParent::Owner(parent) = entry.parent
                            && descendants.contains(&parent)
                        {
                            descendants.insert(*child);
                        }
                    }
                    if descendants.len() == before {
                        break;
                    }
                }
                descendants.contains(&query)
            })
        })
        .collect()
}

fn transition_matrix(routes: bool) {
    let seed = b"exact-unit-projection";
    let roots: Vec<_> = (0..2)
        .map(|id| {
            if routes {
                OwnerKey::Target(TargetId::migrate(seed, id))
            } else {
                OwnerKey::Declaration(DeclarationId::migrate(seed, id))
            }
        })
        .collect();
    let children: Vec<_> = (0..3)
        .map(|id| {
            if routes {
                OwnerKey::HttpRoute(HttpRouteId::migrate(seed, id))
            } else {
                OwnerKey::Port(PortId::migrate(seed, id))
            }
        })
        .collect();
    let nested: Vec<_> = (0..3)
        .map(|id| OwnerKey::Expression(ExpressionId::migrate(seed, id)))
        .collect();
    let role = if routes {
        OwnershipRole::TargetHttpRoute
    } else {
        OwnershipRole::DeclarationPort
    };
    let snapshot =
        |mut code: usize| {
            let mut map = Parents::new();
            for (index, child) in children.iter().enumerate() {
                let selected = code % 3;
                code /= 3;
                if selected != 0 {
                    map.insert(*child, edge(roots[selected - 1], role));
                    if !routes {
                        map.insert(nested[index], edge(*child, OwnershipRole::ExpressionRoot(
                        crate::platform::witness::ExpressionRootRole::PortImplementation)));
                    }
                }
            }
            map
        };
    let queries: Vec<_> = roots
        .iter()
        .chain(&children)
        .chain(&nested)
        .copied()
        .collect();
    let mut witness =
        rebuild_full_witness(&crate::platform::kernel::tests::witness_snapshot()).unwrap();
    let mut compared = 0;
    for before in 0..27 {
        witness.entries.ownership = snapshot(before);
        for after in 0..27 {
            let after = snapshot(after);
            let derived = delta(&witness.entries.ownership, &after);
            let mut ownership = CandidateOwnership::new(&derived, &witness);
            for query in &queries {
                let actual =
                    owning_units(*query, &mut ownership, &mut ImpactWork::default(), 64).unwrap();
                assert_eq!(
                    actual,
                    oracle(*query, &roots, &[&witness.entries.ownership, &after])
                );
                assert!(
                    actual.iter().all(|owner| matches!(
                        owner,
                        OwnerKey::Declaration(_) | OwnerKey::Target(_)
                    ))
                );
                compared += 1;
            }
        }
    }
    assert_eq!(compared, 5_832);
}

#[test]
fn port_add_delete_move_and_nested_child_projection_match_forward_closure() {
    transition_matrix(false);
}

#[test]
fn route_add_delete_move_projection_matches_forward_closure() {
    transition_matrix(true);
}

#[path = "impact_unit_boundary_tests.rs"]
mod boundaries;
