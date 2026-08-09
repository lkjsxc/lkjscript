use crate::hir::{self, Type};

pub(super) fn verifier_demands_compare(body: &hir::Expr, parameter: &str) -> bool {
    crate::stack::grow(|| verifier_demands_compare_inner(body, parameter))
}

fn verifier_demands_compare_inner(body: &hir::Expr, parameter: &str) -> bool {
    if let hir::ExprKind::Operation {
        operation: hir::Operation::EqualValue,
        args,
        ..
    } = &body.kind
    {
        return args.len() == 2
            && args
                .iter()
                .all(|value| matches!(&value.ty, Type::Param(name) if name == parameter));
    }
    verifier_expression_children(body)
        .into_iter()
        .any(|child| verifier_demands_compare(child, parameter))
}

pub(super) fn verifier_compare_only(body: &hir::Expr, parameter: &str) -> bool {
    crate::stack::grow(|| verifier_compare_only_inner(body, parameter))
}

fn verifier_compare_only_inner(body: &hir::Expr, parameter: &str) -> bool {
    match &body.kind {
        hir::ExprKind::Load(_) | hir::ExprKind::Move { .. } => {
            matches!(&body.ty, Type::Param(name) if name == parameter)
        }
        hir::ExprKind::Operation {
            operation: hir::Operation::EqualValue,
            args,
            ..
        } => {
            args.len() == 2
                && args
                    .iter()
                    .all(|value| verifier_compare_only(value, parameter))
        }
        hir::ExprKind::Do(values) => {
            !values.is_empty()
                && values
                    .iter()
                    .all(|value| verifier_compare_only(value, parameter))
        }
        hir::ExprKind::Return { value } => verifier_compare_only(value, parameter),
        _ => false,
    }
}

fn verifier_expression_children(expression: &hir::Expr) -> Vec<&hir::Expr> {
    use hir::ExprKind as Kind;
    match &expression.kind {
        Kind::Hole => Vec::new(),
        Kind::Match { .. } => {
            unreachable!("semantic matches must be lowered before memory verification")
        }
        Kind::Call { args, .. }
        | Kind::Operation { args, .. }
        | Kind::Do(args)
        | Kind::Loop { body: args, .. }
        | Kind::ProductValue { fields: args, .. }
        | Kind::EnumValue { fields: args, .. } => args.iter().collect(),
        Kind::F64FromI64Exact(value)
        | Kind::F64FromI64Rounded(value)
        | Kind::I64FromF64Exact(value)
        | Kind::I64FromF64Trunc(value)
        | Kind::Return { value }
        | Kind::Break { value, .. }
        | Kind::Trap { value }
        | Kind::Exit { code: value }
        | Kind::ProductField { value, .. }
        | Kind::EnumIsVariant { value, .. }
        | Kind::EnumField { value, .. }
        | Kind::EnumUnwrap { value, .. } => vec![value],
        Kind::If {
            condition,
            then_branch,
            else_branch,
        } => vec![condition, then_branch, else_branch],
        Kind::While {
            condition, body, ..
        } => std::iter::once(condition.as_ref()).chain(body).collect(),
        Kind::Let { bindings, body } => bindings
            .iter()
            .map(|binding| &binding.value)
            .chain(std::iter::once(body.as_ref()))
            .collect(),
        Kind::MutableLocal { initial, body, .. }
        | Kind::WithProductField {
            value: initial,
            replacement: body,
            ..
        } => vec![initial, body],
        Kind::SetLocal { value, .. } => vec![value],
        Kind::LitI64(_)
        | Kind::LitF64(_)
        | Kind::LitBool(_)
        | Kind::LitUnit
        | Kind::EmptyList
        | Kind::LitStr(_)
        | Kind::LitBytes(_)
        | Kind::Load(_)
        | Kind::Move { .. }
        | Kind::Borrow { .. }
        | Kind::BorrowBytes { .. }
        | Kind::Continue { .. }
        | Kind::MatchUnreachable { .. }
        | Kind::QuoteSymbol(_) => Vec::new(),
    }
}
