pub(super) fn drop_last_copy_operands(
    function: &Function,
    instruction: &Instruction,
    block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    layouts: &LayoutInterner,
    builder: &mut FunctionBuilder,
) -> Result<(), LoweringError> {
    for operand in instruction.kind.operands() {
        if consuming_operand(&instruction.kind, operand)
            || !layouts
                .structural()
                .copy_type(source_type(function, operand)?)
            || used_after(function, instruction.id, operand)
        {
            continue;
        }
        let _ = lower_drop(function, operand, block, locals, value_types, builder)?;
    }
    Ok(())
}

pub(in crate::lower) fn consuming_operand(kind: &InstructionKind, value: ValueId) -> bool {
    match kind {
        InstructionKind::StructuralPublish {
            value: consumed, ..
        }
        | InstructionKind::DestinationFinish {
            destination: consumed,
        }
        | InstructionKind::DestinationAbort {
            destination: consumed,
        }
        | InstructionKind::AggregateConsumePayload {
            value: consumed, ..
        }
        | InstructionKind::Drop {
            value: consumed, ..
        }
        | InstructionKind::Move {
            value: consumed, ..
        }
        | InstructionKind::EndBorrow {
            value: consumed, ..
        } => *consumed == value,
        InstructionKind::DestinationFieldInit {
            destination,
            value: field,
            ..
        } => *destination == value || *field == value,
        InstructionKind::Call {
            arguments,
            consuming,
            ..
        } => arguments
            .iter()
            .zip(consuming)
            .any(|(argument, consuming)| *argument == value && *consuming),
        _ => false,
    }
}

fn used_after(function: &Function, instruction: ValueId, value: ValueId) -> bool {
    let Some((block, index)) = function.blocks.iter().find_map(|block| {
        block
            .instructions
            .iter()
            .position(|candidate| candidate.id == instruction)
            .map(|index| (block, index))
    }) else {
        return true;
    };
    block.instructions[index.saturating_add(1)..]
        .iter()
        .any(|candidate| candidate.kind.operands().contains(&value))
        || block.terminator.operands().contains(&value)
}
