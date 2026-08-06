use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_edge(
    function: &Function,
    edge: lkjscript_native::BlockId,
    target: lkjscript_ir::BlockId,
    arguments: &[ValueId],
    failure_cleanup: Option<lkjscript_ir::FailureCleanupRoots>,
    blocks: &[lkjscript_native::BlockId],
    locals: &[LocalId],
    value_types: &[ValueType],
    layouts: &LayoutInterner,
    builder: &mut FunctionBuilder,
) -> Result<(), LoweringError> {
    let target_index = target.index().ok_or_else(|| invalid_edges(function.id))?;
    let target_block = function
        .blocks
        .get(target_index)
        .filter(|block| block.id == target)
        .ok_or_else(|| invalid_edges(function.id))?;
    let native_target = *blocks
        .get(target_index)
        .ok_or_else(|| invalid_edges(function.id))?;
    if target_block.parameters.len() != arguments.len() {
        return Err(invalid_edges(function.id));
    }
    // Read every source before writing any target. This gives block arguments
    // parallel-copy semantics even when a loop rotates parameter locals.
    let values = read_values(builder, edge, locals, arguments, function.id)?;
    for (parameter, value) in target_block.parameters.iter().zip(values) {
        let local = value_local(locals, parameter.id, function.id)?;
        builder
            .write_local(edge, local, value)
            .map_err(LoweringError::backend)?;
    }
    if target_block.metadata.loop_header {
        let poll = builder
            .runtime_call(edge, RuntimeCallSlot::Poll, Vec::new())
            .map_err(LoweringError::backend)?;
        builder
            .set_instruction_failure_cleanup(
                poll,
                lower_failure_cleanup_id(
                    function,
                    failure_cleanup,
                    locals,
                    value_types,
                    layouts,
                    &[],
                )?,
            )
            .map_err(LoweringError::backend)?;
    }
    builder
        .branch(edge, native_target)
        .map_err(LoweringError::backend)
}

pub(super) fn invalid_edges(function: FunctionId) -> LoweringError {
    LoweringError::new(
        LoweringFailureCode::InvalidFunction,
        Some(function),
        "SSA edge metadata is inconsistent",
    )
}
