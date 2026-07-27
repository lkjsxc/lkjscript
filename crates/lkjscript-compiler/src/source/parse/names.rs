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
    if let Some(removed) = lkjscript_contracts::removed_spelling(name) {
        return Err(super::limits::syntax_error(
            origin,
            super::lines::line_span(line),
            format!("{} is removed; use {}", removed.old, removed.replacement),
        ));
    }
    if !is_source_identifier(name) {
        let message = if name.contains('"') {
            format!("invalid {kind} {name:?}; use string-literal/ ... /string-literal instead of quotes")
        } else {
            format!("invalid {kind} {name:?}; expected lowercase ASCII kebab-case")
        };
        return Err(super::limits::syntax_error(
            origin,
            super::lines::line_span(line),
            message,
        ));
    }
    Ok(())
}

pub(crate) fn is_source_identifier(name: &str) -> bool {
    lkjscript_contracts::is_identifier(name)
}

pub(super) fn is_numeric_literal_spelling(name: &str) -> bool {
    let unsigned = name.strip_prefix('-').unwrap_or(name);
    if unsigned.is_empty() {
        return false;
    }
    match unsigned.split_once('.') {
        Some((whole, fraction)) => {
            !whole.is_empty()
                && !fraction.is_empty()
                && whole.bytes().all(|byte| byte.is_ascii_digit())
                && fraction.bytes().all(|byte| byte.is_ascii_digit())
        }
        None => unsigned.bytes().all(|byte| byte.is_ascii_digit()),
    }
}
