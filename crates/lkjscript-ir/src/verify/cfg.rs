use std::collections::HashSet;

use crate::verify::*;
use crate::{BlockId, Function, IrError};

pub(crate) fn predecessors(function: &Function) -> crate::Result<Vec<Vec<BlockId>>> {
    let mut result = vec![Vec::new(); function.blocks.len()];
    for block in &function.blocks {
        for successor in successors(&block.terminator) {
            let Some(slot) = successor.index().and_then(|index| result.get_mut(index)) else {
                return fail("SSA terminator references a missing block");
            };
            slot.push(block.id);
        }
    }
    Ok(result)
}

pub(crate) fn reachable(function: &Function) -> crate::Result<HashSet<BlockId>> {
    let mut result = HashSet::new();
    let mut work = vec![function.entry];
    while let Some(current) = work.pop() {
        if !result.insert(current) {
            continue;
        }
        let current = block(function, current)?;
        work.extend(successors(&current.terminator));
    }
    Ok(result)
}

pub(crate) fn dominators(
    function: &Function,
    predecessors: &[Vec<BlockId>],
) -> crate::Result<Dominators> {
    let block_count = function.blocks.len();
    let word_count = block_count.saturating_add(63) / 64;
    let initial_cells = block_count
        .checked_mul(word_count)
        .ok_or_else(|| IrError::new("SSA dominator state size overflow"))?;
    if initial_cells > SSA_VERIFY_MAX_CFG_WORK {
        return fail(format!(
            "SSA CFG verification work exceeds {SSA_VERIFY_MAX_CFG_WORK}"
        ));
    }
    let mut result = vec![vec![u64::MAX; word_count]; block_count];
    let entry_index = function
        .entry
        .index()
        .ok_or_else(|| IrError::new("SSA entry BlockId cannot be indexed"))?;
    let entry = result
        .get_mut(entry_index)
        .ok_or_else(|| IrError::new("SSA entry BlockId is missing"))?;
    entry.fill(0);
    set_dominator_bit(entry, function.entry)?;

    let mut work = initial_cells;
    let mut changed = true;
    while changed {
        changed = false;
        for block in &function.blocks {
            if block.id == function.entry {
                continue;
            }
            let preds = block
                .id
                .index()
                .and_then(|index| predecessors.get(index))
                .ok_or_else(|| IrError::new("SSA predecessor metadata is inconsistent"))?;
            let mut next = if let Some(first) = preds.first() {
                result
                    .get(first.index().unwrap_or(usize::MAX))
                    .cloned()
                    .ok_or_else(|| IrError::new("SSA predecessor BlockId is invalid"))?
            } else {
                vec![0; word_count]
            };
            charge_cfg_work(&mut work, word_count)?;
            for predecessor in preds.iter().skip(1) {
                let other = result
                    .get(predecessor.index().unwrap_or(usize::MAX))
                    .ok_or_else(|| IrError::new("SSA predecessor BlockId is invalid"))?;
                charge_cfg_work(&mut work, word_count)?;
                for (word, other_word) in next.iter_mut().zip(other) {
                    *word &= *other_word;
                }
            }
            set_dominator_bit(&mut next, block.id)?;
            let current = block
                .id
                .index()
                .and_then(|index| result.get_mut(index))
                .ok_or_else(|| IrError::new("SSA dominator metadata is inconsistent"))?;
            if *current != next {
                *current = next;
                changed = true;
            }
        }
    }
    Ok(result)
}

pub(crate) fn charge_cfg_work(work: &mut usize, amount: usize) -> crate::Result<()> {
    *work = work
        .checked_add(amount)
        .ok_or_else(|| IrError::new("SSA CFG verification work overflow"))?;
    if *work > SSA_VERIFY_MAX_CFG_WORK {
        return fail(format!(
            "SSA CFG verification work exceeds {SSA_VERIFY_MAX_CFG_WORK}"
        ));
    }
    Ok(())
}

pub(crate) fn set_dominator_bit(bits: &mut [u64], block: BlockId) -> crate::Result<()> {
    let index = block
        .index()
        .ok_or_else(|| IrError::new("SSA BlockId cannot be indexed"))?;
    let word = bits
        .get_mut(index / 64)
        .ok_or_else(|| IrError::new("SSA dominator bit index is invalid"))?;
    *word |= 1_u64 << (index % 64);
    Ok(())
}

pub(crate) fn dominates(
    dominators: &Dominators,
    block: BlockId,
    candidate: BlockId,
) -> crate::Result<bool> {
    let block_index = block
        .index()
        .ok_or_else(|| IrError::new("SSA use BlockId cannot be indexed"))?;
    let candidate_index = candidate
        .index()
        .ok_or_else(|| IrError::new("SSA definition BlockId cannot be indexed"))?;
    let bits = dominators
        .get(block_index)
        .ok_or_else(|| IrError::new("SSA dominance metadata is inconsistent"))?;
    let word = bits
        .get(candidate_index / 64)
        .ok_or_else(|| IrError::new("SSA dominance bit index is invalid"))?;
    Ok(word & (1_u64 << (candidate_index % 64)) != 0)
}

pub(crate) fn verify_loops(
    function: &Function,
    dominators: &Dominators,
    reachable: &HashSet<BlockId>,
) -> crate::Result<()> {
    let mut headers = HashSet::new();
    for block in &function.blocks {
        if !reachable.contains(&block.id) {
            continue;
        }
        for successor in successors(&block.terminator) {
            if dominates(dominators, block.id, successor)? {
                let target = block_by_id(function, successor)?;
                if !target.metadata.loop_header {
                    return fail(format!(
                        "SSA backedge targets unmarked loop header {}",
                        successor.raw()
                    ));
                }
                headers.insert(successor);
            }
        }
    }
    for block in &function.blocks {
        if block.metadata.loop_header && !headers.contains(&block.id) {
            return fail(format!(
                "SSA block {} is marked loop-header without a backedge",
                block.id.raw()
            ));
        }
        if block.metadata.loop_header && block.metadata.frame_state.is_none() {
            return fail(format!(
                "SSA loop header {} has no frame state",
                block.id.raw()
            ));
        }
    }
    Ok(())
}
