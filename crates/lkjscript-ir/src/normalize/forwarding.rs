use std::collections::HashMap;

use crate::normalize::*;
use crate::{BlockId, Terminator, ValueId, VerifiedProgram};

pub fn empty_block_forwarding(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        loop {
            let candidate = function.blocks.iter().find_map(|block| {
                let Terminator::Branch { target, arguments } = &block.terminator else {
                    return None;
                };
                if block.id == function.entry
                    || block.metadata.loop_header
                    || !block.instructions.is_empty()
                    || target == &block.id
                    || arguments.len() != block.parameters.len()
                    || !arguments.iter().all(|argument| {
                        block
                            .parameters
                            .iter()
                            .any(|parameter| parameter.id == *argument)
                    })
                {
                    return None;
                }
                Some((
                    block.id,
                    *target,
                    block.parameters.clone(),
                    arguments.clone(),
                ))
            });
            let Some((candidate, target, parameters, outgoing)) = candidate else {
                break;
            };
            let mut changed = false;
            for block in &mut function.blocks {
                changed |= forward_terminator(
                    &mut block.terminator,
                    candidate,
                    target,
                    &parameters,
                    &outgoing,
                );
            }
            if !changed {
                break;
            }
            function.blocks.retain(|block| block.id != candidate);
        }
    }
    compact_blocks(&mut program)?;
    compact_values(&mut program)?;
    finish(program)
}

pub(crate) fn forward_terminator(
    terminator: &mut Terminator,
    candidate: BlockId,
    target: BlockId,
    parameters: &[crate::BlockParameter],
    outgoing: &[ValueId],
) -> bool {
    match terminator {
        Terminator::Branch {
            target: edge_target,
            arguments,
        } if *edge_target == candidate => {
            let incoming = arguments.clone();
            *edge_target = target;
            *arguments = substitute_edge(parameters, outgoing, &incoming);
            true
        }
        Terminator::ConditionalBranch {
            true_target,
            true_arguments,
            false_target,
            false_arguments,
            ..
        } => {
            let mut changed = false;
            if *true_target == candidate {
                let incoming = true_arguments.clone();
                *true_target = target;
                *true_arguments = substitute_edge(parameters, outgoing, &incoming);
                changed = true;
            }
            if *false_target == candidate {
                let incoming = false_arguments.clone();
                *false_target = target;
                *false_arguments = substitute_edge(parameters, outgoing, &incoming);
                changed = true;
            }
            changed
        }
        _ => false,
    }
}

pub(crate) fn substitute_edge(
    parameters: &[crate::BlockParameter],
    outgoing: &[ValueId],
    incoming: &[ValueId],
) -> Vec<ValueId> {
    let substitutions: HashMap<ValueId, ValueId> = parameters
        .iter()
        .zip(incoming)
        .map(|(parameter, incoming)| (parameter.id, *incoming))
        .collect();
    outgoing
        .iter()
        .map(|value| substitutions.get(value).copied().unwrap_or(*value))
        .collect()
}
