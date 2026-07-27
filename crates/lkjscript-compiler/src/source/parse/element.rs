use crate::source::{
    DiagnosticCategory, SourceDiagnostic, SourceNode, SourceOrigin, SourceResult, SourceSpan,
    SyntaxKind, Token, TokenKind,
};

pub(super) fn parse_element(
    tokens: &[Token],
    index: usize,
    name: &str,
    origin: &SourceOrigin,
) -> SourceResult<(SourceNode, usize)> {
    let open_span = tokens[index].span;
    let leading_trivia = tokens[index].leading_trivia.clone();
    let mut cursor = index + 1;
    let mut children = Vec::new();
    loop {
        match tokens.get(cursor) {
            Some(Token {
                kind: TokenKind::Close(close),
                span: close_span,
                ..
            }) if close == name => {
                let span = SourceSpan {
                    start: open_span.start,
                    end: close_span.end,
                };
                let before_close_trivia = tokens[cursor].leading_trivia.clone();
                if name == "string-literal" {
                    return match children.as_slice() {
                        [SourceNode {
                            kind: SyntaxKind::Str { value },
                            ..
                        }] => Ok((
                            SourceNode {
                                kind: SyntaxKind::Str {
                                    value: value.clone(),
                                },
                                span,
                                leading_trivia,
                                before_close_trivia,
                                children: Vec::new(),
                            },
                            cursor + 1,
                        )),
                        _ => Err(super::limits::syntax_error(
                            origin,
                            span,
                            "string-literal/ must contain one lkjscript text value",
                        )),
                    };
                }
                return Ok((
                    SourceNode {
                        kind: SyntaxKind::Call {
                            name: name.to_string(),
                        },
                        span,
                        leading_trivia,
                        before_close_trivia,
                        children,
                    },
                    cursor + 1,
                ));
            }
            Some(Token {
                kind: TokenKind::Close(other),
                span,
                ..
            }) => {
                return Err(SourceDiagnostic::new(
                    "LKJ-SRC-UNMATCHED-MARKER",
                    DiagnosticCategory::SourceSyntax,
                    format!("mismatched close marker /{other}; expected /{name}"),
                    origin.clone(),
                    *span,
                )
                .with_related(
                    format!("opening marker {name}/"),
                    origin.clone(),
                    open_span,
                ));
            }
            None => {
                return Err(SourceDiagnostic::new(
                    "LKJ-SRC-UNMATCHED-MARKER",
                    DiagnosticCategory::SourceSyntax,
                    format!("unclosed marker {name}/; expected /{name}"),
                    origin.clone(),
                    open_span,
                ));
            }
            _ => {
                let (child, next) = super::syntax::parse_expr(tokens, cursor, origin)?;
                children.push(child);
                cursor = next;
            }
        }
    }
}
