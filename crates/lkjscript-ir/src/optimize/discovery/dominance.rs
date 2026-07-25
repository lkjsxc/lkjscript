use std::collections::VecDeque;

use crate::optimize::*;
use crate::{BlockId, Function};

pub(crate) struct DiscoveryDominance {
    pub(crate) sets: Vec<Vec<u64>>,
    pub(crate) reachable: Vec<bool>,
}

impl DiscoveryDominance {
    pub(crate) fn compute(
        function: &Function,
        budget: &mut Budget,
    ) -> Result<Self, OptimizationError> {
        let count = function.blocks.len();
        let words = count.saturating_add(63) / 64;
        budget.charge(
            (count as u64)
                .saturating_mul(words as u64)
                .saturating_add((count as u64).saturating_mul(3)),
        )?;
        let mut predecessors = vec![Vec::new(); count];
        for block in &function.blocks {
            let source = block.id.index().ok_or_else(|| {
                input_index_error("discovery block ID cannot index predecessor table")
            })?;
            for successor in discovery_successors(&block.terminator)
                .into_iter()
                .flatten()
            {
                let target = successor.index().ok_or_else(|| {
                    input_index_error("discovery successor ID cannot index predecessor table")
                })?;
                let list = predecessors.get_mut(target).ok_or_else(|| {
                    input_index_error("discovery successor is outside verified function")
                })?;
                budget.charge(1)?;
                list.push(source);
            }
        }
        let entry = function
            .entry
            .index()
            .ok_or_else(|| input_index_error("discovery entry cannot index verified function"))?;
        let mut reachable = vec![false; count];
        let mut queue = VecDeque::new();
        if let Some(slot) = reachable.get_mut(entry) {
            *slot = true;
            queue.push_back(entry);
        }
        while let Some(block_index) = queue.pop_front() {
            budget.charge(1)?;
            let block = function
                .blocks
                .get(block_index)
                .ok_or_else(|| input_index_error("discovery reachability lost a block"))?;
            for successor in discovery_successors(&block.terminator)
                .into_iter()
                .flatten()
            {
                let successor = successor.index().ok_or_else(|| {
                    input_index_error("discovery reachable successor cannot index")
                })?;
                let slot = reachable.get_mut(successor).ok_or_else(|| {
                    input_index_error("discovery reachable successor is outside function")
                })?;
                if !*slot {
                    *slot = true;
                    queue.push_back(successor);
                }
            }
        }
        let mut sets = vec![vec![0_u64; words]; count];
        for block in 0..count {
            if reachable[block] {
                for candidate in 0..count {
                    if reachable[candidate] {
                        sets[block][candidate / 64] |= 1_u64 << (candidate % 64);
                        budget.charge(1)?;
                    }
                }
            }
            sets[block][block / 64] |= 1_u64 << (block % 64);
        }
        sets[entry].fill(0);
        sets[entry][entry / 64] |= 1_u64 << (entry % 64);
        loop {
            let mut changed = false;
            for block in 0..count {
                if block == entry || !reachable[block] {
                    continue;
                }
                budget.charge(words as u64)?;
                let mut next = vec![u64::MAX; words];
                let mut saw_predecessor = false;
                for predecessor in predecessors[block]
                    .iter()
                    .copied()
                    .filter(|predecessor| reachable[*predecessor])
                {
                    saw_predecessor = true;
                    for (word, predecessor_word) in next.iter_mut().zip(&sets[predecessor]) {
                        *word &= *predecessor_word;
                        budget.charge(1)?;
                    }
                }
                if !saw_predecessor {
                    next.fill(0);
                }
                if let Some(last) = next.last_mut() {
                    let excess = words.saturating_mul(64).saturating_sub(count);
                    if excess > 0 {
                        *last &= u64::MAX >> excess;
                    }
                }
                next[block / 64] |= 1_u64 << (block % 64);
                if sets[block] != next {
                    sets[block] = next;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Ok(Self { sets, reachable })
    }

    pub(crate) fn is_reachable(&self, block: BlockId) -> bool {
        block
            .index()
            .and_then(|index| self.reachable.get(index))
            .copied()
            .unwrap_or(false)
    }

    pub(crate) fn definition_dominates(
        &self,
        definition_block: BlockId,
        definition_index: usize,
        use_block: BlockId,
        use_index: usize,
    ) -> bool {
        if !self.is_reachable(definition_block) || !self.is_reachable(use_block) {
            return false;
        }
        if definition_block == use_block {
            return definition_index < use_index;
        }
        let Some(definition) = definition_block.index() else {
            return false;
        };
        use_block
            .index()
            .and_then(|block| self.sets.get(block))
            .and_then(|set| set.get(definition / 64))
            .is_some_and(|word| word & (1_u64 << (definition % 64)) != 0)
    }
}

pub(crate) fn discovery_successors(terminator: &crate::Terminator) -> [Option<BlockId>; 2] {
    match terminator {
        crate::Terminator::Branch { target, .. } => [Some(*target), None],
        crate::Terminator::ConditionalBranch {
            true_target,
            false_target,
            ..
        } => [Some(*true_target), Some(*false_target)],
        _ => [None, None],
    }
}

// The checker below intentionally duplicates semantic and CFG derivation. It
// must remain independent from discovery helpers and discovery dominators.
