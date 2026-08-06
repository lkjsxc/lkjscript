fn validate_plan(
    proto: &FunctionProto,
    roots: crate::FailureCleanupRoots,
    state: &State,
    require_place_coverage: bool,
) -> Result<()> {
    let mut locals = HashSet::new();
    let mut covered_places = HashSet::new();
    let mut covered_destinations = HashSet::new();
    for root in roots.ids() {
        let mut current = Some(root);
        while let Some(id) = current {
            let node = proto
            .failure_cleanups
            .get(id.index().unwrap_or(usize::MAX))
            .ok_or_else(|| Error::msg("bytecode failure-cleanup chain lost a node"))?;
        current = node.next;
        let local = match node.action {
            FailureCleanupAction::EndBorrow { local, place, kind } => {
                validate_loan(local, place, kind, state)?;
                local
            }
            FailureCleanupAction::DropUnique { local, place, kind } => {
                validate_unique_drop(local, place, kind, state)?;
                if let (Some(place), Some(owner)) = (place, local_owner(state, local)) {
                    covered_places.insert((place, owner));
                }
                local
            }
            FailureCleanupAction::DropResource { local, kind, .. } => {
                if !matches!(
                    state.locals.get(local).copied().flatten(),
                    Some(Kind::Resource { kind: actual, .. }) if actual == kind
                ) {
                    return Err(Error::msg(format!(
                        "bytecode failure cleanup resource local has wrong kind: local {} is {:?}, expected {kind:?}",
                        local,
                        state.locals.get(local).copied().flatten()
                    )));
                }
                local
            }
            FailureCleanupAction::EndStructuralBorrow {
                local,
                place,
                representation,
            } => {
                validate_structural_loan(local, place, representation, state)?;
                local
            }
            FailureCleanupAction::DropStructural {
                local,
                place,
                representation,
            } => {
                validate_structural_drop(local, place, representation, state)?;
                if let (Some(place), Some(owner)) = (place, local_owner(state, local)) {
                    covered_places.insert((place, owner));
                }
                local
            }
            FailureCleanupAction::AbortStructuralDestination { local, destination } => {
                let value = state.locals.get(local).copied().flatten();
                let Some(Kind::StructuralDestination {
                    destination: actual,
                    identity,
                }) = value
                else {
                    return Err(Error::msg(
                        "bytecode failure destination abort has wrong local kind",
                    ));
                };
                if actual != destination
                    || state
                        .structural_destinations
                        .get(&identity)
                        .is_none_or(|active| active.destination != destination)
                {
                    return Err(Error::msg(
                        "bytecode failure destination abort references inactive metadata",
                    ));
                }
                covered_destinations.insert((identity, destination));
                local
            }
        };
            if !locals.insert(local) {
                return Err(Error::msg(
                    "bytecode failure cleanup duplicates one local action",
                ));
            }
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
        if !covered_places.contains(&(place, *owner)) {
            return Err(Error::msg(
                "bytecode failure cleanup omits a current unique place owner",
            ));
        }
    }
    for (identity, destination) in &state.structural_destinations {
        if !covered_destinations.contains(&(*identity, destination.destination)) {
            return Err(Error::msg(
                "bytecode failure cleanup omits an active structural destination",
            ));
        }
    }
    Ok(())
}
