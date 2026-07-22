//! Lexer tokens for canonical, line-oriented lkjscript source.

use lkjscript_core::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// A column-one `name/` line.
    Open(String),
    /// A column-one `/name` line.
    Close(String),
    /// A complete atom line.
    Atom(String),
    /// Raw contents of `str/`, `name/`, or `import/`.
    Str(String),
}

pub fn lex(source: &str) -> Result<Vec<Token>> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let line_no = i + 1;
        if line.is_empty() || line.starts_with(";;") {
            i += 1;
            continue;
        }
        if let Some(name) = text_open_name(line) {
            let (text, next) = lex_text_block(&lines, i, name)?;
            out.push(Token::Open(name.to_string()));
            out.push(Token::Str(text));
            out.push(Token::Close(name.to_string()));
            i = next;
            continue;
        }
        reject_whitespace(line, line_no)?;
        if let Some(name) = line.strip_prefix('/') {
            validate_name(name, line_no, "close marker")?;
            out.push(Token::Close(name.to_string()));
        } else if let Some(name) = line.strip_suffix('/') {
            validate_name(name, line_no, "open marker")?;
            out.push(Token::Open(name.to_string()));
        } else {
            validate_name(line, line_no, "atom")?;
            out.push(Token::Atom(line.to_string()));
        }
        i += 1;
    }
    Ok(out)
}

fn text_open_name(line: &str) -> Option<&'static str> {
    match line {
        "str/" => Some("str"),
        "name/" => Some("name"),
        "import/" => Some("import"),
        _ => None,
    }
}

fn lex_text_block(lines: &[&str], open: usize, name: &str) -> Result<(String, usize)> {
    let close = format!("/{name}");
    let escaped_close = format!("\\{close}");
    let mut content = Vec::new();
    let mut i = open + 1;
    while let Some(&line) = lines.get(i) {
        if line == close {
            if name != "str" {
                validate_single_line_text(name, &content, open + 1)?;
            }
            return Ok((content.join("\n"), i + 1));
        }
        if line == escaped_close {
            content.push(close.clone());
        } else {
            content.push(line.to_string());
        }
        i += 1;
    }
    Err(Error::msg(format!(
        "line {}: unclosed {name}/ text block",
        open + 1
    )))
}

fn validate_single_line_text(name: &str, content: &[String], line_no: usize) -> Result<()> {
    if content.len() != 1 || content[0].is_empty() {
        return Err(Error::msg(format!(
            "line {line_no}: {name}/ needs exactly one non-empty text line"
        )));
    }
    if content[0].trim() != content[0] {
        return Err(Error::msg(format!(
            "line {}: {name}/ text cannot have surrounding whitespace",
            line_no + 1
        )));
    }
    Ok(())
}

fn reject_whitespace(line: &str, line_no: usize) -> Result<()> {
    if line.chars().any(char::is_whitespace) {
        return Err(Error::msg(format!(
            "line {line_no}: lkjscript uses one column-one marker or atom per line"
        )));
    }
    Ok(())
}

fn validate_name(name: &str, line_no: usize, kind: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::msg(format!("line {line_no}: empty {kind}")));
    }
    if !name.as_bytes().iter().copied().all(is_ident_byte) {
        let hint = if name.contains('"') {
            "; use str/ ... /str instead of quotes"
        } else {
            ""
        };
        return Err(Error::msg(format!(
            "line {line_no}: invalid {kind} {name:?}{hint}"
        )));
    }
    Ok(())
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn lexes_markers_atoms_and_text() {
        let source = "print/\nstr/\nhi world\n/str\n/print\nn\n";
        let tokens = lex(source).expect("lex");
        assert_eq!(
            tokens,
            vec![
                Token::Open("print".into()),
                Token::Open("str".into()),
                Token::Str("hi world".into()),
                Token::Close("str".into()),
                Token::Close("print".into()),
                Token::Atom("n".into()),
            ]
        );
    }

    #[test]
    fn text_blocks_preserve_lines_and_escape_the_close_marker() {
        let tokens = lex("str/\nfirst\n\\/str\nthird\n/str\n").expect("lex");
        assert_eq!(tokens[1], Token::Str("first\n/str\nthird".into()));
    }

    #[test]
    fn lexes_comment() {
        let tokens = lex(";; hi\n1\n").expect("lex");
        assert_eq!(tokens, vec![Token::Atom("1".into())]);
    }

    #[test]
    fn rejects_indentation_and_packed_tokens() {
        assert!(lex("  one\n").is_err());
        assert!(lex("one two\n").is_err());
    }

    #[test]
    fn rejects_quote_delimiters() {
        let error = lex("\"hi\"\n").expect_err("quotes must fail");
        assert!(error.as_str().contains("use str/"));
    }
}
