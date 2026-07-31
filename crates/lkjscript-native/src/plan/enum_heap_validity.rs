use super::*;

pub(super) fn enum_operation_types_are_valid(
    operation: &HeapOperation,
    inputs: &[ValueType],
    result: ValueType,
) -> bool {
    match operation {
        HeapOperation::EnumValue {
            enum_id,
            variant,
            layout,
            substitutions,
            fields,
            ..
        } => {
            stable(*enum_id)
                && stable(*variant)
                && stable(*layout)
                && substitutions.len() <= 16
                && usize::from(*fields) == inputs.len()
                && inputs
                    .iter()
                    .copied()
                    .all(super::heap_validity::is_legacy_heap_value)
                && matches!(result, ValueType::Reference(ReferenceType::Enum(_, _)))
        }
        HeapOperation::EnumIsVariant {
            enum_id,
            variant,
            layout,
            ..
        } => {
            stable(*enum_id)
                && stable(*variant)
                && stable(*layout)
                && matches!(inputs, [ValueType::Reference(ReferenceType::Enum(_, _))])
                && result == ValueType::Bool
        }
        HeapOperation::EnumField {
            enum_id,
            variant,
            field,
            layout,
            field_index,
            field_type,
            ..
        } => {
            stable(*enum_id)
                && stable(*variant)
                && stable(*field)
                && stable(*layout)
                && *field_index < 16
                && super::heap_validity::is_legacy_heap_value(*field_type)
                && matches!(inputs, [ValueType::Reference(ReferenceType::Enum(_, _))])
                && result == *field_type
        }
        _ => false,
    }
}

fn stable(identity: [u8; 32]) -> bool {
    identity != [0; 32]
}
