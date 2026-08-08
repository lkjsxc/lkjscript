use std::collections::VecDeque;

use super::{
    instruction::apply_instruction, merge::merge_state, Kind, OwnerIdentity, ParameterOwnerKind,
    State, UniquePlaceState,
};
use crate::{Chunk, DecodedInstruction, Error, FunctionProto, Result};

mod blocks;

use blocks::ControlFlowGraph;

pub(super) fn validate_control_flow(
    chunk: &Chunk,
    proto: &FunctionProto,
    instructions: &[DecodedInstruction],
    is_main: bool,
) -> Result<()> {
    let graph = ControlFlowGraph::build(proto, instructions)?;
    for range in &proto.failure_cleanup_ranges {
        let start = usize::try_from(range.start)
            .map_err(|_| Error::msg("bytecode failure-cleanup start exceeds host usize"))?;
        let end = usize::try_from(range.end)
            .map_err(|_| Error::msg("bytecode failure-cleanup end exceeds host usize"))?;
        if !graph.is_instruction_boundary(instructions, start)
            || (end != proto.code.len() && !graph.is_instruction_boundary(instructions, end))
        {
            return Err(Error::msg(
                "bytecode failure-cleanup range is not instruction aligned",
            ));
        }
    }

    let locals = initial_locals(chunk, proto, is_main)?;
    let globals = if is_main {
        vec![None; chunk.global_names.len()]
    } else if chunk.global_prototypes.is_empty() {
        vec![Some(Kind::Any); chunk.global_names.len()]
    } else {
        chunk
            .global_prototypes
            .iter()
            .map(|prototype| prototype.map_or(Some(Kind::Any), |index| Some(Kind::Closure(index))))
            .collect()
    };
    let mut unique_places = Vec::new();
    unique_places
        .try_reserve_exact(proto.unique_places)
        .map_err(|_| Error::host("bytecode unique-place state reservation failed"))?;
    unique_places.resize(proto.unique_places, UniquePlaceState::Inactive);
    for (index, place) in proto.parameter_unique_places.iter().copied().enumerate() {
        let Some(place) = place else {
            continue;
        };
        let owner = match locals.get(index).copied().flatten() {
            Some(Kind::Bytes(owner) | Kind::ByteVector(owner)) => owner,
            _ => {
                return Err(Error::msg(
                    "bytecode parameter owner-place metadata requires exact unique owner type",
                ))
            }
        };
        let target = unique_places
            .get_mut(place)
            .ok_or_else(|| Error::msg("bytecode parameter owner PlaceId is out of range"))?;
        if !matches!(target, UniquePlaceState::Inactive) {
            return Err(Error::msg("bytecode parameter owner PlaceId is duplicated"));
        }
        *target = UniquePlaceState::Active {
            owner: Some(owner),
            transferred: None,
        };
    }
    for (index, place) in proto
        .parameter_structural_places
        .iter()
        .copied()
        .enumerate()
    {
        let Some(place) = place else {
            continue;
        };
        let owner = match locals.get(index).copied().flatten() {
            Some(Kind::StructuralOwner { owner, .. }) => owner,
            _ => {
                return Err(Error::msg(
                    "bytecode structural parameter place requires an exact owner",
                ))
            }
        };
        let target = unique_places
            .get_mut(place)
            .ok_or_else(|| Error::msg("bytecode structural parameter PlaceId is out of range"))?;
        if !matches!(target, UniquePlaceState::Inactive) {
            return Err(Error::msg(
                "bytecode structural parameter PlaceId is duplicated",
            ));
        }
        *target = UniquePlaceState::Active {
            owner: Some(owner),
            transferred: None,
        };
    }

    let mut states = Vec::new();
    states
        .try_reserve_exact(graph.len())
        .map_err(|_| Error::host("bytecode block-entry state reservation failed"))?;
    states.resize_with(graph.len(), || None);
    states[0] = Some(State::new(
        proto,
        Vec::new(),
        locals,
        globals,
        unique_places,
    ));

    let mut pending = VecDeque::new();
    pending
        .try_reserve(graph.len())
        .map_err(|_| Error::host("bytecode block worklist reservation failed"))?;
    pending.push_back(0_usize);
    let mut queued = Vec::new();
    queued
        .try_reserve_exact(graph.len())
        .map_err(|_| Error::host("bytecode block worklist-state reservation failed"))?;
    queued.resize(graph.len(), false);
    queued[0] = true;

    while let Some(block_index) = pending.pop_front() {
        queued[block_index] = false;
        let block = graph
            .block(block_index)
            .ok_or_else(|| Error::msg("validator CFG block index out of range"))?;
        let mut state = states
            .get(block_index)
            .and_then(Clone::clone)
            .ok_or_else(|| Error::msg("validator CFG block-entry state is missing"))?;
        let first = instructions
            .get(block.start)
            .copied()
            .ok_or_else(|| Error::msg("validator CFG block start is out of range"))?;
        let mut cleanup_ranges = super::failure_cleanup::RangeCursor::new(proto, first.offset())?;
        for instruction in instructions
            .get(block.start..block.end)
            .ok_or_else(|| Error::msg("validator CFG block range is out of bounds"))?
            .iter()
            .copied()
        {
            super::failure_cleanup::validate_at_offset(
                proto,
                instruction.offset(),
                &state,
                &mut cleanup_ranges,
            )?;
            apply_instruction(chunk, proto, instruction, &mut state, is_main)?;
        }
        #[cfg(debug_assertions)]
        debug_assert!(state.cleanup_requirement_is_consistent(proto));

        let predecessor = instructions
            .get(block.end - 1)
            .copied()
            .ok_or_else(|| Error::msg("validator CFG block terminator is missing"))?;
        for successor in graph
            .successors(proto, instructions, block, predecessor)?
            .into_iter()
            .flatten()
        {
            let target = states
                .get_mut(successor)
                .ok_or_else(|| Error::msg("validator CFG successor block is out of range"))?;
            if merge_state(target, &state, proto, predecessor)? && !queued[successor] {
                pending.push_back(successor);
                queued[successor] = true;
            }
        }
    }
    Ok(())
}

include!("parameters.rs");
