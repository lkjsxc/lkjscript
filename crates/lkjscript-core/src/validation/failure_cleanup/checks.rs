use super::{Kind, OwnerIdentity, State, UniquePlaceState};
use crate::{Error, Result, UniqueValueKind};

pub(super) fn validate_loan(
    local: usize,
    place: usize,
    kind: UniqueValueKind,
    state: &State,
) -> Result<()> {
    let actual = state.locals.get(local).copied().flatten();
    let owner = match (kind, actual) {
        (UniqueValueKind::Bytes, Some(Kind::BytesBorrow { owner, .. })) => owner,
        (
            UniqueValueKind::ByteSlice,
            Some(Kind::ByteSlice {
                owner,
                mutable: false,
                ..
            }),
        )
        | (
            UniqueValueKind::ByteSliceMut,
            Some(Kind::ByteSlice {
                owner,
                mutable: true,
                ..
            }),
        ) => owner,
        _ => return Err(Error::msg("bytecode failure loan-end has wrong local kind")),
    };
    if !matches!(
        state.unique_places.get(place),
        Some(UniquePlaceState::Active { owner: Some(actual), .. }) if *actual == owner
    ) {
        return Err(Error::msg(
            "bytecode failure loan-end does not match its owner place",
        ));
    }
    Ok(())
}

pub(super) fn validate_unique_drop(
    local: usize,
    place: Option<usize>,
    kind: UniqueValueKind,
    state: &State,
) -> Result<()> {
    let actual = state.locals.get(local).copied().flatten();
    let owner = match (kind, actual) {
        (UniqueValueKind::Bytes, Some(Kind::Bytes(owner)))
        | (UniqueValueKind::ByteVector, Some(Kind::ByteVector(owner))) => owner,
        _ => {
            return Err(Error::msg(format!(
                "bytecode failure unique drop local {local} has wrong kind {actual:?}; expected {kind:?}",
            )))
        }
    };
    if let Some(place) = place {
        if !matches!(
            state.unique_places.get(place),
            Some(UniquePlaceState::Active { owner: Some(actual), .. }) if *actual == owner
        ) {
            return Err(Error::msg(
                "bytecode failure unique drop does not match its owner place",
            ));
        }
    } else if state.unique_places.iter().any(|place| {
        matches!(place, UniquePlaceState::Active { owner: Some(actual), .. } if *actual == owner)
    }) {
        return Err(Error::msg(
            "bytecode unplaced failure drop aliases a placed owner",
        ));
    }
    Ok(())
}

pub(super) fn validate_structural_loan(
    local: usize,
    place: usize,
    representation: crate::StructuralRepresentationId,
    state: &State,
) -> Result<()> {
    let owner = match state.locals.get(local).copied().flatten() {
        Some(Kind::StructuralView {
            representation: actual,
            owner,
            ..
        }) if actual == representation => owner,
        _ => {
            return Err(Error::msg(
                "bytecode failure structural loan has wrong local kind",
            ))
        }
    };
    if !matches!(
        state.unique_places.get(place),
        Some(UniquePlaceState::Active { owner: Some(actual), .. }) if *actual == owner
    ) {
        return Err(Error::msg(
            "bytecode failure structural loan does not match its owner place",
        ));
    }
    Ok(())
}

pub(super) fn validate_structural_drop(
    local: usize,
    place: Option<usize>,
    representation: crate::StructuralRepresentationId,
    state: &State,
) -> Result<()> {
    let owner = match state.locals.get(local).copied().flatten() {
        Some(Kind::StructuralOwner {
            representation: actual,
            owner,
            ..
        }) if actual == representation => owner,
        actual => {
            return Err(Error::msg(format!(
                "bytecode failure structural drop local {local} has wrong kind {actual:?}; expected representation {}",
                representation.raw(),
            )))
        }
    };
    if let Some(place) = place {
        if !matches!(
            state.unique_places.get(place),
            Some(UniquePlaceState::Active { owner: Some(actual), .. }) if *actual == owner
        ) {
            return Err(Error::msg(format!(
                concat!(
                    "bytecode failure structural drop does not match its owner place: ",
                    "local owner {}, place {} state {:?}",
                ),
                owner,
                place,
                state.unique_places.get(place),
            )));
        }
    } else if state.unique_places.iter().any(|place| {
        matches!(place, UniquePlaceState::Active { owner: Some(actual), .. } if *actual == owner)
    }) {
        return Err(Error::msg(
            "bytecode unplaced structural drop aliases a placed owner",
        ));
    }
    Ok(())
}

pub(super) fn local_owner(state: &State, local: usize) -> Option<OwnerIdentity> {
    match state.locals.get(local).copied().flatten() {
        Some(Kind::Bytes(owner) | Kind::ByteVector(owner))
        | Some(Kind::StructuralOwner { owner, .. }) => Some(owner),
        _ => None,
    }
}
