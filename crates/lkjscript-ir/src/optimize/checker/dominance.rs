use std::collections::VecDeque;

use crate::optimize::*;
use crate::{BlockId, Function};

pub(crate) struct CheckerDominance {
    pub(crate) matrix: Vec<Vec<u64>>,
    pub(crate) reachable: Vec<bool>,
}

impl CheckerDominance {
    pub(crate) fn derive(
        function: &Function,
        budget: &mut Budget,
    ) -> Result<Self, OptimizationError> {
        let block_count = function.blocks.len();
        let word_count = block_count.saturating_add(63) / 64;
        budget.charge(
            (block_count as u64)
                .saturating_mul(word_count as u64)
                .saturating_add((block_count as u64).saturating_mul(3)),
        )?;
        let mut incoming = vec![Vec::<usize>::new(); block_count];
        for source_block in &function.blocks {
            let source_index = source_block
                .id
                .index()
                .ok_or_else(|| input_index_error("checker source block ID cannot index"))?;
            match &source_block.terminator {
                crate::Terminator::Branch { target, .. } => {
                    checker_add_incoming(&mut incoming, *target, source_index, budget)?;
                }
                crate::Terminator::ConditionalBranch {
                    true_target,
                    false_target,
                    ..
                } => {
                    checker_add_incoming(&mut incoming, *true_target, source_index, budget)?;
                    checker_add_incoming(&mut incoming, *false_target, source_index, budget)?;
                }
                _ => {}
            }
        }
        let entry = function
            .entry
            .index()
            .ok_or_else(|| input_index_error("checker entry block ID cannot index"))?;
        let mut reachable = vec![false; block_count];
        let mut worklist = VecDeque::new();
        let entry_slot = reachable
            .get_mut(entry)
            .ok_or_else(|| input_index_error("checker entry block is outside function"))?;
        *entry_slot = true;
        worklist.push_back(entry);
        while let Some(source) = worklist.pop_front() {
            budget.charge(1)?;
            let terminator = &function
                .blocks
                .get(source)
                .ok_or_else(|| input_index_error("checker reachability source is missing"))?
                .terminator;
            match terminator {
                crate::Terminator::Branch { target, .. } => {
                    checker_mark_reachable(&mut reachable, &mut worklist, *target)?;
                }
                crate::Terminator::ConditionalBranch {
                    true_target,
                    false_target,
                    ..
                } => {
                    checker_mark_reachable(&mut reachable, &mut worklist, *true_target)?;
                    checker_mark_reachable(&mut reachable, &mut worklist, *false_target)?;
                }
                _ => {}
            }
        }
        let mut matrix = vec![vec![0_u64; word_count]; block_count];
        for row in 0..block_count {
            if reachable[row] {
                for candidate in 0..block_count {
                    if reachable[candidate] {
                        matrix[row][candidate / 64] |= 1_u64 << (candidate % 64);
                        budget.charge(1)?;
                    }
                }
            }
            matrix[row][row / 64] |= 1_u64 << (row % 64);
        }
        matrix[entry].fill(0);
        matrix[entry][entry / 64] |= 1_u64 << (entry % 64);
        let mut changed = true;
        while changed {
            changed = false;
            for row in 0..block_count {
                if row == entry || !reachable[row] {
                    continue;
                }
                budget.charge(word_count as u64)?;
                let mut intersection = vec![u64::MAX; word_count];
                let mut any = false;
                for predecessor in incoming[row]
                    .iter()
                    .copied()
                    .filter(|predecessor| reachable[*predecessor])
                {
                    any = true;
                    for word_index in 0..word_count {
                        intersection[word_index] &= matrix[predecessor][word_index];
                        budget.charge(1)?;
                    }
                }
                if !any {
                    intersection.fill(0);
                }
                if let Some(last) = intersection.last_mut() {
                    let unused = word_count.saturating_mul(64).saturating_sub(block_count);
                    if unused != 0 {
                        *last &= u64::MAX >> unused;
                    }
                }
                intersection[row / 64] |= 1_u64 << (row % 64);
                if matrix[row] != intersection {
                    matrix[row] = intersection;
                    changed = true;
                }
            }
        }
        Ok(Self { matrix, reachable })
    }

    pub(crate) fn reached(&self, block: BlockId) -> bool {
        block
            .index()
            .and_then(|index| self.reachable.get(index))
            .copied()
            .unwrap_or(false)
    }

    pub(crate) fn proves_definition_before(
        &self,
        definition_block: BlockId,
        definition_instruction: usize,
        use_block: BlockId,
        use_instruction: usize,
    ) -> bool {
        if !self.reached(definition_block) || !self.reached(use_block) {
            return false;
        }
        if definition_block == use_block {
            return definition_instruction < use_instruction;
        }
        let Some(definition_index) = definition_block.index() else {
            return false;
        };
        use_block
            .index()
            .and_then(|row| self.matrix.get(row))
            .and_then(|bits| bits.get(definition_index / 64))
            .is_some_and(|word| word & (1_u64 << (definition_index % 64)) != 0)
    }
}

pub(crate) fn checker_add_incoming(
    incoming: &mut [Vec<usize>],
    target: BlockId,
    source: usize,
    budget: &mut Budget,
) -> Result<(), OptimizationError> {
    let target = target
        .index()
        .ok_or_else(|| input_index_error("checker successor block ID cannot index"))?;
    let list = incoming
        .get_mut(target)
        .ok_or_else(|| input_index_error("checker successor block is outside function"))?;
    budget.charge(1)?;
    list.push(source);
    Ok(())
}

pub(crate) fn checker_mark_reachable(
    reachable: &mut [bool],
    worklist: &mut VecDeque<usize>,
    block: BlockId,
) -> Result<(), OptimizationError> {
    let index = block
        .index()
        .ok_or_else(|| input_index_error("checker reachable block ID cannot index"))?;
    let slot = reachable
        .get_mut(index)
        .ok_or_else(|| input_index_error("checker reachable block is outside function"))?;
    if !*slot {
        *slot = true;
        worklist.push_back(index);
    }
    Ok(())
}
