use super::graph::{charge, find_instruction};
use super::ActiveVariant;
use crate::{Function, InstructionKind, ValueId};
use std::collections::HashSet;

pub(super) fn value(
    function: &Function,
    candidate: ValueId,
    active: ActiveVariant,
    visited: &mut HashSet<ValueId>,
    work: &mut usize,
) -> crate::Result<bool> {
    charge(work)?;
    if !visited.insert(candidate) {
        return Ok(false);
    }
    let Some(definition) = find_instruction(function, candidate) else {
        return Ok(false);
    };
    Ok(match definition.kind {
        InstructionKind::EnumValue {
            enum_id,
            variant,
            layout,
            ..
        } => {
            ActiveVariant {
                enum_id,
                variant,
                layout,
            } == active
        }
        InstructionKind::Copy(source) => value(function, source, active, visited, work)?,
        _ => false,
    })
}
