fn aggregate_field_borrow(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let (reference, owner) = aggregate_field_input(chunk, proto, instruction, state, "borrow")?;
    let result = field_result_kind(chunk, reference.result, owner, false, proto, instruction)?;
    state.stack.push(result);
    Ok(())
}

fn aggregate_field_copy(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let (reference, owner) = aggregate_field_input(chunk, proto, instruction, state, "copy")?;
    let result = match reference.result.route {
        StructuralFieldRoute::Copy => {
            field_result_kind(chunk, reference.result, owner, false, proto, instruction)?
        }
        StructuralFieldRoute::Structural(type_id) => {
            let ty = chunk.structural_types.get(type_id.index()).ok_or_else(|| {
                instruction_error(
                    proto,
                    instruction.op(),
                    instruction.offset(),
                    "structural copy field type metadata is missing",
                )
            })?;
            if ty.mode != crate::StructuralTypeMode::Copy {
                return fail(
                    proto,
                    instruction,
                    "structural copy field target is not copy-mode",
                );
            }
            Kind::StructuralOwner {
                representation: owner_representation_for_type(
                    chunk,
                    type_id,
                    proto,
                    instruction,
                )?,
                owner: fresh_identity(proto, instruction, 2)?,
                active_variant: None,
            }
        }
        StructuralFieldRoute::Unique
        | StructuralFieldRoute::Resource
        | StructuralFieldRoute::LegacyHeap => {
            return fail(
                proto,
                instruction,
                "structural field copy crosses an unsupported ownership route",
            )
        }
    };
    state.stack.push(result);
    Ok(())
}

fn aggregate_field_input(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
    operation: &str,
) -> Result<(crate::StructuralAggregateFieldRef, u32)> {
    let index = instruction_operand(proto, instruction)?;
    let reference = chunk
        .structural_aggregate_fields
        .get(index)
        .copied()
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "structural aggregate-field metadata is missing",
            )
        })?;
    let input = pop(state, proto, instruction)?;
    let Kind::StructuralOwnerRef {
        representation,
        owner,
        active_variant,
    } = input
    else {
        return fail(
            proto,
            instruction,
            &format!("aggregate field {operation} expects an owner reference"),
        );
    };
    require_same_type(
        chunk,
        representation,
        reference.representation,
        proto,
        instruction,
    )?;
    if active_variant != reference.active_variant {
        return fail(
            proto,
            instruction,
            &format!("aggregate field {operation} references an inactive payload"),
        );
    }
    Ok((reference, owner))
}
