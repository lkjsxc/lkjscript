use crate::{EnumMetadata, NumericError, SystemErrorKind, Utf8ErrorKind};

pub(super) fn valid(definition: &EnumMetadata) -> bool {
    let id = definition.id.bytes();
    if id == crate::OPTION_ID {
        return header(definition, "option", 1, 2, crate::OPTION_LAYOUT)
            && variant(definition, crate::OPTION_NONE_ID, "none", 1, &[])
            && variant(
                definition,
                crate::OPTION_SOME_ID,
                "some",
                0,
                &[(crate::OPTION_VALUE_ID, "value")],
            );
    }
    if id == crate::RESULT_ID {
        return header(definition, "result", 2, 2, crate::RESULT_LAYOUT)
            && variant(
                definition,
                crate::RESULT_OK_ID,
                "ok",
                0,
                &[(crate::RESULT_OK_VALUE_ID, "value")],
            )
            && variant(
                definition,
                crate::RESULT_ERR_ID,
                "err",
                1,
                &[(crate::RESULT_ERR_ERROR_ID, "error")],
            );
    }
    if id == crate::NUMERIC_ERROR_ID {
        let errors = [
            NumericError::NonFinite,
            NumericError::OutOfRange,
            NumericError::Fractional,
            NumericError::Inexact,
        ];
        return header(
            definition,
            "numeric-error",
            0,
            4,
            crate::NUMERIC_ERROR_LAYOUT,
        ) && errors.into_iter().all(|error| {
            variant(
                definition,
                error.variant_id(),
                error.name(),
                error.physical_tag(),
                &[],
            )
        });
    }
    if id == crate::UTF8_ERROR_ID {
        return header(definition, "utf8-error", 0, 6, crate::UTF8_ERROR_LAYOUT)
            && Utf8ErrorKind::ALL.into_iter().all(|error| {
                variant_names(
                    definition,
                    error.variant_id(),
                    error.name(),
                    error.physical_tag(),
                    &["offset"],
                )
            });
    }
    if id == crate::SYSTEM_ERROR_ID {
        return header(definition, "system-error", 0, 8, crate::SYSTEM_ERROR_LAYOUT)
            && SystemErrorKind::ALL.into_iter().all(|error| {
                let fields: &[&str] = if error == SystemErrorKind::Utf8 {
                    &["error"]
                } else {
                    &["code", "detail"]
                };
                variant_names(
                    definition,
                    error.variant_id(),
                    error.name(),
                    error.physical_tag(),
                    fields,
                )
            });
    }
    true
}

fn header(
    definition: &EnumMetadata,
    name: &str,
    arity: u8,
    variants: usize,
    layout: [u8; 32],
) -> bool {
    definition.name == name
        && definition.type_parameter_count == arity
        && definition.variants.len() == variants
        && definition.layout.bytes() == layout
}

fn variant(
    definition: &EnumMetadata,
    id: [u8; 32],
    name: &str,
    tag: u16,
    fields: &[([u8; 32], &str)],
) -> bool {
    definition.variants.iter().any(|item| {
        item.id.bytes() == id
            && item.name == name
            && item.physical_tag == tag
            && item.fields.len() == fields.len()
            && item.fields.iter().zip(fields).all(|(actual, expected)| {
                actual.id.bytes() == expected.0 && actual.name == expected.1
            })
    })
}

fn variant_names(
    definition: &EnumMetadata,
    id: [u8; 32],
    name: &str,
    tag: u16,
    fields: &[&str],
) -> bool {
    definition.variants.iter().any(|item| {
        item.id.bytes() == id
            && item.name == name
            && item.physical_tag == tag
            && item.fields.len() == fields.len()
            && item
                .fields
                .iter()
                .zip(fields)
                .all(|(actual, expected)| actual.name == *expected)
    })
}
