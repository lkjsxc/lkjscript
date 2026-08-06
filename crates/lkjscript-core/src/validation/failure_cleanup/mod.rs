use std::collections::HashSet;

use super::{Kind, State, UniquePlaceState};
use crate::{Error, FailureCleanupAction, FunctionProto, Result, UniqueValueKind};
use checks::{
    local_owner, validate_loan, validate_structural_drop, validate_structural_loan,
    validate_unique_drop,
};

mod checks;

pub(super) fn validate_at_offset(
    proto: &FunctionProto,
    offset: usize,
    state: &State,
) -> Result<()> {
    let host_offset = offset;
    let offset = u64::try_from(host_offset)
        .map_err(|_| Error::msg("bytecode instruction offset exceeds cleanup range width"))?;
    let range = proto
        .failure_cleanup_ranges
        .iter()
        .find(|range| range.start <= offset && offset < range.end);
    let required = state
        .unique_places
        .iter()
        .any(|place| matches!(place, UniquePlaceState::Active { owner: Some(_), .. }))
        || !state.structural_destinations.is_empty()
        || state.locals.iter().enumerate().any(|(index, kind)| {
            kind.is_some_and(|kind| {
                matches!(
                    kind,
                    Kind::BytesBorrow { .. }
                        | Kind::ByteSlice { .. }
                        | Kind::StructuralView { .. }
                        | Kind::StructuralDestination { .. }
                ) && !borrowed_parameter(proto, index)
            })
        });
    let Some(range) = range else {
        let metadata_present =
            !proto.failure_cleanups.is_empty() || !proto.failure_cleanup_ranges.is_empty();
        return if required && metadata_present {
            Err(Error::msg(
                "bytecode live unique state lacks failure-cleanup range",
            ))
        } else {
            Ok(())
        };
    };
    if range.start != offset {
        return Ok(());
    }
    if let Some(plan) = range.plan {
        validate_plan(proto, plan, state, true).map_err(|error| {
            Error::msg(format!(
                "failure cleanup at byte {offset} opcode {}: {error}",
                proto.code.get(host_offset).copied().unwrap_or(u8::MAX)
            ))
        })?;
    } else if required {
        return Err(Error::msg(format!(
            "bytecode live unique state has an empty failure-cleanup range at offset {offset} opcode {}",
            proto.code.get(host_offset).copied().unwrap_or(u8::MAX)
        )));
    }
    if let Some(unentered) = range.unentered_plan {
        validate_plan(
            proto,
            crate::FailureCleanupRoots::single(unentered),
            state,
            false,
        )
        .map_err(|error| {
            Error::msg(format!(
                "unentered failure cleanup at byte {offset}: {error}"
            ))
        })?;
    }
    Ok(())
}

fn borrowed_parameter(proto: &FunctionProto, index: usize) -> bool {
    index < proto.arity
        && matches!(
            proto.parameter_uniques.get(index).copied().flatten(),
            Some(UniqueValueKind::ByteSlice | UniqueValueKind::ByteSliceMut)
        )
}

include!("plan.rs");
