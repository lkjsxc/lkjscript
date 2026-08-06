use super::graph::{charge, edge_argument, find_instruction, incoming};
use crate::{Function, InstructionKind, ValueId};
use std::collections::HashSet;

pub(super) fn equivalent(
    function: &Function,
    left: ValueId,
    right: ValueId,
    visited: &mut HashSet<(ValueId, ValueId)>,
    work: &mut usize,
) -> crate::Result<bool> {
    charge(work)?;
    if left == right {
        return Ok(true);
    }
    if !visited.insert((left, right)) {
        return Ok(false);
    }
    if let Some(arguments) = parameter_arguments(function, left)? {
        for argument in arguments {
            if !equivalent(function, argument, right, visited, work)? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    if let Some(arguments) = parameter_arguments(function, right)? {
        for argument in arguments {
            if !equivalent(function, left, argument, visited, work)? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    let left_kind = find_instruction(function, left).map(|item| &item.kind);
    let right_kind = find_instruction(function, right).map(|item| &item.kind);
    match (left_kind, right_kind) {
        (Some(InstructionKind::Copy(value) | InstructionKind::StructuralCopy { value, .. }), _) => {
            equivalent(function, *value, right, visited, work)
        }
        (_, Some(InstructionKind::Copy(value) | InstructionKind::StructuralCopy { value, .. })) => {
            equivalent(function, left, *value, visited, work)
        }
        (
            Some(InstructionKind::ProductField {
                product: lp,
                field: lf,
                value: lv,
            }),
            Some(InstructionKind::ProductField {
                product: rp,
                field: rf,
                value: rv,
            }),
        ) if lp == rp && lf == rf => equivalent(function, *lv, *rv, visited, work),
        (
            Some(InstructionKind::EnumField {
                enum_id: le,
                variant: lv,
                field: lf,
                field_index: lfi,
                layout: ll,
                value: li,
            }),
            Some(InstructionKind::EnumField {
                enum_id: re,
                variant: rv,
                field: rf,
                field_index: rfi,
                layout: rl,
                value: ri,
            }),
        ) if le == re && lv == rv && lf == rf && lfi == rfi && ll == rl => {
            equivalent(function, *li, *ri, visited, work)
        }
        _ => Ok(false),
    }
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
            return predecessors
                .into_iter()
                .map(|predecessor| edge_argument(predecessor, block.id, index))
                .collect::<crate::Result<Vec<_>>>()
                .map(Some);
        }
    }
    Ok(None)
}
