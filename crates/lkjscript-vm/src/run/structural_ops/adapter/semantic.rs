fn semantic_for_type(
    chunk: &ValidatedChunk,
    type_id: StructuralTypeId,
    value: HostValue,
) -> Result<SemanticValue> {
    let ty = chunk
        .structural_types()
        .get(type_id.index())
        .filter(|ty| ty.id == type_id)
        .ok_or_else(|| Error::msg("host aggregate structural type is stale"))?;
    match value {
        HostValue::String(value) if ty.runtime_type.kind == StructuralKind::String => Ok(
            SemanticValue::new(ty.runtime_type, SemanticPayload::String(value.into_bytes())),
        ),
        HostValue::Path(value) if ty.runtime_type.kind == StructuralKind::Path => Ok(
            SemanticValue::new(ty.runtime_type, SemanticPayload::Path(value)),
        ),
        HostValue::Option { element, value } => {
            if !option_layout_matches(chunk, ty.layout, &element) {
                return Err(Error::msg("host Option structural metadata mismatch"));
            }
            let (variant, fields) = match value {
                Some(value) => (lkjscript_core::OPTION_SOME_ID, vec![*value]),
                None => (lkjscript_core::OPTION_NONE_ID, Vec::new()),
            };
            enum_semantic(chunk, ty.runtime_type, ty.layout, variant, fields)
        }
        HostValue::Result { ok, error, value } => {
            if !result_layout_matches(chunk, ty.layout, &ok, &error) {
                return Err(Error::msg("host Result structural metadata mismatch"));
            }
            let (variant, field) = match value {
                Ok(value) => (lkjscript_core::RESULT_OK_ID, *value),
                Err(value) => (lkjscript_core::RESULT_ERR_ID, *value),
            };
            enum_semantic(chunk, ty.runtime_type, ty.layout, variant, vec![field])
        }
        HostValue::SystemError { kind, detail } => {
            if !system_error_layout_matches(chunk, ty.layout) || kind == SystemErrorKind::Utf8 {
                return Err(Error::msg("host SystemError structural metadata mismatch"));
            }
            enum_semantic(
                chunk,
                ty.runtime_type,
                ty.layout,
                kind.variant_id(),
                vec![
                    HostValue::option(HostValueType::I64, None),
                    HostValue::option(HostValueType::String, Some(HostValue::String(detail))),
                ],
            )
        }
        HostValue::SystemUtf8(error) => {
            if !system_error_layout_matches(chunk, ty.layout) {
                return Err(Error::msg(
                    "host UTF-8 SystemError structural metadata mismatch",
                ));
            }
            enum_semantic(
                chunk,
                ty.runtime_type,
                ty.layout,
                SystemErrorKind::Utf8.variant_id(),
                vec![HostValue::Utf8Error(error)],
            )
        }
        HostValue::NumericError(error) => {
            if !numeric_error_layout_matches(chunk, ty.layout) {
                return Err(Error::msg("host NumericError structural metadata mismatch"));
            }
            enum_semantic(
                chunk,
                ty.runtime_type,
                ty.layout,
                error.variant_id(),
                Vec::new(),
            )
        }
        HostValue::Utf8Error(error) => {
            if !utf8_error_layout_matches(chunk, ty.layout) {
                return Err(Error::msg("host Utf8Error structural metadata mismatch"));
            }
            let offset = i64::try_from(error.offset)
                .map_err(|_| Error::msg("UTF-8 error offset exceeds I64"))?;
            enum_semantic(
                chunk,
                ty.runtime_type,
                ty.layout,
                error.kind.variant_id(),
                vec![HostValue::I64(offset)],
            )
        }
        _ => Err(Error::msg(
            "host structural value does not match exact type metadata",
        )),
    }
}

fn enum_semantic(
    chunk: &ValidatedChunk,
    value_type: lkjscript_core::StructuralType,
    layout_id: lkjscript_core::StructuralLayoutId,
    variant_id: [u8; 32],
    fields: Vec<HostValue>,
) -> Result<SemanticValue> {
    let layout = chunk
        .structural_layouts()
        .get(layout_id.index())
        .filter(|layout| layout.id == layout_id)
        .ok_or_else(|| Error::msg("host enum structural layout is stale"))?;
    let StructuralLayoutKind::Enum { variants, .. } = &layout.kind else {
        return Err(Error::msg("host enum structural layout is not enum"));
    };
    let variant = variants
        .iter()
        .find(|variant| variant.variant.bytes() == variant_id)
        .ok_or_else(|| Error::msg("host enum structural variant is missing"))?;
    if variant.fields.len() != fields.len() {
        return Err(Error::msg(
            "host enum structural active payload is malformed",
        ));
    }
    let active_payload = variant
        .fields
        .iter()
        .zip(fields)
        .map(|(field, value)| semantic_for_field(chunk, field, value))
        .collect::<Result<Vec<_>>>()?;
    Ok(SemanticValue::new(
        value_type,
        SemanticPayload::Enum {
            tag: variant.physical_tag,
            active_payload: active_payload.into(),
        },
    ))
}
