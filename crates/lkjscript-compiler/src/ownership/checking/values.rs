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
            if is_affine_resource(ty) {
                let place = places
                    .get(&reference.binding)
                    .ok_or_else(|| Error::msg("affine typed resource has no ownership place"))?;
                if state.initialized.get(place) != Some(&true) {
                    return Err(Error::msg(
                        "affine typed resource was already moved or dropped",
                    ));
                }
            }
            let static_bytes = program
                .binding(reference.binding)
                .is_some_and(|binding| binding.kind == BindingKind::StaticBytesLocal);
            if is_owned(ty) && !static_bytes {
                return Err(Error::msg(
                    "byte-vector is affine and cannot be loaded or copied; use move/ name /move",
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
                        "byte-slice-mut is affine and may be used only once in this slice",
                    ));
                }
            }
        }
        ExprKind::Move { place, binding } => {
            if places.get(&binding.binding) != Some(place) {
                return Err(Error::msg("move has mismatched place/binding identity"));
            }
            if !state.initialized.get(place).copied().unwrap_or(false) {
                return Err(Error::msg("use after move or double move of affine value"));
            }
            if state
                .loans
                .get(place)
                .is_some_and(|loans| !loans.is_empty())
            {
                return Err(Error::msg("cannot move affine value while it is borrowed"));
            }
            state.initialized.insert(*place, false);
        }
        ExprKind::BorrowBytes {
            place,
            loan,
            binding,
        } => {
            if places.get(&binding.binding) != Some(place) {
                return Err(Error::msg(
                    "bytes borrow has mismatched place/binding identity",
                ));
            }
            if !state.initialized.get(place).copied().unwrap_or(false) {
                return Err(Error::msg("cannot borrow dynamic bytes after move"));
            }
            if state.loans.values().flatten().any(|item| item.id == *loan) {
                return Err(Error::msg("duplicate LoanId in bytes ownership facts"));
            }
            let live = state.loans.entry(*place).or_default();
            if live.iter().any(|item| item.kind == BorrowKind::Mutable) {
                return Err(Error::msg("dynamic bytes conflicts with an exclusive loan"));
            }
            live.push(Loan {
                id: *loan,
                kind: BorrowKind::Shared,
                binding: None,
            });
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
                return Err(Error::msg("cannot borrow byte-vector after move"));
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
                    "conflicting shared and exclusive byte-vector loans",
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
