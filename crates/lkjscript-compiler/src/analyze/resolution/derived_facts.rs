use crate::analyze::*;

pub(in crate::analyze) fn fold_effects(expressions: &[Expr]) -> EffectSet {
    expressions
        .iter()
        .fold(EffectSet::PURE, |effects, expression| {
            effects.union(expression.effects)
        })
}

pub(in crate::analyze) fn callable_arity(ty: &Type) -> Option<usize> {
    match ty {
        Type::Fn { params, .. } => Some(params.len()),
        Type::Forall { body, .. } => callable_arity(body),
        _ => None,
    }
}

pub(in crate::analyze) fn is_contextual_name(name: &str) -> bool {
    matches!(
        name,
        "if" | "while"
            | "do"
            | "let"
            | "var"
            | "quote"
            | "set"
            | "move"
            | "borrow"
            | "borrow-mut"
            | "empty-list"
            | "none"
            | "product"
            | "fields"
            | "field"
            | "product-value"
            | "with-field"
            | "bind"
            | "fn"
            | "def"
            | "main"
            | "sig"
            | "params"
            | "forall"
            | "bounds"
            | "bound"
            | "trait"
            | "impl"
            | "for"
            | "type"
            | "import"
            | "name"
    )
}
