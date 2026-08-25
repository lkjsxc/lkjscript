//! Bounded owner-summary rebuilding over one exact candidate overlay.

use super::relation_view::CandidateRelations;
use super::{
    BoundOwnerSummary, CanonicalBaseRead, DerivedDelta, ImpactAdmission, KernelOverlay,
    WitnessBaseRead, WitnessReadWork,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{OwnerKey, PackageId, RelationEdge};
use crate::platform::witness::{
    OwnerSummary, OwnerSummaryDigest, OwnershipEntry, SummaryRead, rebuild_selected_owner_summaries,
};
use std::cell::RefCell;
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
    pub base_summaries_selected: u64,
    pub edits: Vec<OwnerSummaryEdit>,
    pub new_objects: BTreeMap<OwnerSummaryDigest, Vec<u8>>,
    pub read_work: WitnessReadWork,
    pub relation_edges_read: u64,
}

pub fn derive_summary_delta<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    overlay: &KernelOverlay<'_, B>,
    derived: &DerivedDelta,
    base_witness: &W,
) -> Result<SummaryDelta, Diagnostic> {
    derive_summary_delta_for_with_relation_limit(
        overlay,
        derived,
        base_witness,
        derived.summary_candidates.clone(),
        ImpactAdmission::default().maximum_relation_edges,
        ImpactAdmission::default().maximum_relation_fanout,
    )
}

pub fn derive_summary_delta_for<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    overlay: &KernelOverlay<'_, B>,
    derived: &DerivedDelta,
    base_witness: &W,
    selected: BTreeSet<OwnerKey>,
) -> Result<SummaryDelta, Diagnostic> {
    derive_summary_delta_for_with_relation_limit(
        overlay,
        derived,
        base_witness,
        selected,
        ImpactAdmission::default().maximum_relation_edges,
        ImpactAdmission::default().maximum_relation_fanout,
    )
}

pub(crate) fn derive_summary_delta_for_with_relation_limit<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    overlay: &KernelOverlay<'_, B>,
    derived: &DerivedDelta,
    base_witness: &W,
    selected: BTreeSet<OwnerKey>,
    maximum_relation_edges: u64,
    maximum_relation_fanout: u64,
) -> Result<SummaryDelta, Diagnostic> {
    let view = CandidateSummaryView::new(
        overlay,
        derived,
        base_witness,
        maximum_relation_edges,
        maximum_relation_fanout,
    );
    let rebuilt = rebuild_selected_owner_summaries(&view, &selected)?;
    let mut edits = Vec::new();
    let mut new_objects = BTreeMap::new();
    let mut base_summaries_selected = 0_u64;
    for owner in &selected {
        let before_bound = view.base_bound_summary(*owner)?;
        base_summaries_selected =
            base_summaries_selected.saturating_add(u64::from(before_bound.is_some()));
        let before = before_bound.as_ref().map(|bound| bound.summary.clone());
        let before_digest = before_bound.map(|bound| bound.digest);
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
        base_summaries_selected,
        edits,
        new_objects,
        read_work: view.work(),
        relation_edges_read: view.relation_edges_read(),
    })
}

struct CandidateSummaryView<'a, B: ?Sized, W: ?Sized> {
    overlay: &'a KernelOverlay<'a, B>,
    base_witness: &'a W,
    ownership_edits: BTreeMap<OwnerKey, Option<OwnershipEntry>>,
    ownership_cache: RefCell<BTreeMap<OwnerKey, Option<OwnershipEntry>>>,
    summary_cache: RefCell<BTreeMap<OwnerKey, Option<BoundOwnerSummary>>>,
    relations: RefCell<CandidateRelations<'a, W>>,
    read_work: RefCell<WitnessReadWork>,
    remaining_relation_edges: RefCell<u64>,
    maximum_relation_fanout: u64,
    relation_edges_read: RefCell<u64>,
}

