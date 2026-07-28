use crate::ownership::*;

pub(in crate::ownership) fn merge_conditional_cleanup(
    left: State,
    right: State,
    places: &BTreeMap<BindingId, PlaceId>,
    future: &BTreeSet<BindingId>,
) -> Result<State> {
    if left.loans != right.loans
        || left.reference_loans != right.reference_loans
        || left.consumed_ref_mut != right.consumed_ref_mut
    {
        return Err(join_error());
    }
    let place_ids: BTreeSet<_> = left
        .initialized
        .keys()
        .chain(right.initialized.keys())
        .copied()
        .collect();
    let mut merged = left;
    for place in place_ids {
        let left_initialized = merged.initialized.get(&place).copied().unwrap_or(false);
        let right_initialized = right.initialized.get(&place).copied().unwrap_or(false);
        if left_initialized == right_initialized {
            merged.initialized.insert(place, left_initialized);
            continue;
        }
        let needed_after_join = places
            .iter()
            .any(|(binding, known)| *known == place && future.contains(binding));
        if needed_after_join {
            return Err(join_error());
        }
        merged.initialized.insert(place, false);
    }
    Ok(merged)
}

fn join_error() -> Error {
    Error::msg("ownership and loan state cannot use conditional cleanup at a reachable branch join")
}
