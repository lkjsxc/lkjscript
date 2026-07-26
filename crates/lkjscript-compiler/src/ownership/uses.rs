use crate::ownership::*;

pub(in crate::ownership) fn uses(expression: &Expr) -> BTreeSet<BindingId> {
    let mut output = BTreeSet::new();
    collect_uses(expression, &mut output);
    output
}

pub(in crate::ownership) fn collect_uses(expression: &Expr, output: &mut BTreeSet<BindingId>) {
    match &expression.kind {
        ExprKind::Load(reference)
        | ExprKind::Move {
            binding: reference, ..
        } => {
            output.insert(reference.binding);
        }
        ExprKind::Borrow { binding, .. } => {
            output.insert(binding.binding);
        }
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::While { body: args, .. } => {
            for item in args {
                collect_uses(item, output);
            }
            if let ExprKind::While { condition, .. } = &expression.kind {
                collect_uses(condition, output);
            }
        }
        ExprKind::F64FromI64Exact(value)
        | ExprKind::F64FromI64Rounded(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value) => collect_uses(value, output),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_uses(condition, output);
            collect_uses(then_branch, output);
            collect_uses(else_branch, output);
        }
        ExprKind::Let { bindings, body } => {
            for binding in bindings {
                collect_uses(&binding.value, output);
            }
            collect_uses(body, output);
        }
        ExprKind::MutableLocal { initial, body, .. } => {
            collect_uses(initial, output);
            collect_uses(body, output);
        }
        ExprKind::SetLocal { value, .. } | ExprKind::ProductField { value, .. } => {
            collect_uses(value, output);
        }
        ExprKind::ProductValue { fields, .. } => {
            for field in fields {
                collect_uses(field, output);
            }
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            collect_uses(value, output);
            collect_uses(replacement, output);
        }
        _ => {}
    }
}

pub(in crate::ownership) fn uses_bindings(
    bindings: &[crate::hir::LocalDefinition],
    body: &Expr,
    future: &BTreeSet<BindingId>,
) -> BTreeSet<BindingId> {
    let mut result = future.clone();
    for binding in bindings {
        result.extend(uses(&binding.value));
    }
    result.extend(uses(body));
    result
}

pub(in crate::ownership) fn contains_ownership_action(expression: &Expr) -> bool {
    if matches!(
        expression.kind,
        ExprKind::Move { .. } | ExprKind::Borrow { .. }
    ) {
        return true;
    }
    let mut actions = false;
    walk_children(expression, &mut |child| {
        actions |= contains_ownership_action(child);
    });
    actions
}

pub(in crate::ownership) fn walk_children(expression: &Expr, action: &mut impl FnMut(&Expr)) {
    match &expression.kind {
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::While { body: args, .. } => {
            for child in args {
                action(child);
            }
            if let ExprKind::While { condition, .. } = &expression.kind {
                action(condition);
            }
        }
        ExprKind::F64FromI64Exact(value)
        | ExprKind::F64FromI64Rounded(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value) => action(value),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            action(condition);
            action(then_branch);
            action(else_branch);
        }
        ExprKind::Let { bindings, body } => {
            for binding in bindings {
                action(&binding.value);
            }
            action(body);
        }
        ExprKind::MutableLocal { initial, body, .. } => {
            action(initial);
            action(body);
        }
        ExprKind::SetLocal { value, .. } | ExprKind::ProductField { value, .. } => action(value),
        ExprKind::ProductValue { fields, .. } => {
            for field in fields {
                action(field);
            }
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            action(value);
            action(replacement);
        }
        _ => {}
    }
}
