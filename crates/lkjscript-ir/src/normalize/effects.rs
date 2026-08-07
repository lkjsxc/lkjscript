use std::collections::{HashMap, HashSet};

use crate::normalize::*;
use crate::{
    EffectSet, FailureBehavior, FrameState, Instruction, InstructionKind, ValueId, VerifiedProgram,
};

pub fn effect_aware_dce(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        let definitions: HashMap<ValueId, Vec<ValueId>> = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .map(|instruction| (instruction.id, instruction.kind.operands()))
            .collect();
        let mut live = HashSet::new();
        for block in &function.blocks {
            live.extend(block.terminator.operands());
            if let Some(frame) = &block.metadata.frame_state {
                add_frame_values(&mut live, frame);
            }
            for instruction in &block.instructions {
                if must_retain_instruction(instruction) {
                    live.insert(instruction.id);
                }
                if let Some(frame) = &instruction.metadata.frame_state {
                    add_frame_values(&mut live, frame);
                }
            }
        }
        let mut pending: Vec<ValueId> = live.iter().copied().collect();
        while let Some(value) = pending.pop() {
            if let Some(operands) = definitions.get(&value) {
                for operand in operands {
                    if live.insert(*operand) {
                        pending.push(*operand);
                    }
                }
            }
        }
        for block in &mut function.blocks {
            block.instructions.retain(|instruction| {
                live.contains(&instruction.id) || must_retain_instruction(instruction)
            });
        }
    }
    compact_values(&mut program)?;
    finish(program)
}

fn must_retain_instruction(instruction: &Instruction) -> bool {
    !instruction.metadata.effects.is_pure()
        || instruction.metadata.frame_state.is_some()
        || matches!(
            instruction.kind,
            InstructionKind::Call { .. }
                | InstructionKind::PlaceInit { .. }
                | InstructionKind::PlaceEnd { .. }
                | InstructionKind::EndBorrow { .. }
                | InstructionKind::Drop { .. }
                | InstructionKind::Move { .. }
                | InstructionKind::Borrow { .. }
                | InstructionKind::MemoryWitnessIndependentOwner { .. }
                | InstructionKind::MemoryWitnessCompare { .. }
                | InstructionKind::MemoryWitnessDispose { .. }
        )
}

pub(crate) fn add_frame_values(live: &mut HashSet<ValueId>, frame: &FrameState) {
    live.extend(frame.locals.iter().map(|local| local.value));
    live.extend(frame.operand_stack.iter().copied());
}

pub(crate) fn failure_behavior(effects: EffectSet) -> FailureBehavior {
    match (
        effects.contains(EffectSet::MAY_TRAP),
        effects.contains(EffectSet::MAY_EXIT) || effects.contains(EffectSet::ALLOCATES),
    ) {
        (false, false) => FailureBehavior::None,
        (true, false) => FailureBehavior::Trap,
        (false, true) => FailureBehavior::StructuredOutcome,
        (true, true) => FailureBehavior::TrapOrOutcome,
    }
}
