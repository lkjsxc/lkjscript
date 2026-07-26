pub(crate) const EDITION_MARKER: &str = "edition/\n2\n/edition\n";

use crate::source::{
    DiagnosticCategory, SourceDiagnostic, SourceNode, SourceOrigin, SourceResult, SourceSpan,
    SyntaxKind, Token, TokenKind,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SourceEdition {
    Edition1,
    Edition2,
}

impl SourceEdition {
    pub const fn number(self) -> u32 {
        match self {
            Self::Edition1 => 1,
            Self::Edition2 => 2,
        }
    }
}

pub(crate) fn validate_marker(
    source: &str,
    forms: &mut [SourceNode],
    tokens: &[Token],
    origin: &SourceOrigin,
) -> SourceResult<SourceEdition> {
    let mut marker_index = None;
    for (index, form) in forms.iter().enumerate() {
        if is_edition_form(form) {
            if marker_index.is_some() {
                return Err(marker_error(
                    origin,
                    form.span,
                    "duplicate Edition 2 marker",
                ));
            }
            marker_index = Some(index);
        }
    }
    let Some(index) = marker_index else {
        if let Some(form) = forms.iter().find(|form| is_edition2_only_form(form)) {
            return Err(marker_error(
                origin,
                form.span,
                "Edition 2 syntax requires the exact first edition marker",
            ));
        }
        return Ok(SourceEdition::Edition1);
    };
    if !exact_marker(source, &forms[index], tokens) {
        return Err(marker_error(
            origin,
            forms[index].span,
            "malformed Edition 2 marker",
        ));
    }
    if index != 0 {
        return Err(marker_error(
            origin,
            forms[index].span,
            "Edition 2 marker must be the first semantic form",
        ));
    }
    forms[index].kind = SyntaxKind::EditionMarker;
    Ok(SourceEdition::Edition2)
}

fn is_edition_form(node: &SourceNode) -> bool {
    matches!(&node.kind, SyntaxKind::Call { name } if name == "edition")
}

fn is_edition2_only_form(node: &SourceNode) -> bool {
    matches!(&node.kind, SyntaxKind::Call { name } if name == "enum")
}

fn exact_marker(source: &str, node: &SourceNode, tokens: &[Token]) -> bool {
    if !node.before_close_trivia.is_empty()
        || !matches!(
            node.children.as_slice(),
            [SourceNode {
                kind: SyntaxKind::I64 { value: 2 },
                leading_trivia,
                before_close_trivia,
                children,
                ..
            }] if leading_trivia.is_empty() && before_close_trivia.is_empty() && children.is_empty()
        )
    {
        return false;
    }
    let start = node.span.start().byte() as usize;
    let exact_source = source
        .as_bytes()
        .get(start..start.saturating_add(EDITION_MARKER.len()))
        == Some(EDITION_MARKER.as_bytes());
    exact_source && tokens.windows(3).any(|window| {
        matches!(
            window,
            [Token { kind: TokenKind::Open(open), span: open_span, .. },
             Token { kind: TokenKind::Atom(value), leading_trivia, .. },
             Token { kind: TokenKind::Close(close), span: close_span, leading_trivia: close_trivia }]
                if open == "edition"
                    && value == "2"
                    && close == "edition"
                    && open_span.start() == node.span.start()
                    && close_span.end() == node.span.end()
                    && leading_trivia.is_empty()
                    && close_trivia.is_empty()
        )
    })
}

fn marker_error(origin: &SourceOrigin, span: SourceSpan, message: &str) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-EDITION",
        DiagnosticCategory::SourceSyntax,
        message,
        origin.clone(),
        span,
    )
}
