use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_instruction(
    program: &lkjscript_ir::Program,
    function: &Function,
    instruction: &Instruction,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    native_functions: &[(FunctionId, lkjscript_native::FunctionId)],
    layouts: &LayoutInterner,
    builder: &mut FunctionBuilder,
) -> Result<(), LoweringError> {
    let output = match &instruction.kind {
        InstructionKind::Constant(constant) => match constant {
            Constant::Unit => builder.unit(block),
            Constant::Bool(value) => builder.bool_const(block, *value),
            Constant::I64(value) => builder.i64_const(block, *value),
            Constant::F64(value) => builder.f64_const_bits(block, value.to_bits()),
            Constant::Str(value) => builder.heap_call(
                block,
                heap_descriptor(
                    HeapOperation::ConstantStr(value.clone()),
                    Vec::new(),
                    value_type(value_types, instruction.id)?,
                )?,
                Vec::new(),
            ),
            Constant::EmptyList => builder.heap_call(
                block,
                heap_descriptor(
                    HeapOperation::EmptyList,
                    Vec::new(),
                    value_type(value_types, instruction.id)?,
                )?,
                Vec::new(),
            ),
            Constant::Symbol(_) => return unsupported_operation(function.id, "Symbol constant"),
        },
        InstructionKind::Copy(value) => {
            let value = read_value(builder, block, locals, *value, function.id)?;
            Ok(value)
        }
        InstructionKind::Runtime {
            operation,
            arguments,
            ..
        } => lower_runtime(
            function,
            *operation,
            arguments,
            RuntimeLoweringContext {
                block,
                locals,
                value_types,
                result_type: value_type(value_types, instruction.id)?,
                result_ssa_type: &instruction.ty,
                layouts,
            },
            builder,
        ),
        InstructionKind::F64FromI64Exact { .. }
        | InstructionKind::F64FromI64Rounded { .. }
        | InstructionKind::I64FromF64Exact { .. }
        | InstructionKind::I64FromF64Trunc { .. } => Ok(lower_numeric_conversion(
            function,
            instruction,
            block,
            locals,
            value_types,
            layouts,
            builder,
        )?),
        InstructionKind::Call {
            target: CallTarget::Direct(callee),
            arguments,
            ..
        } => {
            builder
                .runtime_call(block, RuntimeCallSlot::PollV1, Vec::new())
                .map_err(LoweringError::backend)?;
            let arguments = read_values(builder, block, locals, arguments, function.id)?;
            let callee = native_function(native_functions, *callee)?;
            builder.call(block, callee, arguments)
        }
        InstructionKind::Call {
            target: CallTarget::Indirect(_),
            ..
        } => {
            return Err(LoweringError::new(
                LoweringFailureCode::IndirectCall,
                Some(function.id),
                "indirect native calls are unsupported",
            ));
        }
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
                        product: u32::from(product.raw()),
                        fields: u8::try_from(fields.len())
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
                        product: u32::from(product.raw()),
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
                        product: u32::from(product.raw()),
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
        InstructionKind::EnumValue { .. }
        | InstructionKind::EnumIsVariant { .. }
        | InstructionKind::EnumField { .. } => lower_enum_instruction(
            program,
            function,
            instruction,
            block,
            locals,
            value_types,
            layouts,
            builder,
        ),
        _ => return unsupported_operation(function.id, "ownership/reference operation"),
    }
    .map_err(LoweringError::backend)?;
    builder
        .set_instruction_source(output, SourceOrigin::new(instruction.metadata.origin.node))
        .map_err(LoweringError::backend)?;
    let local = value_local(locals, instruction.id, function.id)?;
    builder
        .write_local(block, local, output)
        .map_err(LoweringError::backend)?;
    Ok(())
}
