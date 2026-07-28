use std::collections::HashSet;

use super::{Kind, State, UniquePlaceState};
use crate::{
    Error, FailureCleanupAction, FailureCleanupPlan, FunctionProto, Result, UniqueValueKind,
};
use checks::{local_owner, validate_loan, validate_unique_drop};

mod checks;

pub(super) fn validate_at_offset(
    proto: &FunctionProto,
    offset: usize,
    state: &State,
) -> Result<()> {
    let range = proto
        .failure_cleanup_ranges
        .iter()
        .find(|range| usize::from(range.start) <= offset && offset < usize::from(range.end));
    let required = state
        .unique_places
        .iter()
        .any(|place| matches!(place, UniquePlaceState::Active { owner: Some(_), .. }))
        || state.locals.iter().enumerate().any(|(index, kind)| {
            kind.is_some_and(|kind| {
                matches!(kind, Kind::BytesBorrow { .. } | Kind::ByteSlice { .. })
                    && !borrowed_parameter(proto, index)
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
    if usize::from(range.start) != offset {
        return Ok(());
    }
    if let Some(plan) = range.plan {
        let plan = proto
            .failure_cleanups
            .get(usize::from(plan))
            .ok_or_else(|| Error::msg("bytecode failure-cleanup range lost its plan"))?;
        validate_plan(proto, plan, state, true)?;
    } else if required {
        return Err(Error::msg(format!(
            "bytecode live unique state has an empty failure-cleanup range at offset {offset} opcode {}",
            proto.code.get(offset).copied().unwrap_or(u8::MAX)
        )));
    }
    if let Some(unentered) = range.unentered_plan {
        let unentered = proto
            .failure_cleanups
            .get(usize::from(unentered))
            .ok_or_else(|| Error::msg("bytecode unentered cleanup range lost its plan"))?;
        validate_plan(proto, unentered, state, false)?;
    }
    Ok(())
}

fn borrowed_parameter(proto: &FunctionProto, index: usize) -> bool {
    index < usize::from(proto.arity)
        && matches!(
            proto.parameter_uniques.get(index).copied().flatten(),
            Some(UniqueValueKind::ByteSlice | UniqueValueKind::ByteSliceMut)
        )
}

fn validate_plan(
    proto: &FunctionProto,
    plan: &FailureCleanupPlan,
    state: &State,
    require_place_coverage: bool,
) -> Result<()> {
    let mut locals = HashSet::new();
    for action in &plan.actions {
        let local = match action {
            FailureCleanupAction::EndBorrow { local, place, kind } => {
                validate_loan(*local, *place, *kind, state)?;
                *local
            }
            FailureCleanupAction::DropUnique { local, place, kind } => {
                validate_unique_drop(*local, *place, *kind, state)?;
                *local
            }
            FailureCleanupAction::DropResource { local, kind, .. } => {
                if state.locals.get(usize::from(*local)).copied().flatten()
                    != Some(Kind::Resource(*kind))
                {
                    return Err(Error::msg(
                        "bytecode failure cleanup resource local has wrong kind",
                    ));
                }
                *local
            }
        };
        if !locals.insert(local) {
            return Err(Error::msg(
                "bytecode failure cleanup duplicates one local action",
            ));
        }
    }
    if !require_place_coverage {
        return Ok(());
    }
    for (place, state_place) in state.unique_places.iter().enumerate() {
        let UniquePlaceState::Active {
            owner: Some(owner), ..
        } = state_place
        else {
            continue;
        };
        let place =
            u8::try_from(place).map_err(|_| Error::msg("bytecode unique place exceeds u8"))?;
        let covered = plan.actions.iter().any(|action| {
            matches!(
                action,
                FailureCleanupAction::DropUnique {
                    local,
                    place: Some(actual),
                    ..
                } if *actual == place && local_owner(state, *local) == Some(*owner)
            )
        });
        if !covered {
            return Err(Error::msg(
                "bytecode failure cleanup omits a current unique place owner",
            ));
        }
    }
    let _ = proto;
    Ok(())
}
