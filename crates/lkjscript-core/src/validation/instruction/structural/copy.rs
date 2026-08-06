fn structural_copy(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let representation = crate::StructuralRepresentationId::new(
        u64::try_from(instruction_operand(proto, instruction)?)
            .map_err(|_| crate::Error::msg("structural copy representation exceeds u64"))?,
    );
    let input = pop(state, proto, instruction)?;
    let actual = match input {
        Kind::StructuralOwnerRef { representation, .. }
        | Kind::StructuralOwner { representation, .. } => representation,
        _ => return fail(proto, instruction, "structural copy expects an owner reference"),
    };
    require_same_type(chunk, actual, representation, proto, instruction)?;
    let metadata = chunk
        .structural_representations
        .get_structural(representation)
        .filter(|item| {
            item.id == representation
                && item.category == crate::StructuralValueCategory::Owner
        })
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "structural copy lacks exact owner representation",
            )
        })?;
    let type_metadata = chunk
        .structural_types
        .get_structural(metadata.type_id)
        .ok_or_else(|| crate::Error::msg("structural copy owner type is missing"))?;
    if type_metadata.mode == crate::StructuralTypeMode::Affine {
        return fail(
            proto,
            instruction,
            "structural copy cannot duplicate an affine owner",
        );
    }
    state.stack.push(Kind::StructuralOwner {
        representation,
        owner: fresh_identity(proto, instruction, 2)?,
        active_variant: None,
    });
    Ok(())
}
