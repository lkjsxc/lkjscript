use super::graph::{push, visit, Graph, Observation};
use crate::{InstructionKind, ValueId};
use std::collections::HashSet;

pub(super) fn equivalent(
    graph: &Graph<'_>,
    left: ValueId,
    right: ValueId,
    observation: &mut Observation,
) -> crate::Result<bool> {
    let mut visited = HashSet::new();
    let mut pending = Vec::new();
    push(
        &mut pending,
        (left, right),
        "SSA active-variant equivalence worklist allocation failed",
    )?;
    while let Some((left, right)) = pending.pop() {
        observation.observe()?;
        if left == right {
            continue;
        }
        if !visit(
            &mut visited,
            (left, right),
            "SSA active-variant equivalence visited allocation failed",
        )? {
            return Ok(false);
        }
        if let Some((block, index)) = graph.parameter(left) {
            let predecessors = graph.predecessors(block)?;
            if !predecessors.is_empty() {
                for predecessor in predecessors.iter().rev() {
                    push(
                        &mut pending,
                        (graph.edge_argument(*predecessor, block, index)?, right),
                        "SSA active-variant equivalence worklist allocation failed",
                    )?;
                }
                continue;
            }
        }
        if let Some((block, index)) = graph.parameter(right) {
            let predecessors = graph.predecessors(block)?;
            if !predecessors.is_empty() {
                for predecessor in predecessors.iter().rev() {
                    push(
                        &mut pending,
                        (left, graph.edge_argument(*predecessor, block, index)?),
                        "SSA active-variant equivalence worklist allocation failed",
                    )?;
                }
                continue;
            }
        }
        let left_kind = graph.instruction(left).map(|item| &item.kind);
        let right_kind = graph.instruction(right).map(|item| &item.kind);
        let next = match (left_kind, right_kind) {
            (
                Some(InstructionKind::Copy(value) | InstructionKind::StructuralCopy { value, .. }),
                _,
            ) => Some((*value, right)),
            (
                _,
                Some(InstructionKind::Copy(value) | InstructionKind::StructuralCopy { value, .. }),
            ) => Some((left, *value)),
            (
                Some(InstructionKind::ProductField {
                    product: left_product,
                    field: left_field,
                    value: left_value,
                }),
                Some(InstructionKind::ProductField {
                    product: right_product,
                    field: right_field,
                    value: right_value,
                }),
            ) if left_product == right_product && left_field == right_field => {
                Some((*left_value, *right_value))
            }
            (
                Some(InstructionKind::EnumField {
                    enum_id: left_enum,
                    variant: left_variant,
                    field: left_field,
                    field_index: left_field_index,
                    layout: left_layout,
                    value: left_value,
                }),
                Some(InstructionKind::EnumField {
                    enum_id: right_enum,
                    variant: right_variant,
                    field: right_field,
                    field_index: right_field_index,
                    layout: right_layout,
                    value: right_value,
                }),
            ) if left_enum == right_enum
                && left_variant == right_variant
                && left_field == right_field
                && left_field_index == right_field_index
                && left_layout == right_layout =>
            {
                Some((*left_value, *right_value))
            }
            _ => None,
        };
        let Some(next) = next else {
            return Ok(false);
        };
        push(
            &mut pending,
            next,
            "SSA active-variant equivalence worklist allocation failed",
        )?;
    }
    Ok(true)
}
