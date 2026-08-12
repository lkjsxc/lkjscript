use super::graph::{push, visit, Graph, Observation};
use crate::{BlockId, InstructionKind, ValueId};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct TruePredecessor {
    pub(super) predecessor: BlockId,
    pub(super) target: BlockId,
    pub(super) value: ValueId,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct TraceTask {
    block: BlockId,
    condition: ValueId,
    value: ValueId,
    incoming: Option<TruePredecessor>,
}

pub(super) fn true_predecessors(
    graph: &Graph<'_>,
    block: BlockId,
    condition: ValueId,
    value: ValueId,
    observation: &mut Observation,
) -> crate::Result<Option<Vec<TruePredecessor>>> {
    let mut visited = HashSet::new();
    let mut pending = Vec::new();
    let mut possible = Vec::new();
    push(
        &mut pending,
        TraceTask {
            block,
            condition,
            value,
            incoming: None,
        },
        "SSA active-variant boolean predecessor worklist allocation failed",
    )?;
    while let Some(task) = pending.pop() {
        observation.observe()?;
        if !visit(
            &mut visited,
            task,
            "SSA active-variant boolean predecessor visited allocation failed",
        )? {
            // A cyclic Boolean merge cannot be refined safely by this local proof.
            return Ok(None);
        }
        if let Some(InstructionKind::Copy(source)) =
            graph.instruction(task.condition).map(|item| &item.kind)
        {
            push(
                &mut pending,
                TraceTask {
                    condition: *source,
                    ..task
                },
                "SSA active-variant boolean predecessor worklist allocation failed",
            )?;
            continue;
        }
        let Some((condition_block, condition_index)) = graph.parameter(task.condition) else {
            if definitely_false(graph, task.condition, observation)? {
                continue;
            }
            let Some(incoming) = task.incoming else {
                return Ok(None);
            };
            push(
                &mut possible,
                incoming,
                "SSA active-variant boolean predecessor allocation failed",
            )?;
            continue;
        };
        if condition_block != task.block {
            let Some(incoming) = task.incoming else {
                return Ok(None);
            };
            push(
                &mut possible,
                incoming,
                "SSA active-variant boolean predecessor allocation failed",
            )?;
            continue;
        }
        let value_index = graph
            .parameter(task.value)
            .filter(|(value_block, _)| *value_block == task.block)
            .map(|(_, index)| index);
        let predecessors = graph.predecessors(task.block)?;
        if predecessors.is_empty() {
            let Some(incoming) = task.incoming else {
                return Ok(None);
            };
            push(
                &mut possible,
                incoming,
                "SSA active-variant boolean predecessor allocation failed",
            )?;
            continue;
        }
        for predecessor in predecessors.iter().rev() {
            let condition = graph.edge_argument(*predecessor, task.block, condition_index)?;
            if definitely_false(graph, condition, observation)? {
                continue;
            }
            let value = match value_index {
                Some(index) => graph.edge_argument(*predecessor, task.block, index)?,
                None => task.value,
            };
            push(
                &mut pending,
                TraceTask {
                    block: *predecessor,
                    condition,
                    value,
                    incoming: Some(TruePredecessor {
                        predecessor: *predecessor,
                        target: task.block,
                        value,
                    }),
                },
                "SSA active-variant boolean predecessor worklist allocation failed",
            )?;
        }
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
