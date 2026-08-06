use lkjscript_core::Limits;

use crate::source::{
    DiagnosticCategory, SourceDiagnostic, SourceOrigin, SourceResult, SourceSpan, Token, TokenKind,
};

pub(super) fn check_nesting_safety(
    tokens: &[Token],
    limits: &Limits,
    origin: &SourceOrigin,
) -> SourceResult<()> {
    let mut depth = 0_u32;
    let mut marker_stack: Vec<(bool, bool)> = Vec::new();
    for token in tokens {
        match &token.kind {
            TokenKind::Open(name) => {
                let in_match =
                    name == "match" || marker_stack.last().is_some_and(|(_, inside)| *inside);
                let counted = !in_match || !transparent_match_marker(name);
                if counted {
                    depth = depth.saturating_add(1);
                }
                let physical_limit = limits.max_nest_depth.saturating_mul(4);
                if depth > limits.max_nest_depth || marker_stack.len() >= physical_limit as usize {
                    return Err(resource_error(
                        origin,
                        token.span,
                        format!(
                            concat!(
                                "semantic nest depth exceeded (>{}) or match marker depth ",
                                "exceeded (>{}); extract a def",
                            ),
                            limits.max_nest_depth, physical_limit
                        ),
                    ));
                }
                marker_stack.push((counted, in_match));
            }
            TokenKind::Close(_) => {
                if marker_stack.pop().is_some_and(|(counted, _)| counted) {
                    depth = depth.saturating_sub(1);
                }
            }
            TokenKind::Atom(_) | TokenKind::Str(_) | TokenKind::Bytes(_) => {}
        }
    }
    Ok(())
}

fn transparent_match_marker(name: &str) -> bool {
    matches!(
        name,
        "arms"
            | "arm"
            | "fields"
            | "variant-field"
            | "variant-field-pattern"
            | "product-field-pattern"
            | "name"
            | "type"
            | "variant"
    )
}

pub(super) fn syntax_error(
    origin: &SourceOrigin,
    span: SourceSpan,
    message: impl Into<String>,
) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-SYNTAX",
        DiagnosticCategory::SourceSyntax,
        message,
        origin.clone(),
        span,
    )
}

pub(super) fn resource_error(
    origin: &SourceOrigin,
    span: SourceSpan,
    message: impl Into<String>,
) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-LIMIT",
        DiagnosticCategory::ResourceLimit,
        message,
        origin.clone(),
        span,
    )
}

pub(super) fn allocation_error(
    origin: &SourceOrigin,
    span: SourceSpan,
    context: &str,
) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-HOST",
        DiagnosticCategory::SourceLoading,
        format!("host could not reserve memory for {context}"),
        origin.clone(),
        span,
    )
}
