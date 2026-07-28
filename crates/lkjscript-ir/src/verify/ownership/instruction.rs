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
            process_place_end(function, *place, state, live_loans)?;
        }
        InstructionKind::EndBorrow { place, loan, value } => {
            process_end_borrow(*place, *loan, *value, state, live_loans)?;
        }
        InstructionKind::Drop {
            place,
            value,
            glue,
            kind,
        } => process_drop(*place, *value, *glue, *kind, state, live_loans)?,
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
                loan: *loan,
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
                if matches!(value_type(types, *argument)?, SsaType::ByteSliceMut) {
                    return fail(
                        "SSA byte-slice-mut user-call forwarding is unavailable in this slice",
                    );
                }
            }
            consume_affine_arguments(arguments, state, types, true, false)?;
        }
        InstructionKind::Runtime {
            operation,
            arguments,
            ..
        } => {
            let closes = matches!(
                operation,
                crate::RuntimeOp::SysClose
                    | crate::RuntimeOp::SysSqliteClose
                    | crate::RuntimeOp::SysSqliteFinalize
            );
            let pending = if closes {
                let [value] = arguments.as_slice() else {
                    return fail("SSA resource close must consume one exact owner");
                };
                current_owner_place(state, *value).map(|place| (place, *value))
            } else {
                None
            };
            consume_affine_arguments(arguments, state, types, false, closes)?;
            if let Some((place, value)) = pending {
                if state.pending_drops.insert(place, value).is_some() {
                    return fail("SSA resource close duplicated a pending Drop event");
                }
            }
        }
        InstructionKind::Constant(_)
        | InstructionKind::Copy(_)
        | InstructionKind::FunctionRef(_)
        | InstructionKind::F64FromI64Exact { .. }
        | InstructionKind::F64FromI64Rounded { .. }
        | InstructionKind::I64FromF64Exact { .. }
        | InstructionKind::I64FromF64Trunc { .. }
        | InstructionKind::ProductValue { .. }
        | InstructionKind::ProductField { .. }
        | InstructionKind::WithProductField { .. }
        | InstructionKind::EnumValue { .. }
        | InstructionKind::EnumIsVariant { .. }
        | InstructionKind::EnumField { .. } => {}
    }

    if is_owned_value(&instruction.ty) && !matches!(instruction.kind, InstructionKind::Move { .. })
    {
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
