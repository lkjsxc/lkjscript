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
    delta_index: Option<CandidateRelationDeltaIndex>,
    outgoing: BTreeMap<OwnerKey, Vec<RelationEdge>>,
    incoming: BTreeMap<OwnerKey, Vec<RelationEdge>>,
    incoming_packages: BTreeMap<PackageId, Vec<RelationEdge>>,
    work: WitnessReadWork,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct IndexedRelationDelta {
    removed: Vec<RelationEdge>,
    added: Vec<RelationEdge>,
}

#[derive(Debug, Default)]
struct CandidateRelationDeltaIndex {
    outgoing: BTreeMap<OwnerKey, IndexedRelationDelta>,
    incoming: BTreeMap<OwnerKey, IndexedRelationDelta>,
    incoming_packages: BTreeMap<PackageId, IndexedRelationDelta>,
}

impl<'a, W: WitnessBaseRead + ?Sized> CandidateRelations<'a, W> {
    pub fn new(package: PackageId, derived: &'a DerivedDelta, base: &'a W) -> Self {
        Self {
            package,
            derived,
            base,
            delta_index: None,
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
        let index_work = self.ensure_delta_index(remaining_after_base(maximum_work, &base)?)?;
        let delta = self
            .delta_index
            .as_ref()
            .and_then(|index| index.outgoing.get(&owner))
            .cloned()
            .unwrap_or_default();
        self.apply_delta(
            base,
            delta,
            "change_relation_forward_remove",
            maximum_work,
            maximum_fanout,
            index_work,
        )
    }

    pub fn incoming(
        &mut self,
        owner: OwnerKey,
        maximum_work: u64,
        maximum_fanout: u64,
    ) -> Result<CandidateRelationRead, Diagnostic> {
        require_relation_capacity(maximum_work, maximum_fanout)?;
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
        let index_work = self.ensure_delta_index(remaining_after_base(maximum_work, &base)?)?;
        let delta = self
            .delta_index
            .as_ref()
            .and_then(|index| index.incoming.get(&owner))
            .cloned()
            .unwrap_or_default();
        self.apply_delta(
            base,
            delta,
            "change_relation_reverse_remove",
            maximum_work,
            maximum_fanout,
            index_work,
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
        let index_work = self.ensure_delta_index(remaining_after_base(maximum_work, &base)?)?;
        let delta = self
            .delta_index
            .as_ref()
            .and_then(|index| index.incoming_packages.get(&package))
            .cloned()
            .unwrap_or_default();
        self.apply_delta(
            base,
            delta,
            "change_relation_package_remove",
            maximum_work,
            maximum_fanout,
            index_work,
        )
    }

    fn ensure_delta_index(&mut self, maximum_work: u64) -> Result<u64, Diagnostic> {
        if self.delta_index.is_some() {
            return Ok(0);
        }
        let removed = u64::try_from(self.derived.relations.removed.len()).unwrap_or(u64::MAX);
        let added = u64::try_from(self.derived.relations.added.len()).unwrap_or(u64::MAX);
        let relation_count = removed.saturating_add(added);
        if relation_count > maximum_work {
            return Err(relation_error(
                DiagnosticClass::Resource,
                "change_budget_relation_edges",
                format!(
                    "candidate relation delta indexing requires {relation_count} edge inspections with only {maximum_work} remaining"
                ),
            ));
        }
        let mut index = CandidateRelationDeltaIndex::default();
        for edge in &self.derived.relations.removed {
            index_relation_delta(&mut index, self.package, *edge, true);
        }
        for edge in &self.derived.relations.added {
            index_relation_delta(&mut index, self.package, *edge, false);
        }
        self.delta_index = Some(index);
        Ok(relation_count)
    }

    fn apply_delta(
        &self,
        base: impl IntoIterator<Item = RelationEdge>,
        delta: IndexedRelationDelta,
        missing_code: &'static str,
        maximum_work: u64,
        maximum_fanout: u64,
        initial_work: u64,
    ) -> Result<CandidateRelationRead, Diagnostic> {
        let mut edges_examined = initial_work;
        let mut relations = BTreeSet::new();
        for edge in base {
            charge_relation_edge(&mut edges_examined, maximum_work)?;
            insert_relation(&mut relations, edge, maximum_fanout)?;
        }
        for edge in delta.removed {
            charge_relation_edge(&mut edges_examined, maximum_work)?;
            if !relations.remove(&edge) {
                return Err(relation_error(
                    DiagnosticClass::Corrupt,
                    missing_code,
                    "candidate relation delta removes an absent base edge",
                ));
            }
        }
        for edge in delta.added {
            charge_relation_edge(&mut edges_examined, maximum_work)?;
            insert_relation(&mut relations, edge, maximum_fanout)?;
        }
        Ok(CandidateRelationRead {
            edges: relations.into_iter().collect(),
            edges_examined,
        })
    }
}

fn index_relation_delta(
    index: &mut CandidateRelationDeltaIndex,
    local_package: PackageId,
    edge: RelationEdge,
    removed: bool,
) {
    if let RelationEndpoint::Owner(ExactOwnerKey { package, owner }) = edge.source
        && package == local_package
    {
        insert_indexed_relation(index.outgoing.entry(owner).or_default(), edge, removed);
    }
    if let RelationEndpoint::Owner(ExactOwnerKey { package, owner }) = edge.target {
        if package == local_package {
            insert_indexed_relation(index.incoming.entry(owner).or_default(), edge, removed);
        }
        insert_indexed_relation(
            index.incoming_packages.entry(package).or_default(),
            edge,
            removed,
        );
    }
}

fn insert_indexed_relation(indexed: &mut IndexedRelationDelta, edge: RelationEdge, removed: bool) {
    if removed {
        indexed.removed.push(edge);
    } else {
        indexed.added.push(edge);
    }
}

fn remaining_after_base(maximum_work: u64, base: &[RelationEdge]) -> Result<u64, Diagnostic> {
    maximum_work
        .checked_sub(u64::try_from(base.len()).unwrap_or(u64::MAX))
        .ok_or_else(|| {
            relation_error(
                DiagnosticClass::Resource,
                "change_budget_relation_edges",
                "candidate base relation prefix exceeded the remaining edge budget",
            )
        })
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

    #[test]
    fn candidate_relation_delta_index_is_bounded_charged_once_and_preserves_fanout() {
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
        let mut bounded = CandidateRelations::new(package, &derived, &witness);
        let error = bounded
            .outgoing(selected_owner, 2, MAXIMUM_RELATION_PREFIX_ITEMS as u64)
            .expect_err("delta indexing must reject before exceeding the edge budget");
        assert_eq!(error.class, DiagnosticClass::Resource);
        assert_eq!(error.code, "change_budget_relation_edges");

        let mut indexed = CandidateRelations::new(package, &derived, &witness);
        let first_read = indexed
            .outgoing(selected_owner, 32, MAXIMUM_RELATION_PREFIX_ITEMS as u64)
            .expect("the complete delta index accepts an exact edge-budget fit");
        assert_eq!(first_read.edges_examined, 32);
        assert!(first_read.edges.is_empty());
        let indexed_owner =
            OwnerKey::Declaration(DeclarationId::migrate(b"candidate-relation-nonmatching", 0));
        let second_read = indexed
            .outgoing(indexed_owner, 2, MAXIMUM_RELATION_PREFIX_ITEMS as u64)
            .expect("a later endpoint reads only its indexed delta edge");
        assert_eq!(second_read.edges_examined, 1);
        assert_eq!(second_read.edges.len(), 1);

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
        let empty = DerivedDelta::default();
        let view = CandidateRelations::new(package, &empty, &witness);
        assert_eq!(
            view.apply_delta(
                [first, second],
                IndexedRelationDelta::default(),
                "change_relation_test_remove",
                2,
                1,
                0,
            )
            .expect_err("the second endpoint edge must exceed one-edge fanout")
            .code,
            "change_budget_relation_fanout"
        );
    }
}
