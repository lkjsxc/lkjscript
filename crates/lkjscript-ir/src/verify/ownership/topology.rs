use std::collections::{BTreeSet, VecDeque};

use crate::verify::*;
use crate::{BlockId, Function};

pub(crate) fn block_is_cyclic(
    function: &Function,
    start: BlockId,
    work: &mut usize,
) -> crate::Result<bool> {
    let start_block = block_by_id(function, start)?;
    let mut queue: VecDeque<BlockId> = successors(&start_block.terminator).into();
    let mut seen = BTreeSet::new();
    while let Some(current) = queue.pop_front() {
        charge_ownership_work(work, 1)?;
        if current == start {
            return Ok(true);
        }
        if !seen.insert(current) {
            continue;
        }
        let current = block_by_id(function, current)?;
        queue.extend(successors(&current.terminator));
    }
    Ok(false)
}
