use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_product_instruction(
    function: &Function,
    instruction: &Instruction,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    layouts: &LayoutInterner,
    builder: &mut FunctionBuilder,
) -> Result<(), LoweringError> {
    let output = match &instruction.kind {
        InstructionKind::ProductValue { product, fields } => {
            let arguments = read_values(builder, block, locals, fields, function.id)?;
            let inputs = fields
                .iter()
                .map(|value| value_type(value_types, *value))
                .collect::<Result<Vec<_>, _>>()?;
            builder.heap_call(
                block,
                heap_descriptor(
                    HeapOperation::ProductValue {
                        product: product.raw(),
                        fields: u64::try_from(fields.len())
                            .map_err(|_| lkjscript_native::PlanError::InvalidHeapCall)?,
                    },
                    inputs,
                    value_type(value_types, instruction.id)?,
                )?,
                arguments,
            )
        }
        InstructionKind::ProductField {
            product,
            field,
            value,
        } => {
            let argument = read_value(builder, block, locals, *value, function.id)?;
            builder.heap_call(
                block,
                heap_descriptor(
                    HeapOperation::ProductField {
                        product: product.raw(),
                        field: *field,
                        field_type: value_type(value_types, instruction.id)?,
                    },
                    vec![value_type(value_types, *value)?],
                    value_type(value_types, instruction.id)?,
                )?,
                vec![argument],
            )
        }
        InstructionKind::WithProductField {
            product,
            field,
            value,
            replacement,
        } => {
            let arguments =
                read_values(builder, block, locals, &[*value, *replacement], function.id)?;
            builder.heap_call(
                block,
                heap_descriptor(
                    HeapOperation::WithProductField {
                        product: product.raw(),
                        field: *field,
                        field_type: value_type(value_types, *replacement)?,
                    },
                    vec![
                        value_type(value_types, *value)?,
                        value_type(value_types, *replacement)?,
                    ],
                    value_type(value_types, instruction.id)?,
                )?,
                arguments,
            )
        }
        _ => return unsupported_operation(function.id, "non-product instruction"),
    }
    .map_err(LoweringError::backend)?;
    write_instruction_output(
        function,
        instruction,
        block,
        locals,
        value_types,
        layouts,
        output,
        builder,
    )
}
