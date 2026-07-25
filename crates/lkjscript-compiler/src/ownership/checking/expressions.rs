use crate::ownership::*;

pub(in crate::ownership) fn check_expr(
    program: &Program,
    expression: &Expr,
    places: &BTreeMap<BindingId, PlaceId>,
    state: &mut State,
    future: &BTreeSet<BindingId>,
    context: UseContext,
) -> Result<()> {
    expire_dead_loans(state, &uses(expression).union(future).copied().collect());
    reject_unsupported_type_placement(&expression.ty)?;
    match &expression.kind {
        ExprKind::Load(_) | ExprKind::Move { .. } | ExprKind::Borrow { .. } => {
            check_values_expr(program, expression, places, state, future, context)?;
        }
        ExprKind::Call { .. }
        | ExprKind::Operation { .. }
        | ExprKind::Do(_)
        | ExprKind::If { .. }
        | ExprKind::While { .. } => {
            check_control_expr(program, expression, places, state, future, context)?;
        }
        ExprKind::Let { .. }
        | ExprKind::MutableLocal { .. }
        | ExprKind::SetLocal { .. }
        | ExprKind::ProductValue { .. }
        | ExprKind::ProductField { .. }
        | ExprKind::WithProductField { .. } => {
            check_scopes_expr(program, expression, places, state, future, context)?;
        }
        _ => {}
    }
    Ok(())
}
