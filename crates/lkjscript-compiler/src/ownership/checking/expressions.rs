use crate::ownership::*;

pub(in crate::ownership) fn check_expr(
    program: &Program,
    expression: &Expr,
    plan: &OwnershipPlan,
    cursor: &mut ExprCursor,
    state: &mut State,
    future: &mut FutureUses,
    context: UseContext,
) -> Result<()> {
    crate::stack::grow(|| {
        check_expr_inner(program, expression, plan, cursor, state, future, context)
    })
}

fn check_expr_inner(
    program: &Program,
    expression: &Expr,
    plan: &OwnershipPlan,
    cursor: &mut ExprCursor,
    state: &mut State,
    future: &mut FutureUses,
    context: UseContext,
) -> Result<()> {
    let current = cursor.enter(plan)?;
    reject_unsupported_type_placement(&expression.ty)?;
    match &expression.kind {
        ExprKind::Load(_)
        | ExprKind::Move { .. }
        | ExprKind::Borrow { .. }
        | ExprKind::BorrowBytes { .. } => {
            check_values_expr(program, expression, current, plan, state, future, context)?;
        }
        ExprKind::Call { .. }
        | ExprKind::Operation { .. }
        | ExprKind::F64FromI64Exact(_)
        | ExprKind::F64FromI64Rounded(_)
        | ExprKind::I64FromF64Exact(_)
        | ExprKind::I64FromF64Trunc(_)
        | ExprKind::Do(_)
        | ExprKind::If { .. }
        | ExprKind::While { .. }
        | ExprKind::Loop { .. }
        | ExprKind::Return { .. }
        | ExprKind::Break { .. }
        | ExprKind::Continue { .. }
        | ExprKind::Trap { .. }
        | ExprKind::Exit { .. } => {
            check_control_expr(
                program, expression, current, plan, cursor, state, future, context,
            )?;
        }
        ExprKind::Let { .. }
        | ExprKind::MutableLocal { .. }
        | ExprKind::SetLocal { .. }
        | ExprKind::ProductValue { .. }
        | ExprKind::ProductField { .. }
        | ExprKind::WithProductField { .. }
        | ExprKind::EnumValue { .. }
        | ExprKind::EnumIsVariant { .. }
        | ExprKind::EnumField { .. }
        | ExprKind::EnumUnwrap { .. } => {
            check_scopes_expr(
                program, expression, current, plan, cursor, state, future, context,
            )?;
        }
        _ => {}
    }
    Ok(())
}
