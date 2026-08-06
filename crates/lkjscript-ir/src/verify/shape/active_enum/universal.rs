use super::graph::{visit, Graph, Observation};
use super::ActiveVariant;
use crate::{InstructionKind, ValueId};
use std::collections::HashSet;

pub(super) fn value(
    graph: &Graph<'_>,
    mut candidate: ValueId,
    active: ActiveVariant,
    observation: &mut Observation,
) -> crate::Result<bool> {
    let mut visited = HashSet::new();
    loop {
        observation.observe()?;
        if !visit(
            &mut visited,
            candidate,
            "SSA active-variant universal visited allocation failed",
        )? {
            return Ok(false);
        }
        let Some(definition) = graph.instruction(candidate) else {
            return Ok(false);
        };
        match definition.kind {
            InstructionKind::EnumValue {
                enum_id,
                variant,
                layout,
                ..
            } => {
                return Ok(ActiveVariant {
                    enum_id,
                    variant,
                    layout,
                } == active);
            }
            InstructionKind::Copy(source) => candidate = source,
            _ => return Ok(false),
        }
    }
}
