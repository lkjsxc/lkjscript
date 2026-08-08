fn aggregate_tag(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let expected = representation_operand(chunk, proto, instruction)?;
    let input = pop(state, proto, instruction)?;
    let Kind::StructuralOwnerRef { representation, .. } = input else {
        return fail(
            proto,
            instruction,
            "aggregate tag expects an owner reference",
        );
    };
    require_same_type(chunk, representation, expected.id, proto, instruction)?;
    let layout = chunk
        .structural_layouts
        .get_structural(expected.layout)
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "structural layout is missing",
            )
        })?;
    if !matches!(layout.kind, crate::StructuralLayoutKind::Enum { .. }) {
        return fail(proto, instruction, "aggregate tag requires an enum layout");
    }
    state.stack.push(Kind::I64);
    Ok(())
}

fn aggregate_consume_payload(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let index = instruction_operand(proto, instruction)?;
    let reference = chunk
        .structural_payloads
        .get(index)
        .copied()
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "structural payload metadata is missing",
            )
        })?;
    let input = pop(state, proto, instruction)?;
    let Kind::StructuralOwner {
        representation,
        owner,
        active_variant,
    } = input
    else {
        return fail(proto, instruction, "payload consume expects an owner");
    };
    require_same_type(
        chunk,
        representation,
        reference.representation,
        proto,
        instruction,
    )?;
    if active_variant.is_some_and(|variant| variant != reference.variant) {
        return fail(
            proto,
            instruction,
            "payload consume references an inactive variant",
        );
    }
    reject_live_view(state, owner, proto, instruction)?;
    state.clear_unique_owner(owner);
    let result = match reference.result.route {
        StructuralFieldRoute::Copy => field_result_kind(
            chunk,
            reference.result,
            owner,
            false,
            proto,
            instruction,
        )?,
        StructuralFieldRoute::Structural(_) => Kind::StructuralOwner {
            representation: reference.result_representation.ok_or_else(|| {
                instruction_error(
                    proto,
                    instruction.op(),
                    instruction.offset(),
                    "structural payload result representation is missing",
                )
            })?,
            owner: fresh_identity(proto, instruction, 2)?,
            active_variant: None,
        },
        StructuralFieldRoute::Unique => unique_field_result(reference.result, proto, instruction)?,
        StructuralFieldRoute::Resource => {
            let kind = reference.result.resource.ok_or_else(|| {
                instruction_error(
                    proto,
                    instruction.op(),
                    instruction.offset(),
                    "resource payload lacks exact resource metadata",
                )
            })?;
            resource_kind(kind, proto, instruction)?
        }
        StructuralFieldRoute::LegacyHeap => {
            return fail(
                proto,
                instruction,
                "payload consume crosses an unsupported ownership route",
            )
        }
    };
    state.stack.push(result);
    Ok(())
}

include!("aggregate/string.rs");
