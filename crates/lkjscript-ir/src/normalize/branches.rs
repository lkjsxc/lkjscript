use std::collections::{HashMap, HashSet};

use crate::normalize::*;
use crate::{Constant, InstructionKind, Terminator, ValueId, VerifiedProgram};

pub fn simplify_branches(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        let constants: HashMap<ValueId, bool> = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(|instruction| match instruction.kind {
                InstructionKind::Constant(Constant::Bool(value)) => Some((instruction.id, value)),
                _ => None,
            })
            .collect();
        for block in &mut function.blocks {
            let replacement = match &block.terminator {
                Terminator::ConditionalBranch {
                    condition,
                    true_target,
                    true_arguments,
                    false_target,
                    false_arguments,
                } => constants.get(condition).map(|condition| {
                    if *condition {
                        Terminator::Branch {
                            target: *true_target,
                            arguments: true_arguments.clone(),
                        }
                    } else {
                        Terminator::Branch {
                            target: *false_target,
                            arguments: false_arguments.clone(),
                        }
                    }
                }),
                _ => None,
            };
            if let Some(replacement) = replacement {
                block.terminator = replacement;
            }
        }
        // Verification runs after every isolated pass. Remove blocks made
        // unreachable by a proven constant edge here so they cannot retain
        // stale cross-block ownership/value uses until the later dedicated
        // unreachable-block pass.
        let mut reachable = HashSet::new();
        let mut work = vec![function.entry];
        while let Some(current) = work.pop() {
            if !reachable.insert(current) {
                continue;
            }
            if let Some(block) = function.blocks.iter().find(|block| block.id == current) {
                work.extend(successors(&block.terminator));
            }
        }
        function
            .blocks
            .retain(|block| reachable.contains(&block.id));
        clear_stale_loop_headers(function);
    }
    compact_blocks(&mut program)?;
    compact_values(&mut program)?;
    finish(program)
}
