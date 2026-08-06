use std::collections::BTreeMap;

use crate::verify::*;
use crate::{Block, Function, IrError, Program, SsaType, ValueId};

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
    _state: &mut OwnershipState,
    _last_use: &BTreeMap<ValueId, usize>,
    _position: usize,
) {
    // Affine owners leave verifier state only through executable ownership events.
}

pub(crate) fn verify_frame_affine_available(
    program: &Program,
    function: &Function,
    frame: Option<&crate::FrameState>,
    state: &OwnershipState,
    types: &[SsaType],
    nonowned_affine: &std::collections::HashSet<ValueId>,
) -> crate::Result<()> {
    let Some(frame) = frame else {
        return Ok(());
    };
    verify_terminator_affine_available(
        program,
        state,
        frame_values(frame),
        types,
        nonowned_affine,
    )?;
    for local in &frame.locals {
        if !is_owned_value(program, value_type(types, local.value)?) {
            continue;
        }
        let Some(place) = function
            .places
            .iter()
            .find(|place| place.binding == local.binding)
        else {
            if matches!(value_type(types, local.value)?, SsaType::Resource(_)) {
                continue;
            }
            return Err(IrError::new("SSA frame Owned local has no exact PlaceId"));
        };
        if place.drop_glue.is_some() && state.owners.get(&place.id) != Some(&local.value) {
            return fail("SSA frame Owned local does not match its current place owner");
        }
    }
    Ok(())
}

pub(crate) fn verify_terminator_affine_available(
    program: &Program,
    state: &OwnershipState,
    values: impl IntoIterator<Item = ValueId>,
    types: &[SsaType],
    nonowned_affine: &std::collections::HashSet<ValueId>,
) -> crate::Result<()> {
    for value in values {
        if is_affine(program, value_type(types, value)?)
            && !nonowned_affine.contains(&value)
            && !state.affine.contains_key(&value)
        {
            return fail(format!(
                "SSA metadata or terminator reuses unavailable affine value {} of type {:?}",
                value.raw(),
                value_type(types, value)?,
            ));
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
