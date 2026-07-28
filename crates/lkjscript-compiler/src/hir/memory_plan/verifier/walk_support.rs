use super::*;
use crate::hir::{Expr, ExprKind, Operation};

pub(super) fn affine(ty: &Type) -> bool {
    matches!(ty, Type::Owned(inner) if inner.as_ref() == &Type::Buf)
        || matches!(ty, Type::Resource(_))
}

pub(super) fn uses_binding(expression: &Expr, binding: BindingId) -> bool {
    match expression.kind {
        ExprKind::Load(reference)
        | ExprKind::Move {
            binding: reference, ..
        }
        | ExprKind::Borrow {
            binding: reference, ..
        } => reference.binding == binding,
        _ => children(expression)
            .into_iter()
            .any(|child| uses_binding(child, binding)),
    }
}

pub(super) fn resource_consumed(expression: &Expr, binding: BindingId) -> bool {
    if matches!(&expression.kind, ExprKind::Move { binding: item, .. } if item.binding == binding) {
        return true;
    }
    if let ExprKind::Operation {
        operation, args, ..
    } = &expression.kind
    {
        if matches!(
            operation,
            Operation::DropResource | Operation::SysSqliteClose | Operation::SysSqliteFinalize
        ) && args.iter().any(|argument| uses_binding(argument, binding))
        {
            return true;
        }
    }
    children(expression)
        .into_iter()
        .any(|child| resource_consumed(child, binding))
}

pub(super) fn children(expression: &Expr) -> Vec<&Expr> {
    match &expression.kind {
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::Loop { body: args, .. }
        | ExprKind::ProductValue { fields: args, .. }
        | ExprKind::EnumValue { fields: args, .. } => args.iter().collect(),
        ExprKind::While {
            condition, body, ..
        } => std::iter::once(condition.as_ref())
            .chain(body.iter())
            .collect(),
        ExprKind::F64FromI64Exact(value)
        | ExprKind::F64FromI64Rounded(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value)
        | ExprKind::Return { value }
        | ExprKind::Break { value, .. }
        | ExprKind::Trap { value }
        | ExprKind::Exit { code: value }
        | ExprKind::SetLocal { value, .. }
        | ExprKind::ProductField { value, .. }
        | ExprKind::EnumIsVariant { value, .. }
        | ExprKind::EnumField { value, .. }
        | ExprKind::EnumUnwrap { value, .. } => vec![value],
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => vec![condition, then_branch, else_branch],
        ExprKind::Let { bindings, body } => bindings
            .iter()
            .map(|binding| &binding.value)
            .chain(std::iter::once(body.as_ref()))
            .collect(),
        ExprKind::MutableLocal { initial, body, .. } => vec![initial, body],
        ExprKind::WithProductField {
            value, replacement, ..
        } => vec![value, replacement],
        _ => Vec::new(),
    }
}
