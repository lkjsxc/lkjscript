//! Lexer tokens for attribute-less XML-like source.

use lkjscript2026_core::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Open(String),
    Close(String),
    Empty(String),
    Text(String),
}

pub fn lex(source: &str) -> Result<Vec<Token>> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if bytes.get(i + 1) == Some(&b'!') {
                i = skip_comment(bytes, i)?;
                continue;
            }
            i = lex_tag(bytes, i, &mut out)?;
            continue;
        }
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i] != b'<' {
            i += 1;
        }
        let raw = std::str::from_utf8(&bytes[start..i])
            .map_err(|_| Error::msg("invalid utf-8 text"))?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            if raw.contains(' ') {
                out.push(Token::Text(" ".into()));
            }
        } else {
            out.push(Token::Text(trimmed.to_string()));
        }
    }
    Ok(out)
}

fn skip_comment(bytes: &[u8], mut i: usize) -> Result<usize> {
    if !bytes[i..].starts_with(b"<!--") {
        return Err(Error::msg("bad comment opener"));
    }
    i += 4;
    while i + 2 < bytes.len() {
        if &bytes[i..i + 3] == b"-->" {
            return Ok(i + 3);
        }
        i += 1;
    }
    Err(Error::msg("unterminated comment"))
}

fn lex_tag(bytes: &[u8], mut i: usize, out: &mut Vec<Token>) -> Result<usize> {
    i += 1;
    let closing = if bytes.get(i) == Some(&b'/') {
        i += 1;
        true
    } else {
        false
    };
    let start = i;
    while i < bytes.len()
        && bytes[i] != b'>'
        && bytes[i] != b'/'
        && !bytes[i].is_ascii_whitespace()
    {
        i += 1;
    }
    if start == i {
        return Err(Error::msg("empty tag name"));
    }
    let name = std::str::from_utf8(&bytes[start..i])
        .map_err(|_| Error::msg("invalid tag utf-8"))?
        .to_string();
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if bytes.get(i) == Some(&b'/') && bytes.get(i + 1) == Some(&b'>') {
        if closing {
            return Err(Error::msg("invalid </name/>"));
        }
        out.push(Token::Empty(name));
        return Ok(i + 2);
    }
    if bytes.get(i) != Some(&b'>') {
        return Err(Error::msg(format!(
            "attributes are forbidden near <{name}>"
        )));
    }
    i += 1;
    if closing {
        out.push(Token::Close(name));
    } else {
        out.push(Token::Open(name));
    }
    Ok(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_empty_and_text() {
        let t = lex("<print>hi</print><n/>").expect("lex");
        assert_eq!(t[0], Token::Open("print".into()));
        assert_eq!(t[1], Token::Text("hi".into()));
        assert_eq!(t[2], Token::Close("print".into()));
        assert_eq!(t[3], Token::Empty("n".into()));
    }
}
