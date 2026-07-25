use std::collections::HashMap;

use crate::optimize::*;
use crate::{
    BlockId, Constant, Instruction, InstructionKind, Program, RuntimeOp, Signature, SsaType,
    ValueId,
};

#[derive(Clone, Copy)]
pub(crate) struct DiscoveryDefinition<'a> {
    pub(crate) block: BlockId,
    pub(crate) instruction_index: Option<usize>,
    pub(crate) ty: &'a SsaType,
    pub(crate) instruction: Option<&'a Instruction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) struct DiscoveryPosition {
    pub(crate) block: BlockId,
    pub(crate) instruction_index: usize,
    pub(crate) value: ValueId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) struct DiscoveryExpressionKey<'a> {
    pub(crate) operation: RuntimeOp,
    pub(crate) arguments: &'a [ValueId],
    pub(crate) signature: &'a Signature,
    pub(crate) ty: &'a SsaType,
}

pub(crate) struct DiscoveryFunctionIndexes<'a> {
    pub(crate) definitions: Vec<Option<DiscoveryDefinition<'a>>>,
    pub(crate) constants_i64: Vec<Option<i64>>,
    pub(crate) expressions: HashMap<DiscoveryExpressionKey<'a>, Vec<DiscoveryPosition>>,
    pub(crate) dominance: DiscoveryDominance,
}

pub(crate) struct DiscoveryIndexes<'a> {
    pub(crate) functions: Vec<DiscoveryFunctionIndexes<'a>>,
}

impl<'a> DiscoveryIndexes<'a> {
    pub(crate) fn build(
        program: &'a Program,
        shape: &ProgramShape,
        budget: &mut Budget,
    ) -> Result<Self, OptimizationError> {
        budget.charge(shape.functions)?;
        let mut functions = Vec::with_capacity(program.functions.len());
        for function in &program.functions {
            let value_count = function.blocks.iter().try_fold(0_usize, |total, block| {
                total
                    .checked_add(block.parameters.len())
                    .and_then(|value| value.checked_add(block.instructions.len()))
                    .ok_or_else(budget_error)
            })?;
            budget.charge((value_count as u64).saturating_mul(2))?;
            let mut definitions = vec![None; value_count];
            let mut constants_i64 = vec![None; value_count];
            for block in &function.blocks {
                for parameter in &block.parameters {
                    discovery_insert_definition(
                        &mut definitions,
                        parameter.id,
                        DiscoveryDefinition {
                            block: block.id,
                            instruction_index: None,
                            ty: &parameter.ty,
                            instruction: None,
                        },
                    )?;
                }
                for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                    discovery_insert_definition(
                        &mut definitions,
                        instruction.id,
                        DiscoveryDefinition {
                            block: block.id,
                            instruction_index: Some(instruction_index),
                            ty: &instruction.ty,
                            instruction: Some(instruction),
                        },
                    )?;
                    if let InstructionKind::Constant(Constant::I64(value)) = instruction.kind {
                        let slot = instruction
                            .id
                            .index()
                            .and_then(|index| constants_i64.get_mut(index));
                        if let Some(slot) = slot {
                            *slot = Some(value);
                        }
                    }
                    budget.charge(1)?;
                }
            }
            let dominance = DiscoveryDominance::compute(function, budget)?;
            let mut indexes = DiscoveryFunctionIndexes {
                definitions,
                constants_i64,
                expressions: HashMap::with_capacity(value_count),
                dominance,
            };
            for block in &function.blocks {
                if !indexes.dominance.is_reachable(block.id) {
                    continue;
                }
                for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                    if let Some(key) = discovery_expression_key(&indexes, instruction, budget)? {
                        budget.charge(1)?;
                        indexes
                            .expressions
                            .entry(key)
                            .or_default()
                            .push(DiscoveryPosition {
                                block: block.id,
                                instruction_index,
                                value: instruction.id,
                            });
                    }
                }
            }
            functions.push(indexes);
        }
        Ok(Self { functions })
    }
}

pub(crate) fn discovery_insert_definition<'a>(
    definitions: &mut [Option<DiscoveryDefinition<'a>>],
    value: ValueId,
    definition: DiscoveryDefinition<'a>,
) -> Result<(), OptimizationError> {
    let slot = value
        .index()
        .and_then(|index| definitions.get_mut(index))
        .ok_or_else(|| input_index_error("discovery ValueId is not dense"))?;
    if slot.replace(definition).is_some() {
        return Err(input_index_error("discovery found duplicate ValueId"));
    }
    Ok(())
}

pub(crate) fn input_index_error(detail: &str) -> OptimizationError {
    OptimizationError::new(OptimizationFailureCode::InputVerification, detail)
}
