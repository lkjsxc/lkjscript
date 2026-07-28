use super::*;

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
