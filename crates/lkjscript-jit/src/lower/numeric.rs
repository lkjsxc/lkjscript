use super::*;

pub(super) fn lower_numeric_conversion(
    function: &Function,
    instruction: &Instruction,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    builder: &mut FunctionBuilder,
) -> Result<lkjscript_native::ValueId, LoweringError> {
    let (value, operation) = match &instruction.kind {
        InstructionKind::F64FromI64Exact { value } => (*value, HeapOperation::F64FromI64Exact),
        InstructionKind::F64FromI64Rounded { value } => (*value, HeapOperation::F64FromI64Rounded),
        InstructionKind::I64FromF64Exact { value } => (*value, HeapOperation::I64FromF64Exact),
        InstructionKind::I64FromF64Trunc { value } => (*value, HeapOperation::I64FromF64Trunc),
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
