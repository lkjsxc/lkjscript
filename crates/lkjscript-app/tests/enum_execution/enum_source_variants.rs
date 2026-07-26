use super::source;

pub(super) fn nested_source() -> String {
    source()
        .replace(
            "->\nMaybe/\nI64\n/Maybe\n/sig",
            "->\nMaybe/\nMaybe/\nI64\n/Maybe\n/Maybe\n/sig",
        )
        .replacen(
            "variant-value/\ntype/\nMaybe/\nI64\n/Maybe\n/type",
            "variant-value/\ntype/\nMaybe/\nMaybe/\nI64\n/Maybe\n/Maybe\n/type",
            1,
        )
        .replace(
            "variant-field/\nname/\nvalue\n/name\n42\n/variant-field",
            "variant-field/\nname/\nvalue\n/name\nvariant-value/\ntype/\nMaybe/\nI64\n/Maybe\n/type\nvariant/\nSome\n/variant\nfields/\nvariant-field/\nname/\nvalue\n/name\n7\n/variant-field\n/fields\n/variant-value\n/variant-field",
        )
}

pub(super) fn nullary_source() -> String {
    source().replace(
        "variant/\nSome\n/variant\nfields/\nvariant-field/\nname/\nvalue\n/name\n42\n/variant-field\n/fields",
        "variant/\nNone\n/variant\nfields/\n/fields",
    )
}
