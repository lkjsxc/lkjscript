use crate::hir::{Expr, ExprKind, Type};

use super::resolution::relevant;

#[derive(Clone, Debug)]
pub(super) struct ResolvedOperation {
    pub file: usize,
    pub name: &'static str,
    pub argument_types: Vec<Type>,
}

pub(super) fn collect_resolved(expression: &Expr, output: &mut Vec<ResolvedOperation>) {
    if let ExprKind::Operation {
        operation, args, ..
    } = &expression.kind
    {
        if let Some(name) = relevant(*operation) {
            output.push(ResolvedOperation {
                file: expression.origin.raw() as usize,
                name,
                argument_types: args.iter().map(|argument| argument.ty.clone()).collect(),
            });
        }
    }
    collect_children(expression, output);
}

fn collect_children(expression: &Expr, output: &mut Vec<ResolvedOperation>) {
    match &expression.kind {
        ExprKind::Call { args, .. } | ExprKind::Operation { args, .. } => {
            collect_many(args, output)
        }
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
        | ExprKind::EnumUnwrap { value, .. } => collect_resolved(value, output),
        ExprKind::Do(expressions)
        | ExprKind::Loop {
            body: expressions, ..
        } => {
            collect_many(expressions, output);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_resolved(condition, output);
            collect_resolved(then_branch, output);
            collect_resolved(else_branch, output);
        }
        ExprKind::While {
            condition, body, ..
        } => {
            collect_resolved(condition, output);
            collect_many(body, output);
        }
        ExprKind::Let { bindings, body } => {
            for binding in bindings {
                collect_resolved(&binding.value, output);
            }
            collect_resolved(body, output);
        }
        ExprKind::MutableLocal { initial, body, .. } => {
            collect_resolved(initial, output);
            collect_resolved(body, output);
        }
        ExprKind::ProductValue { fields, .. } | ExprKind::EnumValue { fields, .. } => {
            collect_many(fields, output);
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            collect_resolved(value, output);
            collect_resolved(replacement, output);
        }
        ExprKind::LitI64(_)
        | ExprKind::LitF64(_)
        | ExprKind::LitBool(_)
        | ExprKind::LitUnit
        | ExprKind::EmptyList
        | ExprKind::LitStr(_)
        | ExprKind::Load(_)
        | ExprKind::Move { .. }
        | ExprKind::Borrow { .. }
        | ExprKind::Continue { .. }
        | ExprKind::MatchUnreachable { .. }
        | ExprKind::QuoteSymbol(_) => {}
    }
}

fn collect_many(expressions: &[Expr], output: &mut Vec<ResolvedOperation>) {
    for expression in expressions {
        collect_resolved(expression, output);
    }
}
