fn place_init(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let (place, slot) = place_and_slot(proto, instruction)?;
    let (_, owner, _) = local_owner(state, slot, proto, instruction)?;
    if state.unique_places.iter().enumerate().any(|(index, item)| {
        index != place
            && matches!(item, UniquePlaceState::Active { owner: Some(value), .. } if *value == owner)
    }) {
        return fail(
            proto,
            instruction,
            "structural owner is already bound to another place",
        );
    }
    let target = state.unique_places.get_mut(place).ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "structural place index is out of range",
        )
    })?;
    match *target {
        UniquePlaceState::Inactive
        | UniquePlaceState::Active {
            owner: None,
            transferred: None,
        } => {
            *target = UniquePlaceState::Active {
                owner: Some(owner),
                transferred: None,
            };
        }
        UniquePlaceState::Active { .. } => {
            return fail(
                proto,
                instruction,
                "structural place is already initialized",
            )
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
    let (representation, owner, active_variant) = local_owner(state, slot, proto, instruction)?;
    require_place_owner(state, place, owner, proto, instruction)?;
    reject_live_view(state, owner, proto, instruction)?;
    state.locals[slot] = None;
    state.unique_places[place] = UniquePlaceState::Active {
        owner: None,
        transferred: Some(owner),
    };
    state.stack.push(Kind::StructuralOwner {
        representation,
        owner,
        active_variant,
    });
    Ok(())
}

fn drop_owner(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let (place, slot) = place_and_slot(proto, instruction)?;
    let (_, owner, _) = local_owner(state, slot, proto, instruction)?;
    let exact = state.unique_places.get(place).is_some_and(|item| {
        matches!(
            item,
            UniquePlaceState::Active { owner: Some(value), .. }
                | UniquePlaceState::Active { transferred: Some(value), .. }
                if *value == owner
        )
    });
    if !exact {
        return fail(
            proto,
            instruction,
            "structural Drop does not name the current or transferred place owner",
        );
    }
    reject_live_view(state, owner, proto, instruction)?;
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
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "structural place index is out of range",
        )
    })?;
    match *target {
        UniquePlaceState::Active { owner: None, .. } => *target = UniquePlaceState::Inactive,
        UniquePlaceState::Active { owner: Some(_), .. } => {
            return fail(proto, instruction, "structural PlaceEnd is missing Drop")
        }
        UniquePlaceState::Inactive => {
            return fail(proto, instruction, "structural place is already ended")
        }
    }
    state.stack.push(Kind::Unit);
    Ok(())
}
