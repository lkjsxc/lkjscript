fn store_local(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let value = pop(state, proto, instruction)?;
    if !matches!(
        value,
        Kind::StructuralOwner { .. }
            | Kind::StructuralOwnerRef { .. }
            | Kind::StructuralView { used: false, .. }
            | Kind::StructuralDestination { .. }
    ) {
        return fail(
            proto,
            instruction,
            &format!("structural store expects a fresh structural value, got {value}"),
        );
    }
    let target = state.locals.get_mut(slot).ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "structural local index is out of range",
        )
    })?;
    if target.is_some_and(|kind| !matches!(kind, Kind::StructuralOwnerRef { .. })) {
        return fail(
            proto,
            instruction,
            "structural store would overwrite a live affine value",
        );
    }
    *target = Some(value);
    Ok(())
}

fn take_local(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let value = state.locals.get(slot).copied().flatten().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "structural local is empty after move",
        )
    })?;
    match value {
        Kind::StructuralOwner { owner, .. } => reject_live_view(state, owner, proto, instruction)?,
        Kind::StructuralDestination { .. } | Kind::StructuralOwnerRef { .. } => {}
        Kind::StructuralView { .. } => {
            return fail(
                proto,
                instruction,
                "structural take expects an owner or destination",
            )
        }
        _ => {
            return fail(
                proto,
                instruction,
                "structural local has the wrong category",
            )
        }
    }
    state.locals[slot] = None;
    state.stack.push(value);
    Ok(())
}

fn load_view(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let value = state.locals.get(slot).copied().flatten().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "structural view local is empty",
        )
    })?;
    let Kind::StructuralView {
        representation,
        owner,
        mutable,
        used: false,
    } = value
    else {
        return fail(
            proto,
            instruction,
            "structural view is stale, already used, or has the wrong category",
        );
    };
    let observed = Kind::StructuralView {
        representation,
        owner,
        mutable,
        used: true,
    };
    if mutable {
        state.locals[slot] = Some(observed);
    }
    state.stack.push(observed);
    Ok(())
}

fn end_view(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let value = state.locals.get(slot).copied().flatten().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "structural EndBorrow local is empty",
        )
    })?;
    let Kind::StructuralView { owner, .. } = value else {
        return fail(
            proto,
            instruction,
            "structural EndBorrow expects one exact view",
        );
    };
    if state
        .stack
        .iter()
        .any(|kind| matches!(kind, Kind::StructuralView { owner: active, .. } if *active == owner))
    {
        return fail(
            proto,
            instruction,
            "structural EndBorrow has a live operand-stack view",
        );
    }
    state.locals[slot] = None;
    state.stack.push(Kind::Unit);
    Ok(())
}

fn load_owner_ref(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let value = state.locals.get(slot).copied().flatten().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "structural owner local is empty",
        )
    })?;
    let (representation, owner, active_variant) = match value {
        Kind::StructuralOwner {
            representation,
            owner,
            active_variant,
        }
        | Kind::StructuralOwnerRef {
            representation,
            owner,
            active_variant,
        } => (representation, owner, active_variant),
        _ => {
            return fail(
                proto,
                instruction,
                "structural owner reference expects an owner",
            )
        }
    };
    state.stack.push(Kind::StructuralOwnerRef {
        representation,
        owner,
        active_variant,
    });
    Ok(())
}
