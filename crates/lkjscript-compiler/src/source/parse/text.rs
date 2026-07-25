use crate::source::{
    DiagnosticCategory, SourceDiagnostic, SourceOrigin, SourceResult, Token, TokenKind,
};

use super::lines::Line;

pub(super) fn lex_text_block(
    lines: &[Line<'_>],
    open: usize,
    name: &str,
    origin: &SourceOrigin,
    leading_trivia: Vec<String>,
) -> SourceResult<(Vec<Token>, usize)> {
    let close = format!("/{name}");
    let escaped_close = format!("\\{close}");
    let mut content = Vec::new();
    let mut index = open + 1;
    while let Some(line) = lines.get(index) {
        if line.text == close {
            if name != "str" {
                validate_single_line_text(name, &content, &lines[open], origin)?;
            }
            let text_start = lines
                .get(open + 1)
                .map_or(line.start, |content_line| content_line.start);
            let text_end = if index == open + 1 {
                text_start
            } else {
                lines[index - 1].content_end
            };
            let text_span = super::lines::span_at(lines, text_start, text_end);
            return Ok((
                vec![
                    Token {
                        kind: TokenKind::Open(name.to_string()),
                        span: super::lines::line_span(&lines[open]),
                        leading_trivia,
                    },
                    Token {
                        kind: TokenKind::Str(content.join("\n")),
                        span: text_span,
                        leading_trivia: Vec::new(),
                    },
                    Token {
                        kind: TokenKind::Close(name.to_string()),
                        span: super::lines::line_span(line),
                        leading_trivia: Vec::new(),
                    },
                ],
                index + 1,
            ));
        }
        if line.text == escaped_close {
            content.push(close.clone());
        } else {
            content.push(line.text.to_string());
        }
        index += 1;
    }
    Err(SourceDiagnostic::new(
        "LKJ-SRC-UNMATCHED-MARKER",
        DiagnosticCategory::SourceSyntax,
        format!("unclosed {name}/ text block; expected /{name}"),
        origin.clone(),
        super::lines::line_span(&lines[open]),
    ))
}

fn validate_single_line_text(
    name: &str,
    content: &[String],
    open: &Line<'_>,
    origin: &SourceOrigin,
) -> SourceResult<()> {
    if content.len() != 1 || content[0].is_empty() {
        return Err(super::limits::syntax_error(
            origin,
            super::lines::line_span(open),
            format!("{name}/ needs exactly one non-empty text line"),
        ));
    }
    if content[0].trim() != content[0] {
        return Err(super::limits::syntax_error(
            origin,
            super::lines::line_span(open),
            format!("{name}/ text cannot have surrounding whitespace"),
        ));
    }
    Ok(())
}
