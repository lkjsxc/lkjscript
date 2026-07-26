use crate::prelude_contract as contract;
use crate::{EnumMetadata, SsaType};

pub(super) fn valid(definition: &EnumMetadata) -> bool {
    let id = definition.id.bytes();
    if id == contract::OPTION_ID {
        return header(definition, "Option", &["T"], contract::OPTION_LAYOUT, 2)
            && variant(definition, contract::OPTION_NONE_ID, "None", 1, &[], &[])
            && variant(
                definition,
                contract::OPTION_SOME_ID,
                "Some",
                0,
                &[("value", SsaType::TypeParameter("T".into()))],
                &[contract::OPTION_VALUE_ID],
            );
    }
    if id == contract::RESULT_ID {
        return header(
            definition,
            "Result",
            &["T", "E"],
            contract::RESULT_LAYOUT,
            2,
        ) && variant(
            definition,
            contract::RESULT_OK_ID,
            "Ok",
            0,
            &[("value", SsaType::TypeParameter("T".into()))],
            &[contract::RESULT_OK_VALUE_ID],
        ) && variant(
            definition,
            contract::RESULT_ERR_ID,
            "Err",
            1,
            &[("error", SsaType::TypeParameter("E".into()))],
            &[contract::RESULT_ERR_ERROR_ID],
        );
    }
    if id == contract::NUMERIC_ERROR_ID {
        let names = ["NonFinite", "OutOfRange", "Fractional", "Inexact"];
        let tags = [0, 3, 1, 2];
        return header(
            definition,
            "NumericError",
            &[],
            contract::NUMERIC_ERROR_LAYOUT,
            4,
        ) && contract::NUMERIC_ERROR_VARIANTS
            .into_iter()
            .zip(names)
            .zip(tags)
            .all(|((id, name), tag)| variant(definition, id, name, tag, &[], &[]));
    }
    if id == contract::UTF8_ERROR_ID {
        let names = [
            "UnexpectedContinuation",
            "InvalidLeadingByte",
            "MissingContinuation",
            "OverlongEncoding",
            "Surrogate",
            "OutOfRange",
        ];
        let tags = [1, 0, 4, 5, 2, 3];
        return header(definition, "Utf8Error", &[], contract::UTF8_ERROR_LAYOUT, 6)
            && contract::UTF8_ERROR_VARIANTS
                .into_iter()
                .zip(names)
                .zip(tags)
                .all(|((id, name), tag)| {
                    variant(definition, id, name, tag, &[("offset", SsaType::I64)], &[])
                });
    }
    if id == contract::SYSTEM_ERROR_ID {
        return valid_system_error(definition);
    }
    true
}

fn valid_system_error(definition: &EnumMetadata) -> bool {
    let names = [
        "Io",
        "Network",
        "Terminal",
        "Time",
        "Random",
        "Sqlite",
        "Utf8",
        "Unsupported",
    ];
    let tags = [0, 4, 1, 3, 7, 2, 5, 6];
    header(
        definition,
        "SystemError",
        &[],
        contract::SYSTEM_ERROR_LAYOUT,
        8,
    ) && contract::SYSTEM_ERROR_VARIANTS
        .into_iter()
        .zip(names)
        .zip(tags)
        .enumerate()
        .all(|(index, ((id, name), tag))| {
            let fields = if index == 6 {
                vec![(
                    "error",
                    SsaType::Enum {
                        id: crate::EnumId::new(contract::UTF8_ERROR_ID),
                        arguments: Vec::new(),
                    },
                )]
            } else {
                vec![
                    ("code", contract::option(SsaType::I64)),
                    ("detail", contract::option(SsaType::Str)),
                ]
            };
            variant(definition, id, name, tag, &fields, &[])
        })
}

fn header(
    definition: &EnumMetadata,
    name: &str,
    parameters: &[&str],
    layout: [u8; 32],
    variants: usize,
) -> bool {
    definition.name == name
        && definition
            .type_parameters
            .iter()
            .map(String::as_str)
            .eq(parameters.iter().copied())
        && definition.layout.identity.bytes() == layout
        && !definition.layout.recursive
        && definition.variants.len() == variants
}

fn variant(
    definition: &EnumMetadata,
    id: [u8; 32],
    name: &str,
    tag: u16,
    fields: &[(&str, SsaType)],
    field_ids: &[[u8; 32]],
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
                .enumerate()
                .all(|(index, (actual, expected))| {
                    actual.name == expected.0
                        && actual.ty == expected.1
                        && field_ids
                            .get(index)
                            .is_none_or(|id| actual.id.bytes() == *id)
                })
    })
}
