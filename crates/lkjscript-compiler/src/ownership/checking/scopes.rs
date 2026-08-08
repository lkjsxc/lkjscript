use crate::ownership::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::ownership) fn check_scopes_expr(
    program: &Program,
    expression: &Expr,
    current: usize,
    plan: &OwnershipPlan,
    cursor: &mut ExprCursor,
    state: &mut State,
    future: &mut FutureUses,
    _context: UseContext,
) -> Result<()> {
    let parent = plan.range(current)?;
    match &expression.kind {
        ExprKind::Let { bindings, body } => {
            for local in bindings {
                let child = cursor.peek_range(plan)?;
                let checkpoint = future.push_suffix(child, parent)?;
                let initializer_context = if matches!(local.value.kind, ExprKind::Borrow { .. }) {
                    UseContext::DirectLetInitializer
                } else {
                    UseContext::Ordinary
                };
                let result = check_expr(
                    program,
                    &local.value,
                    plan,
                    cursor,
                    state,
                    future,
                    initializer_context,
                );
                future.restore(checkpoint);
                result?;
                if (!local.static_bytes
                    && is_owned(&expression_of_binding(program, local.binding)?))
                    || is_affine_resource(&expression_of_binding(program, local.binding)?)
                {
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
            check_expr(
                program,
                body,
                plan,
                cursor,
                state,
                future,
                UseContext::Ordinary,
            )?;
            for local in bindings.iter().rev() {
                end_reference_binding(state, local.binding);
                require_resource_consumed(program, local.binding, local.place, state)?;
                end_place_scope(state, local.place)?;
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
            let child = cursor.peek_range(plan)?;
            let checkpoint = future.push_suffix(child, parent)?;
            let result = check_expr(
                program,
                initial,
                plan,
                cursor,
                state,
                future,
                UseContext::Ordinary,
            );
            future.restore(checkpoint);
            result?;
            if is_owned(&expression_of_binding(program, *binding)?)
                || is_affine_resource(&expression_of_binding(program, *binding)?)
            {
                state.initialized.insert(*place, true);
            }
            check_expr(
                program,
                body,
                plan,
                cursor,
                state,
                future,
                UseContext::Ordinary,
            )?;
            end_reference_binding(state, *binding);
            require_resource_consumed(program, *binding, *place, state)?;
            end_place_scope(state, *place)?;
            state.consumed_ref_mut.remove(binding);
        }
        ExprKind::SetLocal { target, value, .. } => {
            check_expr(
                program,
                value,
                plan,
                cursor,
                state,
                future,
                UseContext::Ordinary,
            )?;
            let ty = expression_of_binding(program, *target)?;
            if is_owned(&ty) || is_affine_resource(&ty) {
                let place = plan
                    .place(*target)
                    .ok_or_else(|| Error::msg("byte-vector assignment target has no PlaceId"))?;
                expire_dead_loans_for_place(state, place, plan, None, future)?;
                if state.initialized.get(&place).copied().unwrap_or(false) {
                    return Err(Error::msg(
                        "affine assignment is only reinitialization after move or drop",
                    ));
                }
                if state
                    .loans
                    .get(&place)
                    .is_some_and(|loans| !loans.is_empty())
                {
                    return Err(Error::msg(
                        "cannot reinitialize affine value while borrowed",
                    ));
                }
                state.initialized.insert(place, true);
            }
        }
        ExprKind::ProductValue { fields, .. } => {
            check_sequence(program, fields, parent, plan, cursor, state, future)?;
        }
        ExprKind::ProductField { value, .. } => {
            check_expr(
                program,
                value,
                plan,
                cursor,
                state,
                future,
                UseContext::Ordinary,
            )?;
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            let child = cursor.peek_range(plan)?;
            let checkpoint = future.push_suffix(child, parent)?;
            let result = check_expr(
                program,
                value,
                plan,
                cursor,
                state,
                future,
                UseContext::Ordinary,
            );
            future.restore(checkpoint);
            result?;
            check_expr(
                program,
                replacement,
                plan,
                cursor,
                state,
                future,
                UseContext::Ordinary,
            )?;
        }
        ExprKind::EnumValue { fields, .. } => {
            check_sequence(program, fields, parent, plan, cursor, state, future)?;
        }
        ExprKind::EnumIsVariant { value, .. }
        | ExprKind::EnumField { value, .. }
        | ExprKind::EnumUnwrap { value, .. } => {
            check_expr(
                program,
                value,
                plan,
                cursor,
                state,
                future,
                UseContext::Ordinary,
            )?;
        }
        _ => unreachable!("ownership expression category mismatch"),
    }
    Ok(())
}

fn require_resource_consumed(
    program: &Program,
    binding: BindingId,
    place: PlaceId,
    state: &State,
) -> Result<()> {
    let ty = expression_of_binding(program, binding)?;
    if ty == Type::Resource(lkjscript_core::ResourceKind::InputStream)
        && state.initialized.get(&place) == Some(&true)
    {
        let name = program
            .binding(binding)
            .map_or("<unknown>", |binding| binding.name.as_str());
        Err(Error::msg(format!(
            "borrowed standard-input resource local {name} ({binding:?}) cannot become a guest-owned cleanup obligation"
        )))
    } else {
        Ok(())
    }
}
