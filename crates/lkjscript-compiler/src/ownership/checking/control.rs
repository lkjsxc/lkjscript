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
    control: &mut ControlFlow,
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
            check_arguments(program, args, parent, plan, cursor, state, future, control)?;
        }
        ExprKind::Operation {
            operation, args, ..
        } => {
            check_arguments(program, args, parent, plan, cursor, state, future, control)?;
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
                control,
                UseContext::Ordinary,
            )?;
        }
        ExprKind::Do(expressions) => {
            check_sequence(
                program,
                expressions,
                parent,
                plan,
                cursor,
                state,
                future,
                control,
            )?;
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
                control,
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
                control,
            )?;
            check_conditional_branch(
                program,
                else_branch,
                right_diverges,
                plan,
                cursor,
                &mut right,
                future,
                control,
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
            loop_id,
            condition,
            body,
        } => {
            prepare_loop_entry(current, parent, plan, state, future)?;
            let entry = state.clone();
            check_expr(
                program,
                condition,
                plan,
                cursor,
                state,
                future,
                control,
                UseContext::Ordinary,
            )?;
            if project_loop_transfer_state(state, &entry) != entry {
                return Err(loop_state_error());
            }
            control.enter(*loop_id, entry.clone())?;
            check_sequence(program, body, parent, plan, cursor, state, future, control)?;
            if body_falls_through(body) && *state != entry {
                return Err(loop_state_error());
            }
            control.exit(*loop_id)?;
            *state = entry;
        }
        ExprKind::Loop { loop_id, body, .. } => {
            prepare_loop_entry(current, parent, plan, state, future)?;
            let entry = state.clone();
            control.enter(*loop_id, entry.clone())?;
            check_sequence(program, body, parent, plan, cursor, state, future, control)?;
            if body_falls_through(body) && *state != entry {
                return Err(loop_state_error());
            }
            control.exit(*loop_id)?;
            *state = entry;
        }
        ExprKind::Break { loop_id, value } => {
            check_expr(
                program,
                value,
                plan,
                cursor,
                state,
                future,
                control,
                UseContext::Ordinary,
            )?;
            control.check_transfer(*loop_id, state)?;
        }
        ExprKind::Continue { loop_id } => {
            control.check_transfer(*loop_id, state)?;
        }
        ExprKind::Return { value } | ExprKind::Trap { value } | ExprKind::Exit { code: value } => {
            check_expr(
                program,
                value,
                plan,
                cursor,
                state,
                future,
                control,
                UseContext::Ordinary,
            )?;
        }
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
    control: &mut ControlFlow,
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
            control,
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
            control,
            UseContext::Ordinary,
        )
    }
}

fn prepare_loop_entry(
    current: usize,
    parent: ExprRange,
    plan: &OwnershipPlan,
    state: &mut State,
    future: &FutureUses,
) -> Result<()> {
    expire_dead_loans(state, plan, Some(parent), future)?;
    if plan.uses_reference_binding(current)? || !state.loans.is_empty() {
        return Err(Error::msg(
            "loop-carried lexical loans are unsupported in the initial ownership slice",
        ));
    }
    Ok(())
}

fn body_falls_through(body: &[Expr]) -> bool {
    body.last()
        .is_none_or(|expression| expression.ty != Type::Never)
}

impl ControlFlow {
    fn enter(&mut self, target: LoopId, entry: State) -> Result<()> {
        if self.loops.iter().any(|frame| frame.target == target) {
            return Err(Error::msg(
                "ownership checker found a duplicate active lexical loop identity",
            ));
        }
        self.loops
            .try_reserve(1)
            .map_err(|_| Error::host("ownership loop-context allocation failed"))?;
        self.loops.push(LoopOwnership { target, entry });
        Ok(())
    }

    fn check_transfer(&self, target: LoopId, state: &State) -> Result<()> {
        let frame = self
            .loops
            .last()
            .filter(|frame| frame.target == target)
            .ok_or_else(invalid_loop_target)?;
        if project_loop_transfer_state(state, &frame.entry) != frame.entry {
            return Err(loop_state_error());
        }
        Ok(())
    }

    fn exit(&mut self, target: LoopId) -> Result<()> {
        let frame = self.loops.pop().ok_or_else(invalid_loop_target)?;
        if frame.target != target {
            return Err(invalid_loop_target());
        }
        Ok(())
    }
}

fn project_loop_transfer_state(state: &State, entry: &State) -> State {
    let mut projected = state.clone();
    projected
        .initialized
        .retain(|place, _| entry.initialized.contains_key(place));
    for place in entry.initialized.keys() {
        projected.initialized.entry(*place).or_insert(false);
    }
    projected
        .loans
        .retain(|place, _| entry.initialized.contains_key(place));
    for loans in projected.loans.values_mut() {
        loans.retain(|loan| loan.binding.is_none());
    }
    projected.loans.retain(|_, loans| !loans.is_empty());
    projected
        .reference_loans
        .retain(|binding, _| entry.reference_loans.contains_key(binding));
    projected
        .pinned_references
        .retain(|binding, _| entry.pinned_references.contains_key(binding));
    projected
        .consumed_ref_mut
        .retain(|binding| entry.consumed_ref_mut.contains(binding));
    projected
}

fn invalid_loop_target() -> Error {
    Error::msg("ownership control transfer does not target the nearest active lexical loop")
}

fn loop_state_error() -> Error {
    Error::msg(
        "loop-carried ownership initialization state must be equal after an iteration or local exit",
    )
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
