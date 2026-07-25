use crate::source::{SourceNode, SourceOrigin, SourceResult, SourceSpan, SyntaxKind};

pub(super) fn atom_from_name(
    name: &str,
    span: SourceSpan,
    leading_trivia: Vec<String>,
    origin: &SourceOrigin,
) -> SourceResult<SourceNode> {
    let kind = if name == "unit" {
        SyntaxKind::Unit
    } else if name == "nil" {
        return Err(super::limits::syntax_error(
            origin,
            span,
            "nil was removed; use unit, none/ T /none, or empty-list/ T /empty-list",
        ));
    } else if name == "true" {
        SyntaxKind::Bool { value: true }
    } else if name == "false" {
        SyntaxKind::Bool { value: false }
    } else {
        let unsigned = name.strip_prefix('-').unwrap_or(name);
        if is_ascii_digits(unsigned) {
            let value = name.parse::<i64>().map_err(|_| {
                super::limits::syntax_error(
                    origin,
                    span,
                    format!("I64 literal out of range: {name}"),
                )
            })?;
            SyntaxKind::I64 { value }
        } else if let Some((whole, fraction)) = unsigned.split_once('.') {
            if is_ascii_digits(whole) && is_ascii_digits(fraction) && !fraction.contains('.') {
                let value = name.parse::<f64>().map_err(|_| {
                    super::limits::syntax_error(
                        origin,
                        span,
                        format!("invalid F64 literal: {name}"),
                    )
                })?;
                if !value.is_finite() {
                    return Err(super::limits::syntax_error(
                        origin,
                        span,
                        format!("F64 literal must be finite: {name}"),
                    ));
                }
                SyntaxKind::F64 { value }
            } else if looks_numeric(name) {
                return Err(super::limits::syntax_error(
                    origin,
                    span,
                    format!("invalid numeric literal: {name}"),
                ));
            } else {
                SyntaxKind::Symbol {
                    name: name.to_string(),
                }
            }
        } else if looks_numeric(name) || is_non_finite_spelling(name) {
            return Err(super::limits::syntax_error(
                origin,
                span,
                format!("invalid numeric literal: {name}"),
            ));
        } else {
            SyntaxKind::Symbol {
                name: name.to_string(),
            }
        }
    };
    Ok(SourceNode {
        kind,
        span,
        leading_trivia,
        before_close_trivia: Vec::new(),
        children: Vec::new(),
    })
}

fn is_ascii_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn looks_numeric(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.first().is_some_and(u8::is_ascii_digit)
        || matches!(bytes, [b'-' | b'+', digit, ..] if digit.is_ascii_digit())
        || matches!(bytes, [b'-' | b'+', b'.'])
        || matches!(bytes, [b'-' | b'+', b'.', digit, ..] if digit.is_ascii_digit())
        || matches!(bytes, [b'-' | b'+', b'-' | b'+', digit, ..] if digit.is_ascii_digit() || *digit == b'.')
        || matches!(bytes, [b'.'])
        || matches!(bytes, [b'.', digit, ..] if digit.is_ascii_digit())
}

fn is_non_finite_spelling(text: &str) -> bool {
    matches!(
        text.to_ascii_lowercase().as_str(),
        "nan" | "inf" | "infinity" | "-nan" | "-inf" | "-infinity" | "+nan" | "+inf" | "+infinity"
    )
}
