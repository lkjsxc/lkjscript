//! One parser for format-string structure. Source normalization resolves names
//! and the elaborator consumes the same descriptors with positional arguments.
use crate::ast::{Diagnostic, Span};
#[derive(Clone)]
pub(crate) struct Specification {
    pub width: usize,
    pub flags: u8,
    pub fill: char,
    pub method: String,
}
#[derive(Clone)]
pub(crate) enum Part {
    Text(String),
    Value {
        argument: String,
        specification: Specification,
        spelling: String,
    },
}
fn invalid(span: Span, message: &str) -> Diagnostic {
    Diagnostic::new(span, "E_FORMAT", message)
}
fn specification(text: &str, span: Span) -> Result<Specification, Diagnostic> {
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0usize;
    let mut result = Specification {
        width: 0,
        flags: 0,
        fill: ' ',
        method: "display".to_string(),
    };
    let alignment = |value| match value {
        '<' => 8u8,
        '>' => 16u8,
        '^' => 24u8,
        _ => 0u8,
    };
    if chars.len() >= 2 && alignment(chars[1]) != 0 {
        result.fill = chars[0];
        result.flags |= alignment(chars[1]);
        at = 2;
    } else if chars.first().is_some_and(|value| alignment(*value) != 0) {
        result.flags |= alignment(chars[0]);
        at = 1;
    }
    if chars.get(at) == Some(&'+') {
        result.flags |= 1;
        at += 1;
    } else if chars.get(at) == Some(&'-') {
        at += 1;
    }
    if chars.get(at) == Some(&'#') {
        result.flags |= 2;
        at += 1;
    }
    if chars.get(at) == Some(&'0') {
        result.flags |= 4;
        at += 1;
    }
    while let Some(value) = chars.get(at) {
        if !value.is_ascii_digit() {
            break;
        }
        result.flags |= 32; // An explicitly written width may itself be zero.
        result.width = result
            .width
            .checked_mul(10)
            .and_then(|width| width.checked_add(*value as usize - '0' as usize))
            .ok_or_else(|| invalid(span, "format width does not fit usize"))?;
        at += 1;
    }
    if at < chars.len() {
        result.method = match chars[at] {
            'x' => "lower_hex",
            'X' => "upper_hex",
            'b' => "binary",
            'o' => "octal",
            '?' => return Err(Diagnostic::new(
                span,
                "E_UNSUPPORTED",
                "Debug formatting needs its source trait implementation",
            )),
            _ => return Err(Diagnostic::new(
                span,
                "E_UNSUPPORTED",
                "format specification requires a source formatting contract not implemented yet",
            )),
        }.to_string();
        at += 1;
    }
    if at != chars.len() {
        return Err(Diagnostic::new(
            span,
            "E_UNSUPPORTED",
            "dynamic format widths and precision are not implemented yet",
        ));
    }
    Ok(result)
}
pub(crate) fn parse(text: &str, span: Span) -> Result<Vec<Part>, Diagnostic> {
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0usize;
    let mut literal = String::new();
    let mut parts = Vec::new();
    while at < chars.len() {
        if chars[at] == '{' {
            if chars.get(at + 1) == Some(&'{') {
                literal.push('{');
                at += 2;
                continue;
            }
            if !literal.is_empty() {
                parts.push(Part::Text(std::mem::take(&mut literal)));
            }
            at += 1;
            let start = at;
            while at < chars.len() && chars[at] != '}' {
                if chars[at] == '{' {
                    return Err(invalid(span, "nested opening brace in a format argument"));
                }
                at += 1;
            }
            if at == chars.len() {
                return Err(invalid(span, "unclosed format argument"));
            }
            let field: String = chars[start..at].iter().collect();
            at += 1;
            let (argument, spelling) = field.split_once(':').unwrap_or((&field, ""));
            let argument = argument.trim();
            if !argument
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                return Err(invalid(span, "format argument must be an index or name"));
            }
            parts.push(Part::Value {
                argument: argument.to_string(),
                specification: specification(spelling, span)?,
                spelling: spelling.to_string(),
            });
        } else if chars[at] == '}' {
            if chars.get(at + 1) != Some(&'}') {
                return Err(invalid(span, "unmatched closing brace in format string"));
            }
            literal.push('}');
            at += 2;
        } else {
            literal.push(chars[at]);
            at += 1;
        }
    }
    if !literal.is_empty() {
        parts.push(Part::Text(literal));
    }
    Ok(parts)
}
