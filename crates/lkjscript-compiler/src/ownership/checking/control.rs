use crate::ownership::*;

pub(in crate::ownership) fn check_control_expr(
    program: &Program,
    expression: &Expr,
    places: &BTreeMap<BindingId, PlaceId>,
    state: &mut State,
    future: &BTreeSet<BindingId>,
    _context: UseContext,
) -> Result<()> {
    match &expression.kind {
        ExprKind::Call { args, .. } => {
            for argument in args {
                if is_owned(&argument.ty) && !matches!(argument.kind, ExprKind::Move { .. }) {
                    return Err(Error::msg(
                        "byte-vector call arguments require explicit move of a whole local place",
                    ));
                }
            }
            check_arguments(program, args, places, state, future)?;
        }
        ExprKind::Operation {
            operation, args, ..
        } => {
            check_arguments(program, args, places, state, future)?;
            if matches!(
                operation,
                Operation::DropResource | Operation::SysSqliteClose | Operation::SysSqliteFinalize
            ) {
                consume_resource(args, places, state)?;
            }
        }
        ExprKind::F64FromI64Exact(value)
        | ExprKind::F64FromI64Rounded(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value) => {
            check_expr(program, value, places, state, future, UseContext::Ordinary)?;
        }
        ExprKind::Do(expressions) => check_sequence(program, expressions, places, state, future)?,
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let branch_uses = uses(then_branch)
                .union(&uses(else_branch))
                .copied()
                .collect();
            check_expr(
                program,
                condition,
                places,
                state,
                &branch_uses,
                UseContext::Ordinary,
            )?;
            let mut left = state.clone();
            let mut right = state.clone();
            check_expr(
                program,
                then_branch,
                places,
                &mut left,
                future,
                UseContext::Ordinary,
            )?;
            check_expr(
                program,
                else_branch,
                places,
                &mut right,
                future,
                UseContext::Ordinary,
            )?;
            expire_dead_loans(&mut left, future);
            expire_dead_loans(&mut right, future);
            match (then_branch.ty == Type::Never, else_branch.ty == Type::Never) {
                (true, false) => *state = right,
                (false, true) => *state = left,
                (true, true) => {}
                (false, false) if left == right => *state = left,
                (false, false) => {
                    *state = merge_conditional_cleanup(left, right, places, future)?;
                }
            }
        }
        ExprKind::While {
            condition, body, ..
        } => {
            if contains_ownership_action(condition)
                || body.iter().any(contains_ownership_action)
                || uses_reference_binding(program, condition)?
                || body.iter().try_fold(false, |found, item| {
                    Ok::<bool, Error>(found || uses_reference_binding(program, item)?)
                })?
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
                places,
                state,
                future,
                UseContext::Ordinary,
            )?;
            check_sequence(program, body, places, state, future)?;
            if *state != before {
                return Err(Error::msg(
                    "ownership initialization state must be equal after a loop iteration",
                ));
            }
        }
        ExprKind::Loop { body, .. } => {
            if body.iter().any(contains_ownership_action)
                || body.iter().try_fold(false, |found, item| {
                    Ok::<bool, Error>(found || uses_reference_binding(program, item)?)
                })?
                || !state.loans.is_empty()
            {
                return Err(Error::msg(
                    "loop-carried moves or loans are unsupported in the initial ownership slice",
                ));
            }
            check_sequence(program, body, places, state, future)?;
        }
        ExprKind::Return { value }
        | ExprKind::Break { value, .. }
        | ExprKind::Trap { value }
        | ExprKind::Exit { code: value } => {
            check_expr(program, value, places, state, future, UseContext::Ordinary)?;
        }
        ExprKind::Continue { .. } => {}
        _ => unreachable!("ownership expression category mismatch"),
    }
    Ok(())
}

fn consume_resource(
    arguments: &[Expr],
    places: &BTreeMap<BindingId, PlaceId>,
    state: &mut State,
) -> Result<()> {
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
    let place = places
        .get(&reference.binding)
        .ok_or_else(|| Error::msg("drop resource has no ownership place"))?;
    if state.initialized.get(place) != Some(&true) {
        return Err(Error::msg(
            "affine typed resource was already moved or dropped",
        ));
    }
    state.initialized.insert(*place, false);
    Ok(())
}
