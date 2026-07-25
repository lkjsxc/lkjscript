use crate::ownership::*;

pub(in crate::ownership) fn check_values_expr(
    program: &Program,
    expression: &Expr,
    places: &BTreeMap<BindingId, PlaceId>,
    state: &mut State,
    _future: &BTreeSet<BindingId>,
    context: UseContext,
) -> Result<()> {
    match &expression.kind {
        ExprKind::Load(reference) => {
            let ty = &program
                .binding(reference.binding)
                .ok_or_else(|| Error::msg("ownership load references unknown binding"))?
                .ty;
            if is_owned(ty) {
                return Err(Error::msg(
                    "Owned Buf is affine and cannot be loaded or copied; use move/ name /move",
                ));
            }
            if is_ref_mut(ty) || is_ref(ty) {
                if context != UseContext::ExactReferenceArgument {
                    return Err(Error::msg(
                        "lexical references may be used only as exact reference arguments in this slice",
                    ));
                }
                if let Some((place, loan)) = state.reference_loans.get(&reference.binding) {
                    if !state
                        .loans
                        .get(place)
                        .is_some_and(|loans| loans.iter().any(|item| item.id == *loan))
                    {
                        return Err(Error::msg("use of lexical reference after its loan ended"));
                    }
                }
                if is_ref_mut(ty) && !state.consumed_ref_mut.insert(reference.binding) {
                    return Err(Error::msg(
                        "RefMut Buf is affine and may be used only once in this slice",
                    ));
                }
            }
        }
        ExprKind::Move { place, binding } => {
            if places.get(&binding.binding) != Some(place) {
                return Err(Error::msg("move has mismatched place/binding identity"));
            }
            if !state.initialized.get(place).copied().unwrap_or(false) {
                return Err(Error::msg("use after move or double move of Owned Buf"));
            }
            if state
                .loans
                .get(place)
                .is_some_and(|loans| !loans.is_empty())
            {
                return Err(Error::msg("cannot move Owned Buf while it is borrowed"));
            }
            state.initialized.insert(*place, false);
        }
        ExprKind::Borrow {
            place,
            loan,
            kind,
            binding,
        } => {
            if !matches!(
                context,
                UseContext::ExactReferenceArgument | UseContext::DirectLetInitializer
            ) {
                return Err(Error::msg(concat!(
                    "borrow is permitted only as an exact direct reference argument or ",
                    "direct let initializer in the initial ownership slice"
                )));
            }
            if places.get(&binding.binding) != Some(place) {
                return Err(Error::msg("borrow has mismatched place/binding identity"));
            }
            if !state.initialized.get(place).copied().unwrap_or(false) {
                return Err(Error::msg("cannot borrow Owned Buf after move"));
            }
            if state.loans.values().flatten().any(|item| item.id == *loan) {
                return Err(Error::msg("duplicate LoanId in ownership facts"));
            }
            let live = state.loans.entry(*place).or_default();
            if (*kind == BorrowKind::Mutable && !live.is_empty())
                || (*kind == BorrowKind::Shared
                    && live.iter().any(|item| item.kind == BorrowKind::Mutable))
            {
                return Err(Error::msg(
                    "conflicting shared and exclusive Owned Buf loans",
                ));
            }
            live.push(Loan {
                id: *loan,
                kind: *kind,
                binding: None,
            });
        }
        _ => unreachable!("ownership expression category mismatch"),
    }
    Ok(())
}