impl<'a, B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>
    CandidateSummaryView<'a, B, W>
{
    fn new(
        overlay: &'a KernelOverlay<'a, B>,
        derived: &'a DerivedDelta,
        base_witness: &'a W,
        maximum_relation_edges: u64,
        maximum_relation_fanout: u64,
    ) -> Self {
        Self {
            overlay,
            base_witness,
            ownership_edits: derived
                .ownership
                .iter()
                .map(|edit| (edit.key, edit.after))
                .collect(),
            ownership_cache: RefCell::new(BTreeMap::new()),
            summary_cache: RefCell::new(BTreeMap::new()),
            relations: RefCell::new(CandidateRelations::new(
                overlay.package_id(),
                derived,
                base_witness,
            )),
            read_work: RefCell::new(WitnessReadWork::default()),
            remaining_relation_edges: RefCell::new(maximum_relation_edges),
            maximum_relation_fanout,
            relation_edges_read: RefCell::new(0),
        }
    }

    fn base_ownership(&self, owner: OwnerKey) -> Result<Option<OwnershipEntry>, Diagnostic> {
        if !self.ownership_cache.borrow().contains_key(&owner) {
            let read = self.base_witness.read_ownership(owner)?;
            self.read_work.borrow_mut().add(read.work);
            self.ownership_cache.borrow_mut().insert(owner, read.value);
        }
        Ok(self.ownership_cache.borrow().get(&owner).copied().flatten())
    }

    fn base_bound_summary(&self, owner: OwnerKey) -> Result<Option<BoundOwnerSummary>, Diagnostic> {
        if !self.summary_cache.borrow().contains_key(&owner) {
            let read = self.base_witness.read_owner_summary(owner)?;
            self.read_work.borrow_mut().add(read.work);
            self.summary_cache.borrow_mut().insert(owner, read.value);
        }
        Ok(self.summary_cache.borrow().get(&owner).cloned().flatten())
    }

    fn work(&self) -> WitnessReadWork {
        let mut work = *self.read_work.borrow();
        work.add(self.relations.borrow().work());
        work
    }

    fn relation_edges_read(&self) -> u64 {
        *self.relation_edges_read.borrow()
    }
}

impl<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized> SummaryRead
    for CandidateSummaryView<'_, B, W>
{
    fn package_id(&self) -> PackageId {
        self.overlay.package_id()
    }

    fn owner(
        &self,
        owner: OwnerKey,
    ) -> Result<Option<crate::platform::kernel::OwnerRecord>, Diagnostic> {
        self.overlay.owner(owner)
    }

    fn dependency(
        &self,
        package: PackageId,
    ) -> Result<Option<crate::platform::kernel::DependencyRecord>, Diagnostic> {
        self.overlay.dependency(package)
    }

    fn ownership(&self, owner: OwnerKey) -> Result<Option<OwnershipEntry>, Diagnostic> {
        match self.ownership_edits.get(&owner) {
            Some(candidate) => Ok(*candidate),
            None => self.base_ownership(owner),
        }
    }

    fn outgoing_relations(&self, owner: OwnerKey) -> Result<Vec<RelationEdge>, Diagnostic> {
        let remaining = *self.remaining_relation_edges.borrow();
        let read =
            self.relations
                .borrow_mut()
                .outgoing(owner, remaining, self.maximum_relation_fanout)?;
        let observed = read.edges_examined;
        *self.remaining_relation_edges.borrow_mut() =
            remaining.checked_sub(observed).ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "change_budget_relation_edges",
                    "summary relation traversal exceeded its declared edge budget",
                )
            })?;
        let total = self
            .relation_edges_read
            .borrow()
            .checked_add(observed)
            .unwrap_or(u64::MAX);
        *self.relation_edges_read.borrow_mut() = total;
        Ok(read.edges)
    }

    fn base_summary(&self, owner: OwnerKey) -> Result<Option<OwnerSummary>, Diagnostic> {
        Ok(self.base_bound_summary(owner)?.map(|bound| bound.summary))
    }
}

fn summary_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
