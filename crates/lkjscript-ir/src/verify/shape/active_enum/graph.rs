use std::collections::HashSet;
use std::hash::Hash;

use crate::verify::*;
use crate::{Block, BlockId, Function, Instruction, IrError, ValueId};

#[derive(Clone, Copy, Debug)]
enum ValueLocation {
    Parameter { block: BlockId, index: usize },
    Instruction { block: BlockId, index: usize },
}

pub(crate) struct Graph<'a> {
    function: &'a Function,
    cfg: &'a ControlFlowGraph,
    values: Vec<Option<ValueLocation>>,
}

#[derive(Default)]
pub(super) struct Observation {
    steps: u64,
}

impl<'a> Graph<'a> {
    pub(crate) fn build(
        function: &'a Function,
        cfg: &'a ControlFlowGraph,
        value_count: usize,
    ) -> crate::Result<Self> {
        let mut values = Vec::new();
        values
            .try_reserve_exact(value_count)
            .map_err(|_| IrError::new("SSA active-variant value index allocation failed"))?;
        values.resize(value_count, None);
        for block in &function.blocks {
            for (index, parameter) in block.parameters.iter().enumerate() {
                let slot = parameter
                    .id
                    .index()
                    .and_then(|index| values.get_mut(index))
                    .ok_or_else(|| {
                        IrError::new("SSA active-variant parameter index is inconsistent")
                    })?;
                *slot = Some(ValueLocation::Parameter {
                    block: block.id,
                    index,
                });
            }
            for (index, instruction) in block.instructions.iter().enumerate() {
                let slot = instruction
                    .id
                    .index()
                    .and_then(|index| values.get_mut(index))
                    .ok_or_else(|| {
                        IrError::new("SSA active-variant instruction index is inconsistent")
                    })?;
                *slot = Some(ValueLocation::Instruction {
                    block: block.id,
                    index,
                });
            }
        }
        if values.iter().any(Option::is_none) {
            return fail("SSA active-variant value index is incomplete");
        }
        Ok(Self {
            function,
            cfg,
            values,
        })
    }

    pub(super) fn instruction(&self, value: ValueId) -> Option<&'a Instruction> {
        let ValueLocation::Instruction { block, index } =
            self.values.get(value.index()?).and_then(|item| *item)?
        else {
            return None;
        };
        self.function
            .blocks
            .get(block.index()?)
            .and_then(|block| block.instructions.get(index))
    }

    pub(super) fn parameter(&self, value: ValueId) -> Option<(BlockId, usize)> {
        let ValueLocation::Parameter { block, index } =
            self.values.get(value.index()?).and_then(|item| *item)?
        else {
            return None;
        };
        Some((block, index))
    }

    pub(super) fn block(&self, block: BlockId) -> crate::Result<&'a Block> {
        block_by_id(self.function, block)
    }

    pub(super) fn predecessors(&self, block: BlockId) -> crate::Result<&[BlockId]> {
        self.cfg.predecessors(block)
    }

    pub(super) fn edge_argument(
        &self,
        predecessor: BlockId,
        target: BlockId,
        index: usize,
    ) -> crate::Result<ValueId> {
        let block = self.block(predecessor)?;
        let arguments = match &block.terminator {
            crate::Terminator::Branch {
                target: edge,
                arguments,
            } if *edge == target => arguments,
            crate::Terminator::ConditionalBranch {
                true_target,
                true_arguments,
                ..
            } if *true_target == target => true_arguments,
            crate::Terminator::ConditionalBranch {
                false_target,
                false_arguments,
                ..
            } if *false_target == target => false_arguments,
            _ => return fail("SSA active-variant predecessor edge is inconsistent"),
        };
        arguments
            .get(index)
            .copied()
            .ok_or_else(|| IrError::new("SSA active-variant edge argument is missing"))
    }
}

impl Observation {
    pub(super) fn observe(&mut self) -> crate::Result<()> {
        self.steps = self
            .steps
            .checked_add(1)
            .ok_or_else(|| IrError::new("SSA active-variant observation overflow"))?;
        Ok(())
    }
}

pub(super) fn visit<T>(
    visited: &mut HashSet<T>,
    value: T,
    allocation_error: &'static str,
) -> crate::Result<bool>
where
    T: Copy + Eq + Hash,
{
    if visited.contains(&value) {
        return Ok(false);
    }
    visited
        .try_reserve(1)
        .map_err(|_| IrError::new(allocation_error))?;
    Ok(visited.insert(value))
}

pub(super) fn push<T>(
    work: &mut Vec<T>,
    value: T,
    allocation_error: &'static str,
) -> crate::Result<()> {
    work.try_reserve(1)
        .map_err(|_| IrError::new(allocation_error))?;
    work.push(value);
    Ok(())
}
