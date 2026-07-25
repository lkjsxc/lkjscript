use crate::ownership::*;

pub(in crate::ownership) fn expire_dead_loans(
    state: &mut State,
    live_bindings: &BTreeSet<BindingId>,
) {
    let dead: Vec<BindingId> = state
        .reference_loans
        .keys()
        .copied()
        .filter(|binding| !live_bindings.contains(binding))
        .collect();
    for binding in dead {
        end_reference_binding(state, binding);
    }
}

pub(in crate::ownership) fn end_reference_binding(state: &mut State, binding: BindingId) {
    if let Some((place, loan)) = state.reference_loans.remove(&binding) {
        end_loan(state, place, loan);
    }
}

pub(in crate::ownership) fn end_place_scope(state: &mut State, place: PlaceId) {
    state.initialized.remove(&place);
    state.loans.remove(&place);
    let references: Vec<BindingId> = state
        .reference_loans
        .iter()
        .filter_map(|(binding, (owner, _))| (*owner == place).then_some(*binding))
        .collect();
    for binding in references {
        state.reference_loans.remove(&binding);
        state.consumed_ref_mut.remove(&binding);
    }
}

pub(in crate::ownership) fn end_loan(state: &mut State, place: PlaceId, loan: LoanId) {
    if let Some(loans) = state.loans.get_mut(&place) {
        loans.retain(|item| item.id != loan);
        if loans.is_empty() {
            state.loans.remove(&place);
        }
    }
}
