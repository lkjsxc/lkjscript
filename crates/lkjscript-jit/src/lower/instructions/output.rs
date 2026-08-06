use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn write_instruction_output(
    function: &Function,
    instruction: &Instruction,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    layouts: &LayoutInterner,
    output: lkjscript_native::ValueId,
    builder: &mut FunctionBuilder,
) -> Result<(), LoweringError> {
    let source = match instruction.metadata.origin {
        lkjscript_ir::Origin::Source { node, .. } => SourceOrigin::new(node),
        lkjscript_ir::Origin::Synthetic => SourceOrigin::synthetic(),
    };
    builder
        .set_instruction_source(output, source)
        .map_err(LoweringError::backend)?;
    builder
        .set_instruction_failure_cleanup(
            output,
            lower_failure_cleanup(function, instruction, locals, value_types, layouts)?,
        )
        .map_err(LoweringError::backend)?;
    let local = value_local(locals, instruction.id, function.id)?;
    builder
        .write_local(block, local, output)
        .map_err(LoweringError::backend)?;
    structural::drop_last_copy_operands(
        function,
        instruction,
        block,
        locals,
        value_types,
        layouts,
        builder,
    )
}
