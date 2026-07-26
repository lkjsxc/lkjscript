use super::Expression;

pub(super) fn requires_edition2(expression: &Expression) -> bool {
    match expression {
        Expression::Loop { .. }
        | Expression::Return { .. }
        | Expression::Break { .. }
        | Expression::Continue {}
        | Expression::Trap { .. }
        | Expression::Exit { .. } => true,
        Expression::Let { bindings, body } => {
            bindings
                .iter()
                .any(|binding| requires_edition2(&binding.value))
                || requires_edition2(body)
        }
        Expression::Var { initial, body, .. } => {
            requires_edition2(initial) || requires_edition2(body)
        }
        Expression::Set { value, .. } | Expression::Field { value, .. } => requires_edition2(value),
        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => {
            requires_edition2(condition)
                || requires_edition2(then_branch)
                || requires_edition2(else_branch)
        }
        Expression::While { condition, body } => {
            requires_edition2(condition) || body.iter().any(requires_edition2)
        }
        Expression::Do { expressions } => expressions.iter().any(requires_edition2),
        Expression::ProductValue { fields, .. } | Expression::VariantValue { fields, .. } => {
            fields.iter().any(|field| requires_edition2(&field.value))
        }
        Expression::Match { scrutinee, arms } => {
            requires_edition2(scrutinee) || arms.iter().any(|arm| requires_edition2(&arm.body))
        }
        Expression::WithField {
            value, replacement, ..
        } => requires_edition2(value) || requires_edition2(replacement),
        Expression::BuiltinCall {
            operation,
            arguments,
        } => operation.0.edition2_only() || arguments.iter().any(requires_edition2),
        Expression::UserCall { arguments, .. } => arguments.iter().any(requires_edition2),
        _ => false,
    }
}
