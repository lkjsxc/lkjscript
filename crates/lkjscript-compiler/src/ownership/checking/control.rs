use crate::ownership::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::ownership) fn check_control_expr(
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
        ExprKind::Call { args, .. } => {
            for argument in args {
                if is_owned(&argument.ty) && !matches!(argument.kind, ExprKind::Move { .. }) {
                    return Err(Error::msg(
                        "byte-vector call arguments require explicit move of a whole local place",
                    ));
                }
            }
            check_arguments(program, args, parent, plan, cursor, state, future)?;
        }
        ExprKind::Operation {
            operation, args, ..
        } => {
            check_arguments(program, args, parent, plan, cursor, state, future)?;
            if matches!(
                operation,
                Operation::DropResource | Operation::SysSqliteClose | Operation::SysSqliteFinalize
            ) {
                consume_resource(args, plan, state)?;
            }
        }
        ExprKind::F64FromI64Exact(value)
        | ExprKind::F64FromI64Rounded(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value) => {
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
        ExprKind::Do(expressions) => {
            check_sequence(program, expressions, parent, plan, cursor, state, future)?;
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition_range = cursor.peek_range(plan)?;
            let checkpoint = future.push_suffix(condition_range, parent)?;
            let result = check_expr(
                program,
                condition,
                plan,
                cursor,
                state,
                future,
                UseContext::Ordinary,
            );
            future.restore(checkpoint);
            result?;

            let left_diverges = then_branch.ty == Type::Never;
            let right_diverges = else_branch.ty == Type::Never;
            let mut left = state.clone();
            let mut right = state.clone();
            check_conditional_branch(
                program,
                then_branch,
                left_diverges,
                plan,
                cursor,
                &mut left,
                future,
            )?;
            check_conditional_branch(
                program,
                else_branch,
                right_diverges,
                plan,
                cursor,
                &mut right,
                future,
            )?;
            if !left_diverges {
                expire_dead_loans(&mut left, plan, None, future)?;
            }
            if !right_diverges {
                expire_dead_loans(&mut right, plan, None, future)?;
            }
            match (left_diverges, right_diverges) {
                (true, false) => *state = right,
                (false, true) => *state = left,
                (true, true) => {}
                (false, false) if left == right => *state = left,
                (false, false) => {
                    *state = merge_conditional_cleanup(left, right, plan, future)?;
                }
            }
        }
        ExprKind::While {
            condition, body, ..
        } => {
            expire_dead_loans(state, plan, Some(parent), future)?;
            if plan.contains_ownership_action(current)?
                || plan.uses_reference_binding(current)?
                || !state.loans.is_empty()
            {
                return Err(Error::msg(
                    "loop-carried moves or loans are unsupported in the initial ownership slice",
                ));
            }
            let before = state.clone();
            check_expr(
                program,
                condition,
                plan,
                cursor,
                state,
                future,
                UseContext::Ordinary,
            )?;
            check_sequence(program, body, parent, plan, cursor, state, future)?;
            if *state != before {
                return Err(Error::msg(
                    "ownership initialization state must be equal after a loop iteration",
                ));
            }
        }
        ExprKind::Loop { body, .. } => {
            expire_dead_loans(state, plan, Some(parent), future)?;
            if plan.contains_ownership_action(current)?
                || plan.uses_reference_binding(current)?
                || !state.loans.is_empty()
            {
                return Err(Error::msg(
                    "loop-carried moves or loans are unsupported in the initial ownership slice",
                ));
            }
            check_sequence(program, body, parent, plan, cursor, state, future)?;
        }
        ExprKind::Return { value }
        | ExprKind::Break { value, .. }
        | ExprKind::Trap { value }
        | ExprKind::Exit { code: value } => {
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
        ExprKind::Continue { .. } => {}
        _ => unreachable!("ownership expression category mismatch"),
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn check_conditional_branch(
    program: &Program,
    branch: &Expr,
    diverges: bool,
    plan: &OwnershipPlan,
    cursor: &mut ExprCursor,
    state: &mut State,
    outer_future: &mut FutureUses,
) -> Result<()> {
    let branch_range = cursor.peek_range(plan)?;
    if diverges {
        let mut branch_future = FutureUses::default();
        expire_dead_loans(state, plan, Some(branch_range), &branch_future)?;
        check_expr(
            program,
            branch,
            plan,
            cursor,
            state,
            &mut branch_future,
            UseContext::Ordinary,
        )
    } else {
        expire_dead_loans(state, plan, Some(branch_range), outer_future)?;
        check_expr(
            program,
            branch,
            plan,
            cursor,
            state,
            outer_future,
            UseContext::Ordinary,
        )
    }
}

fn consume_resource(arguments: &[Expr], plan: &OwnershipPlan, state: &mut State) -> Result<()> {
    let [Expr {
        kind: ExprKind::Load(reference),
        ty: Type::Resource(_),
        ..
    }] = arguments
    else {
        return Err(Error::msg(
            "drop expects one direct affine typed resource local",
        ));
    };
    let place = plan
        .place(reference.binding)
        .ok_or_else(|| Error::msg("drop resource has no ownership place"))?;
    if state.initialized.get(&place) != Some(&true) {
        return Err(Error::msg(
            "affine typed resource was already moved or dropped",
        ));
    }
    state.initialized.insert(place, false);
    Ok(())
}
