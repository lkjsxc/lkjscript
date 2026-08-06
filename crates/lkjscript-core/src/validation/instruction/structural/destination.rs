fn destination_create(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let destination = destination_operand(chunk, proto, instruction)?;
    let identity = fresh_identity(proto, instruction, 1)?;
    if state
        .structural_destinations
        .insert(
            identity,
            StructuralDestinationState {
                destination: destination.id,
                initialized: vec![false; destination.fields.len()],
            },
        )
        .is_some()
    {
        return fail(
            proto,
            instruction,
            "structural destination identity was reused",
        );
    }
    state.stack.push(Kind::StructuralDestination {
        destination: destination.id,
        identity,
    });
    Ok(())
}

fn destination_field_init(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let reference = instruction_operand(proto, instruction)?;
    let reference = chunk
        .structural_destination_fields
        .get(reference)
        .copied()
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "structural destination-field metadata is missing",
            )
        })?;
    let value = pop(state, proto, instruction)?;
    let destination_value = pop(state, proto, instruction)?;
    let Kind::StructuralDestination {
        destination,
        identity,
    } = destination_value
    else {
        return fail(
            proto,
            instruction,
            "destination field init expects a destination",
        );
    };
    if destination != reference.destination {
        return fail(
            proto,
            instruction,
            "destination field metadata does not match its value",
        );
    }
    let metadata = chunk
        .structural_destinations
        .get(destination.index())
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "structural destination metadata is missing",
            )
        })?;
    let expected = metadata
        .fields
        .get(usize::try_from(reference.field).map_err(|_| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "destination field exceeds host index width",
            )
        })?)
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "destination field is out of range",
            )
        })?;
    require_field_value(chunk, *expected, value, proto, instruction)?;
    let current = state
        .structural_destinations
        .get_mut(&identity)
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "destination is inactive",
            )
        })?;
    let initialized = current
        .initialized
        .get_mut(usize::try_from(reference.field).map_err(|_| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "destination field exceeds host index width",
            )
        })?)
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "destination field is out of range",
            )
        })?;
    if *initialized {
        return fail(proto, instruction, "destination field is initialized twice");
    }
    *initialized = true;
    state.stack.push(destination_value);
    Ok(())
}

fn destination_finish(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let expected = destination_operand(chunk, proto, instruction)?;
    let value = pop(state, proto, instruction)?;
    let Kind::StructuralDestination {
        destination,
        identity,
    } = value
    else {
        return fail(
            proto,
            instruction,
            "destination finish expects a destination",
        );
    };
    if destination != expected.id {
        return fail(proto, instruction, "destination finish metadata is stale");
    }
    let current = state
        .structural_destinations
        .remove(&identity)
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "destination is inactive",
            )
        })?;
    if current.initialized.iter().any(|initialized| !initialized) {
        return fail(proto, instruction, "destination finish is incomplete");
    }
    state.stack.push(Kind::StructuralOwner {
        representation: expected.owner_representation,
        owner: identity,
        active_variant: expected.active_variant,
    });
    Ok(())
}

fn destination_abort(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let expected = destination_operand(chunk, proto, instruction)?;
    let value = pop(state, proto, instruction)?;
    let Kind::StructuralDestination {
        destination,
        identity,
    } = value
    else {
        return fail(
            proto,
            instruction,
            "destination abort expects a destination",
        );
    };
    if destination != expected.id || state.structural_destinations.remove(&identity).is_none() {
        return fail(
            proto,
            instruction,
            "destination abort references an inactive destination",
        );
    }
    state.stack.push(Kind::Unit);
    Ok(())
}
