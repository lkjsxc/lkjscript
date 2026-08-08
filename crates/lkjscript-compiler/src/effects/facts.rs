use super::*;

pub(super) fn recompute_expr(expression: &mut Expr, summaries: &[Option<EffectSet>]) -> EffectSet {
    crate::stack::grow(|| recompute_expr_inner(expression, summaries))
}

fn recompute_expr_inner(expression: &mut Expr, summaries: &[Option<EffectSet>]) -> EffectSet {
    let effects = match &mut expression.kind {
        ExprKind::Hole => EffectSet::UNKNOWN,
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
        | ExprKind::MatchUnreachable { .. }
        | ExprKind::QuoteSymbol(_) => EffectSet::PURE,
        ExprKind::Call { callee, args, .. } => {
            let callee_effects = if callee.storage == BindingStorage::Function {
                callee
                    .binding
                    .index()
                    .and_then(|index| summaries.get(index))
                    .copied()
                    .flatten()
                    .unwrap_or(EffectSet::CONSERVATIVE_CALL)
            } else {
                EffectSet::CONSERVATIVE_CALL
            };
            recompute_slice(args, summaries).union(callee_effects)
        }
        ExprKind::Operation {
            operation, args, ..
        } => recompute_slice(args, summaries).union(operation.effects()),
        ExprKind::F64FromI64Rounded(value) => recompute_expr(value, summaries),
        ExprKind::F64FromI64Exact(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value) => {
            recompute_expr(value, summaries).union(EffectSet::ALLOCATES)
        }
        ExprKind::Do(expressions) => recompute_slice(expressions, summaries),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => recompute_expr(condition, summaries)
            .union(recompute_expr(then_branch, summaries))
            .union(recompute_expr(else_branch, summaries)),
        ExprKind::While {
            condition, body, ..
        } => recompute_expr(condition, summaries)
            .union(recompute_slice(body, summaries))
            .union(EffectSet::MAY_DIVERGE),
        ExprKind::Loop { body, .. } => {
            recompute_slice(body, summaries).union(EffectSet::MAY_DIVERGE)
        }
        ExprKind::Return { value } | ExprKind::Break { value, .. } => {
            recompute_expr(value, summaries).union(EffectSet::MAY_DIVERGE)
        }
        ExprKind::Continue { .. } => EffectSet::MAY_DIVERGE,
        ExprKind::Trap { value } => recompute_expr(value, summaries).union(EffectSet::MAY_TRAP),
        ExprKind::Exit { code } => recompute_expr(code, summaries)
            .union(EffectSet::HOST_IO)
            .union(EffectSet::MAY_EXIT),
        ExprKind::Let { bindings, body } => bindings
            .iter_mut()
            .fold(EffectSet::PURE, |effects, binding| {
                effects.union(recompute_expr(&mut binding.value, summaries))
            })
            .union(recompute_expr(body, summaries)),
        ExprKind::MutableLocal { initial, body, .. } => {
            recompute_expr(initial, summaries).union(recompute_expr(body, summaries))
        }
        ExprKind::SetLocal { value, .. } => {
            recompute_expr(value, summaries).union(EffectSet::MUTATES_LOCAL)
        }
        ExprKind::ProductValue { fields, .. } | ExprKind::EnumValue { fields, .. } => {
            recompute_slice(fields, summaries).union(EffectSet::ALLOCATES)
        }
        ExprKind::ProductField { value, .. }
        | ExprKind::EnumIsVariant { value, .. }
        | ExprKind::EnumField { value, .. }
        | ExprKind::EnumUnwrap { value, .. } => {
            recompute_expr(value, summaries).union(EffectSet::READS_MEMORY)
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => recompute_expr(value, summaries)
            .union(recompute_expr(replacement, summaries))
            .union(EffectSet::READS_MEMORY)
            .union(EffectSet::ALLOCATES),
    };
    expression.effects = effects;
    effects
}

pub(super) fn recompute_slice(
    expressions: &mut [Expr],
    summaries: &[Option<EffectSet>],
) -> EffectSet {
    expressions
        .iter_mut()
        .fold(EffectSet::PURE, |effects, expression| {
            effects.union(recompute_expr(expression, summaries))
        })
}
