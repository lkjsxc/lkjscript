//! Exact bounded forward and reverse relation reads over one derived delta.

use super::{DerivedDelta, WitnessBaseRead, WitnessReadWork};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{ExactOwnerKey, OwnerKey, PackageId, RelationEdge, RelationEndpoint};
use crate::platform::witness::MAXIMUM_RELATION_PREFIX_ITEMS;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct CandidateRelationRead {
    pub edges: Vec<RelationEdge>,
    /// Base and delta edges inspected while producing `edges`, including nonmatching delta edges.
    pub edges_examined: u64,
}

pub(crate) struct CandidateRelations<'a, W: ?Sized> {
    package: PackageId,
    derived: &'a DerivedDelta,
    base: &'a W,
    outgoing: BTreeMap<OwnerKey, Vec<RelationEdge>>,
    incoming: BTreeMap<OwnerKey, Vec<RelationEdge>>,
    incoming_packages: BTreeMap<PackageId, Vec<RelationEdge>>,
    work: WitnessReadWork,
}

impl<'a, W: WitnessBaseRead + ?Sized> CandidateRelations<'a, W> {
    pub fn new(package: PackageId, derived: &'a DerivedDelta, base: &'a W) -> Self {
        Self {
            package,
            derived,
            base,
            outgoing: BTreeMap::new(),
            incoming: BTreeMap::new(),
            incoming_packages: BTreeMap::new(),
            work: WitnessReadWork::default(),
        }
    }

    pub const fn work(&self) -> WitnessReadWork {
        self.work
    }

    pub fn outgoing(
        &mut self,
        owner: OwnerKey,
        maximum_work: u64,
        maximum_fanout: u64,
    ) -> Result<CandidateRelationRead, Diagnostic> {
        require_relation_capacity(maximum_work, maximum_fanout)?;
        let source = RelationEndpoint::Owner(ExactOwnerKey {
            package: self.package,
            owner,
        });
        let base = if let Some(cached) = self.outgoing.get(&owner) {
            cached.clone()
        } else {
            let read = self
                .base
                .read_outgoing_relations(owner, bounded_maximum(maximum_work, maximum_fanout))?;
            self.work.add(read.work);
            if read.value.truncated {
                return Err(relation_error(
                    DiagnosticClass::Resource,
                    relation_prefix_code(maximum_work, maximum_fanout),
                    "outgoing relation prefix exceeds the remaining work or per-owner fanout budget",
                ));
            }
            self.outgoing.insert(owner, read.value.edges.clone());
            read.value.edges
        };
        self.apply_delta(
            base,
            |edge| edge.source == source,
            "change_relation_forward_remove",
            maximum_work,
            maximum_fanout,
        )
    }

    pub fn incoming(
        &mut self,
        owner: OwnerKey,
        maximum_work: u64,
        maximum_fanout: u64,
    ) -> Result<CandidateRelationRead, Diagnostic> {
        require_relation_capacity(maximum_work, maximum_fanout)?;
        let target = RelationEndpoint::Owner(ExactOwnerKey {
            package: self.package,
            owner,
        });
        let base = if let Some(cached) = self.incoming.get(&owner) {
            cached.clone()
        } else {
            let read = self
                .base
                .read_incoming_relations(owner, bounded_maximum(maximum_work, maximum_fanout))?;
            self.work.add(read.work);
            if read.value.truncated {
                return Err(relation_error(
                    DiagnosticClass::Resource,
                    relation_prefix_code(maximum_work, maximum_fanout),
                    "incoming relation prefix exceeds the remaining work or per-owner fanout budget",
                ));
            }
            self.incoming.insert(owner, read.value.edges.clone());
            read.value.edges
        };
        self.apply_delta(
            base,
            |edge| edge.target == target,
            "change_relation_reverse_remove",
            maximum_work,
            maximum_fanout,
        )
    }

    pub fn incoming_package(
        &mut self,
        package: PackageId,
        maximum_work: u64,
        maximum_fanout: u64,
    ) -> Result<CandidateRelationRead, Diagnostic> {
        require_relation_capacity(maximum_work, maximum_fanout)?;
        let base = if let Some(cached) = self.incoming_packages.get(&package) {
            cached.clone()
        } else {
            let read = self.base.read_incoming_package_relations(
                package,
                bounded_maximum(maximum_work, maximum_fanout),
            )?;
            self.work.add(read.work);
            if read.value.truncated {
                return Err(relation_error(
                    DiagnosticClass::Resource,
                    relation_prefix_code(maximum_work, maximum_fanout),
                    "foreign-package relation prefix exceeds the remaining work or per-package fanout budget",
                ));
            }
            self.incoming_packages
                .insert(package, read.value.edges.clone());
            read.value.edges
        };
        self.apply_delta(
            base,
            |edge| {
                matches!(
                    edge.target,
                    RelationEndpoint::Owner(ExactOwnerKey {
                        package: target_package,
                        ..
                    }) if target_package == package
                )
            },
            "change_relation_package_remove",
            maximum_work,
            maximum_fanout,
        )
    }

    fn apply_delta(
        &self,
        base: impl IntoIterator<Item = RelationEdge>,
        selected: impl Fn(&RelationEdge) -> bool,
        missing_code: &'static str,
        maximum_work: u64,
        maximum_fanout: u64,
    ) -> Result<CandidateRelationRead, Diagnostic> {
        let mut edges_examined = 0_u64;
        let mut relations = BTreeSet::new();
        for edge in base {
            charge_relation_edge(&mut edges_examined, maximum_work)?;
            insert_relation(&mut relations, edge, maximum_fanout)?;
        }
        for edge in &self.derived.relations.removed {
            charge_relation_edge(&mut edges_examined, maximum_work)?;
            if selected(edge) && !relations.remove(edge) {
                return Err(relation_error(
                    DiagnosticClass::Corrupt,
                    missing_code,
                    "candidate relation delta removes an absent base edge",
                ));
            }
        }
        for edge in &self.derived.relations.added {
            charge_relation_edge(&mut edges_examined, maximum_work)?;
            if selected(edge) {
                insert_relation(&mut relations, *edge, maximum_fanout)?;
            }
        }
        Ok(CandidateRelationRead {
            edges: relations.into_iter().collect(),
            edges_examined,
        })
    }
}

