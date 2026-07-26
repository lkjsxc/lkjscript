use super::*;

pub(super) fn lower_numeric_conversion(
    function: &Function,
    instruction: &Instruction,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    layouts: &LayoutInterner,
    builder: &mut FunctionBuilder,
) -> Result<lkjscript_native::ValueId, LoweringError> {
    let error_type = match &instruction.ty {
        SsaType::Enum { id, arguments }
            if id.bytes() == lkjscript_ir::prelude_contract::RESULT_ID && arguments.len() == 2 =>
        {
            Some(lower_type(function.id, &arguments[1], layouts)?)
        }
        _ => None,
    };
    let (value, operation) = match (&instruction.kind, error_type) {
        (InstructionKind::F64FromI64Exact { value }, Some(error_type)) => {
            (*value, HeapOperation::F64FromI64Exact { error_type })
        }
        (InstructionKind::F64FromI64Rounded { value }, None) => {
            let argument = read_value(builder, block, locals, *value, function.id)?;
            return builder
                .i64_to_f64(block, argument)
                .map_err(LoweringError::backend);
        }
        (InstructionKind::I64FromF64Exact { value }, Some(error_type)) => {
            (*value, HeapOperation::I64FromF64Exact { error_type })
        }
        (InstructionKind::I64FromF64Trunc { value }, Some(error_type)) => {
            (*value, HeapOperation::I64FromF64Trunc { error_type })
        }
        _ => return unsupported_operation(function.id, "numeric conversion mismatch"),
    };
    let argument = read_value(builder, block, locals, value, function.id)?;
    builder
        .heap_call(
            block,
            heap_descriptor(
                operation,
                vec![value_type(value_types, value)?],
                value_type(value_types, instruction.id)?,
            )?,
            vec![argument],
        )
        .map_err(LoweringError::backend)
}
