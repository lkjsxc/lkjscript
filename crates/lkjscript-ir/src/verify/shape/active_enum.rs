mod boolean;
mod equivalence;
mod graph;
mod universal;

use crate::verify::*;
use crate::{Block, BlockId, Function, InstructionKind, RuntimeLayoutId, ValueId, VariantId};
use equivalence::equivalent;
use graph::{charge, edge_argument, find_instruction, incoming};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct ActiveVariant {
    enum_id: crate::EnumId,
    variant: VariantId,
    layout: RuntimeLayoutId,
}

pub(super) fn projection(
    function: &Function,
    block: &Block,
    instruction: &crate::Instruction,
) -> crate::Result<()> {
    let InstructionKind::EnumField {
        enum_id,
        variant,
        layout,
        value,
        ..
    } = instruction.kind
    else {
        return Ok(());
    };
    let active = ActiveVariant {
        enum_id,
        variant,
        layout,
    };
    let mut work = 0_usize;
    if matches!(
        find_instruction(function, value).map(|item| &item.kind),
        Some(InstructionKind::EnumValue { .. })
    ) && !universal::value(function, value, active, &mut HashSet::new(), &mut work)?
    {
        return fail("SSA inactive enum projection rejected before access");
    }
    let mut visited = HashSet::new();
    if active_at_entry(function, block.id, value, active, &mut visited, &mut work)?
        || universal::value(function, value, active, &mut HashSet::new(), &mut work)?
    {
        Ok(())
    } else {
        fail(format!(
            concat!(
                "SSA enum projection lacks verified active-variant provenance: ",
                "block {:?}, value {:?}, enum {:?}, variant {:?}"
            ),
            block.id, value, active.enum_id, active.variant
        ))
    }
}

fn active_at_entry(
    function: &Function,
    block: BlockId,
    value: ValueId,
    active: ActiveVariant,
    visited: &mut HashSet<(BlockId, ValueId, ActiveVariant)>,
    work: &mut usize,
) -> crate::Result<bool> {
    charge(work)?;
    if !visited.insert((block, value, active)) {
        return Ok(false);
    }
    let current = block_by_id(function, block)?;
    if let Some(index) = current
        .parameters
        .iter()
        .position(|parameter| parameter.id == value)
    {
        let predecessors = incoming(function, block);
        if predecessors.is_empty() {
            return Ok(false);
        }
        for predecessor in predecessors {
            let argument = edge_argument(predecessor, block, index)?;
            if !active_on_edge(
                function,
                predecessor,
                block,
                argument,
                active,
                visited,
                work,
            )? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    if universal::value(function, value, active, &mut HashSet::new(), work)? {
        return Ok(true);
    }
    let predecessors = incoming(function, block);
    if predecessors.is_empty() {
        return Ok(false);
    }
    for predecessor in predecessors {
        if !active_on_edge(function, predecessor, block, value, active, visited, work)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn active_on_edge(
    function: &Function,
    predecessor: &Block,
    target: BlockId,
    value: ValueId,
    active: ActiveVariant,
    visited: &mut HashSet<(BlockId, ValueId, ActiveVariant)>,
    work: &mut usize,
) -> crate::Result<bool> {
    charge(work)?;
    if let crate::Terminator::ConditionalBranch {
        condition,
        true_target,
        ..
    } = predecessor.terminator
    {
        if true_target == target {
            if let Some(test) = find_instruction(function, condition) {
                if let InstructionKind::EnumIsVariant {
                    enum_id,
                    variant,
                    layout,
                    value: tested,
                } = test.kind
                {
                    if (ActiveVariant {
                        enum_id,
                        variant,
                        layout,
                    }) == active
                        && equivalent(function, tested, value, &mut HashSet::new(), work)?
                    {
                        return Ok(true);
                    }
                }
            }
            if let Some(possible) =
                boolean::true_predecessors(function, predecessor, condition, value, work)?
            {
                if possible.is_empty() {
                    return Ok(false);
                }
                for (source, argument) in possible {
                    if !active_on_edge(
                        function,
                        source,
                        predecessor.id,
                        argument,
                        active,
                        visited,
                        work,
                    )? {
                        return Ok(false);
                    }
                }
                return Ok(true);
            }
        }
    }
    active_at_entry(function, predecessor.id, value, active, visited, work)
}
