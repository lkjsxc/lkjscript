use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_enum_instruction(
    program: &lkjscript_ir::Program,
    function: &Function,
    instruction: &Instruction,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    layouts: &LayoutInterner,
    builder: &mut FunctionBuilder,
) -> Result<lkjscript_native::ValueId, lkjscript_native::PlanError> {
    match &instruction.kind {
        InstructionKind::EnumValue {
            enum_id,
            variant,
            layout,
            fields,
        } => {
            let definition = enum_definition(program, function.id, *enum_id)
                .map_err(|_| lkjscript_native::PlanError::InvalidHeapCall)?;
            let selected = enum_variant(definition, function.id, *variant)
                .map_err(|_| lkjscript_native::PlanError::InvalidHeapCall)?;
            let arguments = enum_type_arguments(&instruction.ty, *enum_id, function.id)
                .map_err(|_| lkjscript_native::PlanError::InvalidHeapCall)?;
            let substitutions = enum_substitution_layouts(function.id, arguments, layouts)
                .map_err(|_| lkjscript_native::PlanError::InvalidHeapCall)?;
            let native_arguments = read_values(builder, block, locals, fields, function.id)
                .map_err(|_| lkjscript_native::PlanError::UnknownValue)?;
            let input_types = fields
                .iter()
                .map(|value| value_type(value_types, *value))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| lkjscript_native::PlanError::UnknownValue)?;
            builder.heap_call(
                block,
                heap_descriptor(
                    HeapOperation::EnumValue {
                        enum_id: enum_id.bytes(),
                        variant: variant.bytes(),
                        layout: layout.bytes(),
                        physical_tag: selected.physical_tag,
                        substitutions,
                        fields: u8::try_from(fields.len())
                            .map_err(|_| lkjscript_native::PlanError::InvalidHeapCall)?,
                    },
                    input_types,
                    value_type(value_types, instruction.id)
                        .map_err(|_| lkjscript_native::PlanError::UnknownValue)?,
                )?,
                native_arguments,
            )
        }
        InstructionKind::EnumIsVariant {
            enum_id,
            variant,
            layout,
            value,
        } => {
            let definition = enum_definition(program, function.id, *enum_id)
                .map_err(|_| lkjscript_native::PlanError::InvalidHeapCall)?;
            let selected = enum_variant(definition, function.id, *variant)
                .map_err(|_| lkjscript_native::PlanError::InvalidHeapCall)?;
            let argument = read_value(builder, block, locals, *value, function.id)
                .map_err(|_| lkjscript_native::PlanError::UnknownValue)?;
            builder.heap_call(
                block,
                heap_descriptor(
                    HeapOperation::EnumIsVariant {
                        enum_id: enum_id.bytes(),
                        variant: variant.bytes(),
                        layout: layout.bytes(),
                        physical_tag: selected.physical_tag,
                    },
                    vec![value_type(value_types, *value)
                        .map_err(|_| lkjscript_native::PlanError::UnknownValue)?],
                    ValueType::Bool,
                )?,
                vec![argument],
            )
        }
        InstructionKind::EnumField {
            enum_id,
            variant,
            field,
            layout,
            value,
        } => {
            let definition = enum_definition(program, function.id, *enum_id)
                .map_err(|_| lkjscript_native::PlanError::InvalidHeapCall)?;
            let selected = enum_variant(definition, function.id, *variant)
                .map_err(|_| lkjscript_native::PlanError::InvalidHeapCall)?;
            let index = enum_field_index(selected, *field, function.id)
                .map_err(|_| lkjscript_native::PlanError::InvalidHeapCall)?;
            let field_type = value_type(value_types, instruction.id)
                .map_err(|_| lkjscript_native::PlanError::UnknownValue)?;
            let argument = read_value(builder, block, locals, *value, function.id)
                .map_err(|_| lkjscript_native::PlanError::UnknownValue)?;
            builder.heap_call(
                block,
                heap_descriptor(
                    HeapOperation::EnumField {
                        enum_id: enum_id.bytes(),
                        variant: variant.bytes(),
                        field: field.bytes(),
                        layout: layout.bytes(),
                        physical_tag: selected.physical_tag,
                        field_index: index,
                        field_type,
                    },
                    vec![value_type(value_types, *value)
                        .map_err(|_| lkjscript_native::PlanError::UnknownValue)?],
                    field_type,
                )?,
                vec![argument],
            )
        }
        _ => Err(lkjscript_native::PlanError::InvalidHeapCall),
    }
}
