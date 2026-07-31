fn system_error_layout_matches(
    chunk: &ValidatedChunk,
    layout_id: lkjscript_core::StructuralLayoutId,
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
    enum_id.bytes() == lkjscript_core::SYSTEM_ERROR_ID
        && runtime_layout.bytes() == lkjscript_core::SYSTEM_ERROR_LAYOUT
        && variants.len() == SystemErrorKind::ALL.len()
        && SystemErrorKind::ALL.into_iter().all(|kind| {
            variants
                .iter()
                .find(|variant| variant.variant.bytes() == kind.variant_id())
                .is_some_and(|variant| {
                    variant.physical_tag == kind.physical_tag()
                        && if kind == SystemErrorKind::Utf8 {
                            variant.fields.len() == 1
                                && field_matches(
                                    chunk,
                                    &variant.fields[0],
                                    &HostValueType::Utf8Error,
                                )
                        } else {
                            variant.fields.len() == 2
                                && field_matches(
                                    chunk,
                                    &variant.fields[0],
                                    &HostValueType::Option(Box::new(HostValueType::I64)),
                                )
                                && field_matches(
                                    chunk,
                                    &variant.fields[1],
                                    &HostValueType::Option(Box::new(HostValueType::String)),
                                )
                        }
                })
        })
}

fn numeric_error_layout_matches(
    chunk: &ValidatedChunk,
    layout_id: lkjscript_core::StructuralLayoutId,
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
    let errors = [
        lkjscript_core::NumericError::NonFinite,
        lkjscript_core::NumericError::OutOfRange,
        lkjscript_core::NumericError::Fractional,
        lkjscript_core::NumericError::Inexact,
    ];
    enum_id.bytes() == lkjscript_core::NUMERIC_ERROR_ID
        && runtime_layout.bytes() == lkjscript_core::NUMERIC_ERROR_LAYOUT
        && variants.len() == errors.len()
        && errors.into_iter().all(|error| {
            variants
                .iter()
                .find(|variant| variant.variant.bytes() == error.variant_id())
                .is_some_and(|variant| {
                    variant.physical_tag == error.physical_tag() && variant.fields.is_empty()
                })
        })
}

fn utf8_error_layout_matches(
    chunk: &ValidatedChunk,
    layout_id: lkjscript_core::StructuralLayoutId,
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
    enum_id.bytes() == lkjscript_core::UTF8_ERROR_ID
        && runtime_layout.bytes() == lkjscript_core::UTF8_ERROR_LAYOUT
        && variants.len() == lkjscript_core::Utf8ErrorKind::ALL.len()
        && lkjscript_core::Utf8ErrorKind::ALL.into_iter().all(|kind| {
            variants
                .iter()
                .find(|variant| variant.variant.bytes() == kind.variant_id())
                .is_some_and(|variant| {
                    variant.physical_tag == kind.physical_tag()
                        && variant.fields.len() == 1
                        && field_matches(chunk, &variant.fields[0], &HostValueType::I64)
                })
        })
}

fn field_matches(
    chunk: &ValidatedChunk,
    field: &StructuralFieldMetadata,
    expected: &HostValueType,
) -> bool {
    match (field.route, expected) {
        (StructuralFieldRoute::Copy, HostValueType::Unit) => {
            runtime_kind(field, StructuralKind::Unit)
        }
        (StructuralFieldRoute::Copy, HostValueType::Bool) => {
            runtime_kind(field, StructuralKind::Bool)
        }
        (StructuralFieldRoute::Copy, HostValueType::I64) => {
            runtime_kind(field, StructuralKind::I64)
        }
        (StructuralFieldRoute::Copy, HostValueType::F64) => {
            runtime_kind(field, StructuralKind::F64)
        }
        (StructuralFieldRoute::Unique, HostValueType::Bytes) => {
            runtime_kind(field, StructuralKind::Bytes)
        }
        (StructuralFieldRoute::Structural(type_id), _) => type_matches(chunk, type_id, expected),
        (StructuralFieldRoute::Resource, HostValueType::Resource(_)) => true,
        _ => false,
    }
}

fn runtime_kind(field: &StructuralFieldMetadata, kind: StructuralKind) -> bool {
    field.runtime_type.is_some_and(|ty| ty.kind == kind)
}
