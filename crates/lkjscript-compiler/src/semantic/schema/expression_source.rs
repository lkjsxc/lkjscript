mod build;

use super::{Expression, ExpressionBinding, ExpressionField};
use crate::source::{SourceNode, SourceSpan, SyntaxKind};
use build::{
    call, convert_many, f64_source, leaf, name_form, symbol, unary_name, validate_user_call,
};

pub(super) fn to_source(expression: &Expression, span: SourceSpan) -> Result<SourceNode, String> {
    let source = match expression {
        Expression::Unit {} => leaf(SyntaxKind::Unit, span),
        Expression::Bool { value } => leaf(SyntaxKind::Bool { value: *value }, span),
        Expression::I64 { value } => leaf(SyntaxKind::I64 { value: *value }, span),
        Expression::F64 { value } => f64_source(value, span)?,
        Expression::String { value } => leaf(
            SyntaxKind::Str {
                value: value.clone(),
            },
            span,
        ),
        Expression::NameReference { name } => symbol(name, span)?,
        Expression::EmptyList { element } => call("empty-list", element.to_atoms(span)?, span),
        Expression::None { value_type } => call("none", value_type.to_atoms(span)?, span),
        Expression::Let { bindings, body } => let_source(bindings, body, span)?,
        Expression::Var {
            name,
            value_type,
            initial,
            body,
        } => call(
            "var",
            vec![
                name_form(name, span)?,
                call("type", value_type.to_atoms(span)?, span),
                to_source(initial, span)?,
                to_source(body, span)?,
            ],
            span,
        ),
        Expression::Set { name, value } => call(
            "set",
            vec![symbol(name, span)?, to_source(value, span)?],
            span,
        ),
        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => call(
            "if",
            vec![
                to_source(condition, span)?,
                to_source(then_branch, span)?,
                to_source(else_branch, span)?,
            ],
            span,
        ),
        Expression::While { condition, body } => {
            let mut children = vec![to_source(condition, span)?];
            children.extend(convert_many(body, span)?);
            call("while", children, span)
        }
        Expression::Do { expressions } => call("do", convert_many(expressions, span)?, span),
        Expression::Quote { name } => call("quote", vec![symbol(name, span)?], span),
        Expression::ProductValue { product, fields } => product_value(product, fields, span)?,
        Expression::Field { value, field } => call(
            "field",
            vec![to_source(value, span)?, symbol(field, span)?],
            span,
        ),
        Expression::WithField {
            value,
            field,
            replacement,
        } => call(
            "with-field",
            vec![
                to_source(value, span)?,
                symbol(field, span)?,
                to_source(replacement, span)?,
            ],
            span,
        ),
        Expression::Move { name } => unary_name("move", name, span)?,
        Expression::Borrow { name } => unary_name("borrow", name, span)?,
        Expression::BorrowMut { name } => unary_name("borrow-mut", name, span)?,
        Expression::BuiltinCall {
            operation,
            arguments,
        } => call(operation.0.name(), convert_many(arguments, span)?, span),
        Expression::UserCall { name, arguments } => {
            validate_user_call(name)?;
            call(name, convert_many(arguments, span)?, span)
        }
    };
    Ok(source)
}

fn let_source(
    bindings: &[ExpressionBinding],
    body: &Expression,
    span: SourceSpan,
) -> Result<SourceNode, String> {
    let mut children = Vec::with_capacity(bindings.len() + 1);
    for binding in bindings {
        children.push(call(
            "bind",
            vec![
                symbol(&binding.name, span)?,
                to_source(&binding.value, span)?,
            ],
            span,
        ));
    }
    children.push(to_source(body, span)?);
    Ok(call("let", children, span))
}

fn product_value(
    product: &str,
    fields: &[ExpressionField],
    span: SourceSpan,
) -> Result<SourceNode, String> {
    let mut children = vec![symbol(product, span)?];
    for field in fields {
        children.push(call(
            "field",
            vec![symbol(&field.name, span)?, to_source(&field.value, span)?],
            span,
        ));
    }
    Ok(call("product-value", children, span))
}
