fn semantic_for_field(
    chunk: &ValidatedChunk,
    field: &StructuralFieldMetadata,
    value: HostValue,
) -> Result<SemanticValue> {
    let expected = field
        .runtime_type
        .ok_or_else(|| Error::msg("host structural field lacks exact runtime type"))?;
    match (field.route, value) {
        (StructuralFieldRoute::Copy, HostValue::Unit) if expected.kind == StructuralKind::Unit => {
            Ok(SemanticValue::new(
                expected,
                SemanticPayload::Inline(InlineStructuralValue::Unit),
            ))
        }
        (StructuralFieldRoute::Copy, HostValue::Bool(value))
            if expected.kind == StructuralKind::Bool =>
        {
            Ok(SemanticValue::new(
                expected,
                SemanticPayload::Inline(InlineStructuralValue::Bool(value)),
            ))
        }
        (StructuralFieldRoute::Copy, HostValue::I64(value))
            if expected.kind == StructuralKind::I64 =>
        {
            Ok(SemanticValue::new(
                expected,
                SemanticPayload::Inline(InlineStructuralValue::I64(value)),
            ))
        }
        (StructuralFieldRoute::Copy, HostValue::F64Bits(value))
            if expected.kind == StructuralKind::F64 =>
        {
            Ok(SemanticValue::new(
                expected,
                SemanticPayload::Inline(InlineStructuralValue::F64Bits(value)),
            ))
        }
        (StructuralFieldRoute::Unique, HostValue::Bytes(value))
            if expected.kind == StructuralKind::Bytes =>
        {
            Ok(SemanticValue::new(expected, SemanticPayload::Bytes(value)))
        }
        (StructuralFieldRoute::Structural(type_id), value) => {
            let semantic = semantic_for_type(chunk, type_id, value)?;
            if semantic.value_type != expected {
                return Err(Error::msg("host structural field exact type mismatch"));
            }
            Ok(semantic)
        }
        (StructuralFieldRoute::Resource, _) => Err(Error::msg(
            "resource field cannot enter StructuralValueRuntime",
        )),
        _ => Err(Error::msg("host structural field payload shape mismatch")),
    }
}

fn exact_owner_representation(
    chunk: &ValidatedChunk,
    type_id: StructuralTypeId,
) -> Result<StructuralRepresentationId> {
    let mut matches = chunk
        .structural_representations()
        .iter()
        .filter(|item| item.type_id == type_id && item.category == StructuralValueCategory::Owner)
        .map(|item| item.id);
    let selected = matches
        .next()
        .ok_or_else(|| Error::msg("host aggregate lacks owner representation metadata"))?;
    if matches.next().is_some() {
        return Err(Error::msg(
            "host aggregate owner representation is ambiguous",
        ));
    }
    Ok(selected)
}
