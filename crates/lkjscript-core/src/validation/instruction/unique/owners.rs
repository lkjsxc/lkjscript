use super::support::*;
use super::{Kind, OwnerIdentity, State};
use crate::validation::UniquePlaceState;
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

pub(super) fn apply(
    _chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    match instruction.op() {
        Op::ByteVectorNew => new_owner(proto, instruction, state)?,
        Op::ByteVectorPlaceInit => place_init(proto, instruction, state)?,
        Op::ByteVectorMove => move_owner(proto, instruction, state)?,
        Op::ByteVectorBorrow | Op::ByteVectorBorrowMut => {
            borrow(proto, instruction, state)?;
        }
        Op::StoreUniqueLocal => store_owner(proto, instruction, state)?,
        Op::TakeUniqueLocal => take_owner(proto, instruction, state)?,
        _ => unreachable!("owner opcode family checked"),
    }
    Ok(())
}

fn new_owner(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    expect_pop(state, Kind::I64, proto, instruction)?;
    let owner = OwnerIdentity::instruction(instruction.offset(), 1);
    state.stack.push(Kind::ByteVector(owner));
    Ok(())
}

fn place_init(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let (place, slot) = place_and_slot(proto, instruction)?;
    let owner = local_owner(state, slot, proto, instruction)?;
    if state.unique_places.iter().enumerate().any(|(index, item)| {
        index != place
            && matches!(item, UniquePlaceState::Active { owner: Some(value), .. } if *value == owner)
    }) {
        return Err(error(
            proto,
            instruction,
            "byte-vector owner is already bound to another place",
        ));
    }
    let target = state.unique_places.get(place).copied().ok_or_else(|| {
        error(
            proto,
            instruction,
            "byte-vector place index is out of range",
        )
    })?;
    match target {
        UniquePlaceState::Inactive
        | UniquePlaceState::Active {
            owner: None,
            transferred: None,
        } => state.set_unique_place(
            place,
            UniquePlaceState::Active {
                owner: Some(owner),
                transferred: None,
            },
        ),
        UniquePlaceState::Active { .. } => {
            return Err(error(
                proto,
                instruction,
                "byte-vector place is already initialized",
            ));
        }
    }
    state.stack.push(Kind::Unit);
    Ok(())
}

fn move_owner(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let (place, slot) = place_and_slot(proto, instruction)?;
    let owner = local_owner(state, slot, proto, instruction)?;
    expect_place_owner(state, place, owner, proto, instruction)?;
    reject_live_loan(state, owner, proto, instruction)?;
    state.set_local(proto, slot, None);
    state.set_unique_place(
        place,
        UniquePlaceState::Active {
            owner: None,
            transferred: Some(owner),
        },
    );
    state.stack.push(Kind::ByteVector(owner));
    Ok(())
}

fn borrow(proto: &FunctionProto, instruction: DecodedInstruction, state: &mut State) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let owner = local_owner(state, slot, proto, instruction)?;
    if !state.unique_places.iter().any(
        |place| matches!(place, UniquePlaceState::Active { owner: Some(value), .. } if *value == owner),
    ) {
        return Err(error(
            proto,
            instruction,
            "byte-vector borrow source is not a current whole-place owner",
        ));
    }
    let mutable = instruction.op() == Op::ByteVectorBorrowMut;
    if live_views(state, owner)
        .any(|kind| mutable || matches!(kind, Kind::ByteSlice { mutable: true, .. }))
    {
        return Err(error(
            proto,
            instruction,
            "byte-vector borrow conflicts with a live loan",
        ));
    }
    state.stack.push(Kind::ByteSlice {
        owner,
        mutable,
        used: false,
    });
    Ok(())
}

fn store_owner(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let value = pop(state, proto, instruction)?;
    if !matches!(value, Kind::Bytes(_) | Kind::ByteVector(_)) {
        return Err(error(
            proto,
            instruction,
            "StoreUniqueLocal expects an exact dynamic unique owner",
        ));
    }
    if state.locals.get(slot) == Some(&Some(Kind::StaticBytes)) {
        state.set_local(proto, slot, None);
    }
    store_empty_local(state, slot, value, proto, instruction)
}

fn take_owner(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let value = state
        .locals
        .get(slot)
        .copied()
        .flatten()
        .ok_or_else(|| error(proto, instruction, "unique local is empty"))?;
    let owner = match value {
        Kind::Bytes(owner) | Kind::ByteVector(owner) => owner,
        _ => return Err(error(proto, instruction, "unique local has wrong type")),
    };
    reject_live_loan(state, owner, proto, instruction)?;
    state.set_local(proto, slot, None);
    state.stack.push(value);
    Ok(())
}
