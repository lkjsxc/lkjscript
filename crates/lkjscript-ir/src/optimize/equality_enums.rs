use crate::InstructionKind;

pub(crate) fn exact_enum_instruction_kind_equal(
    left: &InstructionKind,
    right: &InstructionKind,
) -> bool {
    match (left, right) {
        (
            InstructionKind::EnumValue {
                enum_id: left_enum,
                variant: left_variant,
                layout: left_layout,
                fields: left_fields,
            },
            InstructionKind::EnumValue {
                enum_id: right_enum,
                variant: right_variant,
                layout: right_layout,
                fields: right_fields,
            },
        ) => {
            left_enum == right_enum
                && left_variant == right_variant
                && left_layout == right_layout
                && left_fields == right_fields
        }
        (
            InstructionKind::EnumIsVariant {
                enum_id: left_enum,
                variant: left_variant,
                layout: left_layout,
                value: left_value,
            },
            InstructionKind::EnumIsVariant {
                enum_id: right_enum,
                variant: right_variant,
                layout: right_layout,
                value: right_value,
            },
        ) => {
            left_enum == right_enum
                && left_variant == right_variant
                && left_layout == right_layout
                && left_value == right_value
        }
        (
            InstructionKind::EnumField {
                enum_id: left_enum,
                variant: left_variant,
                field: left_field,
                layout: left_layout,
                value: left_value,
            },
            InstructionKind::EnumField {
                enum_id: right_enum,
                variant: right_variant,
                field: right_field,
                layout: right_layout,
                value: right_value,
            },
        ) => {
            left_enum == right_enum
                && left_variant == right_variant
                && left_field == right_field
                && left_layout == right_layout
                && left_value == right_value
        }
        _ => false,
    }
}
