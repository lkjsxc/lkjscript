mod failure;
mod nonowned;
use failure::{expected_failure_cleanup, verify_failure_cleanup, verify_failure_cleanup_plan};
pub(crate) use nonowned::{edge_arguments_to, nonowned_affine_values};

use std::collections::{BTreeMap, HashSet};

use crate::verify::*;
use crate::{
    Block, BlockId, FailureCleanupAction, FailureCleanupId, Function, Instruction, InstructionKind,
    IrError, Program, SsaType, Terminator, ValueId,
};

pub(crate) fn process_ownership_block(
    program: &Program,
    function: &Function,
    block: &Block,
    mut state: OwnershipState,
    types: &[SsaType],
    nonowned_affine: &HashSet<ValueId>,
    work: &mut usize,
) -> crate::Result<Vec<(BlockId, OwnershipState)>> {
    let last_use = ownership_last_uses(block);
    let mut live_loans: BTreeMap<crate::PlaceId, Vec<LiveLoan>> = BTreeMap::new();

    for (index, instruction) in block.instructions.iter().enumerate() {
        charge_ownership_work(work, 1)?;
        expire_unplaced_affine(&mut state, &last_use, index);
        verify_frame_affine_available(
            program,
            function,
            instruction.metadata.frame_state.as_ref(),
            &state,
            types,
            nonowned_affine,
        )
        .map_err(|error| {
            IrError::new(format!(
                "SSA instruction {} frame state: {error}",
                instruction.id.raw()
            ))
        })?;
        for operand in instruction.kind.operands() {
            let pending_drop_operand =
                matches!(
                    instruction.kind,
                    InstructionKind::Drop {
                        kind: crate::DropEventKind::ExplicitClose,
                        ..
                    }
                ) && state.pending_drops.values().any(|value| *value == operand);
            let ended_borrow_operand =
                matches!(instruction.kind, InstructionKind::EndBorrow { .. });
            if is_affine(program, value_type(types, operand)?)
                && !nonowned_affine.contains(&operand)
                && !state.affine.contains_key(&operand)
                && !pending_drop_operand
                && !ended_borrow_operand
            {
                return fail(format!(
                    "SSA reuses unavailable affine ValueId {} in instruction {} {:?} after move or transfer",
                    operand.raw(),
                    instruction.id.raw(),
                    instruction.kind
                ));
            }
        }

        verify_failure_cleanup(
            program,
            function,
            instruction,
            &state,
            &live_loans,
            types,
            nonowned_affine,
        )?;
        process_ownership_instruction(
            program,
            function,
            instruction,
            &mut state,
            &mut live_loans,
            types,
            nonowned_affine,
        )?;
    }

    let terminator_index = block.instructions.len();
    expire_unplaced_affine(&mut state, &last_use, terminator_index);
    verify_frame_affine_available(
        program,
        function,
        block.metadata.frame_state.as_ref(),
        &state,
        types,
        nonowned_affine,
    )
    .map_err(|error| IrError::new(format!("SSA block terminator frame state: {error}")))?;
    if live_loans.values().any(|loans| !loans.is_empty()) {
        return fail("SSA live loan reaches a block terminator without EndBorrow");
    }
    verify_failure_cleanup_plan(
        function,
        block.metadata.failure_cleanup,
        &expected_failure_cleanup(
            program,
            function,
            &state,
            &live_loans,
            types,
            nonowned_affine,
            &[],
        )?,
        "block terminator",
    )?;
    process_ownership_terminator(
        program,
        function,
        block,
        &state,
        types,
        nonowned_affine,
        work,
    )
}

include!("block/terminator.rs");
