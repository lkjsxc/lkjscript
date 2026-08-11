use crate::ownership::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::ownership) fn check_arguments(
    program: &Program,
    args: &[Expr],
    parent: ExprRange,
    plan: &OwnershipPlan,
    cursor: &mut ExprCursor,
    state: &mut State,
    future: &mut FutureUses,
    control: &mut ControlFlow,
) -> Result<()> {
    let mut temporary = Vec::new();
    temporary
        .try_reserve(args.len())
        .map_err(|_| Error::host("ownership temporary-loan allocation failed"))?;
    let mut pinned = Vec::new();
    pinned
        .try_reserve(args.len())
        .map_err(|_| Error::host("ownership reference-pin allocation failed"))?;
    for argument in args {
        let child = cursor.peek_range(plan)?;
        let checkpoint = future.push_suffix(child, parent)?;
        let context = if is_ref(&argument.ty) || is_ref_mut(&argument.ty) {
            UseContext::ExactReferenceArgument
        } else {
            UseContext::Ordinary
        };
        let result = check_expr(
            program, argument, plan, cursor, state, future, control, context,
        );
        future.restore(checkpoint);
        result?;
        match argument.kind {
            ExprKind::Borrow { place, loan, .. } | ExprKind::BorrowBytes { place, loan, .. } => {
                temporary.push((place, loan));
            }
            ExprKind::Load(reference) if is_ref(&argument.ty) || is_ref_mut(&argument.ty) => {
                pin_reference(state, reference.binding)?;
                pinned.push(reference.binding);
            }
            _ => {}
        }
    }
    for (place, loan) in temporary {
        end_loan(state, place, loan);
    }
    for binding in pinned.into_iter().rev() {
        unpin_reference(state, binding)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(in crate::ownership) fn check_sequence(
    program: &Program,
    expressions: &[Expr],
    parent: ExprRange,
    plan: &OwnershipPlan,
    cursor: &mut ExprCursor,
    state: &mut State,
    future: &mut FutureUses,
    control: &mut ControlFlow,
) -> Result<()> {
    for expression in expressions {
        let child = cursor.peek_range(plan)?;
        let checkpoint = future.push_suffix(child, parent)?;
        let result = check_expr(
            program,
            expression,
            plan,
            cursor,
            state,
            future,
            control,
            UseContext::Ordinary,
        );
        future.restore(checkpoint);
        result?;
    }
    Ok(())
}
