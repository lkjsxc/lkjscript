use crate::hir::{Expr, ExprKind, Operation};

use super::*;

#[derive(Clone, Copy, Eq, PartialEq)]
struct CheckState {
    live: bool,
    branch_drop: bool,
}

pub(super) fn verify_drop_classes(program: &hir::Program, plan: &HirMemoryPlan) -> Result<()> {
    for obligation in &plan.obligations {
        let expected = if matches!(obligation.kind, MemoryObligationKind::EndBorrow) {
            None
        } else {
            let entry = plan
                .entry(obligation.entry)
                .ok_or_else(|| Error::msg("memory verifier drop entry is missing"))?;
            let MemorySubject::Place { binding, .. } = entry.subject else {
                return Err(Error::msg(
                    "memory verifier drop obligation is not a whole place",
                ));
            };
            let body = verified_function_body(program, obligation.function)?;
            Some(verifier_drop_class(body, BindingId::new(binding))?)
        };
        if obligation.drop_class != expected {
            return Err(Error::msg("HIR memory-plan drop classification mismatch"));
        }
    }
    Ok(())
}

fn verified_function_body(program: &hir::Program, function: MemoryFunctionId) -> Result<&Expr> {
    let index = function
        .index()
        .ok_or_else(|| Error::msg("memory verifier function identity exceeds usize"))?;
    if let Some(function) = program.functions.get(index) {
        Ok(&function.body)
    } else if index == program.functions.len() {
        Ok(&program.main.body)
    } else {
        Err(Error::msg("memory verifier function identity is missing"))
    }
}

fn verifier_drop_class(body: &Expr, binding: BindingId) -> Result<MemoryDropClass> {
    let state = check_drop_flow(
        body,
        binding,
        CheckState {
            live: true,
            branch_drop: false,
        },
    )?;
    Ok(if state.branch_drop {
        MemoryDropClass::Conditional
    } else if state.live {
        MemoryDropClass::Static
    } else {
        MemoryDropClass::Dead
    })
}

fn check_drop_flow(
    expression: &Expr,
    binding: BindingId,
    mut state: CheckState,
) -> Result<CheckState> {
    if verifier_direct_consume(expression, binding) {
        if !state.live {
            return Err(verifier_open_error());
        }
        state.live = false;
        return Ok(state);
    }
    match &expression.kind {
        ExprKind::SetLocal { target, value, .. } if *target == binding => {
            state = check_drop_flow(value, binding, state)?;
            if state.live {
                return Err(verifier_open_error());
            }
            state.live = true;
            Ok(state)
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let entry = check_drop_flow(condition, binding, state)?;
            let left = check_drop_flow(then_branch, binding, entry)?;
            let right = check_drop_flow(else_branch, binding, entry)?;
            match (then_branch.ty == Type::Never, else_branch.ty == Type::Never) {
                (true, false) => Ok(right),
                (false, true) => Ok(left),
                (true, true) => Ok(entry),
                (false, false) if left.live == right.live => Ok(CheckState {
                    live: left.live,
                    branch_drop: left.branch_drop || right.branch_drop,
                }),
                (false, false) => Ok(CheckState {
                    live: false,
                    branch_drop: true,
                }),
            }
        }
        ExprKind::While { .. } | ExprKind::Loop { .. } => {
            let after = check_drop_children(expression, binding, state)?;
            if after == state {
                Ok(state)
            } else {
                Err(verifier_open_error())
            }
        }
        _ => check_drop_children(expression, binding, state),
    }
}

fn check_drop_children(
    expression: &Expr,
    binding: BindingId,
    mut state: CheckState,
) -> Result<CheckState> {
    for child in children(expression) {
        state = check_drop_flow(child, binding, state)?;
    }
    Ok(state)
}

fn verifier_direct_consume(expression: &Expr, binding: BindingId) -> bool {
    match &expression.kind {
        ExprKind::Move { binding: moved, .. } => moved.binding == binding,
        ExprKind::Operation {
            operation:
                Operation::DropResource | Operation::SysSqliteClose | Operation::SysSqliteFinalize,
            args,
            ..
        } => args.iter().any(|argument| uses_binding(argument, binding)),
        _ => false,
    }
}

fn verifier_open_error() -> Error {
    Error::msg("memory verifier rejects an open or multiply consumed whole place")
}
