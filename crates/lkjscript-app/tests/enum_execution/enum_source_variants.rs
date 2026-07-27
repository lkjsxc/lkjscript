use super::source;

pub(super) fn nested_source() -> String {
    source()
        .replace(
            "output/\nmaybe/\ni64\n/maybe\n/output",
            "output/\nmaybe/\nmaybe/\ni64\n/maybe\n/maybe\n/output",
        )
        .replacen(
            "variant-value/\ntype/\nmaybe/\ni64\n/maybe\n/type",
            "variant-value/\ntype/\nmaybe/\nmaybe/\ni64\n/maybe\n/maybe\n/type",
            1,
        )
        .replace(
            "variant-field/\nname/\nvalue\n/name\n42\n/variant-field",
            "variant-field/\nname/\nvalue\n/name\nvariant-value/\ntype/\nmaybe/\ni64\n/maybe\n/type\nvariant/\nsome\n/variant\nfields/\nvariant-field/\nname/\nvalue\n/name\n7\n/variant-field\n/fields\n/variant-value\n/variant-field",
        )
}

pub(super) fn nullary_source() -> String {
    source().replace(
        "variant/\nsome\n/variant\nfields/\nvariant-field/\nname/\nvalue\n/name\n42\n/variant-field\n/fields",
        "variant/\nnone\n/variant\nfields/\n/fields",
    )
}
