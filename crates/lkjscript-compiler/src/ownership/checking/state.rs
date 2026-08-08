use crate::ownership::*;

pub(in crate::ownership) fn expire_dead_loans(
    state: &mut State,
    plan: &OwnershipPlan,
    current: Option<ExprRange>,
    future: &FutureUses,
) -> Result<()> {
    let mut dead = Vec::new();
    dead.try_reserve(state.reference_loans.len())
        .map_err(|_| Error::host("ownership dead-loan allocation failed"))?;
    dead.extend(state.reference_loans.keys().copied().filter(|binding| {
        !reference_is_pinned(state, *binding) && !plan.binding_live(*binding, current, future)
    }));
    for binding in dead {
        end_reference_binding(state, binding);
    }
    Ok(())
}

pub(in crate::ownership) fn expire_dead_loans_for_place(
    state: &mut State,
    place: PlaceId,
    plan: &OwnershipPlan,
    current: Option<ExprRange>,
    future: &FutureUses,
) -> Result<()> {
    let Some(loans) = state.loans.get(&place) else {
        return Ok(());
    };
    let mut dead = Vec::new();
    dead.try_reserve(loans.len())
        .map_err(|_| Error::host("ownership place-loan allocation failed"))?;
    dead.extend(loans.iter().filter_map(|loan| {
        loan.binding.filter(|binding| {
            !reference_is_pinned(state, *binding) && !plan.binding_live(*binding, current, future)
        })
    }));
    for binding in dead {
        end_reference_binding(state, binding);
    }
    Ok(())
}

pub(in crate::ownership) fn pin_reference(state: &mut State, binding: BindingId) -> Result<()> {
    if let Some(count) = state.pinned_references.get_mut(&binding) {
        *count = count
            .checked_add(1)
            .ok_or_else(|| Error::host("ownership reference-pin count overflow"))?;
        return Ok(());
    }
    state
        .pinned_references
        .try_reserve(1)
        .map_err(|_| Error::host("ownership reference-pin allocation failed"))?;
    state.pinned_references.insert(binding, 1);
    Ok(())
}

pub(in crate::ownership) fn unpin_reference(state: &mut State, binding: BindingId) -> Result<()> {
    let count = state
        .pinned_references
        .get_mut(&binding)
        .ok_or_else(|| Error::msg("ownership reference pin is missing"))?;
    if *count == 1 {
        state.pinned_references.remove(&binding);
    } else {
        *count -= 1;
    }
    Ok(())
}

fn reference_is_pinned(state: &State, binding: BindingId) -> bool {
    state
        .pinned_references
        .get(&binding)
        .is_some_and(|count| *count != 0)
}

pub(in crate::ownership) fn end_reference_binding(state: &mut State, binding: BindingId) {
    state.pinned_references.remove(&binding);
    if let Some((place, loan)) = state.reference_loans.remove(&binding) {
        end_loan(state, place, loan);
    }
}

pub(in crate::ownership) fn end_place_scope(state: &mut State, place: PlaceId) -> Result<()> {
    state.initialized.remove(&place);
    state.loans.remove(&place);
    let mut references = Vec::new();
    references
        .try_reserve(state.reference_loans.len())
        .map_err(|_| Error::host("ownership scope-reference allocation failed"))?;
    references.extend(
        state
            .reference_loans
            .iter()
            .filter_map(|(binding, (owner, _))| (*owner == place).then_some(*binding)),
    );
    for binding in references {
        state.reference_loans.remove(&binding);
        state.pinned_references.remove(&binding);
        state.consumed_ref_mut.remove(&binding);
    }
    Ok(())
}

pub(in crate::ownership) fn end_loan(state: &mut State, place: PlaceId, loan: LoanId) {
    if let Some(loans) = state.loans.get_mut(&place) {
        loans.retain(|item| item.id != loan);
        if loans.is_empty() {
            state.loans.remove(&place);
        }
    }
}
