//! Bounded owner-summary rebuilding over one exact candidate overlay.

use super::relation_view::CandidateRelations;
use super::{DerivedDelta, KernelOverlay};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{OwnerKey, PackageId, RelationEdge};
use crate::platform::witness::{
    FullWitness, OwnerSummary, OwnerSummaryDigest, OwnershipEntry, SummaryRead,
    rebuild_selected_owner_summaries,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnerSummaryEdit {
    pub owner: OwnerKey,
    pub before_digest: Option<OwnerSummaryDigest>,
    pub after_digest: Option<OwnerSummaryDigest>,
    pub before: Option<OwnerSummary>,
    pub after: Option<OwnerSummary>,
}

#[derive(Clone, Debug, Default)]
pub struct SummaryDelta {
    pub selected: BTreeSet<OwnerKey>,
    pub edits: Vec<OwnerSummaryEdit>,
    pub new_objects: BTreeMap<OwnerSummaryDigest, Vec<u8>>,
}

pub fn derive_summary_delta(
    overlay: &KernelOverlay<'_>,
    derived: &DerivedDelta,
    base_witness: &FullWitness,
) -> Result<SummaryDelta, Diagnostic> {
    derive_summary_delta_for(
        overlay,
        derived,
        base_witness,
        derived.summary_candidates.clone(),
    )
}

pub fn derive_summary_delta_for(
    overlay: &KernelOverlay<'_>,
    derived: &DerivedDelta,
    base_witness: &FullWitness,
    selected: BTreeSet<OwnerKey>,
) -> Result<SummaryDelta, Diagnostic> {
    let view = CandidateSummaryView::new(overlay, derived, base_witness);
    let rebuilt = rebuild_selected_owner_summaries(&view, &selected)?;
    let mut edits = Vec::new();
    let mut new_objects = BTreeMap::new();
    for owner in &selected {
        let before = base_witness.summaries.get(owner).cloned();
        let before_digest = base_witness.entries.summaries.get(owner).copied();
        match (&before, before_digest) {
            (Some(summary), Some(binding)) => {
                let (actual, _) = crate::platform::witness::encode_owner_summary(summary)?;
                if actual != binding {
                    return Err(summary_error(
                        DiagnosticClass::Corrupt,
                        "change_summary_base_binding",
                        "base summary object disagrees with its witness binding",
                    ));
                }
            }
            (None, None) => {}
            _ => {
                return Err(summary_error(
                    DiagnosticClass::Corrupt,
                    "change_summary_base_missing",
                    "base summary object and witness binding have different domains",
                ));
            }
        }
        let after = rebuilt.get(owner).cloned();
        let (after_digest, after_bytes) = match &after {
            Some(summary) => {
                let (digest, bytes) = crate::platform::witness::encode_owner_summary(summary)?;
                (Some(digest), Some(bytes))
            }
            None => (None, None),
        };
        if before_digest == after_digest {
            if before != after {
                return Err(summary_error(
                    DiagnosticClass::Corrupt,
                    "change_summary_digest_collision",
                    "equal summary digests identify different summary values",
                ));
            }
            continue;
        }
        if let (Some(digest), Some(bytes)) = (after_digest, after_bytes)
            && let Some(previous) = new_objects.insert(digest, bytes.clone())
            && previous != bytes
        {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "change_summary_object_collision",
                "one summary digest is bound to different candidate bytes",
            ));
        }
        edits.push(OwnerSummaryEdit {
            owner: *owner,
            before_digest,
            after_digest,
            before,
            after,
        });
    }
    Ok(SummaryDelta {
        selected,
        edits,
        new_objects,
    })
}

struct CandidateSummaryView<'a> {
    overlay: &'a KernelOverlay<'a>,
    derived: &'a DerivedDelta,
    base_witness: &'a FullWitness,
    ownership: BTreeMap<OwnerKey, Option<OwnershipEntry>>,
}

impl<'a> CandidateSummaryView<'a> {
    fn new(
        overlay: &'a KernelOverlay<'a>,
        derived: &'a DerivedDelta,
        base_witness: &'a FullWitness,
    ) -> Self {
        Self {
            overlay,
            derived,
            base_witness,
            ownership: derived
                .ownership
                .iter()
                .map(|edit| (edit.key, edit.after))
                .collect(),
        }
    }
}

impl SummaryRead for CandidateSummaryView<'_> {
    fn package_id(&self) -> PackageId {
        self.overlay.base().root.package_id
    }

    fn owner(&self, owner: OwnerKey) -> Option<&crate::platform::kernel::OwnerRecord> {
        self.overlay.owner(owner)
    }

    fn dependency(&self, package: PackageId) -> Option<&crate::platform::kernel::DependencyRecord> {
        self.overlay.dependency(package)
    }

    fn ownership(&self, owner: OwnerKey) -> Option<OwnershipEntry> {
        match self.ownership.get(&owner) {
            Some(candidate) => *candidate,
            None => self.base_witness.entries.ownership.get(&owner).copied(),
        }
    }

    fn outgoing_relations(&self, owner: OwnerKey) -> Result<Vec<RelationEdge>, Diagnostic> {
        CandidateRelations::new(self.package_id(), self.derived, self.base_witness).outgoing(owner)
    }

    fn base_summary(&self, owner: OwnerKey) -> Option<&OwnerSummary> {
        self.base_witness.summaries.get(&owner)
    }
}

fn summary_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
