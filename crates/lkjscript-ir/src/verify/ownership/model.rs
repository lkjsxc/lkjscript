use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::{BlockId, ValueId};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum AffineProvenance {
    Place(crate::PlaceId),
    Fresh(ValueId),
    Transferred(ValueId),
    External(ValueId),
    Loan(crate::LoanId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AffineFact {
    pub(crate) provenance: AffineProvenance,
    pub(crate) transferred: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OwnershipState {
    pub(crate) active_places: Arc<BTreeSet<crate::PlaceId>>,
    pub(crate) owners: Arc<BTreeMap<crate::PlaceId, ValueId>>,
    pub(crate) pending_drops: Arc<BTreeMap<crate::PlaceId, ValueId>>,
    pub(crate) affine: Arc<BTreeMap<ValueId, AffineFact>>,
}

impl OwnershipState {
    pub(crate) fn active_places_mut(&mut self) -> &mut BTreeSet<crate::PlaceId> {
        Arc::make_mut(&mut self.active_places)
    }

    pub(crate) fn owners_mut(&mut self) -> &mut BTreeMap<crate::PlaceId, ValueId> {
        Arc::make_mut(&mut self.owners)
    }

    pub(crate) fn pending_drops_mut(&mut self) -> &mut BTreeMap<crate::PlaceId, ValueId> {
        Arc::make_mut(&mut self.pending_drops)
    }

    pub(crate) fn affine_mut(&mut self) -> &mut BTreeMap<ValueId, AffineFact> {
        Arc::make_mut(&mut self.affine)
    }

    pub(crate) fn clear_affine(&mut self) {
        self.affine = Arc::default();
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BorrowDefinition {
    pub(crate) block: BlockId,
    pub(crate) place: crate::PlaceId,
    pub(crate) loan: crate::LoanId,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LiveLoan {
    pub(crate) loan: crate::LoanId,
    pub(crate) kind: crate::BorrowKind,
    pub(crate) value: ValueId,
}
