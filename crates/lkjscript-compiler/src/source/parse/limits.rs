use lkjscript_core::Limits;

use crate::source::{
    DiagnosticCategory, SourceDiagnostic, SourceOrigin, SourceResult, SourceSpan, Token, TokenKind,
};

pub(super) fn check_file_limits(
    tokens: &[Token],
    limits: &Limits,
    origin: &SourceOrigin,
) -> SourceResult<()> {
    let token_count = u32::try_from(tokens.len()).unwrap_or(u32::MAX);
    if token_count > limits.max_tokens_per_file {
        return Err(resource_error(
            origin,
            tokens
                .first()
                .map_or(SourceSpan::zero(), |token| token.span),
            format!(
                "token budget exceeded ({token_count} > {}); split via import",
                limits.max_tokens_per_file
            ),
        ));
    }
    let mut depth = 0_u32;
    let mut child_stack = Vec::new();
    let mut marker_stack: Vec<(bool, bool)> = Vec::new();
    for token in tokens {
        match &token.kind {
            TokenKind::Open(name) => {
                count_child(&mut child_stack, limits, origin, token.span)?;
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
                child_stack.push(0_u32);
                marker_stack.push((counted, in_match));
            }
            TokenKind::Close(_) => {
                child_stack.pop();
                if marker_stack.pop().is_some_and(|(counted, _)| counted) {
                    depth = depth.saturating_sub(1);
                }
            }
            TokenKind::Atom(_) | TokenKind::Str(_) | TokenKind::Bytes(_) => {
                count_child(&mut child_stack, limits, origin, token.span)?;
            }
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

fn count_child(
    stack: &mut [u32],
    limits: &Limits,
    origin: &SourceOrigin,
    span: SourceSpan,
) -> SourceResult<()> {
    if let Some(count) = stack.last_mut() {
        *count = count.saturating_add(1);
        if *count > limits.max_children {
            return Err(resource_error(
                origin,
                span,
                format!(
                    "too many children (>{}); split args / extract helper",
                    limits.max_children
                ),
            ));
        }
    }
    Ok(())
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
