use crate::optimize::*;
use crate::{verify, Program, VerifiedProgram};

pub(crate) fn run_cleanup(
    candidate: Program,
    budget: &mut Budget,
) -> Result<Program, OptimizationError> {
    let shape = preflight_program(&candidate, budget)?;
    budget.charge_validation(&shape)?;
    let mut verified = verify(candidate)
        .map_err(|error| illegal_edit(&format!("certified edit-stage SSA failed: {error}")))?;
    // Delete disconnected components before copy propagation so legal
    // unreachable SCCs cannot retain cross-block copy references while dense
    // values are compacted.
    verified = cleanup_pass(verified, unreachable_blocks, budget)?;
    verified = cleanup_pass(verified, copy_propagate, budget)?;
    verified = cleanup_pass(verified, simplify_branches, budget)?;
    verified = cleanup_pass(verified, empty_block_forwarding, budget)?;
    verified = cleanup_pass(verified, effect_aware_dce, budget)?;
    verified = cleanup_pass(verified, direct_call_resolution, budget)?;
    verified = cleanup_pass(verified, canonical_block_order, budget)?;
    Ok(verified.into_program())
}

pub(crate) fn cleanup_pass(
    input: VerifiedProgram,
    pass: fn(&VerifiedProgram) -> crate::Result<VerifiedProgram>,
    budget: &mut Budget,
) -> Result<VerifiedProgram, OptimizationError> {
    budget.charge_cleanup_pass()?;
    let shape = preflight_program(input.program(), budget)?;
    let worst_case_pass_work = shape
        .allocation_units()
        .saturating_add(shape.blocks.saturating_mul(shape.blocks))
        .saturating_add(shape.instructions)
        .saturating_add(shape.operands);
    budget.charge(worst_case_pass_work)?;
    // Each isolated baseline pass ordinarily verifies the output once.
    budget.charge_validation(&shape)?;
    let output = pass(&input).map_err(|error| {
        OptimizationError::new(
            OptimizationFailureCode::OutputVerification,
            error.to_string(),
        )
    })?;
    budget.check_growth(instruction_count(output.program()))?;
    Ok(output)
}

pub(crate) fn instruction_count(program: &Program) -> u64 {
    program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .fold(0_u64, |count, block| {
            count.saturating_add(block.instructions.len() as u64)
        })
}
