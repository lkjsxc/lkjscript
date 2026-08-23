//! Exact bounded forward and reverse relation reads over one derived delta.

use super::DerivedDelta;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{ExactOwnerKey, OwnerKey, PackageId, RelationEdge, RelationEndpoint};
use crate::platform::witness::FullWitness;
use std::collections::BTreeSet;

pub(crate) struct CandidateRelations<'a> {
    package: PackageId,
    derived: &'a DerivedDelta,
    base: &'a FullWitness,
}

impl<'a> CandidateRelations<'a> {
    pub const fn new(package: PackageId, derived: &'a DerivedDelta, base: &'a FullWitness) -> Self {
        Self {
            package,
            derived,
            base,
        }
    }

    pub fn outgoing(&self, owner: OwnerKey) -> Result<Vec<RelationEdge>, Diagnostic> {
        let source = RelationEndpoint::Owner(ExactOwnerKey {
            package: self.package,
            owner,
        });
        let relations = &self.base.entries.relations;
        let start = relations.partition_point(|edge| edge.source < source);
        let end = relations.partition_point(|edge| edge.source <= source);
        self.apply_delta(
            relations[start..end].iter().copied(),
            |edge| edge.source == source,
            "change_relation_forward_remove",
        )
    }

    pub fn incoming(&self, owner: OwnerKey) -> Result<Vec<RelationEdge>, Diagnostic> {
        let target = RelationEndpoint::Owner(ExactOwnerKey {
            package: self.package,
            owner,
        });
        let relations = &self.base.entries.reverse_relations;
        let start = relations.partition_point(|edge| edge.target < target);
        let end = relations.partition_point(|edge| edge.target <= target);
        self.apply_delta(
            relations[start..end].iter().copied(),
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

fn relation_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}
