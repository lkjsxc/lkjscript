use std::collections::{HashMap, HashSet};

use crate::optimize::passes::*;
use crate::{Block, BlockId, Terminator, VerifiedProgram};

pub fn canonical_block_order(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        let blocks: HashMap<BlockId, Block> = function
            .blocks
            .iter()
            .cloned()
            .map(|block| (block.id, block))
            .collect();
        let mut order = Vec::with_capacity(function.blocks.len());
        let mut seen = HashSet::new();
        canonical_walk(function.entry, &blocks, &mut seen, &mut order);
        let mut remaining: Vec<BlockId> = blocks
            .keys()
            .copied()
            .filter(|id| !seen.contains(id))
            .collect();
        remaining.sort_unstable();
        order.extend(remaining);
        function.blocks = order
            .into_iter()
            .filter_map(|id| blocks.get(&id).cloned())
            .collect();
    }
    compact_blocks(&mut program)?;
    compact_values(&mut program)?;
    finish(program)
}

pub(crate) fn canonical_walk(
    current: BlockId,
    blocks: &HashMap<BlockId, Block>,
    seen: &mut HashSet<BlockId>,
    order: &mut Vec<BlockId>,
) {
    if !seen.insert(current) {
        return;
    }
    order.push(current);
    let Some(block) = blocks.get(&current) else {
        return;
    };
    for successor in successors(&block.terminator) {
        canonical_walk(successor, blocks, seen, order);
    }
}

pub(crate) fn clear_stale_loop_headers(function: &mut crate::Function) {
    let stale: Vec<BlockId> = function
        .blocks
        .iter()
        .filter(|block| block.metadata.loop_header)
        .filter_map(|header| {
            let mut seen = HashSet::new();
            let mut work = successors(&header.terminator);
            let mut cyclic = false;
            while let Some(current) = work.pop() {
                if current == header.id {
                    cyclic = true;
                    break;
                }
                if !seen.insert(current) {
                    continue;
                }
                if let Some(block) = function.blocks.iter().find(|block| block.id == current) {
                    work.extend(successors(&block.terminator));
                }
            }
            (!cyclic).then_some(header.id)
        })
        .collect();
    for id in stale {
        if let Some(block) = function.blocks.iter_mut().find(|block| block.id == id) {
            block.metadata.loop_header = false;
            block.metadata.frame_state = None;
        }
    }
}

pub(crate) fn successors(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Branch { target, .. } => vec![*target],
        Terminator::ConditionalBranch {
            true_target,
            false_target,
            ..
        } => vec![*true_target, *false_target],
        _ => Vec::new(),
    }
}
