use super::*;

pub(super) fn lower_constant(
    function: &Function,
    instruction: &Instruction,
    constant: &Constant,
    block: lkjscript_native::BlockId,
    value_types: &[ValueType],
    static_bytes: &HashMap<Vec<u8>, lkjscript_native::StaticBytesIdentity>,
    builder: &mut FunctionBuilder,
) -> Result<Result<lkjscript_native::ValueId, lkjscript_native::PlanError>, LoweringError> {
    Ok(match constant {
        Constant::Unit => builder.unit(block),
        Constant::Bool(value) => builder.bool_const(block, *value),
        Constant::I64(value) => builder.i64_const(block, *value),
        Constant::F64(value) => builder.f64_const_bits(block, value.to_bits()),
        Constant::Str(value) => lower_heap_constant(
            block,
            HeapOperation::ConstantStr(value.clone()),
            value_type(value_types, instruction.id)?,
            builder,
        ),
        Constant::EmptyList => lower_heap_constant(
            block,
            HeapOperation::EmptyList,
            value_type(value_types, instruction.id)?,
            builder,
        ),
        Constant::StaticBytes(bytes) => builder.static_bytes_const(
            block,
            *static_bytes.get(bytes).ok_or_else(|| {
                LoweringError::new(
                    LoweringFailureCode::InvalidFunction,
                    Some(function.id),
                    "native static bytes image identity is absent",
                )
            })?,
        ),
        Constant::Symbol(_) => return unsupported_operation(function.id, "Symbol constant"),
    })
}

pub(super) fn lower_heap_constant(
    block: lkjscript_native::BlockId,
    operation: HeapOperation,
    result_type: ValueType,
    builder: &mut FunctionBuilder,
) -> Result<lkjscript_native::ValueId, lkjscript_native::PlanError> {
    builder.heap_call(
        block,
        heap_descriptor(operation, Vec::new(), result_type)?,
        Vec::new(),
    )
}
