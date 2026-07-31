fn view_representation_for_type(
    chunk: &Chunk,
    type_id: crate::StructuralTypeId,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<StructuralRepresentationId> {
    representation_for_type(
        chunk,
        type_id,
        StructuralValueCategory::View,
        proto,
        instruction,
        "structural field view representation is missing",
    )
}

fn owner_representation_for_type(
    chunk: &Chunk,
    type_id: crate::StructuralTypeId,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<StructuralRepresentationId> {
    representation_for_type(
        chunk,
        type_id,
        StructuralValueCategory::Owner,
        proto,
        instruction,
        "structural field owner representation is missing",
    )
}

fn representation_for_type(
    chunk: &Chunk,
    type_id: crate::StructuralTypeId,
    category: StructuralValueCategory,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    message: &str,
) -> Result<StructuralRepresentationId> {
    chunk
        .structural_representations
        .iter()
        .find(|item| item.type_id == type_id && item.category == category)
        .map(|item| item.id)
        .ok_or_else(|| instruction_error(proto, instruction.op(), instruction.offset(), message))
}
