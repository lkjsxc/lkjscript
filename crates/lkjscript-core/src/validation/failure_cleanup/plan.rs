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
                if !matches!(
                    state.locals.get(usize::from(*local)).copied().flatten(),
                    Some(Kind::Resource { kind: actual, .. }) if actual == *kind
                ) {
                    return Err(Error::msg(format!(
                        "bytecode failure cleanup resource local has wrong kind: local {} is {:?}, expected {kind:?}",
                        local,
                        state.locals.get(usize::from(*local)).copied().flatten()
                    )));
                }
                *local
            }
            FailureCleanupAction::EndStructuralBorrow {
                local,
                place,
                representation,
            } => {
                validate_structural_loan(*local, *place, *representation, state)?;
                *local
            }
            FailureCleanupAction::DropStructural {
                local,
                place,
                representation,
            } => {
                validate_structural_drop(*local, *place, *representation, state)?;
                *local
            }
            FailureCleanupAction::AbortStructuralDestination { local, destination } => {
                let value = state.locals.get(usize::from(*local)).copied().flatten();
                let Some(Kind::StructuralDestination {
                    destination: actual,
                    identity,
                }) = value
                else {
                    return Err(Error::msg(
                        "bytecode failure destination abort has wrong local kind",
                    ));
                };
                if actual != *destination
                    || state
                        .structural_destinations
                        .get(&identity)
                        .is_none_or(|active| active.destination != *destination)
                {
                    return Err(Error::msg(
                        "bytecode failure destination abort references inactive metadata",
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
                }
                | FailureCleanupAction::DropStructural {
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
    for (identity, destination) in &state.structural_destinations {
        let covered = plan.actions.iter().any(|action| {
            matches!(
                action,
                FailureCleanupAction::AbortStructuralDestination {
                    local,
                    destination: actual,
                } if actual == &destination.destination
                    && state.locals.get(usize::from(*local)).copied().flatten()
                        == Some(Kind::StructuralDestination {
                            destination: destination.destination,
                            identity: *identity,
                        })
            )
        });
        if !covered {
            return Err(Error::msg(
                "bytecode failure cleanup omits an active structural destination",
            ));
        }
    }
    let _ = proto;
    Ok(())
}
