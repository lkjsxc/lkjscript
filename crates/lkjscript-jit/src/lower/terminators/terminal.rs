use super::cleanup::drop_copy_parameters;
use super::*;

pub(super) fn lower_terminal(
    function: &Function,
    block: &Block,
    context: TerminatorContext<'_>,
    builder: &mut FunctionBuilder,
    explicit_traps: &mut Vec<(u64, String)>,
) -> Result<(), LoweringError> {
    let native_block = context.native_block;
    let locals = context.locals;
    let value_types = context.value_types;
    let layouts = context.layouts;
    let failure_cleanup = block.metadata.failure_cleanup;
    match &block.terminator {
        Terminator::Return(value) => {
            lower_structural_terminal_cleanup(
                function,
                failure_cleanup,
                Some(*value),
                native_block,
                locals,
                value_types,
                layouts,
                builder,
            )?;
            drop_copy_parameters(
                function,
                block,
                Some(*value),
                native_block,
                locals,
                value_types,
                layouts,
                builder,
            )?;
            let value = read_value(builder, native_block, locals, *value, function.id)?;
            builder
                .return_value(native_block, value)
                .map_err(LoweringError::backend)?;
        }
        Terminator::Trap { value } => {
            let message = static_trap_message(function, *value);
            if message.is_none() {
                lower_structural_trap_message(
                    function,
                    *value,
                    native_block,
                    locals,
                    value_types,
                    builder,
                )?;
            }
            lower_structural_terminal_cleanup(
                function,
                failure_cleanup,
                Some(*value),
                native_block,
                locals,
                value_types,
                layouts,
                builder,
            )?;
            drop_copy_parameters(
                function,
                block,
                Some(*value),
                native_block,
                locals,
                value_types,
                layouts,
                builder,
            )?;
            if let Some(message) = message {
                let site = u64::try_from(explicit_traps.len())
                    .ok()
                    .and_then(|index| index.checked_add(1))
                    .ok_or_else(|| {
                        LoweringError::new(
                            LoweringFailureCode::InvalidFunction,
                            Some(function.id),
                            "native static trap site identity space exhausted",
                        )
                    })?;
                explicit_traps.push((site, message.to_owned()));
                builder
                    .trap_at(native_block, TrapCode::Explicit, site)
                    .map_err(LoweringError::backend)?;
            } else {
                builder
                    .trap(native_block, TrapCode::Explicit)
                    .map_err(LoweringError::backend)?;
            }
        }
        Terminator::Exit { code } => {
            lower_structural_terminal_cleanup(
                function,
                failure_cleanup,
                None,
                native_block,
                locals,
                value_types,
                layouts,
                builder,
            )?;
            drop_copy_parameters(
                function,
                block,
                Some(*code),
                native_block,
                locals,
                value_types,
                layouts,
                builder,
            )?;
            let code = read_value(builder, native_block, locals, *code, function.id)?;
            builder
                .exit(native_block, code)
                .map_err(LoweringError::backend)?;
        }
        Terminator::Outcome { outcome, detail } => {
            lower_structural_terminal_cleanup(
                function,
                failure_cleanup,
                None,
                native_block,
                locals,
                value_types,
                layouts,
                builder,
            )?;
            drop_copy_parameters(
                function,
                block,
                None,
                native_block,
                locals,
                value_types,
                layouts,
                builder,
            )?;
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
        Terminator::Branch { .. } | Terminator::ConditionalBranch { .. } => {
            return Err(invalid_edges(function.id));
        }
    }
    Ok(())
}
