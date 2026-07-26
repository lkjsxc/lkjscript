use super::graph::{charge, edge_argument, find_instruction, incoming};
use crate::{Block, Function, InstructionKind, ValueId};
use std::collections::HashSet;

pub(super) fn true_predecessors<'a>(
    function: &'a Function,
    block: &'a Block,
    condition: ValueId,
    value: ValueId,
    work: &mut usize,
) -> crate::Result<Option<Vec<(&'a Block, ValueId)>>> {
    let Some(index) = block
        .parameters
        .iter()
        .position(|parameter| parameter.id == condition)
    else {
        return Ok(None);
    };
    let value_index = block
        .parameters
        .iter()
        .position(|parameter| parameter.id == value);
    let mut possible = Vec::new();
    for predecessor in incoming(function, block.id) {
        let condition_argument = edge_argument(predecessor, block.id, index)?;
        if definitely_false(function, condition_argument, &mut HashSet::new(), work)? {
            continue;
        }
        let argument = match value_index {
            Some(value_index) => edge_argument(predecessor, block.id, value_index)?,
            None => value,
        };
        possible.push((predecessor, argument));
    }
    Ok(Some(possible))
}

fn definitely_false(
    function: &Function,
    value: ValueId,
    visited: &mut HashSet<ValueId>,
    work: &mut usize,
) -> crate::Result<bool> {
    charge(work)?;
    if !visited.insert(value) {
        return Ok(false);
    }
    if let Some(arguments) = parameter_arguments(function, value)? {
        for argument in arguments {
            if !definitely_false(function, argument, visited, work)? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    Ok(
        match find_instruction(function, value).map(|item| &item.kind) {
            Some(InstructionKind::Constant(crate::Constant::Bool(false))) => true,
            Some(InstructionKind::Copy(source)) => {
                definitely_false(function, *source, visited, work)?
            }
            _ => false,
        },
    )
}

fn parameter_arguments(function: &Function, value: ValueId) -> crate::Result<Option<Vec<ValueId>>> {
    for block in &function.blocks {
        if let Some(index) = block
            .parameters
            .iter()
            .position(|parameter| parameter.id == value)
        {
            let predecessors = incoming(function, block.id);
            if predecessors.is_empty() {
                return Ok(None);
            }
            let mut arguments = Vec::with_capacity(predecessors.len());
            for predecessor in predecessors {
                arguments.push(edge_argument(predecessor, block.id, index)?);
            }
            return Ok(Some(arguments));
        }
    }
    Ok(None)
}
