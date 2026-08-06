use super::graph::{push, visit, Graph, Observation};
use crate::{BlockId, InstructionKind, ValueId};
use std::collections::HashSet;

pub(super) fn true_predecessors(
    graph: &Graph<'_>,
    block: BlockId,
    condition: ValueId,
    value: ValueId,
    observation: &mut Observation,
) -> crate::Result<Option<Vec<(BlockId, ValueId)>>> {
    let Some((condition_block, condition_index)) = graph.parameter(condition) else {
        return Ok(None);
    };
    if condition_block != block {
        return Ok(None);
    }
    let value_index = graph
        .parameter(value)
        .filter(|(value_block, _)| *value_block == block)
        .map(|(_, index)| index);
    let predecessors = graph.predecessors(block)?;
    let mut possible = Vec::new();
    possible
        .try_reserve_exact(predecessors.len())
        .map_err(|_| {
            crate::IrError::new("SSA active-variant boolean predecessor allocation failed")
        })?;
    for predecessor in predecessors {
        let condition_argument = graph.edge_argument(*predecessor, block, condition_index)?;
        if definitely_false(graph, condition_argument, observation)? {
            continue;
        }
        let argument = match value_index {
            Some(value_index) => graph.edge_argument(*predecessor, block, value_index)?,
            None => value,
        };
        possible.push((*predecessor, argument));
    }
    Ok(Some(possible))
}

fn definitely_false(
    graph: &Graph<'_>,
    value: ValueId,
    observation: &mut Observation,
) -> crate::Result<bool> {
    let mut visited = HashSet::new();
    let mut pending = Vec::new();
    push(
        &mut pending,
        value,
        "SSA active-variant boolean worklist allocation failed",
    )?;
    while let Some(value) = pending.pop() {
        observation.observe()?;
        if !visit(
            &mut visited,
            value,
            "SSA active-variant boolean visited allocation failed",
        )? {
            return Ok(false);
        }
        if let Some((block, index)) = graph.parameter(value) {
            let predecessors = graph.predecessors(block)?;
            if !predecessors.is_empty() {
                for predecessor in predecessors.iter().rev() {
                    push(
                        &mut pending,
                        graph.edge_argument(*predecessor, block, index)?,
                        "SSA active-variant boolean worklist allocation failed",
                    )?;
                }
                continue;
            }
        }
        match graph.instruction(value).map(|item| &item.kind) {
            Some(InstructionKind::Constant(crate::Constant::Bool(false))) => {}
            Some(InstructionKind::Copy(source)) => push(
                &mut pending,
                *source,
                "SSA active-variant boolean worklist allocation failed",
            )?,
            _ => return Ok(false),
        }
    }
    Ok(true)
}
