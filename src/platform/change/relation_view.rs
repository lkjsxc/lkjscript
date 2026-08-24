//! Exact bounded forward and reverse relation reads over one derived delta.

use super::{DerivedDelta, WitnessBaseRead, WitnessReadWork};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{ExactOwnerKey, OwnerKey, PackageId, RelationEdge, RelationEndpoint};
use crate::platform::witness::MAXIMUM_RELATION_PREFIX_ITEMS;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct CandidateRelations<'a, W: ?Sized> {
    package: PackageId,
    derived: &'a DerivedDelta,
    base: &'a W,
    outgoing: BTreeMap<OwnerKey, Vec<RelationEdge>>,
    incoming: BTreeMap<OwnerKey, Vec<RelationEdge>>,
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
            work: WitnessReadWork::default(),
        }
    }

    pub const fn work(&self) -> WitnessReadWork {
        self.work
    }

    pub fn outgoing(&mut self, owner: OwnerKey) -> Result<Vec<RelationEdge>, Diagnostic> {
        let source = RelationEndpoint::Owner(ExactOwnerKey {
            package: self.package,
            owner,
        });
        let base = if let Some(cached) = self.outgoing.get(&owner) {
            cached.clone()
        } else {
            let read = self
                .base
                .read_outgoing_relations(owner, MAXIMUM_RELATION_PREFIX_ITEMS)?;
            self.work.add(read.work);
            if read.value.truncated {
                return Err(relation_error(
                    DiagnosticClass::Resource,
                    "change_relation_forward_budget",
                    "outgoing relation prefix exceeds the current per-owner work budget",
                ));
            }
            self.outgoing.insert(owner, read.value.edges.clone());
            read.value.edges
        };
        self.apply_delta(
            base,
            |edge| edge.source == source,
            "change_relation_forward_remove",
        )
    }

    pub fn incoming(&mut self, owner: OwnerKey) -> Result<Vec<RelationEdge>, Diagnostic> {
        let target = RelationEndpoint::Owner(ExactOwnerKey {
            package: self.package,
            owner,
        });
        let base = if let Some(cached) = self.incoming.get(&owner) {
            cached.clone()
        } else {
            let read = self
                .base
                .read_incoming_relations(owner, MAXIMUM_RELATION_PREFIX_ITEMS)?;
            self.work.add(read.work);
            if read.value.truncated {
                return Err(relation_error(
                    DiagnosticClass::Resource,
                    "change_relation_reverse_budget",
                    "incoming relation prefix exceeds the current per-owner work budget",
                ));
            }
            self.incoming.insert(owner, read.value.edges.clone());
            read.value.edges
        };
        self.apply_delta(
            base,
            |edge| edge.target == target,
            "change_relation_reverse_remove",
        )
    }

    fn apply_delta(
        &self,
        base: impl IntoIterator<Item = RelationEdge>,
        selected: impl Fn(&RelationEdge) -> bool,
        missing_code: &'static str,
    ) -> Result<Vec<RelationEdge>, Diagnostic> {
        let mut relations = base.into_iter().collect::<BTreeSet<_>>();
        for edge in self
            .derived
            .relations
            .removed
            .iter()
            .filter(|edge| selected(edge))
        {
            if !relations.remove(edge) {
                return Err(relation_error(
                    DiagnosticClass::Corrupt,
                    missing_code,
                    "candidate relation delta removes an absent base edge",
                ));
            }
        }
        relations.extend(
            self.derived
                .relations
                .added
                .iter()
                .filter(|edge| selected(edge))
                .copied(),
        );
        Ok(relations.into_iter().collect())
    }
}

fn relation_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
