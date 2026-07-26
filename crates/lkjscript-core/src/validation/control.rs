use std::collections::{HashMap, VecDeque};

use super::{
    decode::instruction_error, instruction::apply_instruction, merge::merge_state, Kind, State,
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
    for slot in locals.iter_mut().take(usize::from(proto.arity)) {
        *slot = Some(Kind::Any);
    }
    if is_main {
        for (slot, kind) in locals.iter_mut().zip(&chunk.required_capabilities) {
            *slot = Some(Kind::Capability(*kind));
        }
    }
    let globals = if is_main {
        vec![None; chunk.global_names.len()]
    } else {
        vec![Some(Kind::Any); chunk.global_names.len()]
    };
    states[0] = Some(State {
        stack: Vec::new(),
        locals,
        globals,
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
        apply_instruction(chunk, proto, instruction, &mut state)?;

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
