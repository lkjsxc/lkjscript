use super::{error, DecodedInstruction, FunctionProto, Kind, OwnerIdentity, Result, State};
use crate::validation::instruction::types::instruction_operand;
use crate::validation::instruction::unique::support::{expect_place_owner, place_and_slot};
use crate::validation::UniquePlaceState;

fn local_bytes(
    state: &State,
    slot: usize,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<OwnerIdentity> {
    match state.locals.get(slot).copied().flatten() {
        Some(Kind::Bytes(owner)) => Ok(owner),
        _ => Err(error(
            proto,
            instruction,
            "dynamic bytes local is moved, stale, or wrong-layout",
        )),
    }
}

pub(super) fn place_init(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let (place, slot) = place_and_slot(proto, instruction)?;
    let owner = local_bytes(state, slot, proto, instruction)?;
    if state.unique_places.iter().any(|item| {
        matches!(
            item,
            UniquePlaceState::Active {
                owner: Some(value),
                ..
            } if *value == owner
        )
    }) {
        return Err(error(
            proto,
            instruction,
            "bytes owner is already bound to a place",
        ));
    }
    let target = state
        .unique_places
        .get(place)
        .copied()
        .ok_or_else(|| error(proto, instruction, "bytes place out of range"))?;
    if !matches!(
        target,
        UniquePlaceState::Inactive
            | UniquePlaceState::Active {
                owner: None,
                transferred: None
            }
    ) {
        return Err(error(
            proto,
            instruction,
            "bytes place is already initialized",
        ));
    }
    state.set_unique_place(
        place,
        UniquePlaceState::Active {
            owner: Some(owner),
            transferred: None,
        },
    );
    state.stack.push(Kind::Unit);
    Ok(())
}

pub(super) fn move_owner(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let (place, slot) = place_and_slot(proto, instruction)?;
    let owner = local_bytes(state, slot, proto, instruction)?;
    expect_place_owner(state, place, owner, proto, instruction)?;
    reject_owner_views(state, owner, proto, instruction)?;
    state.set_local(proto, slot, None);
    state.set_unique_place(
        place,
        UniquePlaceState::Active {
            owner: None,
            transferred: Some(owner),
        },
    );
    state.stack.push(Kind::Bytes(owner));
    Ok(())
}

pub(super) fn borrow(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let owner = local_bytes(state, slot, proto, instruction)?;
    if !state.unique_places.iter().any(|place| {
        matches!(
            place,
            UniquePlaceState::Active {
                owner: Some(value),
                ..
            } if *value == owner
        )
    }) {
        return Err(error(
            proto,
            instruction,
            "bytes borrow source is not a current owner",
        ));
    }
    state.stack.push(Kind::BytesBorrow { owner, used: false });
    Ok(())
}

pub(super) fn drop_owner(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let (place, slot) = place_and_slot(proto, instruction)?;
    let owner = local_bytes(state, slot, proto, instruction)?;
    let exact = state.unique_places.get(place).is_some_and(|item| {
        matches!(
            item,
            UniquePlaceState::Active {
                owner: Some(value),
                ..
            } | UniquePlaceState::Active {
                transferred: Some(value),
                ..
            } if *value == owner
        )
    });
    if !exact {
        return Err(error(
            proto,
            instruction,
            "bytes Drop does not name its exact owner",
        ));
    }
    reject_owner_views(state, owner, proto, instruction)?;
    state.set_local(proto, slot, None);
    state.set_unique_place(
        place,
        UniquePlaceState::Active {
            owner: None,
            transferred: None,
        },
    );
    state.stack.push(Kind::Unit);
    Ok(())
}

pub(super) fn place_end(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let place = instruction_operand(proto, instruction)?;
    let target = state
        .unique_places
        .get(place)
        .copied()
        .ok_or_else(|| error(proto, instruction, "bytes place out of range"))?;
    match target {
        UniquePlaceState::Active { owner: None, .. } => {
            state.set_unique_place(place, UniquePlaceState::Inactive);
        }
        UniquePlaceState::Active { owner: Some(_), .. } => {
            return Err(error(proto, instruction, "bytes PlaceEnd is missing Drop"))
        }
        UniquePlaceState::Inactive => {
            return Err(error(proto, instruction, "bytes place already ended"))
        }
    }
    state.stack.push(Kind::Unit);
    Ok(())
}

pub(super) fn reject_owner_views(
    state: &State,
    owner: OwnerIdentity,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let live = state
        .locals
        .iter()
        .filter_map(|slot| *slot)
        .chain(state.stack.iter().copied())
        .any(|kind| matches!(kind, Kind::BytesBorrow { owner: value, .. } if value == owner));
    if live {
        Err(error(proto, instruction, "bytes owner has a live borrow"))
    } else {
        Ok(())
    }
}
