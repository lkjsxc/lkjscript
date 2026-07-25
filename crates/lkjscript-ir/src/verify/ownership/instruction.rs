use std::collections::BTreeMap;

use crate::verify::*;
use crate::{Function, Instruction, InstructionKind, IrError, PlaceId, SsaType};

pub(crate) fn process_ownership_instruction(
    function: &Function,
    instruction: &Instruction,
    state: &mut OwnershipState,
    live_loans: &mut BTreeMap<PlaceId, Vec<LiveLoan>>,
    types: &[SsaType],
) -> crate::Result<()> {
    match &instruction.kind {
        InstructionKind::PlaceInit { place, value } => {
            let _declared = place_by_id(function, *place)?;
            if state.owners.contains_key(place) {
                return fail("SSA initializes an Owned place that already has a current owner");
            }
            if live_loans.get(place).is_some_and(|loans| !loans.is_empty()) {
                return fail("SSA initializes an Owned place while it has a live loan");
            }
            if state.owners.values().any(|owner| owner == value) {
                return fail("SSA assigns the same owner value to multiple PlaceIds");
            }
            let fact = state
                .affine
                .get_mut(value)
                .ok_or_else(|| IrError::new("SSA PlaceInit uses an unavailable affine owner"))?;
            fact.provenance = AffineProvenance::Place(*place);
            fact.transferred = false;
            state.active_places.insert(*place);
            state.owners.insert(*place, *value);
        }
        InstructionKind::PlaceEnd { place } => {
            let _declared = place_by_id(function, *place)?;
            if !state.active_places.remove(place) {
                return fail("SSA PlaceEnd references a place that is not active");
            }
            if live_loans.get(place).is_some_and(|loans| !loans.is_empty()) {
                return fail("SSA ends an Owned place while it has a live loan");
            }
            if let Some(owner) = state.owners.remove(place) {
                state.affine.remove(&owner);
            }
        }
        InstructionKind::Move { place, value } => {
            if state.owners.get(place) != Some(value) {
                return fail("SSA Move does not reference the current owner for its PlaceId");
            }
            if live_loans.get(place).is_some_and(|loans| !loans.is_empty()) {
                return fail("SSA Move conflicts with a live loan");
            }
            let _old = state
                .affine
                .remove(value)
                .ok_or_else(|| IrError::new("SSA Move consumes an unavailable affine owner"))?;
            state.owners.remove(place);
            state.affine.insert(
                instruction.id,
                AffineFact {
                    provenance: AffineProvenance::Place(*place),
                    transferred: true,
                },
            );
        }
        InstructionKind::Borrow {
            place,
            loan,
            kind,
            value,
        } => {
            if state.owners.get(place) != Some(value) {
                return fail("SSA Borrow does not reference the current owner for its PlaceId");
            }
            let loans = live_loans.entry(*place).or_default();
            if (*kind == crate::BorrowKind::Mutable && !loans.is_empty())
                || (*kind == crate::BorrowKind::Shared
                    && loans
                        .iter()
                        .any(|loan| loan.kind == crate::BorrowKind::Mutable))
            {
                return fail("SSA has conflicting live loans for one PlaceId");
            }
            loans.push(LiveLoan {
                kind: *kind,
                value: instruction.id,
            });
            if *kind == crate::BorrowKind::Mutable {
                state.affine.insert(
                    instruction.id,
                    AffineFact {
                        provenance: AffineProvenance::Loan(*loan),
                        transferred: false,
                    },
                );
            }
        }
        InstructionKind::Call { arguments, .. } => {
            for argument in arguments {
                if matches!(value_type(types, *argument)?, SsaType::RefMut(_)) {
                    return fail("SSA RefMut user-call forwarding is unavailable in this slice");
                }
            }
            consume_affine_arguments(arguments, state, types, true)?;
        }
        InstructionKind::Runtime { arguments, .. } => {
            consume_affine_arguments(arguments, state, types, false)?;
        }
        InstructionKind::Constant(_)
        | InstructionKind::Copy(_)
        | InstructionKind::FunctionRef(_)
        | InstructionKind::ProductValue { .. }
        | InstructionKind::ProductField { .. }
        | InstructionKind::WithProductField { .. } => {}
    }

    if is_owned_buf(&instruction.ty) && !matches!(instruction.kind, InstructionKind::Move { .. }) {
        state.affine.insert(
            instruction.id,
            AffineFact {
                provenance: AffineProvenance::Fresh(instruction.id),
                transferred: false,
            },
        );
    }
    Ok(())
}
