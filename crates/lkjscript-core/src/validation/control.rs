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
    let mut states = vec![None; instructions.len()];
    let mut locals = vec![None; usize::from(proto.locals)];
    for (index, slot) in locals.iter_mut().take(usize::from(proto.arity)).enumerate() {
        let resource = proto
            .parameter_resources
            .get(index)
            .copied()
            .flatten()
            .map(Kind::Resource);
        let unique = proto
            .parameter_uniques
            .get(index)
            .copied()
            .flatten()
            .map(|kind| match kind {
                crate::UniqueValueKind::Bytes => {
                    Kind::Bytes(0x8000_0000 | u32::try_from(index).unwrap_or(u32::MAX))
                }
                crate::UniqueValueKind::ByteVector => {
                    Kind::ByteVector(0x8000_0000 | u32::try_from(index).unwrap_or(u32::MAX))
                }
                crate::UniqueValueKind::ByteSlice => Kind::ByteSlice {
                    owner: 0x9000_0000 | u32::try_from(index).unwrap_or(u32::MAX),
                    mutable: false,
                    used: false,
                },
                crate::UniqueValueKind::ByteSliceMut => Kind::ByteSlice {
                    owner: 0x9000_0000 | u32::try_from(index).unwrap_or(u32::MAX),
                    mutable: true,
                    used: false,
                },
            });
        *slot = resource.or(unique).or(Some(Kind::Any));
    }
    if is_main {
        for (slot, kind) in locals.iter_mut().zip(&chunk.required_capabilities) {
            *slot = Some(Kind::Capability(*kind));
        }
    }
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
    let mut unique_places = vec![UniquePlaceState::Inactive; usize::from(proto.unique_places)];
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
            .get_mut(usize::from(place))
            .ok_or_else(|| Error::msg("bytecode parameter owner PlaceId is out of range"))?;
        if !matches!(target, UniquePlaceState::Inactive) {
            return Err(Error::msg("bytecode parameter owner PlaceId is duplicated"));
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

fn successors(
    proto: &FunctionProto,
    instructions: &[DecodedInstruction],
    by_offset: &HashMap<usize, usize>,
    index: usize,
    instruction: DecodedInstruction,
) -> Result<Vec<usize>> {
    let target = || -> Result<usize> {
        let offset = instruction.operand().map(usize::from).ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "missing jump target",
            )
        })?;
        by_offset.get(&offset).copied().ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "jump target is not an instruction boundary",
            )
        })
    };
    match instruction.op().info().control {
        crate::ControlFlow::Return | crate::ControlFlow::Exit | crate::ControlFlow::Trap => {
            Ok(Vec::new())
        }
        crate::ControlFlow::Jump => Ok(vec![target()?]),
        crate::ControlFlow::Branch => {
            let next = index
                .checked_add(1)
                .filter(|next| *next < instructions.len())
                .ok_or_else(|| {
                    instruction_error(
                        proto,
                        instruction.op(),
                        instruction.offset(),
                        "reachable branch falls through the end of the function",
                    )
                })?;
            Ok(vec![target()?, next])
        }
        crate::ControlFlow::Next => {
            let next = index
                .checked_add(1)
                .filter(|next| *next < instructions.len())
                .ok_or_else(|| {
                    instruction_error(
                        proto,
                        instruction.op(),
                        instruction.offset(),
                        "reachable execution falls through the end of the function",
                    )
                })?;
            Ok(vec![next])
        }
    }
}
