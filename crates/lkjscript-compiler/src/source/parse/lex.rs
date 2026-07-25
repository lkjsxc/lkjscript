use crate::source::{SourceOrigin, SourceResult, Token, TokenKind};

use super::lines::Line;

pub(super) struct Lexed {
    pub(super) tokens: Vec<Token>,
    pub(super) trailing_trivia: Vec<String>,
}

pub(super) fn lex(lines: &[Line<'_>], origin: &SourceOrigin) -> SourceResult<Lexed> {
    let mut output = Vec::new();
    let mut pending_trivia = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = &lines[index];
        if line.text.is_empty() || line.text.starts_with(";;") {
            pending_trivia.push(line.text.to_string());
            index += 1;
            continue;
        }
        if let Some(name) = text_open_name(line.text) {
            let (mut text_tokens, next) = super::text::lex_text_block(
                lines,
                index,
                name,
                origin,
                std::mem::take(&mut pending_trivia),
            )?;
            output.append(&mut text_tokens);
            index = next;
            continue;
        }
        super::names::reject_whitespace(line, origin)?;
        if let Some(name) = line.text.strip_prefix('/') {
            super::names::validate_name(name, line, "close marker", origin)?;
            output.push(Token {
                kind: TokenKind::Close(name.to_string()),
                span: super::lines::line_span(line),
                leading_trivia: std::mem::take(&mut pending_trivia),
            });
        } else if let Some(name) = line.text.strip_suffix('/') {
            super::names::validate_name(name, line, "open marker", origin)?;
            output.push(Token {
                kind: TokenKind::Open(name.to_string()),
                span: super::lines::line_span(line),
                leading_trivia: std::mem::take(&mut pending_trivia),
            });
        } else {
            super::names::validate_name(line.text, line, "atom", origin)?;
            output.push(Token {
                kind: TokenKind::Atom(line.text.to_string()),
                span: super::lines::line_span(line),
                leading_trivia: std::mem::take(&mut pending_trivia),
            });
        }
        index += 1;
    }
    Ok(Lexed {
        tokens: output,
        trailing_trivia: pending_trivia,
    })
}

fn text_open_name(line: &str) -> Option<&'static str> {
    match line {
        "str/" => Some("str"),
        "name/" => Some("name"),
        "import/" => Some("import"),
        _ => None,
    }
}
