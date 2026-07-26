use super::*;

pub(super) fn lower_terminator(
    function: &Function,
    block: &Block,
    context: TerminatorContext<'_>,
    builder: &mut FunctionBuilder,
    _explicit_traps: &mut Vec<(u32, String)>,
) -> Result<(), LoweringError> {
    let native_block = context.native_block;
    let edges = context.edges;
    let blocks = context.blocks;
    let locals = context.locals;
    match &block.terminator {
        Terminator::Branch { target, arguments } => {
            let edge = edges.branch.ok_or_else(|| invalid_edges(function.id))?;
            builder
                .branch(native_block, edge)
                .map_err(LoweringError::backend)?;
            lower_edge(function, edge, *target, arguments, blocks, locals, builder)?;
        }
        Terminator::ConditionalBranch {
            condition,
            true_target,
            true_arguments,
            false_target,
            false_arguments,
        } => {
            let condition = read_value(builder, native_block, locals, *condition, function.id)?;
            let when_true = edges.when_true.ok_or_else(|| invalid_edges(function.id))?;
            let when_false = edges.when_false.ok_or_else(|| invalid_edges(function.id))?;
            builder
                .branch_if(native_block, condition, when_true, when_false)
                .map_err(LoweringError::backend)?;
            lower_edge(
                function,
                when_true,
                *true_target,
                true_arguments,
                blocks,
                locals,
                builder,
            )?;
            lower_edge(
                function,
                when_false,
                *false_target,
                false_arguments,
                blocks,
                locals,
                builder,
            )?;
        }
        Terminator::Return(value) => {
            let value = read_value(builder, native_block, locals, *value, function.id)?;
            builder
                .return_value(native_block, value)
                .map_err(LoweringError::backend)?;
        }
        Terminator::Trap { value } => {
            let value = read_value(builder, native_block, locals, *value, function.id)?;
            builder
                .trap_value(native_block, value)
                .map_err(LoweringError::backend)?;
        }
        Terminator::Exit { code } => {
            let code = read_value(builder, native_block, locals, *code, function.id)?;
            builder
                .exit(native_block, code)
                .map_err(LoweringError::backend)?;
        }
        Terminator::Outcome { outcome, detail } => {
            if detail.is_some() {
                return unsupported_operation(function.id, "structured outcome detail");
            }
            let outcome = match outcome {
                StructuredOutcome::DeadlineExceeded => RuntimeOutcome::DeadlineExceeded,
                StructuredOutcome::ResourceLimitExceeded => RuntimeOutcome::ResourceLimitExceeded,
                StructuredOutcome::HostFailure => RuntimeOutcome::HostFailure,
            };
            builder
                .outcome(native_block, outcome)
                .map_err(LoweringError::backend)?;
        }
    }
    Ok(())
}

pub(super) fn lower_edge(
    function: &Function,
    edge: lkjscript_native::BlockId,
    target: lkjscript_ir::BlockId,
    arguments: &[ValueId],
    blocks: &[lkjscript_native::BlockId],
    locals: &[LocalId],
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
        builder
            .runtime_call(edge, RuntimeCallSlot::PollV1, Vec::new())
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
