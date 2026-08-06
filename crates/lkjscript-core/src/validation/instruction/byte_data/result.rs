fn result_owner(
    chunk: &Chunk,
    success: crate::StructuralKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    if chunk.memory_plan.is_none() {
        return Ok(result_kind());
    }
    let type_id = chunk.structural_types.iter().find_map(|ty| {
        let crate::StructuralTypeKind::Enum(enum_id) = ty.kind else {
            return None;
        };
        if enum_id.bytes() != crate::RESULT_ID {
            return None;
        }
        let layout = chunk.structural_layouts.get_structural(ty.layout)?;
        let crate::StructuralLayoutKind::Enum { variants, .. } = &layout.kind else {
            return None;
        };
        variants
            .iter()
            .find(|variant| variant.physical_tag == 0)
            .and_then(|variant| variant.fields.first())
            .and_then(|field| field.runtime_type)
            .filter(|field| field.kind == success)
            .map(|_| ty.id)
    });
    let type_id = type_id.ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "operation result lacks exact structural metadata",
        )
    })?;
    let representation = chunk
        .structural_representations
        .iter()
        .find(|item| {
            item.type_id == type_id && item.category == crate::StructuralValueCategory::Owner
        })
        .map(|item| item.id)
        .ok_or_else(|| Error::msg("operation result lacks structural owner representation"))?;
    Ok(Kind::StructuralOwner {
        representation,
        owner: new_owner(instruction)?,
        active_variant: None,
    })
}
