use super::*;

mod cleanup;
mod edges;
mod terminal;

pub(super) use cleanup::static_trap_message;
use edges::*;
use terminal::*;

pub(super) fn lower_terminator(
    function: &Function,
    block: &Block,
    context: TerminatorContext<'_>,
    builder: &mut FunctionBuilder,
    explicit_traps: &mut Vec<(u32, String)>,
) -> Result<(), LoweringError> {
    let native_block = context.native_block;
    let edges = context.edges;
    let blocks = context.blocks;
    let locals = context.locals;
    let value_types = context.value_types;
    let layouts = context.layouts;
    let failure_cleanup = block.metadata.failure_cleanup;
    match &block.terminator {
        Terminator::Branch { target, arguments } => {
            let edge = edges.branch.ok_or_else(|| invalid_edges(function.id))?;
            builder
                .branch(native_block, edge)
                .map_err(LoweringError::backend)?;
            lower_edge(
                function,
                edge,
                *target,
                arguments,
                failure_cleanup,
                blocks,
                locals,
                value_types,
                layouts,
                builder,
            )?;
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
                failure_cleanup,
                blocks,
                locals,
                value_types,
                layouts,
                builder,
            )?;
            lower_edge(
                function,
                when_false,
                *false_target,
                false_arguments,
                failure_cleanup,
                blocks,
                locals,
                value_types,
                layouts,
                builder,
            )?;
        }
        _ => lower_terminal(function, block, context, builder, explicit_traps)?,
    }
    Ok(())
}
