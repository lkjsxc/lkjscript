use crate::source::{
    DiagnosticCategory, SourceDiagnostic, SourceNode, SourceOrigin, SourceResult, SourceSpan,
    SyntaxKind, Token, TokenKind,
};

struct Frame {
    name: String,
    open_span: SourceSpan,
    leading_trivia: Vec<String>,
    children: Vec<SourceNode>,
}

pub(super) fn parse_tokens(
    tokens: &[Token],
    origin: &SourceOrigin,
) -> SourceResult<Vec<SourceNode>> {
    let mut forms = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();

    for token in tokens {
        match &token.kind {
            TokenKind::Open(name) => {
                frames.try_reserve(1).map_err(|_| {
                    super::limits::allocation_error(origin, token.span, "parser frame stack")
                })?;
                frames.push(Frame {
                    name: super::limits::clone_string(
                        name,
                        origin,
                        token.span,
                        "parser marker name",
                    )?,
                    open_span: token.span,
                    leading_trivia: super::limits::clone_trivia(
                        &token.leading_trivia,
                        origin,
                        token.span,
                    )?,
                    children: Vec::new(),
                });
            }
            TokenKind::Close(close) => {
                let Some(frame) = frames.last() else {
                    return Err(SourceDiagnostic::new(
                        "LKJ-SRC-UNMATCHED-MARKER",
                        DiagnosticCategory::SourceSyntax,
                        format!("unexpected close marker /{close}"),
                        origin.clone(),
                        token.span,
                    ));
                };
                if frame.name != *close {
                    return Err(SourceDiagnostic::new(
                        "LKJ-SRC-UNMATCHED-MARKER",
                        DiagnosticCategory::SourceSyntax,
                        format!("mismatched close marker /{close}; expected /{}", frame.name),
                        origin.clone(),
                        token.span,
                    )
                    .with_related(
                        format!("opening marker {}/", frame.name),
                        origin.clone(),
                        frame.open_span,
                    ));
                }
                let frame = frames.pop().ok_or_else(|| {
                    super::limits::syntax_error(
                        origin,
                        token.span,
                        "parser frame stack became inconsistent",
                    )
                })?;
                let node = close_frame(frame, token, origin)?;
                push_node(node, &mut frames, &mut forms, origin)?;
            }
            TokenKind::Atom(name) => {
                let node = super::atom::atom_from_name(
                    name,
                    token.span,
                    super::limits::clone_trivia(&token.leading_trivia, origin, token.span)?,
                    origin,
                )?;
                push_node(node, &mut frames, &mut forms, origin)?;
            }
            TokenKind::Str(value) => {
                let node = SourceNode {
                    kind: SyntaxKind::Str {
                        value: super::limits::clone_string(
                            value,
                            origin,
                            token.span,
                            "source string literal",
                        )?,
                    },
                    span: token.span,
                    leading_trivia: super::limits::clone_trivia(
                        &token.leading_trivia,
                        origin,
                        token.span,
                    )?,
                    before_close_trivia: Vec::new(),
                    children: Vec::new(),
                };
                push_node(node, &mut frames, &mut forms, origin)?;
            }
            TokenKind::Bytes(value) => {
                let node = SourceNode {
                    kind: SyntaxKind::Bytes {
                        value: super::limits::clone_bytes(
                            value,
                            origin,
                            token.span,
                            "source bytes literal",
                        )?,
                    },
                    span: token.span,
                    leading_trivia: super::limits::clone_trivia(
                        &token.leading_trivia,
                        origin,
                        token.span,
                    )?,
                    before_close_trivia: Vec::new(),
                    children: Vec::new(),
                };
                push_node(node, &mut frames, &mut forms, origin)?;
            }
        }
    }

    if let Some(frame) = frames.last() {
        return Err(SourceDiagnostic::new(
            "LKJ-SRC-UNMATCHED-MARKER",
            DiagnosticCategory::SourceSyntax,
            format!("unclosed marker {}/; expected /{}", frame.name, frame.name),
            origin.clone(),
            frame.open_span,
        ));
    }
    Ok(forms)
}

fn push_node(
    node: SourceNode,
    frames: &mut [Frame],
    forms: &mut Vec<SourceNode>,
    origin: &SourceOrigin,
) -> SourceResult<()> {
    let (output, context) = match frames.last_mut() {
        Some(frame) => (&mut frame.children, "syntax child storage"),
        None => (forms, "top-level syntax forms"),
    };
    output
        .try_reserve(1)
        .map_err(|_| super::limits::allocation_error(origin, node.span, context))?;
    output.push(node);
    Ok(())
}

fn close_frame(frame: Frame, close: &Token, origin: &SourceOrigin) -> SourceResult<SourceNode> {
    let span = SourceSpan {
        start: frame.open_span.start,
        end: close.span.end,
    };
    let before_close_trivia =
        super::limits::clone_trivia(&close.leading_trivia, origin, close.span)?;
    if frame.name == "string-literal" {
        return match frame.children.as_slice() {
            [SourceNode {
                kind: SyntaxKind::Str { value },
                ..
            }] => Ok(SourceNode {
                kind: SyntaxKind::Str {
                    value: super::limits::clone_string(
                        value,
                        origin,
                        span,
                        "canonical string literal",
                    )?,
                },
                span,
                leading_trivia: frame.leading_trivia,
                before_close_trivia,
                children: Vec::new(),
            }),
            _ => Err(super::limits::syntax_error(
                origin,
                span,
                "string-literal/ must contain one lkjscript text value",
            )),
        };
    }
    if frame.name == "bytes-literal" {
        return match frame.children.as_slice() {
            [SourceNode {
                kind: SyntaxKind::Bytes { value },
                ..
            }] => Ok(SourceNode {
                kind: SyntaxKind::Bytes {
                    value: super::limits::clone_bytes(
                        value,
                        origin,
                        span,
                        "canonical bytes literal",
                    )?,
                },
                span,
                leading_trivia: frame.leading_trivia,
                before_close_trivia,
                children: Vec::new(),
            }),
            _ => Err(super::limits::syntax_error(
                origin,
                span,
                "bytes-literal/ must contain one hexadecimal payload",
            )),
        };
    }
    Ok(SourceNode {
        kind: SyntaxKind::Call { name: frame.name },
        span,
        leading_trivia: frame.leading_trivia,
        before_close_trivia,
        children: frame.children,
    })
}
