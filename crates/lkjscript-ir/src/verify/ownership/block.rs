use std::collections::BTreeMap;

use crate::verify::*;
use crate::{Block, BlockId, Function, IrError, SsaType, Terminator};

pub(crate) fn process_ownership_block(
    function: &Function,
    block: &Block,
    mut state: OwnershipState,
    types: &[SsaType],
    work: &mut usize,
) -> crate::Result<Vec<(BlockId, OwnershipState)>> {
    let last_use = ownership_last_uses(block);
    let mut live_loans: BTreeMap<crate::PlaceId, Vec<LiveLoan>> = BTreeMap::new();

    for (index, instruction) in block.instructions.iter().enumerate() {
        charge_ownership_work(work, 1)?;
        expire_unplaced_affine(&mut state, &last_use, index);
        verify_frame_affine_available(
            function,
            instruction.metadata.frame_state.as_ref(),
            &state,
            types,
        )?;
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
            if is_affine(value_type(types, operand)?)
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

        process_ownership_instruction(function, instruction, &mut state, &mut live_loans, types)?;
    }

    let terminator_index = block.instructions.len();
    expire_unplaced_affine(&mut state, &last_use, terminator_index);
    verify_frame_affine_available(function, block.metadata.frame_state.as_ref(), &state, types)?;
    if live_loans.values().any(|loans| !loans.is_empty()) {
        return fail("SSA live loan reaches a block terminator without EndBorrow");
    }
    let terminal = !matches!(
        block.terminator,
        Terminator::Branch { .. } | Terminator::ConditionalBranch { .. }
    );
    let incomplete = !state.pending_drops.is_empty()
        || state
            .owners
            .values()
            .any(|value| value_type(types, *value).is_ok_and(is_owned_buf));
    if terminal && !matches!(block.terminator, Terminator::Return(_)) && incomplete {
        return fail("SSA structured terminator has incomplete whole-place cleanup");
    }

    match &block.terminator {
        Terminator::Branch { target, arguments } => Ok(vec![(
            *target,
            transfer_edge(function, &state, *target, arguments, types, work)?,
        )]),
        Terminator::ConditionalBranch {
            true_target,
            true_arguments,
            false_target,
            false_arguments,
            ..
        } => Ok(vec![
            (
                *true_target,
                transfer_edge(function, &state, *true_target, true_arguments, types, work)?,
            ),
            (
                *false_target,
                transfer_edge(
                    function,
                    &state,
                    *false_target,
                    false_arguments,
                    types,
                    work,
                )?,
            ),
        ]),
        Terminator::Return(value) => {
            if is_affine(value_type(types, *value)?) {
                let fact = state
                    .affine
                    .get(value)
                    .ok_or_else(|| IrError::new("SSA Return reuses an unavailable affine value"))?;
                if is_owned_value(value_type(types, *value)?) {
                    if state.owners.values().any(|owner| owner == value) {
                        return fail(
                            "SSA cannot return an Owned place value without explicit Move",
                        );
                    }
                    if !fact.transferred && !matches!(fact.provenance, AffineProvenance::Fresh(_)) {
                        return fail("SSA affine return lacks explicit Move transfer provenance");
                    }
                }
            }
            if !state.pending_drops.is_empty()
                || state
                    .owners
                    .values()
                    .any(|owner| value_type(types, *owner).is_ok_and(is_owned_buf))
            {
                return fail("SSA Return has incomplete whole-place cleanup");
            }
            Ok(Vec::new())
        }
        Terminator::Exit { code } => {
            verify_terminator_affine_available(&state, [*code], types)?;
            Ok(Vec::new())
        }
        Terminator::Outcome { detail, .. } => {
            verify_terminator_affine_available(&state, detail.iter().copied(), types)?;
            Ok(Vec::new())
        }
        Terminator::Trap { .. } => Ok(Vec::new()),
    }
}
