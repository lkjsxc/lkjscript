use crate::optimize::*;
use crate::{BlockId, EffectSet, FailureBehavior, Instruction, InstructionKind, Program, ValueId};

pub(crate) fn discovery_apply_record(
    candidate: &mut Program,
    record: &OptimizationCertificateRecord,
    definition_block: BlockId,
    definition_index: Option<usize>,
    budget: &mut Budget,
) -> Result<(), OptimizationError> {
    budget.charge(1_u64.saturating_add(record.expected_operands.len() as u64))?;
    if definition_block != record.block {
        return Err(illegal_edit("certificate block ID is stale"));
    }
    let instruction_index =
        definition_index.ok_or_else(|| illegal_edit("certificate targets a block parameter"))?;
    let function = candidate
        .functions
        .get_mut(record.function.index().unwrap_or(usize::MAX))
        .filter(|function| function.id == record.function)
        .ok_or_else(|| illegal_edit("certificate function ID is stale"))?;
    let block = function
        .blocks
        .get_mut(record.block.index().unwrap_or(usize::MAX))
        .filter(|block| block.id == record.block)
        .ok_or_else(|| illegal_edit("certificate block ID is stale"))?;
    let instruction = block
        .instructions
        .get_mut(instruction_index)
        .filter(|instruction| instruction.id == record.value)
        .ok_or_else(|| illegal_edit("certificate value ID is stale"))?;
    match &instruction.kind {
        InstructionKind::Runtime {
            operation,
            arguments,
            ..
        } if *operation == record.expected_operation && *arguments == record.expected_operands => {}
        _ => return Err(illegal_edit("certificate operation or operands are stale")),
    }
    replace_with_copy(instruction, record.replacement);
    Ok(())
}

pub(crate) fn checker_apply_record(
    candidate: &mut Program,
    record: &OptimizationCertificateRecord,
    definition_block: BlockId,
    definition_index: Option<usize>,
) -> Result<(), OptimizationError> {
    if definition_block != record.block {
        return Err(illegal_edit(
            "checker record block differs from private index",
        ));
    }
    let instruction_index = definition_index
        .ok_or_else(|| illegal_edit("checker record has no instruction location"))?;
    let function = candidate
        .functions
        .get_mut(record.function.index().unwrap_or(usize::MAX))
        .filter(|function| function.id == record.function)
        .ok_or_else(|| illegal_edit("checker private function is stale"))?;
    let block = function
        .blocks
        .get_mut(record.block.index().unwrap_or(usize::MAX))
        .filter(|block| block.id == record.block)
        .ok_or_else(|| illegal_edit("checker private block is stale"))?;
    let instruction = block
        .instructions
        .get_mut(instruction_index)
        .filter(|instruction| instruction.id == record.value)
        .ok_or_else(|| illegal_edit("checker private instruction is stale"))?;
    match &instruction.kind {
        InstructionKind::Runtime {
            operation,
            arguments,
            ..
        } if *operation == record.expected_operation && *arguments == record.expected_operands => {}
        _ => {
            return Err(illegal_edit(
                "checker private operation or operands do not match record",
            ));
        }
    }
    replace_with_copy(instruction, record.replacement);
    Ok(())
}

pub(crate) fn replace_with_copy(instruction: &mut Instruction, replacement: ValueId) {
    instruction.kind = InstructionKind::Copy(replacement);
    instruction.metadata.effects = EffectSet::PURE;
    instruction.metadata.failure = FailureBehavior::None;
    instruction.metadata.frame_state = None;
}
