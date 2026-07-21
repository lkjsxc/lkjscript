//! Lexer tokens for slash/whitespace source.

use lkjscript2026_core::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// `name/`
    Open(String),
    /// `/name`
    Close(String),
    /// bare atom (symbol, number literal spelling, bool, nil)
    Atom(String),
    /// `"…"` string literal (unescaped contents)
    Str(String),
}

pub fn lex(source: &str) -> Result<Vec<Token>> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if b == b';' && bytes.get(i + 1) == Some(&b';') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b == b'"' {
            let (s, next) = lex_string(bytes, i)?;
            out.push(Token::Str(s));
            i = next;
            continue;
        }
        if b == b'/' {
            let (name, next) = lex_ident(bytes, i + 1)?;
            if name.is_empty() {
                return Err(Error::msg("close marker needs a name after /"));
            }
            out.push(Token::Close(name));
            i = next;
            continue;
        }
        let (name, next) = lex_ident(bytes, i)?;
        if name.is_empty() {
            return Err(Error::msg(format!("unexpected byte {:?}", b as char)));
        }
        if bytes.get(next) == Some(&b'/') {
            out.push(Token::Open(name));
            i = next + 1;
        } else {
            out.push(Token::Atom(name));
            i = next;
        }
    }
    Ok(out)
}

fn lex_ident(bytes: &[u8], mut i: usize) -> Result<(String, usize)> {
    let start = i;
    while i < bytes.len() && is_ident_byte(bytes[i]) {
        i += 1;
    }
    let name = std::str::from_utf8(&bytes[start..i])
        .map_err(|_| Error::msg("invalid utf-8 ident"))?
        .to_string();
    Ok((name, i))
}

fn is_ident_byte(b: u8) -> bool {
    matches!(
        b,
        b'a'..=b'z'
            | b'A'..=b'Z'
            | b'0'..=b'9'
            | b'_'
            | b'-'
            | b'+'
            | b'*'
            | b'='
            | b'!'
            | b'?'
            | b'<'
            | b'>'
            | b'.'
            | b':'
    )
}

fn lex_string(bytes: &[u8], mut i: usize) -> Result<(String, usize)> {
    i += 1; // opening quote
    let mut out = String::new();
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Ok((out, i + 1)),
            b'\\' => {
                i += 1;
                let Some(&esc) = bytes.get(i) else {
                    return Err(Error::msg("unterminated string escape"));
                };
                match esc {
                    b'\\' => out.push('\\'),
                    b'"' => out.push('"'),
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    other => {
                        return Err(Error::msg(format!(
                            "unknown string escape \\{}",
                            other as char
                        )))
                    }
                }
                i += 1;
            }
            b => {
                out.push(b as char);
                i += 1;
            }
        }
    }
    Err(Error::msg("unterminated string"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_open_close_atom_string() {
        let t = lex(r#"print/ "hi" /print n"#).expect("lex");
        assert_eq!(t[0], Token::Open("print".into()));
        assert_eq!(t[1], Token::Str("hi".into()));
        assert_eq!(t[2], Token::Close("print".into()));
        assert_eq!(t[3], Token::Atom("n".into()));
    }

    #[test]
    fn lexes_comment() {
        let t = lex(";; hi\n1").expect("lex");
        assert_eq!(t, vec![Token::Atom("1".into())]);
    }
}
