fn aggregate_field_borrow(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let (reference, owner) = aggregate_field_input(chunk, proto, instruction, state, "borrow")?;
    let result = match reference.result.route {
        StructuralFieldRoute::Structural(_) => Kind::StructuralView {
            representation: result_representation(
                chunk,
                reference,
                StructuralValueCategory::View,
                proto,
                instruction,
            )?,
            owner,
            mutable: false,
            used: false,
        },
        _ => field_result_kind(chunk, reference.result, owner, false, proto, instruction)?,
    };
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
            let ty = chunk.structural_types.get_structural(type_id).ok_or_else(|| {
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
                representation: result_representation(
                    chunk,
                    reference,
                    StructuralValueCategory::Owner,
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
) -> Result<(crate::StructuralAggregateFieldRef, OwnerIdentity)> {
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

fn result_representation(
    chunk: &Chunk,
    reference: crate::StructuralAggregateFieldRef,
    category: StructuralValueCategory,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<StructuralRepresentationId> {
    let id = reference.result_representation.ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "structural field result representation is missing",
        )
    })?;
    let item = chunk
        .structural_representations
        .get_structural(id)
        .filter(|item| item.id == id)
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "structural field result representation is stale",
            )
        })?;
    if item.category != category {
        return fail(
            proto,
            instruction,
            "structural field result representation has the wrong category",
        );
    }
    Ok(id)
}
