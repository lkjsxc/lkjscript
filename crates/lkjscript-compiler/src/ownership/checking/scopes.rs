use crate::ownership::*;

pub(in crate::ownership) fn check_scopes_expr(
    program: &Program,
    expression: &Expr,
    places: &BTreeMap<BindingId, PlaceId>,
    state: &mut State,
    future: &BTreeSet<BindingId>,
    _context: UseContext,
) -> Result<()> {
    match &expression.kind {
        ExprKind::Let { bindings, body } => {
            for (index, local) in bindings.iter().enumerate() {
                let later = uses_bindings(&bindings[index.saturating_add(1)..], body, future);
                let initializer_context = if matches!(local.value.kind, ExprKind::Borrow { .. }) {
                    UseContext::DirectLetInitializer
                } else {
                    UseContext::Ordinary
                };
                check_expr(
                    program,
                    &local.value,
                    places,
                    state,
                    &later,
                    initializer_context,
                )?;
                if is_owned(&expression_of_binding(program, local.binding)?) {
                    state.initialized.insert(local.place, true);
                }
                if let ExprKind::Borrow {
                    place, loan, kind, ..
                } = local.value.kind
                {
                    if state
                        .reference_loans
                        .insert(local.binding, (place, loan))
                        .is_some()
                    {
                        return Err(Error::msg("duplicate local reference loan binding"));
                    }
                    if let Some(item) = state
                        .loans
                        .get_mut(&place)
                        .and_then(|loans| loans.iter_mut().find(|item| item.id == loan))
                    {
                        item.binding = Some(local.binding);
                        item.kind = kind;
                    }
                }
            }
            check_expr(program, body, places, state, future, UseContext::Ordinary)?;
            for local in bindings.iter().rev() {
                end_reference_binding(state, local.binding);
                end_place_scope(state, local.place);
                state.consumed_ref_mut.remove(&local.binding);
            }
        }
        ExprKind::MutableLocal {
            binding,
            place,
            initial,
            body,
            ..
        } => {
            check_expr(
                program,
                initial,
                places,
                state,
                &uses(body),
                UseContext::Ordinary,
            )?;
            if is_owned(&expression_of_binding(program, *binding)?) {
                state.initialized.insert(*place, true);
            }
            check_expr(program, body, places, state, future, UseContext::Ordinary)?;
            end_reference_binding(state, *binding);
            end_place_scope(state, *place);
            state.consumed_ref_mut.remove(binding);
        }
        ExprKind::SetLocal { target, value, .. } => {
            check_expr(program, value, places, state, future, UseContext::Ordinary)?;
            let ty = expression_of_binding(program, *target)?;
            if is_owned(&ty) {
                let place = places
                    .get(target)
                    .ok_or_else(|| Error::msg("Owned Buf assignment target has no PlaceId"))?;
                if state.initialized.get(place).copied().unwrap_or(false) {
                    return Err(Error::msg(
                        "Owned Buf var assignment is only reinitialization after move in this slice",
                    ));
                }
                if state
                    .loans
                    .get(place)
                    .is_some_and(|loans| !loans.is_empty())
                {
                    return Err(Error::msg("cannot reinitialize Owned Buf while borrowed"));
                }
                state.initialized.insert(*place, true);
            }
        }
        ExprKind::ProductValue { fields, .. } => {
            check_sequence(program, fields, places, state, future)?;
        }
        ExprKind::ProductField { value, .. } => {
            check_expr(program, value, places, state, future, UseContext::Ordinary)?;
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            check_expr(
                program,
                value,
                places,
                state,
                &uses(replacement),
                UseContext::Ordinary,
            )?;
            check_expr(
                program,
                replacement,
                places,
                state,
                future,
                UseContext::Ordinary,
            )?;
        }
        _ => unreachable!("ownership expression category mismatch"),
    }
    Ok(())
}
