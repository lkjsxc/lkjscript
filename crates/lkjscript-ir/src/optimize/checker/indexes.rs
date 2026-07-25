use std::collections::HashMap;

use crate::optimize::*;
use crate::{Constant, InstructionKind, Program, ValueId};

impl<'a> CheckerIndexes<'a> {
    pub(crate) fn build(
        program: &'a Program,
        shape: &ProgramShape,
        budget: &mut Budget,
    ) -> Result<Self, OptimizationError> {
        budget.charge(shape.functions)?;
        let mut functions = Vec::with_capacity(program.functions.len());
        for function in &program.functions {
            let value_count = function.blocks.iter().try_fold(0_usize, |sum, block| {
                sum.checked_add(block.parameters.len())
                    .and_then(|value| value.checked_add(block.instructions.len()))
                    .ok_or_else(budget_error)
            })?;
            budget.charge((value_count as u64).saturating_mul(2))?;
            let mut definitions = vec![None; value_count];
            let mut constants = vec![None; value_count];
            for block in &function.blocks {
                for parameter in &block.parameters {
                    checker_store_definition(
                        &mut definitions,
                        parameter.id,
                        CheckerDefinition {
                            block: block.id,
                            instruction_index: None,
                            ty: &parameter.ty,
                            instruction: None,
                        },
                    )?;
                }
                for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                    checker_store_definition(
                        &mut definitions,
                        instruction.id,
                        CheckerDefinition {
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
                            .and_then(|index| constants.get_mut(index))
                            .ok_or_else(|| {
                                input_index_error("checker constant ValueId is not dense")
                            })?;
                        *slot = Some(value);
                    }
                    budget.charge(1)?;
                }
            }
            let dominance = CheckerDominance::derive(function, budget)?;
            let mut function_indexes = CheckerFunctionIndexes {
                definitions,
                constants,
                expressions: HashMap::with_capacity(value_count),
                dominance,
            };
            for block in &function.blocks {
                if !function_indexes.dominance.reached(block.id) {
                    continue;
                }
                for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                    if let Some(key) =
                        checker_exact_expression_key(&function_indexes, instruction, budget)?
                    {
                        budget.charge(1)?;
                        function_indexes.expressions.entry(key).or_default().push(
                            CheckerPosition {
                                block: block.id,
                                instruction_index,
                                value: instruction.id,
                            },
                        );
                    }
                }
            }
            functions.push(function_indexes);
        }
        Ok(Self { functions })
    }

    pub(crate) fn verify_records(
        &self,
        program: &Program,
        certificate: &OptimizationCertificate,
        budget: &mut Budget,
    ) -> Result<(), OptimizationError> {
        let mut sequence = 0_usize;
        for function in &program.functions {
            let function_index = function
                .id
                .index()
                .ok_or_else(|| input_index_error("checker FunctionId cannot index input"))?;
            let indexes = self
                .functions
                .get(function_index)
                .ok_or_else(|| input_index_error("checker function index is incomplete"))?;
            for block in &function.blocks {
                if !indexes.dominance.reached(block.id) {
                    continue;
                }
                for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                    budget.charge(1)?;
                    let identity = checker_allowed_identity(
                        indexes,
                        block.id,
                        instruction_index,
                        instruction,
                        budget,
                    )?;
                    let expected = if identity.is_some() {
                        identity
                    } else {
                        checker_allowed_gvn(
                            indexes,
                            block.id,
                            instruction_index,
                            instruction,
                            budget,
                        )?
                    };
                    if let Some((kind, operation, operands, replacement)) = expected {
                        let record = certificate.records.get(sequence).ok_or_else(|| {
                            certificate_mismatch("certificate is missing a canonical edit")
                        })?;
                        budget.charge(1_u64.saturating_add(operands.len() as u64))?;
                        if record.sequence != sequence as u64
                            || record.function != function.id
                            || record.block != block.id
                            || record.value != instruction.id
                            || record.kind != kind
                            || record.expected_operation != operation
                            || record.expected_operands != operands
                            || record.replacement != replacement
                        {
                            return Err(certificate_mismatch(
                                "certificate record does not match independently derived edit",
                            ));
                        }
                        sequence = sequence.saturating_add(1);
                    }
                }
            }
        }
        if sequence != certificate.records.len() {
            return Err(certificate_mismatch(
                "certificate has an extra, unreachable, stale, or reordered edit",
            ));
        }
        Ok(())
    }
}

fn checker_store_definition<'a>(
    definitions: &mut [Option<CheckerDefinition<'a>>],
    value: ValueId,
    definition: CheckerDefinition<'a>,
) -> Result<(), OptimizationError> {
    let index = value
        .index()
        .ok_or_else(|| input_index_error("checker ValueId cannot index"))?;
    let slot = definitions
        .get_mut(index)
        .ok_or_else(|| input_index_error("checker ValueId is outside dense index"))?;
    if slot.replace(definition).is_some() {
        return Err(input_index_error("checker found duplicate ValueId"));
    }
    Ok(())
}

pub(crate) fn certificate_mismatch(detail: &str) -> OptimizationError {
    OptimizationError::new(OptimizationFailureCode::CertificateMismatch, detail)
}
