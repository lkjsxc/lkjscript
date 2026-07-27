use super::{Expression, SourceNode, SourceSpan, SyntaxKind};

pub(super) fn f64_source(value: &str, span: SourceSpan) -> Result<SourceNode, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("invalid canonical F64 value {value:?}"))?;
    if !parsed.is_finite() || crate::source::format_f64(parsed) != value {
        return Err(format!("non-canonical or non-finite F64 value {value:?}"));
    }
    Ok(leaf(SyntaxKind::F64 { value: parsed }, span))
}

pub(super) fn unary_name(marker: &str, name: &str, span: SourceSpan) -> Result<SourceNode, String> {
    Ok(call(marker, vec![symbol(name, span)?], span))
}

pub(super) fn name_form(name: &str, span: SourceSpan) -> Result<SourceNode, String> {
    validate_name(name)?;
    Ok(call(
        "name",
        vec![leaf(
            SyntaxKind::Str {
                value: name.to_string(),
            },
            span,
        )],
        span,
    ))
}

pub(super) fn symbol(name: &str, span: SourceSpan) -> Result<SourceNode, String> {
    validate_name(name)?;
    Ok(leaf(
        SyntaxKind::Symbol {
            name: name.to_string(),
        },
        span,
    ))
}

fn validate_name(name: &str) -> Result<(), String> {
    if crate::source::is_source_identifier(name)
        && lkjscript_contracts::removed_spelling(name).is_none()
    {
        Ok(())
    } else {
        Err(format!("invalid source name {name:?}"))
    }
}

pub(super) fn validate_user_call(name: &str) -> Result<(), String> {
    validate_name(name)?;
    if crate::hir::Operation::from_name(name).is_some() || RESERVED_FORMS.contains(&name) {
        Err(format!("{name:?} is not a user function call name"))
    } else {
        Ok(())
    }
}

pub(super) fn convert_many(
    values: &[Expression],
    span: SourceSpan,
) -> Result<Vec<SourceNode>, String> {
    values
        .iter()
        .map(|value| super::to_source(value, span))
        .collect()
}

pub(super) fn call(name: &str, children: Vec<SourceNode>, span: SourceSpan) -> SourceNode {
    leaf_with_children(
        SyntaxKind::Call {
            name: name.to_string(),
        },
        children,
        span,
    )
}

pub(super) fn leaf(kind: SyntaxKind, span: SourceSpan) -> SourceNode {
    leaf_with_children(kind, Vec::new(), span)
}

fn leaf_with_children(kind: SyntaxKind, children: Vec<SourceNode>, span: SourceSpan) -> SourceNode {
    SourceNode {
        kind,
        span,
        leading_trivia: Vec::new(),
        before_close_trivia: Vec::new(),
        children,
    }
}

const RESERVED_FORMS: &[&str] = &[
    "def",
    "main",
    "import",
    "product",
    "trait",
    "impl",
    "name",
    "fn",
    "sig",
    "params",
    "forall",
    "bounds",
    "bound",
    "type",
    "fields",
    "for",
    "field",
    "bind",
    "let",
    "var",
    "set",
    "if",
    "while",
    "loop",
    "return",
    "break",
    "continue",
    "trap",
    "exit",
    "do",
    "quote",
    "move",
    "borrow",
    "borrow-mut",
    "product-value",
    "with-field",
    "empty-list",
    "none",
    "hole",
    "goal",
];
