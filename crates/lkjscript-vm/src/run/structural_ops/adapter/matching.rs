fn declared_type(value: &HostValue) -> HostValueType {
    match value {
        HostValue::Option { element, .. } => HostValueType::Option(Box::new(element.clone())),
        HostValue::Result { ok, error, .. } => {
            HostValueType::Result(Box::new(ok.clone()), Box::new(error.clone()))
        }
        _ => value.value_type(),
    }
}

fn exact_structural_type(
    chunk: &ValidatedChunk,
    declared: &HostValueType,
) -> Result<StructuralTypeId> {
    let mut matches = chunk
        .structural_types()
        .iter()
        .filter(|ty| type_matches(chunk, ty.id, declared))
        .map(|ty| ty.id);
    let selected = matches.next().ok_or_else(|| {
        Error::msg(format!(
            "host aggregate lacks exact structural type metadata: {declared:?}"
        ))
    })?;
    if matches.next().is_some() {
        return Err(Error::msg(
            "host aggregate structural type metadata is ambiguous",
        ));
    }
    Ok(selected)
}

fn type_matches(
    chunk: &ValidatedChunk,
    type_id: StructuralTypeId,
    declared: &HostValueType,
) -> bool {
    let Some(ty) = chunk
        .structural_types()
        .get(type_id.index())
        .filter(|ty| ty.id == type_id)
    else {
        return false;
    };
    match declared {
        HostValueType::String => ty.runtime_type.kind == StructuralKind::String,
        HostValueType::Path => ty.runtime_type.kind == StructuralKind::Path,
        HostValueType::Option(element) => option_layout_matches(chunk, ty.layout, element),
        HostValueType::Result(ok, error) => result_layout_matches(chunk, ty.layout, ok, error),
        HostValueType::SystemError => system_error_layout_matches(chunk, ty.layout),
        HostValueType::Utf8Error => utf8_error_layout_matches(chunk, ty.layout),
        HostValueType::NumericError => numeric_error_layout_matches(chunk, ty.layout),
        HostValueType::Unit
        | HostValueType::Bool
        | HostValueType::I64
        | HostValueType::F64
        | HostValueType::Bytes
        | HostValueType::Resource(_) => false,
    }
}

fn option_layout_matches(
    chunk: &ValidatedChunk,
    layout_id: lkjscript_core::StructuralLayoutId,
    element: &HostValueType,
) -> bool {
    let Some(StructuralLayoutKind::Enum {
        enum_id,
        runtime_layout,
        variants,
    }) = chunk
        .structural_layouts()
        .get(layout_id.index())
        .filter(|layout| layout.id == layout_id)
        .map(|layout| &layout.kind)
    else {
        return false;
    };
    if enum_id.bytes() != lkjscript_core::OPTION_ID
        || runtime_layout.bytes() != lkjscript_core::OPTION_LAYOUT
        || variants.len() != 2
    {
        return false;
    }
    let none = variants
        .iter()
        .find(|variant| variant.variant.bytes() == lkjscript_core::OPTION_NONE_ID);
    let some = variants
        .iter()
        .find(|variant| variant.variant.bytes() == lkjscript_core::OPTION_SOME_ID);
    matches!(none, Some(variant) if variant.physical_tag == 1 && variant.fields.is_empty())
        && matches!(some, Some(variant) if variant.physical_tag == 0
            && variant.fields.len() == 1
            && field_matches(chunk, &variant.fields[0], element))
}

fn result_layout_matches(
    chunk: &ValidatedChunk,
    layout_id: lkjscript_core::StructuralLayoutId,
    ok: &HostValueType,
    error: &HostValueType,
) -> bool {
    let Some(StructuralLayoutKind::Enum {
        enum_id,
        runtime_layout,
        variants,
    }) = chunk
        .structural_layouts()
        .get(layout_id.index())
        .filter(|layout| layout.id == layout_id)
        .map(|layout| &layout.kind)
    else {
        return false;
    };
    let success = variants
        .iter()
        .find(|variant| variant.variant.bytes() == lkjscript_core::RESULT_OK_ID);
    let failure = variants
        .iter()
        .find(|variant| variant.variant.bytes() == lkjscript_core::RESULT_ERR_ID);
    enum_id.bytes() == lkjscript_core::RESULT_ID
        && runtime_layout.bytes() == lkjscript_core::RESULT_LAYOUT
        && variants.len() == 2
        && matches!(success, Some(variant) if variant.physical_tag == 0
            && variant.fields.len() == 1
            && field_matches(chunk, &variant.fields[0], ok))
        && matches!(failure, Some(variant) if variant.physical_tag == 1
            && variant.fields.len() == 1
            && field_matches(chunk, &variant.fields[0], error))
}
