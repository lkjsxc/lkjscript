use crate::hir::Type;
use crate::semantic::schema::Expression;

pub(super) fn required(
    tree: &crate::source::ValidatedSourceTree,
    expression: &Expression,
) -> Vec<String> {
    let parameters = match expression {
        Expression::BuiltinCall { operation, .. } => match &operation.0.signature() {
            Type::Fn { params, .. } => Some(params.clone()),
            _ => None,
        },
        Expression::UserCall { name, .. } => super::super::scope::function_signatures(tree)
            .into_iter()
            .find_map(|(candidate, params, _)| (candidate == *name).then_some(params)),
        _ => None,
    };
    parameters
        .unwrap_or_default()
        .into_iter()
        .filter_map(|ty| match ty {
            Type::Capability(kind) => Some(kind.as_str().to_string()),
            _ => None,
        })
        .collect()
}
