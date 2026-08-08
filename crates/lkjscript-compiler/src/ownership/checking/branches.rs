use crate::ownership::*;

pub(in crate::ownership) fn merge_conditional_cleanup(
    left: State,
    right: State,
    plan: &OwnershipPlan,
    future: &FutureUses,
) -> Result<State> {
    if left.loans != right.loans
        || left.reference_loans != right.reference_loans
        || left.pinned_references != right.pinned_references
        || left.consumed_ref_mut != right.consumed_ref_mut
    {
        return Err(join_error());
    }
    let place_count = left
        .initialized
        .len()
        .checked_add(right.initialized.len())
        .ok_or_else(|| Error::host("ownership branch place count overflow"))?;
    let mut place_ids = Vec::new();
    place_ids
        .try_reserve_exact(place_count)
        .map_err(|_| Error::host("ownership branch-place allocation failed"))?;
    place_ids.extend(left.initialized.keys().copied());
    place_ids.extend(right.initialized.keys().copied());
    place_ids.sort_unstable();
    place_ids.dedup();
    let mut merged = left;
    for place in place_ids {
        let left_initialized = merged.initialized.get(&place).copied().unwrap_or(false);
        let right_initialized = right.initialized.get(&place).copied().unwrap_or(false);
        if left_initialized == right_initialized {
            merged.initialized.insert(place, left_initialized);
            continue;
        }
        let needed_after_join = plan
            .binding_for_place(place)
            .is_some_and(|binding| plan.binding_live(binding, None, future));
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
