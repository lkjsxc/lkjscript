use crate::verify::*;
use crate::{Function, Instruction, InstructionKind, IrError, PlaceId, Program, SsaType};
pub(crate) fn process_ownership_instruction(
    program: &Program,
    function: &Function,
    instruction: &Instruction,
    state: &mut OwnershipState,
    live_loans: &mut std::collections::BTreeMap<PlaceId, Vec<LiveLoan>>,
    types: &[SsaType],
    nonowned_affine: &std::collections::HashSet<crate::ValueId>,
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
                .affine_mut()
                .get_mut(value)
                .ok_or_else(|| IrError::new("SSA PlaceInit uses an unavailable affine owner"))?;
            fact.provenance = AffineProvenance::Place(*place);
            fact.transferred = false;
            state.active_places_mut().insert(*place);
            state.owners_mut().insert(*place, *value);
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
                .affine_mut()
                .remove(value)
                .ok_or_else(|| IrError::new("SSA Move consumes an unavailable affine owner"))?;
            state.owners_mut().remove(place);
            state.affine_mut().insert(
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
                state.affine_mut().insert(
                    instruction.id,
                    AffineFact {
                        provenance: AffineProvenance::Loan(*loan),
                        transferred: false,
                    },
                );
            }
        }
        InstructionKind::Call {
            arguments,
            consuming,
            ..
        } => {
            for argument in arguments {
                if matches!(value_type(types, *argument)?, SsaType::ByteSliceMut) {
                    return fail(
                        "SSA byte-slice-mut user-call forwarding is unavailable in this slice",
                    );
                }
            }
            let consumed = arguments
                .iter()
                .zip(consuming)
                .filter_map(|(argument, consuming)| consuming.then_some(*argument))
                .collect::<Vec<_>>();
            consume_affine_arguments(
                program,
                &consumed,
                state,
                types,
                nonowned_affine,
                true,
                true,
            )?;
        }
        InstructionKind::Runtime {
            operation,
            arguments,
            ..
        } => process_runtime_instruction(
            program,
            operation,
            arguments,
            state,
            types,
            nonowned_affine,
        )?,
        InstructionKind::StructuralPublish { value, .. } => {
            process_structural_publish(value, state)?
        }
        InstructionKind::DestinationFieldInit {
            destination, value, ..
        } => process_destination_field_init(program, destination, value, state, types)?,
        InstructionKind::DestinationFinish { destination }
        | InstructionKind::DestinationAbort { destination } => {
            process_destination_terminal(destination, state)?
        }
        InstructionKind::AggregateFieldBorrow {
            place, loan, value, ..
        } => {
            if state.owners.get(place) != Some(value) {
                return fail(
                    "SSA aggregate field borrow does not reference its current placed owner",
                );
            }
            let loans = live_loans.entry(*place).or_default();
            if loans
                .iter()
                .any(|loan| loan.kind == crate::BorrowKind::Mutable)
            {
                return fail("SSA aggregate field borrow conflicts with a mutable loan");
            }
            loans.push(LiveLoan {
                loan: *loan,
                kind: crate::BorrowKind::Shared,
                value: instruction.id,
            });
        }
        InstructionKind::AggregateConsumePayload { place, value, .. } => {
            process_aggregate_payload(program, function, place, value, state, live_loans, types)?
        }
        InstructionKind::DestinationCreate { .. }
        | InstructionKind::AggregateTag { .. }
        | InstructionKind::StringUtf8View { .. }
        | InstructionKind::StructuralCopy { .. }
        | InstructionKind::MemoryWitnessIndependentOwner { .. }
        | InstructionKind::MemoryWitnessCompare { .. }
        | InstructionKind::MemoryWitnessDispose { .. }
        | InstructionKind::Constant(_)
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
    register_instruction_result(program, instruction, state);
    Ok(())
}

include!("instruction/runtime.rs");
include!("instruction/destination.rs");
include!("instruction/aggregate.rs");
include!("instruction_result.rs");
