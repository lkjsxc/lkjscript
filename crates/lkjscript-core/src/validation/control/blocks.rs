use super::super::decode::instruction_error;
use crate::{ControlFlow, DecodedInstruction, Error, FunctionProto, Result};

#[derive(Clone, Copy, Debug)]
pub(super) struct BasicBlock {
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) struct ControlFlowGraph {
    blocks: Vec<BasicBlock>,
}

impl ControlFlowGraph {
    pub(super) fn build(
        proto: &FunctionProto,
        instructions: &[DecodedInstruction],
    ) -> Result<Self> {
        if instructions.is_empty() {
            return Err(Error::msg(
                "validator cannot build an empty control-flow graph",
            ));
        }
        let mut leaders = Vec::new();
        leaders
            .try_reserve_exact(instructions.len())
            .map_err(|_| Error::host("bytecode basic-block leader reservation failed"))?;
        leaders.resize(instructions.len(), false);
        leaders[0] = true;

        for (index, instruction) in instructions.iter().copied().enumerate() {
            let control = instruction.op().info().control;
            if matches!(control, ControlFlow::Jump | ControlFlow::Branch) {
                leaders[target_instruction(proto, instructions, instruction)?] = true;
            }
            if control != ControlFlow::Next {
                if let Some(next) = index
                    .checked_add(1)
                    .filter(|next| *next < instructions.len())
                {
                    leaders[next] = true;
                }
            }
        }

        let mut blocks = Vec::new();
        let mut previous = None;
        for (index, leader) in leaders.into_iter().enumerate() {
            if !leader {
                continue;
            }
            if let Some(start) = previous.replace(index) {
                push_block(&mut blocks, BasicBlock { start, end: index })?;
            }
        }
        let start =
            previous.ok_or_else(|| Error::msg("bytecode control-flow graph has no entry"))?;
        push_block(
            &mut blocks,
            BasicBlock {
                start,
                end: instructions.len(),
            },
        )?;
        Ok(Self { blocks })
    }

    pub(super) fn len(&self) -> usize {
        self.blocks.len()
    }

    pub(super) fn block(&self, index: usize) -> Option<BasicBlock> {
        self.blocks.get(index).copied()
    }

    pub(super) fn is_instruction_boundary(
        &self,
        instructions: &[DecodedInstruction],
        offset: usize,
    ) -> bool {
        instructions
            .binary_search_by_key(&offset, |instruction| instruction.offset())
            .is_ok()
    }

    pub(super) fn successors(
        &self,
        proto: &FunctionProto,
        instructions: &[DecodedInstruction],
        block: BasicBlock,
        instruction: DecodedInstruction,
    ) -> Result<[Option<usize>; 2]> {
        let target = || -> Result<usize> {
            let instruction = target_instruction(proto, instructions, instruction)?;
            self.block_starting_at(instruction)
        };
        match instruction.op().info().control {
            ControlFlow::Return | ControlFlow::Exit | ControlFlow::Trap => Ok([None, None]),
            ControlFlow::Jump => Ok([Some(target()?), None]),
            ControlFlow::Branch => {
                let next = (block.end < instructions.len()).then_some(block.end);
                let next = next
                    .map(|next| self.block_starting_at(next))
                    .transpose()?
                    .ok_or_else(|| {
                        instruction_error(
                            proto,
                            instruction.op(),
                            instruction.offset(),
                            "reachable branch falls through the end of the function",
                        )
                    })?;
                Ok([Some(target()?), Some(next)])
            }
            ControlFlow::Next => {
                let next = (block.end < instructions.len())
                    .then_some(block.end)
                    .map(|next| self.block_starting_at(next))
                    .transpose()?
                    .ok_or_else(|| {
                        instruction_error(
                            proto,
                            instruction.op(),
                            instruction.offset(),
                            "reachable execution falls through the end of the function",
                        )
                    })?;
                Ok([Some(next), None])
            }
        }
    }

    fn block_starting_at(&self, instruction: usize) -> Result<usize> {
        self.blocks
            .binary_search_by_key(&instruction, |block| block.start)
            .map_err(|_| Error::msg("validator CFG target does not begin a basic block"))
    }
}

fn target_instruction(
    proto: &FunctionProto,
    instructions: &[DecodedInstruction],
    instruction: DecodedInstruction,
) -> Result<usize> {
    let offset = instruction.operand().index().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "missing jump target",
        )
    })?;
    instructions
        .binary_search_by_key(&offset, |candidate| candidate.offset())
        .map_err(|_| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "jump target is not an instruction boundary",
            )
        })
}

fn push_block(blocks: &mut Vec<BasicBlock>, block: BasicBlock) -> Result<()> {
    if blocks.len() == blocks.capacity() {
        blocks
            .try_reserve(1)
            .map_err(|_| Error::host("bytecode basic-block reservation failed"))?;
    }
    blocks.push(block);
    Ok(())
}
