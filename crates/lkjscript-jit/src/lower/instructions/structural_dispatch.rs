use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_instruction(
    function: &Function,
    instruction: &Instruction,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    layouts: &LayoutInterner,
    static_bytes: &HashMap<Vec<u8>, lkjscript_native::StaticBytesIdentity>,
    builder: &mut FunctionBuilder,
) -> Result<bool, LoweringError> {
    let output = match &instruction.kind {
        InstructionKind::Move { value, .. }
            if matches!(
                value_type(value_types, *value)?,
                ValueType::StructuralOwner(_)
            ) =>
        {
            structural::lower_move(function, *value, block, locals, value_types, builder)?
        }
        InstructionKind::EndBorrow { value, .. }
            if matches!(
                value_type(value_types, *value)?,
                ValueType::StructuralView(_)
            ) =>
        {
            structural::lower_end_view(function, *value, block, locals, value_types, builder)?
        }
        InstructionKind::Drop {
            value,
            glue: lkjscript_ir::DropGlueIdentity::Structural(_),
            ..
        } => structural::lower_drop(function, *value, block, locals, value_types, builder)?,
        InstructionKind::Runtime {
            operation: operation @ (RuntimeOp::EmptyStr | RuntimeOp::StrFromI64 | RuntimeOp::StrLen),
            arguments,
            ..
        } => structural::lower_structural_runtime(
            function,
            instruction,
            *operation,
            arguments,
            block,
            locals,
            value_types,
            layouts,
            static_bytes,
            builder,
        )?,
        InstructionKind::Borrow { kind, value, .. }
            if matches!(
                value_type(value_types, *value)?,
                ValueType::StructuralOwner(_)
            ) =>
        {
            structural::lower_borrow(function, *kind, *value, block, locals, layouts, builder)?
        }
        InstructionKind::ProductField { value, .. }
            if layouts
                .structural()
                .selected(structural::source_type(function, *value)?) =>
        {
            structural::lower_structural_instruction(
                function,
                instruction,
                block,
                locals,
                value_types,
                layouts,
                builder,
            )?
        }
        InstructionKind::StructuralPublish { .. }
        | InstructionKind::DestinationCreate { .. }
        | InstructionKind::DestinationFieldInit { .. }
        | InstructionKind::DestinationFinish { .. }
        | InstructionKind::DestinationAbort { .. }
        | InstructionKind::AggregateFieldBorrow { .. }
        | InstructionKind::AggregateTag { .. }
        | InstructionKind::AggregateConsumePayload { .. }
        | InstructionKind::StringUtf8View { .. }
        | InstructionKind::StructuralCopy { .. } => structural::lower_structural_instruction(
            function,
            instruction,
            block,
            locals,
            value_types,
            layouts,
            builder,
        )?,
        _ => return Ok(false),
    };
    write_instruction_output(
        function,
        instruction,
        block,
        locals,
        value_types,
        layouts,
        output,
        builder,
    )
    .map_err(LoweringError::backend)?;
    Ok(true)
}
