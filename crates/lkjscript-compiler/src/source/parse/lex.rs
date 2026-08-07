use crate::source::{SourceOrigin, SourceResult, SourceSpan, Token, TokenKind};

use super::lines::Line;

pub(super) struct Lexed {
    pub(super) tokens: Vec<Token>,
}

pub(super) fn lex(lines: &[Line<'_>], origin: &SourceOrigin) -> SourceResult<Lexed> {
    let mut output = Vec::new();
    output.try_reserve(lines.len()).map_err(|_| {
        super::limits::allocation_error(origin, SourceSpan::zero(), "source token storage")
    })?;
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
            if name == "bytes-literal" {
                decode_bytes_literal(&mut text_tokens, origin)?;
            }
            output.try_reserve(text_tokens.len()).map_err(|_| {
                super::limits::allocation_error(
                    origin,
                    super::lines::line_span(line),
                    "source token storage",
                )
            })?;
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
            if !super::names::is_numeric_literal_spelling(line.text) {
                super::names::validate_name(line.text, line, "atom", origin)?;
            }
            output.push(Token {
                kind: TokenKind::Atom(line.text.to_string()),
                span: super::lines::line_span(line),
                leading_trivia: std::mem::take(&mut pending_trivia),
            });
        }
        index += 1;
    }
    Ok(Lexed { tokens: output })
}

fn text_open_name(line: &str) -> Option<&'static str> {
    match line {
        "string-literal/" => Some("string-literal"),
        "bytes-literal/" => Some("bytes-literal"),
        "name/" => Some("name"),
        "module/" => Some("module"),
        _ => None,
    }
}

fn decode_bytes_literal(tokens: &mut [Token], origin: &SourceOrigin) -> SourceResult<()> {
    let Some(Token {
        kind: TokenKind::Str(payload),
        span,
        ..
    }) = tokens.get_mut(1)
    else {
        return Err(super::limits::syntax_error(
            origin,
            crate::source::SourceSpan::zero(),
            "bytes-literal/ has no payload token",
        ));
    };
    if payload.contains('\n') {
        return Err(super::limits::syntax_error(
            origin,
            *span,
            "bytes-literal/ payload must occupy exactly one line",
        ));
    }
    if payload.len() % 2 != 0 {
        return Err(super::limits::syntax_error(
            origin,
            *span,
            "bytes-literal/ payload must contain an even number of hexadecimal digits",
        ));
    }
    if !payload
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(super::limits::syntax_error(
            origin,
            *span,
            "bytes-literal/ payload accepts only lowercase hexadecimal digits",
        ));
    }
    let decoded_len = payload.len() / 2;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(decoded_len).map_err(|_| {
        super::limits::allocation_error(origin, *span, "decoded bytes-literal payload")
    })?;
    for pair in payload.as_bytes().chunks_exact(2) {
        bytes.push((hex(pair[0]) << 4) | hex(pair[1]));
    }
    tokens[1].kind = TokenKind::Bytes(bytes);
    Ok(())
}

fn hex(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("validated hexadecimal digit"),
    }
}
