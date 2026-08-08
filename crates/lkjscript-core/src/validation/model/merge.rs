use super::{decode::instruction_error, Kind, OwnerIdentity, State, UniquePlaceState};
use crate::{DecodedInstruction, FunctionProto, Result};

pub(super) fn merge_state(
    target: &mut Option<State>,
    incoming: &State,
    proto: &FunctionProto,
    predecessor: DecodedInstruction,
) -> Result<bool> {
    let Some(current) = target else {
        *target = Some(incoming.clone());
        return Ok(true);
    };
    if current.stack.len() != incoming.stack.len() {
        return Err(instruction_error(
            proto,
            predecessor.op(),
            predecessor.offset(),
            "incompatible operand stack depth at CFG join",
        ));
    }
    let mut changed = false;
    for (existing, incoming) in current.stack.iter_mut().zip(&incoming.stack) {
        let merged = merge_kind(*existing, *incoming).ok_or_else(|| {
            instruction_error(
                proto,
                predecessor.op(),
                predecessor.offset(),
                "incompatible operand stack categories at CFG join",
            )
        })?;
        if *existing != merged {
            *existing = merged;
            changed = true;
        }
    }
    for (existing, incoming) in current.locals.iter_mut().zip(&incoming.locals) {
        changed |= merge_slot(existing, *incoming, proto, predecessor, "local")?;
    }
    for (existing, incoming) in current.globals.iter_mut().zip(&incoming.globals) {
        changed |= merge_slot(existing, *incoming, proto, predecessor, "global")?;
    }
    for (existing, incoming) in current
        .unique_places
        .iter_mut()
        .zip(&incoming.unique_places)
    {
        let merged = merge_place(*existing, *incoming).ok_or_else(|| {
            instruction_error(
                proto,
                predecessor.op(),
                predecessor.offset(),
                &format!(
                    "incompatible unique-place ownership state at CFG join: {existing:?} versus {incoming:?}",
                ),
            )
        })?;
        if *existing != merged {
            *existing = merged;
            changed = true;
        }
    }
    if current.structural_destinations != incoming.structural_destinations {
        return Err(instruction_error(
            proto,
            predecessor.op(),
            predecessor.offset(),
            "incompatible structural destination state at CFG join",
        ));
    }
    if changed {
        current.refresh_cleanup_requirement(proto);
    }
    Ok(changed)
}

fn merge_slot(
    existing: &mut Option<Kind>,
    incoming: Option<Kind>,
    proto: &FunctionProto,
    predecessor: DecodedInstruction,
    category: &str,
) -> Result<bool> {
    match (*existing, incoming) {
        (None, _) => Ok(false),
        (Some(_), None) => {
            *existing = None;
            Ok(true)
        }
        (Some(left), Some(right)) => {
            let merged = merge_kind(left, right).ok_or_else(|| {
                instruction_error(
                    proto,
                    predecessor.op(),
                    predecessor.offset(),
                    &format!(
                        "incompatible {category} categories at CFG join: {left:?} versus {right:?}",
                    ),
                )
            })?;
            if Some(merged) == *existing {
                Ok(false)
            } else {
                *existing = Some(merged);
                Ok(true)
            }
        }
    }
}

fn merge_kind(left: Kind, right: Kind) -> Option<Kind> {
    if left == right {
        Some(left)
    } else if left == Kind::Any || right == Kind::Any {
        Some(Kind::Any)
    } else {
        match (left, right) {
            (Kind::ByteVector(owner), Kind::ByteVector(_)) => Some(Kind::ByteVector(owner)),
            (Kind::Bytes(owner), Kind::Bytes(_)) => Some(Kind::Bytes(owner)),
            (
                Kind::StructuralOwner {
                    representation: left_representation,
                    owner,
                    active_variant: left_variant,
                },
                Kind::StructuralOwner {
                    representation: right_representation,
                    active_variant: right_variant,
                    ..
                },
            ) if left_representation == right_representation => Some(Kind::StructuralOwner {
                representation: left_representation,
                owner,
                active_variant: (left_variant == right_variant)
                    .then_some(left_variant)
                    .flatten(),
            }),
            (
                Kind::StructuralOwnerRef {
                    representation: left_representation,
                    active_variant: left_variant,
                    ..
                },
                Kind::StructuralOwnerRef {
                    representation: right_representation,
                    active_variant: right_variant,
                    ..
                },
            ) if left_representation == right_representation => Some(Kind::StructuralOwnerRef {
                representation: left_representation,
                owner: OwnerIdentity::Merged,
                active_variant: (left_variant == right_variant)
                    .then_some(left_variant)
                    .flatten(),
            }),
            _ => None,
        }
    }
}

fn merge_place(left: UniquePlaceState, right: UniquePlaceState) -> Option<UniquePlaceState> {
    match (left, right) {
        (UniquePlaceState::Inactive, UniquePlaceState::Inactive) => {
            Some(UniquePlaceState::Inactive)
        }
        (
            UniquePlaceState::Active {
                owner: left_owner,
                transferred: left_transferred,
            },
            UniquePlaceState::Active {
                owner: right_owner,
                transferred: right_transferred,
            },
        ) => Some(UniquePlaceState::Active {
            owner: merge_identity(left_owner, right_owner)?,
            transferred: merge_identity(left_transferred, right_transferred)?,
        }),
        _ => None,
    }
}

fn merge_identity(
    left: Option<OwnerIdentity>,
    right: Option<OwnerIdentity>,
) -> Option<Option<OwnerIdentity>> {
    match (left, right) {
        (None, None) => Some(None),
        (Some(left), Some(_)) => Some(Some(left)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    include!("merge/tests.rs");
}
