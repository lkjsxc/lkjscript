use crate::source::{SourceOrigin, SourceResult};

use super::lines::Line;

pub(super) fn reject_whitespace(line: &Line<'_>, origin: &SourceOrigin) -> SourceResult<()> {
    if line.text.chars().any(char::is_whitespace) {
        return Err(super::limits::syntax_error(
            origin,
            super::lines::line_span(line),
            "lkjscript uses one column-one marker or atom per line",
        ));
    }
    Ok(())
}

pub(super) fn validate_name(
    name: &str,
    line: &Line<'_>,
    kind: &str,
    origin: &SourceOrigin,
) -> SourceResult<()> {
    if name.is_empty() {
        return Err(super::limits::syntax_error(
            origin,
            super::lines::line_span(line),
            format!("empty {kind}"),
        ));
    }
    if !is_source_identifier(name) {
        let hint = if name.contains('"') {
            "; use str/ ... /str instead of quotes"
        } else {
            ""
        };
        return Err(super::limits::syntax_error(
            origin,
            super::lines::line_span(line),
            format!("invalid {kind} {name:?}{hint}"),
        ));
    }
    Ok(())
}

pub(crate) fn is_source_identifier(name: &str) -> bool {
    !name.is_empty() && name.as_bytes().iter().copied().all(is_ident_byte)
}

fn is_ident_byte(byte: u8) -> bool {
    matches!(
        byte,
        b'a'..=b'z'
            | b'A'..=b'Z'
            | b'0'..=b'9'
            | b'_'
            | b'-'
            | b'+'
            | b'*'
            | b'='
            | b'!'
            | b'?'
            | b'<'
            | b'>'
            | b'.'
            | b':'
    )
}
