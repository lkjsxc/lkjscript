use crate::optimize::*;
use crate::{InstructionKind, Program};

pub(crate) fn discovery_reconstruct(
    input: &Program,
    certificate: &OptimizationCertificate,
    indexes: &DiscoveryIndexes<'_>,
    budget: &mut Budget,
) -> Result<Program, OptimizationError> {
    budget.reconstruction_passes = budget.reconstruction_passes.saturating_add(1);
    let shape = preflight_program(input, budget)?;
    budget.charge(shape.allocation_units())?;
    let mut candidate = input.clone();
    for (sequence, record) in certificate.records.iter().enumerate() {
        if record.sequence != sequence as u64 {
            return Err(illegal_edit(
                "certificate sequence is not dense and ordered",
            ));
        }
        let function_index = record
            .function
            .index()
            .ok_or_else(|| illegal_edit("certificate function ID cannot index"))?;
        let function_indexes = indexes
            .functions
            .get(function_index)
            .ok_or_else(|| illegal_edit("certificate function ID is stale"))?;
        let definition = function_indexes
            .definitions
            .get(record.value.index().unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .ok_or_else(|| illegal_edit("certificate value ID is stale"))?;
        discovery_apply_record(
            &mut candidate,
            record,
            definition.block,
            definition.instruction_index,
            budget,
        )?;
    }
    budget.check_growth(instruction_count(&candidate))?;
    if certificate.records.is_empty() && has_explicit_structural_operations(&candidate) {
        return Ok(candidate);
    }
    run_cleanup(candidate, budget)
}

pub(crate) fn checker_reconstruct(
    input: &Program,
    certificate: &OptimizationCertificate,
    indexes: &CheckerIndexes<'_>,
    budget: &mut Budget,
) -> Result<Program, OptimizationError> {
    budget.reconstruction_passes = budget.reconstruction_passes.saturating_add(1);
    let shape = preflight_program(input, budget)?;
    budget.charge(shape.allocation_units())?;
    let mut private = input.clone();
    for (sequence, record) in certificate.records.iter().enumerate() {
        budget.charge(1_u64.saturating_add(record.expected_operands.len() as u64))?;
        if record.sequence != sequence as u64 {
            return Err(illegal_edit("checker record sequence is not dense"));
        }
        let function_index = record
            .function
            .index()
            .ok_or_else(|| illegal_edit("checker record function cannot index"))?;
        let function_indexes = indexes
            .functions
            .get(function_index)
            .ok_or_else(|| illegal_edit("checker record function is stale"))?;
        let definition = function_indexes
            .definitions
            .get(record.value.index().unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .ok_or_else(|| illegal_edit("checker record value is stale"))?;
        let instruction = definition
            .instruction
            .ok_or_else(|| illegal_edit("checker record targets a block parameter"))?;
        match &instruction.kind {
            InstructionKind::Runtime {
                operation,
                arguments,
                ..
            } if *operation == record.expected_operation
                && *arguments == record.expected_operands => {}
            _ => {
                return Err(illegal_edit(
                    "checker record operation or operands differ from immutable input",
                ));
            }
        }
        checker_apply_record(
            &mut private,
            record,
            definition.block,
            definition.instruction_index,
        )?;
    }
    budget.check_growth(instruction_count(&private))?;
    if certificate.records.is_empty() && has_explicit_structural_operations(&private) {
        return Ok(private);
    }
    run_cleanup(private, budget)
}

fn has_explicit_structural_operations(program: &Program) -> bool {
    program.functions.iter().any(|function| {
        function.blocks.iter().any(|block| {
            block.instructions.iter().any(|instruction| {
                matches!(
                    instruction.kind,
                    InstructionKind::StructuralPublish { .. }
                        | InstructionKind::StructuralCopy { .. }
                        | InstructionKind::MemoryWitnessIndependentOwner { .. }
                        | InstructionKind::MemoryWitnessCompare { .. }
                        | InstructionKind::MemoryWitnessDispose { .. }
                        | InstructionKind::DestinationCreate { .. }
                        | InstructionKind::DestinationFieldInit { .. }
                        | InstructionKind::DestinationFinish { .. }
                        | InstructionKind::DestinationAbort { .. }
                        | InstructionKind::AggregateFieldBorrow { .. }
                        | InstructionKind::AggregateTag { .. }
                        | InstructionKind::AggregateConsumePayload { .. }
                        | InstructionKind::StringUtf8View { .. }
                )
            })
        })
    })
}

pub(crate) fn illegal_edit(detail: &str) -> OptimizationError {
    OptimizationError::new(OptimizationFailureCode::IllegalEdit, detail)
}
