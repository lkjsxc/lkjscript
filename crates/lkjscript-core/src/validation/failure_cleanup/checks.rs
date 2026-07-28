use super::{Kind, State, UniquePlaceState};
use crate::{Error, Result, UniqueValueKind};

pub(super) fn validate_loan(
    local: u8,
    place: u8,
    kind: UniqueValueKind,
    state: &State,
) -> Result<()> {
    let actual = state.locals.get(usize::from(local)).copied().flatten();
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
        state.unique_places.get(usize::from(place)),
        Some(UniquePlaceState::Active { owner: Some(actual), .. }) if *actual == owner
    ) {
        return Err(Error::msg(
            "bytecode failure loan-end does not match its owner place",
        ));
    }
    Ok(())
}

pub(super) fn validate_unique_drop(
    local: u8,
    place: Option<u8>,
    kind: UniqueValueKind,
    state: &State,
) -> Result<()> {
    let actual = state.locals.get(usize::from(local)).copied().flatten();
    let owner = match (kind, actual) {
        (UniqueValueKind::Bytes, Some(Kind::Bytes(owner)))
        | (UniqueValueKind::ByteVector, Some(Kind::ByteVector(owner))) => owner,
        _ => {
            return Err(Error::msg(
                "bytecode failure unique drop has wrong local kind",
            ))
        }
    };
    if let Some(place) = place {
        if !matches!(
            state.unique_places.get(usize::from(place)),
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

pub(super) fn local_owner(state: &State, local: u8) -> Option<u32> {
    match state.locals.get(usize::from(local)).copied().flatten() {
        Some(Kind::Bytes(owner) | Kind::ByteVector(owner)) => Some(owner),
        _ => None,
    }
}