fn insert_relation(
    relations: &mut BTreeSet<RelationEdge>,
    edge: RelationEdge,
    maximum_fanout: u64,
) -> Result<(), Diagnostic> {
    if !relations.contains(&edge)
        && u64::try_from(relations.len()).unwrap_or(u64::MAX) >= maximum_fanout
    {
        return Err(relation_error(
            DiagnosticClass::Resource,
            "change_budget_relation_fanout",
            format!(
                "candidate relation endpoint exceeds the declared {maximum_fanout}-edge fanout budget"
            ),
        ));
    }
    relations.insert(edge);
    Ok(())
}

fn charge_relation_edge(observed: &mut u64, maximum_items: u64) -> Result<(), Diagnostic> {
    if *observed >= maximum_items {
        return Err(relation_error(
            DiagnosticClass::Resource,
            "change_budget_relation_edges",
            format!(
                "candidate relation traversal exceeds the remaining {maximum_items}-edge budget"
            ),
        ));
    }
    *observed += 1;
    Ok(())
}

fn bounded_maximum(maximum_work: u64, maximum_fanout: u64) -> usize {
    usize::try_from(maximum_work.min(maximum_fanout))
        .unwrap_or(usize::MAX)
        .min(MAXIMUM_RELATION_PREFIX_ITEMS)
}

fn require_relation_capacity(maximum_work: u64, maximum_fanout: u64) -> Result<(), Diagnostic> {
    if maximum_work == 0 {
        return Err(relation_error(
            DiagnosticClass::Resource,
            "change_budget_relation_edges",
            "relation traversal has no remaining edge budget",
        ));
    }
    if maximum_fanout == 0 {
        return Err(relation_error(
            DiagnosticClass::Resource,
            "change_budget_relation_fanout",
            "relation traversal has no remaining endpoint-fanout budget",
        ));
    }
    Ok(())
}

fn relation_prefix_code(maximum_work: u64, maximum_fanout: u64) -> &'static str {
    if maximum_work < maximum_fanout {
        "change_budget_relation_edges"
    } else {
        "change_budget_relation_fanout"
    }
}

fn relation_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::semantic_id::DeclarationId;
    use crate::platform::witness::rebuild_full_witness;
    use std::cell::Cell;

    #[test]
    fn nonmatching_delta_scan_stops_before_exceeding_relation_budget() {
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let witness = rebuild_full_witness(&snapshot).expect("base witness");
        let package = snapshot.root.package_id;
        let selected_owner =
            OwnerKey::Declaration(DeclarationId::migrate(b"candidate-relation-selected", 0));
        let selected_source = RelationEndpoint::Owner(ExactOwnerKey {
            package,
            owner: selected_owner,
        });
        let mut derived = DerivedDelta::default();
        for ordinal in 0..32 {
            derived.relations.added.insert(RelationEdge {
                source: RelationEndpoint::Owner(ExactOwnerKey {
                    package,
                    owner: OwnerKey::Declaration(DeclarationId::migrate(
                        b"candidate-relation-nonmatching",
                        ordinal,
                    )),
                }),
                kind: crate::platform::kernel::RelationKind::FunctionCall,
                target: selected_source,
            });
        }
        let view = CandidateRelations::new(package, &derived, &witness);
        let examined = Cell::new(0_u64);
        let error = view
            .apply_delta(
                Vec::new(),
                |edge| {
                    examined.set(examined.get().saturating_add(1));
                    edge.source == selected_source
                },
                "change_relation_test_remove",
                2,
                MAXIMUM_RELATION_PREFIX_ITEMS as u64,
            )
            .expect_err("the third nonmatching delta edge must exceed the budget");
        assert_eq!(error.class, DiagnosticClass::Resource);
        assert_eq!(error.code, "change_budget_relation_edges");
        assert_eq!(examined.get(), 2);

        let first = RelationEdge {
            source: selected_source,
            kind: crate::platform::kernel::RelationKind::FunctionCall,
            target: RelationEndpoint::Owner(ExactOwnerKey {
                package,
                owner: OwnerKey::Declaration(DeclarationId::migrate(
                    b"candidate-relation-fanout",
                    1,
                )),
            }),
        };
        let second = RelationEdge {
            target: RelationEndpoint::Owner(ExactOwnerKey {
                package,
                owner: OwnerKey::Declaration(DeclarationId::migrate(
                    b"candidate-relation-fanout",
                    2,
                )),
            }),
            ..first
        };
        assert_eq!(
            view.apply_delta(
                [first, second],
                |_| false,
                "change_relation_test_remove",
                2,
                1,
            )
            .expect_err("the second endpoint edge must exceed one-edge fanout")
            .code,
            "change_budget_relation_fanout"
        );

        let mut production_view = CandidateRelations::new(package, &derived, &witness);
        assert_eq!(
            production_view
                .outgoing(selected_owner, 2, MAXIMUM_RELATION_PREFIX_ITEMS as u64)
                .expect_err("the bounded outgoing read must reject the same delta scan")
                .code,
            "change_budget_relation_edges"
        );
    }
}
