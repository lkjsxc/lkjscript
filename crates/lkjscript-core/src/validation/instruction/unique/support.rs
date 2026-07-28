use super::super::instruction_error;
use super::{Kind, State};
use crate::validation::UniquePlaceState;
use crate::{DecodedInstruction, FunctionProto, Result};

pub(in crate::validation::instruction) fn place_and_slot(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<(usize, usize)> {
    let packed = instruction.operand().ok_or_else(|| {
        error(
            proto,
            instruction,
            "byte-vector place/local operand is missing",
        )
    })?;
    Ok((usize::from(packed >> 8), usize::from(packed as u8)))
}

pub(super) fn local_owner(
    state: &State,
    slot: usize,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<u32> {
    match state.locals.get(slot).copied().flatten() {
        Some(Kind::ByteVector(owner)) => Ok(owner),
        _ => Err(error(
            proto,
            instruction,
            "byte-vector local is moved, stale, or has the wrong type",
        )),
    }
}

pub(in crate::validation::instruction) fn expect_place_owner(
    state: &State,
    place: usize,
    owner: u32,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    if state.unique_places.get(place)
        == Some(&UniquePlaceState::Active {
            owner: Some(owner),
            transferred: None,
        })
    {
        Ok(())
    } else {
        Err(error(
            proto,
            instruction,
            "byte-vector operation does not name the current place owner",
        ))
    }
}

pub(super) fn store_empty_local(
    state: &mut State,
    slot: usize,
    value: Kind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let target = state
        .locals
        .get_mut(slot)
        .ok_or_else(|| error(proto, instruction, "unique local index is out of range"))?;
    if target.is_some() {
        return Err(error(
            proto,
            instruction,
            &format!(
                "unique local {slot} overwrite of {target:?} with {value:?} would forge or leak an owner/view"
            ),
        ));
    }
    *target = Some(value);
    Ok(())
}

pub(super) fn live_views(state: &State, owner: u32) -> impl Iterator<Item = Kind> + '_ {
    state
        .locals
        .iter()
        .filter_map(|slot| *slot)
        .chain(state.stack.iter().copied())
        .filter(move |kind| matches!(kind, Kind::ByteSlice { owner: value, .. } if *value == owner))
}

pub(super) fn reject_live_loan(
    state: &State,
    owner: u32,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    if live_views(state, owner).next().is_some() {
        Err(error(
            proto,
            instruction,
            "byte-vector owner operation conflicts with a live loan",
        ))
    } else {
        Ok(())
    }
}

pub(in crate::validation::instruction) fn error(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    message: &str,
) -> crate::Error {
    instruction_error(proto, instruction.op(), instruction.offset(), message)
}

pub(super) use super::super::types::{expect_pop, instruction_operand, pop};
