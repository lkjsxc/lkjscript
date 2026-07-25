use crate::source::{
    DiagnosticCategory, SourceDiagnostic, SourceNode, SourceOrigin, SourceResult, SourceSpan,
    SyntaxKind, Token, TokenKind,
};

pub(super) fn parse_tokens(
    tokens: &[Token],
    origin: &SourceOrigin,
) -> SourceResult<Vec<SourceNode>> {
    let mut index = 0;
    let mut forms = Vec::new();
    while index < tokens.len() {
        let (expression, next) = parse_expr(tokens, index, origin)?;
        forms.push(expression);
        index = next;
    }
    Ok(forms)
}

pub(super) fn parse_expr(
    tokens: &[Token],
    index: usize,
    origin: &SourceOrigin,
) -> SourceResult<(SourceNode, usize)> {
    match tokens.get(index) {
        Some(Token {
            kind: TokenKind::Atom(name),
            span,
            leading_trivia,
        }) => Ok((
            super::atom::atom_from_name(name, *span, leading_trivia.clone(), origin)?,
            index + 1,
        )),
        Some(Token {
            kind: TokenKind::Str(value),
            span,
            leading_trivia,
        }) => Ok((
            SourceNode {
                kind: SyntaxKind::Str {
                    value: value.clone(),
                },
                span: *span,
                leading_trivia: leading_trivia.clone(),
                before_close_trivia: Vec::new(),
                children: Vec::new(),
            },
            index + 1,
        )),
        Some(Token {
            kind: TokenKind::Open(name),
            ..
        }) => super::element::parse_element(tokens, index, name, origin),
        Some(Token {
            kind: TokenKind::Close(name),
            span,
            ..
        }) => Err(SourceDiagnostic::new(
            "LKJ-SRC-UNMATCHED-MARKER",
            DiagnosticCategory::SourceSyntax,
            format!("unexpected close marker /{name}"),
            origin.clone(),
            *span,
        )),
        None => Err(super::limits::syntax_error(
            origin,
            SourceSpan::zero(),
            "unexpected end of input",
        )),
    }
}
