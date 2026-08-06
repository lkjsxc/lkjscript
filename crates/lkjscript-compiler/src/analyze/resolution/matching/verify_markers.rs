use crate::analyze::*;

pub(super) fn verify(program: &hir::Program) -> Result<()> {
    let mut counts = vec![0_u64; program.match_plans.len()];
    for function in &program.functions {
        expression(&function.body, program, &mut counts)?;
    }
    expression(&program.main.body, program, &mut counts)?;
    if counts.iter().any(|count| *count != 1) {
        return Err(Error::msg(
            "match plan has missing or duplicate HIR unreachable provenance",
        ));
    }
    Ok(())
}

fn expression(value: &Expr, program: &hir::Program, counts: &mut [u64]) -> Result<()> {
    crate::stack::grow(|| expression_inner(value, program, counts))
}

fn expression_inner(value: &Expr, program: &hir::Program, counts: &mut [u64]) -> Result<()> {
    match &value.kind {
        ExprKind::MatchUnreachable { plan } => {
            let index = usize::try_from(plan.raw())
                .map_err(|_| Error::msg("match unreachable plan identity exceeds usize"))?;
            let planned = program
                .match_plans
                .get(index)
                .filter(|item| item.id == *plan)
                .ok_or_else(|| Error::msg("match unreachable edge has stale plan identity"))?;
            if value.ty != Type::Never || value.origin != planned.origin {
                return Err(Error::msg(
                    "match unreachable edge has stale Never type or origin",
                ));
            }
            counts[index] = counts[index]
                .checked_add(1)
                .ok_or_else(|| Error::msg("match unreachable provenance count overflow"))?;
        }
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::ProductValue { fields: args, .. }
        | ExprKind::EnumValue { fields: args, .. } => {
            for child in args {
                expression(child, program, counts)?;
            }
        }
        ExprKind::F64FromI64Exact(child)
        | ExprKind::F64FromI64Rounded(child)
        | ExprKind::I64FromF64Exact(child)
        | ExprKind::I64FromF64Trunc(child) => expression(child, program, counts)?,
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expression(condition, program, counts)?;
            expression(then_branch, program, counts)?;
            expression(else_branch, program, counts)?;
        }
        ExprKind::While {
            condition, body, ..
        } => {
            expression(condition, program, counts)?;
            for child in body {
                expression(child, program, counts)?;
            }
        }
        ExprKind::Loop { body, .. } => {
            for child in body {
                expression(child, program, counts)?;
            }
        }
        ExprKind::Let { bindings, body } => {
            for binding in bindings {
                expression(&binding.value, program, counts)?;
            }
            expression(body, program, counts)?;
        }
        ExprKind::MutableLocal { initial, body, .. } => {
            expression(initial, program, counts)?;
            expression(body, program, counts)?;
        }
        ExprKind::Return { value }
        | ExprKind::Break { value, .. }
        | ExprKind::Trap { value }
        | ExprKind::Exit { code: value }
        | ExprKind::SetLocal { value, .. }
        | ExprKind::ProductField { value, .. }
        | ExprKind::EnumIsVariant { value, .. }
        | ExprKind::EnumField { value, .. }
        | ExprKind::EnumUnwrap { value, .. } => expression(value, program, counts)?,
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            expression(value, program, counts)?;
            expression(replacement, program, counts)?;
        }
        ExprKind::LitI64(_)
        | ExprKind::LitF64(_)
        | ExprKind::LitBool(_)
        | ExprKind::LitUnit
        | ExprKind::EmptyList
        | ExprKind::LitStr(_)
        | ExprKind::LitBytes(_)
        | ExprKind::Load(_)
        | ExprKind::Move { .. }
        | ExprKind::Borrow { .. }
        | ExprKind::BorrowBytes { .. }
        | ExprKind::Continue { .. }
        | ExprKind::QuoteSymbol(_) => {}
    }
    Ok(())
}
