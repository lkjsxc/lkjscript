use std::collections::{HashMap, VecDeque};

use super::{
    decode::instruction_error, instruction::apply_instruction, merge::merge_state, Kind, State,
    UniquePlaceState,
};
use crate::{Chunk, DecodedInstruction, Error, FunctionProto, Result};

pub(super) fn validate_control_flow(
    chunk: &Chunk,
    proto: &FunctionProto,
    instructions: &[DecodedInstruction],
    is_main: bool,
) -> Result<()> {
    let by_offset: HashMap<usize, usize> = instructions
        .iter()
        .enumerate()
        .map(|(index, instruction)| (instruction.offset(), index))
        .collect();
    for range in &proto.failure_cleanup_ranges {
        let start = usize::from(range.start);
        let end = usize::from(range.end);
        if !by_offset.contains_key(&start)
            || (end != proto.code.len() && !by_offset.contains_key(&end))
        {
            return Err(Error::msg(
                "bytecode failure-cleanup range is not instruction aligned",
            ));
        }
    }
    let mut states = vec![None; instructions.len()];
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
    states[0] = Some(State {
        stack: Vec::new(),
        locals,
        globals,
        unique_places,
        structural_destinations: std::collections::BTreeMap::new(),
    });
    let mut pending = VecDeque::from([0_usize]);

    while let Some(index) = pending.pop_front() {
        let instruction = instructions
            .get(index)
            .copied()
            .ok_or_else(|| Error::msg("validator CFG instruction index out of range"))?;
        let mut state = states
            .get(index)
            .and_then(Clone::clone)
            .ok_or_else(|| Error::msg("validator CFG state is missing"))?;
        super::failure_cleanup::validate_at_offset(proto, instruction.offset(), &state)?;
        apply_instruction(chunk, proto, instruction, &mut state, is_main)?;

        let successors = successors(proto, instructions, &by_offset, index, instruction)?;
        for successor in successors {
            let target = states
                .get_mut(successor)
                .ok_or_else(|| Error::msg("validator CFG successor index out of range"))?;
            let changed = merge_state(target, &state, proto, instruction)?;
            if changed {
                pending.push_back(successor);
            }
        }
    }
    Ok(())
}

include!("successors.rs");
include!("parameters.rs");
