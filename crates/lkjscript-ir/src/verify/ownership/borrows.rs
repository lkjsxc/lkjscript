use std::collections::{BTreeMap, BTreeSet};

use crate::verify::*;
use crate::{Function, InstructionKind, IrError, ValueId};

pub(crate) fn verify_borrow_uses(
    function: &Function,
    borrows: &BTreeMap<ValueId, BorrowDefinition>,
    work: &mut usize,
) -> crate::Result<()> {
    let mut semantic_uses: BTreeMap<ValueId, usize> =
        borrows.keys().copied().map(|value| (value, 0)).collect();
    let mut ended = BTreeSet::new();
    for block in &function.blocks {
        verify_frame_borrows(
            block.metadata.frame_state.as_ref(),
            block.id,
            borrows,
            &ended,
        )?;
        for instruction in &block.instructions {
            charge_ownership_work(work, 1)?;
            if let InstructionKind::EndBorrow { place, loan, value } = instruction.kind {
                let definition = borrows
                    .get(&value)
                    .ok_or_else(|| IrError::new("SSA EndBorrow references a non-borrow value"))?;
                if definition.block != block.id
                    || definition.place != place
                    || definition.loan != loan
                    || !ended.insert(value)
                {
                    return fail("SSA EndBorrow is duplicated or has mismatched provenance");
                }
                if semantic_uses.get(&value).copied().unwrap_or(0) == 0 {
                    return fail("SSA EndBorrow precedes any supported semantic loan use");
                }
            } else {
                for operand in instruction.kind.operands() {
                    let Some(definition) = borrows.get(&operand) else {
                        continue;
                    };
                    if definition.block != block.id {
                        return fail("SSA Borrow result used outside its defining block");
                    }
                    if ended.contains(&operand) {
                        return fail("SSA Borrow result is used after EndBorrow");
                    }
                    if !matches!(
                        instruction.kind,
                        InstructionKind::Runtime { .. } | InstructionKind::Call { .. }
                    ) {
                        return fail("SSA Borrow result has an unsupported non-argument use");
                    }
                    let count = semantic_uses
                        .get_mut(&operand)
                        .ok_or_else(|| IrError::new("SSA Borrow use accounting is inconsistent"))?;
                    *count = count
                        .checked_add(1)
                        .ok_or_else(|| IrError::new("SSA Borrow use count overflow"))?;
                }
            }
            verify_frame_borrows(
                instruction.metadata.frame_state.as_ref(),
                block.id,
                borrows,
                &ended,
            )?;
        }
        for operand in block.terminator.operands() {
            if borrows.contains_key(&operand) {
                return fail("SSA Borrow result cannot cross a block edge or terminator");
            }
        }
    }
    if let Some((value, definition)) = borrows.iter().find(|(value, _)| {
        semantic_uses.get(value).copied().unwrap_or(0) == 0 || !ended.contains(value)
    }) {
        return fail(format!(
            "SSA Borrow result {} for LoanId {} lacks one semantic use and EndBorrow",
            value.raw(),
            definition.loan.raw()
        ));
    }
    Ok(())
}

fn verify_frame_borrows(
    frame: Option<&crate::FrameState>,
    block: crate::BlockId,
    borrows: &BTreeMap<ValueId, BorrowDefinition>,
    ended: &BTreeSet<ValueId>,
) -> crate::Result<()> {
    let Some(frame) = frame else {
        return Ok(());
    };
    for value in frame_values(frame) {
        if let Some(definition) = borrows.get(&value) {
            if definition.block != block || ended.contains(&value) {
                return fail("SSA frame retains a cross-block or ended loan");
            }
        }
    }
    Ok(())
}
