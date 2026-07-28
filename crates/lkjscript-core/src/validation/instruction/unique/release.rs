use super::support::*;
use super::{Kind, State};
use crate::validation::UniquePlaceState;
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

pub(super) fn apply(
    _chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    match instruction.op() {
        Op::ByteVectorDropPlace => drop_owner(proto, instruction, state),
        Op::ByteVectorPlaceEnd => place_end(proto, instruction, state),
        _ => unreachable!("release opcode family checked"),
    }
}

fn drop_owner(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let (place, slot) = place_and_slot(proto, instruction)?;
    let owner = local_owner(state, slot, proto, instruction)?;
    let exact = state.unique_places.get(place).is_some_and(|item| {
        matches!(
            item,
            UniquePlaceState::Active { owner: Some(value), .. }
                | UniquePlaceState::Active { transferred: Some(value), .. }
                if *value == owner
        )
    });
    if !exact {
        return Err(error(
            proto,
            instruction,
            "byte-vector Drop does not name the current or transferred place owner",
        ));
    }
    reject_live_loan(state, owner, proto, instruction)?;
    state.locals[slot] = None;
    state.unique_places[place] = UniquePlaceState::Active {
        owner: None,
        transferred: None,
    };
    state.stack.push(Kind::Unit);
    Ok(())
}

fn place_end(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let place = instruction_operand(proto, instruction)?;
    let target = state.unique_places.get_mut(place).ok_or_else(|| {
        error(
            proto,
            instruction,
            "byte-vector place index is out of range",
        )
    })?;
    match *target {
        UniquePlaceState::Active { owner: None, .. } => *target = UniquePlaceState::Inactive,
        UniquePlaceState::Active { owner: Some(_), .. } => {
            return Err(error(
                proto,
                instruction,
                "byte-vector PlaceEnd is missing Drop",
            ));
        }
        UniquePlaceState::Inactive => {
            return Err(error(
                proto,
                instruction,
                "byte-vector place is already ended",
            ));
        }
    }
    state.stack.push(Kind::Unit);
    Ok(())
}
