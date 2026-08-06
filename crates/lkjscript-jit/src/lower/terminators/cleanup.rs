use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn drop_copy_parameters(
    function: &Function,
    block: &Block,
    returned: Option<ValueId>,
    native_block: lkjscript_native::BlockId,
    locals: &[LocalId],
    value_types: &[ValueType],
    _layouts: &LayoutInterner,
    builder: &mut FunctionBuilder,
) -> Result<(), LoweringError> {
    for parameter in &block.parameters {
        if Some(parameter.id) == returned
            || !matches!(
                value_type(value_types, parameter.id)?,
                ValueType::StructuralOwner(_)
            )
            || block
                .instructions
                .iter()
                .any(|instruction| consuming_operand(&instruction.kind, parameter.id))
        {
            continue;
        }
        let _ = lower_structural_drop(
            function,
            parameter.id,
            native_block,
            locals,
            value_types,
            builder,
        )?;
    }
    Ok(())
}

pub(in crate::lower) fn static_trap_message(function: &Function, value: ValueId) -> Option<&str> {
    let mut message = None;
    let mut uses = 0_usize;
    let mut trap_uses = 0_usize;
    for block in &function.blocks {
        for instruction in &block.instructions {
            if instruction.id == value {
                let InstructionKind::Constant(Constant::Str(text)) = &instruction.kind else {
                    return None;
                };
                message = Some(text.as_str());
            }
            uses = uses.saturating_add(
                instruction
                    .kind
                    .operands()
                    .into_iter()
                    .filter(|operand| *operand == value)
                    .count(),
            );
        }
        let block_uses = block
            .terminator
            .operands()
            .into_iter()
            .filter(|operand| *operand == value)
            .count();
        uses = uses.saturating_add(block_uses);
        if matches!(&block.terminator, Terminator::Trap { value: trap } if *trap == value) {
            trap_uses = trap_uses.saturating_add(1);
        }
    }
    (uses == 1 && trap_uses == 1).then_some(message?)
}
