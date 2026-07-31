fn numeric_result_owner(
    chunk: &Chunk,
    success: crate::StructuralKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    if chunk.memory_plan.is_none() {
        return Ok(Kind::Enum(crate::EnumId::new(crate::RESULT_ID), None));
    }
    let mut matches = chunk.structural_types.iter().filter(|ty| {
        matches!(ty.kind, crate::StructuralTypeKind::Enum(enum_id)
            if enum_id.bytes() == crate::RESULT_ID)
            && result_layout_matches(chunk, ty.layout, success)
    });
    let ty = matches.next().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "numeric result lacks exact structural metadata",
        )
    })?;
    if matches.next().is_some() {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "numeric result structural metadata is ambiguous",
        ));
    }
    let representation = chunk
        .structural_representations
        .iter()
        .find(|representation| {
            representation.type_id == ty.id
                && representation.category == crate::StructuralValueCategory::Owner
        })
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "numeric result lacks structural owner representation",
            )
        })?;
    Ok(Kind::StructuralOwner {
        representation: representation.id,
        owner: super::bytes::new_owner(instruction)?,
        active_variant: None,
    })
}

fn result_layout_matches(
    chunk: &Chunk,
    layout_id: crate::StructuralLayoutId,
    success: crate::StructuralKind,
) -> bool {
    let Some(crate::StructuralLayoutKind::Enum {
        enum_id,
        runtime_layout,
        variants,
    }) = chunk
        .structural_layouts
        .get(layout_id.index())
        .filter(|layout| layout.id == layout_id)
        .map(|layout| &layout.kind)
    else {
        return false;
    };
    if enum_id.bytes() != crate::RESULT_ID
        || runtime_layout.bytes() != crate::RESULT_LAYOUT
        || variants.len() != 2
    {
        return false;
    }
    let ok = variants
        .iter()
        .find(|variant| variant.variant.bytes() == crate::RESULT_OK_ID);
    let error = variants
        .iter()
        .find(|variant| variant.variant.bytes() == crate::RESULT_ERR_ID);
    matches!(ok, Some(variant) if variant.physical_tag == 0
        && variant.fields.len() == 1
        && variant.fields[0].runtime_type.is_some_and(|ty| ty.kind == success))
        && matches!(error, Some(variant) if variant.physical_tag == 1
            && variant.fields.len() == 1
            && matches!(variant.fields[0].route, crate::StructuralFieldRoute::Structural(type_id)
                if numeric_error_type_matches(chunk, type_id)))
}

fn numeric_error_type_matches(chunk: &Chunk, type_id: crate::StructuralTypeId) -> bool {
    let Some(ty) = chunk
        .structural_types
        .get(type_id.index())
        .filter(|ty| ty.id == type_id)
    else {
        return false;
    };
    let crate::StructuralTypeKind::Enum(enum_id) = ty.kind else {
        return false;
    };
    let Some(crate::StructuralLayoutKind::Enum {
        runtime_layout,
        variants,
        ..
    }) = chunk
        .structural_layouts
        .get(ty.layout.index())
        .filter(|layout| layout.id == ty.layout)
        .map(|layout| &layout.kind)
    else {
        return false;
    };
    enum_id.bytes() == crate::NUMERIC_ERROR_ID
        && runtime_layout.bytes() == crate::NUMERIC_ERROR_LAYOUT
        && variants.len() == 4
        && variants.iter().all(|variant| variant.fields.is_empty())
}
