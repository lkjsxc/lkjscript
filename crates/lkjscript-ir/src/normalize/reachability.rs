use std::collections::HashSet;

use crate::normalize::*;
use crate::VerifiedProgram;

pub fn unreachable_blocks(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        let mut reachable = HashSet::new();
        let mut work = vec![function.entry];
        while let Some(current) = work.pop() {
            if !reachable.insert(current) {
                continue;
            }
            let Some(block) = function.blocks.iter().find(|block| block.id == current) else {
                continue;
            };
            work.extend(successors(&block.terminator));
        }
        function
            .blocks
            .retain(|block| reachable.contains(&block.id));
    }
    compact_blocks(&mut program)?;
    compact_values(&mut program)?;
    finish(program)
}
