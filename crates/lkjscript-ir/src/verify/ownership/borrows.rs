use std::collections::BTreeMap;

use crate::verify::*;
use crate::{Function, InstructionKind, IrError, ValueId};

pub(crate) fn verify_borrow_uses(
    function: &Function,
    borrows: &BTreeMap<ValueId, BorrowDefinition>,
    work: &mut usize,
) -> crate::Result<()> {
    let mut semantic_uses: BTreeMap<ValueId, usize> =
        borrows.keys().copied().map(|value| (value, 0)).collect();
    for block in &function.blocks {
        if let Some(frame) = &block.metadata.frame_state {
            for value in frame_values(frame) {
                if let Some(definition) = borrows.get(&value) {
                    if definition.block != block.id {
                        return fail(
                            "SSA Borrow result cannot appear in another block's frame state",
                        );
                    }
                }
            }
        }
        for instruction in &block.instructions {
            charge_ownership_work(work, 1)?;
            for operand in instruction.kind.operands() {
                let Some(definition) = borrows.get(&operand) else {
                    continue;
                };
                if definition.block != block.id {
                    return fail(
                        "SSA Borrow result used outside its defining block is unsupported",
                    );
                }
                let supported = matches!(
                    instruction.kind,
                    InstructionKind::Runtime { .. } | InstructionKind::Call { .. }
                );
                if !supported {
                    return fail(
                        "SSA Borrow result has an unsupported non-argument use in this slice",
                    );
                }
                let Some(count) = semantic_uses.get_mut(&operand) else {
                    return fail("SSA Borrow use accounting is inconsistent");
                };
                *count = count
                    .checked_add(1)
                    .ok_or_else(|| IrError::new("SSA Borrow use count overflow"))?;
            }
            if let Some(frame) = &instruction.metadata.frame_state {
                for value in frame_values(frame) {
                    if let Some(definition) = borrows.get(&value) {
                        if definition.block != block.id {
                            return fail(
                                "SSA Borrow result cannot appear in another block's frame state",
                            );
                        }
                    }
                }
            }
        }
        for operand in block.terminator.operands() {
            if borrows.contains_key(&operand) {
                return fail(
                    "SSA Borrow result cannot cross a block edge or terminator in this slice",
                );
            }
        }
    }
    if let Some((value, definition)) = borrows
        .iter()
        .find(|(value, _)| semantic_uses.get(value).copied().unwrap_or(0) == 0)
    {
        return fail(format!(
            "SSA Borrow result {} for LoanId {} has no supported same-block argument use",
            value.raw(),
            definition.loan.raw()
        ));
    }
    Ok(())
}
