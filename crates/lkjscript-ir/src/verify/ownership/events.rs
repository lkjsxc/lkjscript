use std::collections::BTreeMap;

use crate::verify::*;
use crate::{DropEventKind, DropGlueIdentity, Function, IrError, LoanId, PlaceId, ValueId};

pub(crate) fn process_place_end(
    function: &Function,
    place: PlaceId,
    state: &mut OwnershipState,
    live_loans: &BTreeMap<PlaceId, Vec<LiveLoan>>,
) -> crate::Result<()> {
    let _declared = place_by_id(function, place)?;
    if !state.active_places.remove(&place) {
        return fail("SSA PlaceEnd references a place that is not active");
    }
    if live_loans
        .get(&place)
        .is_some_and(|loans| !loans.is_empty())
    {
        return fail("SSA ends an Owned place while it has a live loan");
    }
    if state.owners.contains_key(&place) {
        return fail("SSA PlaceEnd cannot erase an available affine owner");
    }
    if state.pending_drops.contains_key(&place) {
        return fail("SSA PlaceEnd precedes its required resource Drop event");
    }
    Ok(())
}

pub(crate) fn process_end_borrow(
    place: PlaceId,
    loan: LoanId,
    value: ValueId,
    state: &mut OwnershipState,
    live_loans: &mut BTreeMap<PlaceId, Vec<LiveLoan>>,
) -> crate::Result<()> {
    let loans = live_loans
        .get_mut(&place)
        .ok_or_else(|| IrError::new("SSA EndBorrow references no live loan"))?;
    let index = loans
        .iter()
        .position(|item| item.loan == loan && item.value == value)
        .ok_or_else(|| IrError::new("SSA EndBorrow has mismatched place, loan, or value"))?;
    let ended = loans.remove(index);
    if ended.kind == crate::BorrowKind::Mutable {
        state.affine.remove(&value);
    }
    if loans.is_empty() {
        live_loans.remove(&place);
    }
    Ok(())
}

pub(crate) fn process_drop(
    place: PlaceId,
    value: ValueId,
    glue: DropGlueIdentity,
    kind: DropEventKind,
    state: &mut OwnershipState,
    live_loans: &BTreeMap<PlaceId, Vec<LiveLoan>>,
) -> crate::Result<()> {
    if live_loans
        .get(&place)
        .is_some_and(|loans| !loans.is_empty())
    {
        return fail("SSA Drop precedes EndBorrow for its owner place");
    }
    match kind {
        DropEventKind::ImplicitCleanup => {
            if state.owners.get(&place) != Some(&value)
                || glue != DropGlueIdentity::LegacyTracedByteVector
            {
                return fail("SSA implicit Drop does not discharge its current byte owner");
            }
            state.owners.remove(&place);
            state.affine.remove(&value);
        }
        DropEventKind::ExplicitClose => {
            if state.pending_drops.remove(&place) != Some(value)
                || !matches!(glue, DropGlueIdentity::Resource(_))
            {
                return fail("SSA explicit Drop does not match one completed resource close");
            }
        }
    }
    Ok(())
}
