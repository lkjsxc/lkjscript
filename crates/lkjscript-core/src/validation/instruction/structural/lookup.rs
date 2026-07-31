fn representation_operand<'a>(
    chunk: &'a Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<&'a crate::StructuralRepresentationMetadata> {
    let index = instruction_operand(proto, instruction)?;
    chunk.structural_representations.get(index).ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "structural representation metadata is missing",
        )
    })
}

fn destination_operand<'a>(
    chunk: &'a Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<&'a crate::StructuralDestinationMetadata> {
    let index = instruction_operand(proto, instruction)?;
    chunk.structural_destinations.get(index).ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "structural destination metadata is missing",
        )
    })
}

fn place_and_slot(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<(usize, usize)> {
    let packed = instruction_operand(proto, instruction)?;
    Ok((packed >> u8::BITS, packed & usize::from(u8::MAX)))
}

fn local_owner(
    state: &State,
    slot: usize,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<(StructuralRepresentationId, u32, Option<crate::VariantId>)> {
    match state.locals.get(slot).copied().flatten() {
        Some(Kind::StructuralOwner {
            representation,
            owner,
            active_variant,
        }) => Ok((representation, owner, active_variant)),
        _ => fail(proto, instruction, "structural local is not an owner"),
    }
}

fn require_place_owner(
    state: &State,
    place: usize,
    owner: u32,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    if state.unique_places.get(place).is_some_and(|item| {
        matches!(item, UniquePlaceState::Active { owner: Some(current), .. } if *current == owner)
    }) {
        Ok(())
    } else {
        fail(proto, instruction, "structural operation names a stale place owner")
    }
}
