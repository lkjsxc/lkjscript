use crate::verify::*;
use crate::{Block, BlockId, Function, ValueId};

pub(super) fn incoming(function: &Function, target: BlockId) -> Vec<&Block> {
    function
        .blocks
        .iter()
        .filter(|block| successors(&block.terminator).contains(&target))
        .collect()
}

pub(super) fn edge_argument(
    block: &Block,
    target: BlockId,
    index: usize,
) -> crate::Result<ValueId> {
    let arguments = match &block.terminator {
        crate::Terminator::Branch {
            target: edge,
            arguments,
        } if *edge == target => arguments,
        crate::Terminator::ConditionalBranch {
            true_target,
            true_arguments,
            ..
        } if *true_target == target => true_arguments,
        crate::Terminator::ConditionalBranch {
            false_target,
            false_arguments,
            ..
        } if *false_target == target => false_arguments,
        _ => return fail("SSA active-variant predecessor edge is inconsistent"),
    };
    arguments
        .get(index)
        .copied()
        .ok_or_else(|| crate::IrError::new("SSA active-variant edge argument is missing"))
}

pub(super) fn find_instruction(function: &Function, value: ValueId) -> Option<&crate::Instruction> {
    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find(|item| item.id == value)
}

pub(super) fn charge(work: &mut usize) -> crate::Result<()> {
    *work = work
        .checked_add(1)
        .ok_or_else(|| crate::IrError::new("SSA active-variant verification work overflow"))?;
    if *work > SSA_VERIFY_MAX_CFG_WORK {
        fail("SSA active-variant verification work exceeds bound")
    } else {
        Ok(())
    }
}
