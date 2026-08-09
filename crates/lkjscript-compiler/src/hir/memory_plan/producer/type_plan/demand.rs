fn body_demands_compare(body: &Expr, parameter: &str) -> bool {
    if let ExprKind::Operation {
        operation: Operation::EqualValue,
        args,
        ..
    } = &body.kind
    {
        return args.len() == 2
            && args
                .iter()
                .all(|value| matches!(&value.ty, Type::Param(name) if name == parameter));
    }
    expression_children(body)
        .into_iter()
        .any(|child| body_demands_compare(child, parameter))
}

fn body_is_compare_only(body: &Expr, parameter: &str) -> bool {
    match &body.kind {
        ExprKind::Load(_) | ExprKind::Move { .. } => {
            matches!(&body.ty, Type::Param(name) if name == parameter)
        }
        ExprKind::Operation {
            operation: Operation::EqualValue,
            args,
            ..
        } => {
            args.len() == 2
                && args
                    .iter()
                    .all(|value| body_is_compare_only(value, parameter))
        }
        ExprKind::Do(values) => {
            !values.is_empty() && values.iter().all(|value| body_is_compare_only(value, parameter))
        }
        ExprKind::Return { value } => body_is_compare_only(value, parameter),
        _ => false,
    }
}

fn expression_children(expression: &Expr) -> Vec<&Expr> {
    match &expression.kind {
        ExprKind::Hole => Vec::new(),
        ExprKind::Match { .. } => {
            unreachable!("semantic matches must be lowered before memory planning")
        }
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::Loop { body: args, .. }
        | ExprKind::ProductValue { fields: args, .. }
        | ExprKind::EnumValue { fields: args, .. } => args.iter().collect(),
        ExprKind::F64FromI64Exact(value)
        | ExprKind::F64FromI64Rounded(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value)
        | ExprKind::Return { value }
        | ExprKind::Break { value, .. }
        | ExprKind::Trap { value }
        | ExprKind::Exit { code: value }
        | ExprKind::ProductField { value, .. }
        | ExprKind::EnumIsVariant { value, .. }
        | ExprKind::EnumField { value, .. }
        | ExprKind::EnumUnwrap { value, .. } => vec![value],
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => vec![condition, then_branch, else_branch],
        ExprKind::While {
            condition, body, ..
        } => std::iter::once(condition.as_ref()).chain(body).collect(),
        ExprKind::Let { bindings, body } => bindings
            .iter()
            .map(|binding| &binding.value)
            .chain(std::iter::once(body.as_ref()))
            .collect(),
        ExprKind::MutableLocal { initial, body, .. }
        | ExprKind::WithProductField {
            value: initial,
            replacement: body,
            ..
        } => vec![initial, body],
        ExprKind::SetLocal { value, .. } => vec![value],
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
        | ExprKind::MatchUnreachable { .. }
        | ExprKind::QuoteSymbol(_) => Vec::new(),
    }
}
