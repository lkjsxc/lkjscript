use crate::ownership::*;

pub(in crate::ownership) fn collect_places(
    expression: &Expr,
    output: &mut BTreeMap<BindingId, PlaceId>,
) {
    match &expression.kind {
        ExprKind::Let { bindings, body } => {
            for binding in bindings {
                output.insert(binding.binding, binding.place);
                collect_places(&binding.value, output);
            }
            collect_places(body, output);
        }
        ExprKind::MutableLocal {
            binding,
            place,
            initial,
            body,
            ..
        } => {
            output.insert(*binding, *place);
            collect_places(initial, output);
            collect_places(body, output);
        }
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::While { body: args, .. } => {
            for item in args {
                collect_places(item, output);
            }
            if let ExprKind::While { condition, .. } = &expression.kind {
                collect_places(condition, output);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_places(condition, output);
            collect_places(then_branch, output);
            collect_places(else_branch, output);
        }
        ExprKind::SetLocal { value, .. } | ExprKind::ProductField { value, .. } => {
            collect_places(value, output);
        }
        ExprKind::ProductValue { fields, .. } => {
            for field in fields {
                collect_places(field, output);
            }
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            collect_places(value, output);
            collect_places(replacement, output);
        }
        _ => {}
    }
}
