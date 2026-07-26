use super::{build, to_source};
use crate::semantic::schema::{
    Expression, ExpressionField, MatchExpressionArm, MatchPatternExpression, MatchPatternField,
    TypeExpression,
};
use crate::source::{SourceNode, SourceSpan};

pub(super) fn variant_value(
    value_type: &TypeExpression,
    variant: &str,
    fields: &[ExpressionField],
    span: SourceSpan,
) -> Result<SourceNode, String> {
    let fields = fields
        .iter()
        .map(|field| {
            Ok(build::call(
                "variant-field",
                vec![
                    build::name_form(&field.name, span)?,
                    to_source(&field.value, span)?,
                ],
                span,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(build::call(
        "variant-value",
        vec![
            type_form(value_type, span)?,
            named("variant", variant, span)?,
            build::call("fields", fields, span),
        ],
        span,
    ))
}

pub(super) fn match_expression(
    scrutinee: &Expression,
    arms: &[MatchExpressionArm],
    span: SourceSpan,
) -> Result<SourceNode, String> {
    if arms.is_empty() {
        return Err("match requires at least one arm".into());
    }
    let arms = arms
        .iter()
        .map(|arm| {
            Ok(build::call(
                "arm",
                vec![pattern(&arm.pattern, span)?, to_source(&arm.body, span)?],
                span,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(build::call(
        "match",
        vec![to_source(scrutinee, span)?, build::call("arms", arms, span)],
        span,
    ))
}

fn pattern(value: &MatchPatternExpression, span: SourceSpan) -> Result<SourceNode, String> {
    Ok(match value {
        MatchPatternExpression::Wildcard {} => build::call("wildcard", Vec::new(), span),
        MatchPatternExpression::Binding { name } => {
            build::call("binding", vec![build::name_form(name, span)?], span)
        }
        MatchPatternExpression::Bool { value } => build::call(
            "bool-pattern",
            vec![build::leaf(
                crate::source::SyntaxKind::Bool { value: *value },
                span,
            )],
            span,
        ),
        MatchPatternExpression::I64 { value } => build::call(
            "i64-pattern",
            vec![build::leaf(
                crate::source::SyntaxKind::I64 { value: *value },
                span,
            )],
            span,
        ),
        MatchPatternExpression::Variant {
            value_type,
            variant,
            fields,
        } => build::call(
            "variant-pattern",
            vec![
                type_form(value_type, span)?,
                named("variant", variant, span)?,
                pattern_fields("variant-field-pattern", fields, span)?,
            ],
            span,
        ),
        MatchPatternExpression::Product { value_type, fields } => build::call(
            "product-pattern",
            vec![
                type_form(value_type, span)?,
                pattern_fields("product-field-pattern", fields, span)?,
            ],
            span,
        ),
    })
}

fn pattern_fields(
    marker: &str,
    fields: &[MatchPatternField],
    span: SourceSpan,
) -> Result<SourceNode, String> {
    let fields = fields
        .iter()
        .map(|field| {
            Ok(build::call(
                marker,
                vec![
                    build::name_form(&field.name, span)?,
                    pattern(&field.pattern, span)?,
                ],
                span,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(build::call("fields", fields, span))
}

fn type_form(value: &TypeExpression, span: SourceSpan) -> Result<SourceNode, String> {
    Ok(build::call("type", value.to_atoms(span)?, span))
}

fn named(marker: &str, value: &str, span: SourceSpan) -> Result<SourceNode, String> {
    Ok(build::call(marker, vec![build::symbol(value, span)?], span))
}
