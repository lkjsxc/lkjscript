use std::collections::{BTreeMap, BTreeSet};

use crate::verify::*;
use crate::{Block, Function, IrError, SsaType, ValueId};

pub(crate) fn ownership_last_uses(block: &Block) -> BTreeMap<ValueId, usize> {
    let mut uses = BTreeMap::new();
    if let Some(frame) = &block.metadata.frame_state {
        for value in frame_values(frame) {
            uses.insert(value, block.instructions.len());
        }
    }
    for (index, instruction) in block.instructions.iter().enumerate() {
        for operand in instruction.kind.operands() {
            uses.insert(operand, index);
        }
        if let Some(frame) = &instruction.metadata.frame_state {
            for value in frame_values(frame) {
                uses.insert(value, index);
            }
        }
    }
    for operand in block.terminator.operands() {
        uses.insert(operand, block.instructions.len());
    }
    uses
}

pub(crate) fn expire_unplaced_affine(
    state: &mut OwnershipState,
    last_use: &BTreeMap<ValueId, usize>,
    position: usize,
) {
    let owners: BTreeSet<ValueId> = state.owners.values().copied().collect();
    state.affine.retain(|value, _| {
        owners.contains(value) || last_use.get(value).is_some_and(|last| *last >= position)
    });
}

pub(crate) fn expire_loans(
    live: &mut BTreeMap<crate::PlaceId, Vec<LiveLoan>>,
    last_use: &BTreeMap<ValueId, usize>,
    position: usize,
) {
    for loans in live.values_mut() {
        loans.retain(|loan| {
            last_use
                .get(&loan.value)
                .is_some_and(|last| *last >= position)
        });
    }
    live.retain(|_, loans| !loans.is_empty());
}

pub(crate) fn verify_frame_affine_available(
    function: &Function,
    frame: Option<&crate::FrameState>,
    state: &OwnershipState,
    types: &[SsaType],
) -> crate::Result<()> {
    let Some(frame) = frame else {
        return Ok(());
    };
    verify_terminator_affine_available(state, frame_values(frame), types)?;
    for local in &frame.locals {
        if !is_owned_value(value_type(types, local.value)?) {
            continue;
        }
        let place = function
            .places
            .iter()
            .find(|place| place.binding == local.binding)
            .ok_or_else(|| IrError::new("SSA frame Owned local has no exact PlaceId"))?;
        if state.owners.get(&place.id) != Some(&local.value) {
            return fail("SSA frame Owned local does not match its current place owner");
        }
    }
    Ok(())
}

pub(crate) fn verify_terminator_affine_available(
    state: &OwnershipState,
    values: impl IntoIterator<Item = ValueId>,
    types: &[SsaType],
) -> crate::Result<()> {
    for value in values {
        if is_affine(value_type(types, value)?) && !state.affine.contains_key(&value) {
            return fail("SSA metadata or terminator reuses an unavailable affine value");
        }
    }
    Ok(())
}

pub(crate) fn frame_values(frame: &crate::FrameState) -> impl Iterator<Item = ValueId> + '_ {
    frame
        .locals
        .iter()
        .map(|local| local.value)
        .chain(frame.operand_stack.iter().copied())
}

pub(crate) fn current_owner_place(
    state: &OwnershipState,
    value: ValueId,
) -> Option<crate::PlaceId> {
    let AffineProvenance::Place(place) = &state.affine.get(&value)?.provenance else {
        return None;
    };
    (state.owners.get(place) == Some(&value)).then_some(*place)
}

pub(crate) fn ownership_state_cells(state: &OwnershipState) -> crate::Result<usize> {
    state
        .active_places
        .len()
        .checked_add(state.owners.len())
        .and_then(|cells| cells.checked_add(state.affine.len()))
        .ok_or_else(|| IrError::new("SSA ownership state cell count overflow"))
}

pub(crate) fn charge_ownership_work(work: &mut usize, amount: usize) -> crate::Result<()> {
    *work = work
        .checked_add(amount)
        .ok_or_else(|| IrError::new("SSA ownership CFG verification work overflow"))?;
    if *work > OWNERSHIP_VERIFY_MAX_WORK {
        return fail(format!(
            "SSA ownership CFG verification work exceeded {OWNERSHIP_VERIFY_MAX_WORK}"
        ));
    }
    Ok(())
}
