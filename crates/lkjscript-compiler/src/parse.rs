//! Parse tokens into expression forms.

use crate::ast::Expr;
use crate::lex::Token;
use lkjscript_core::{Error, Result};

pub fn parse_tokens(tokens: &[Token]) -> Result<Vec<Expr>> {
    let mut i = 0;
    let mut forms = Vec::new();
    while i < tokens.len() {
        let (expr, next) = parse_expr(tokens, i)?;
        forms.push(expr);
        i = next;
    }
    Ok(forms)
}

fn parse_expr(tokens: &[Token], i: usize) -> Result<(Expr, usize)> {
    match tokens.get(i) {
        Some(Token::Atom(name)) => Ok((atom_from_name(name)?, i + 1)),
        Some(Token::Str(t)) => Ok((Expr::LitStr(t.clone()), i + 1)),
        Some(Token::Open(name)) => parse_element(tokens, i, name),
        Some(Token::Close(name)) => Err(Error::msg(format!("unexpected /{name}"))),
        None => Err(Error::msg("unexpected end of input")),
    }
}

fn parse_element(tokens: &[Token], i: usize, name: &str) -> Result<(Expr, usize)> {
    let mut j = i + 1;
    let mut kids = Vec::new();
    loop {
        match tokens.get(j) {
            Some(Token::Close(close)) if close == name => {
                return Ok((element_to_expr(name, kids)?, j + 1));
            }
            Some(Token::Close(other)) => {
                return Err(Error::msg(format!(
                    "mismatched close /{other}, expected /{name}"
                )));
            }
            None => return Err(Error::msg(format!("unclosed {name}/"))),
            _ => {
                let (kid, next) = parse_expr(tokens, j)?;
                kids.push(kid);
                j = next;
            }
        }
    }
}

fn element_to_expr(name: &str, kids: Vec<Expr>) -> Result<Expr> {
    if name == "str" {
        return match kids.as_slice() {
            [Expr::LitStr(s)] => Ok(Expr::LitStr(s.clone())),
            _ => Err(Error::msg("str/ must contain one lkjscript text value")),
        };
    }
    if kids.len() == 1 {
        if let Expr::LitStr(s) = &kids[0] {
            if name == "name" || name == "import" {
                return Ok(Expr::Call {
                    name: name.into(),
                    args: vec![Expr::LitStr(s.clone())],
                });
            }
        }
    }
    Ok(Expr::Call {
        name: name.into(),
        args: kids,
    })
}

fn atom_from_name(name: &str) -> Result<Expr> {
    if name == "nil" {
        return Ok(Expr::LitNil);
    }
    if name == "true" {
        return Ok(Expr::LitBool(true));
    }
    if name == "false" {
        return Ok(Expr::LitBool(false));
    }

    let unsigned = name.strip_prefix('-').unwrap_or(name);
    if is_ascii_digits(unsigned) {
        return name
            .parse::<i64>()
            .map(Expr::LitI64)
            .map_err(|_| Error::msg(format!("I64 literal out of range: {name}")));
    }
    if let Some((whole, fraction)) = unsigned.split_once('.') {
        if is_ascii_digits(whole) && is_ascii_digits(fraction) && !fraction.contains('.') {
            let value = name
                .parse::<f64>()
                .map_err(|_| Error::msg(format!("invalid F64 literal: {name}")))?;
            if !value.is_finite() {
                return Err(Error::msg(format!("F64 literal must be finite: {name}")));
            }
            return Ok(Expr::LitF64(value));
        }
    }
    if looks_numeric(name) || is_non_finite_spelling(name) {
        return Err(Error::msg(format!("invalid numeric literal: {name}")));
    }
    Ok(Expr::Symbol(name.to_string()))
}

fn is_ascii_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn looks_numeric(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.first().is_some_and(u8::is_ascii_digit)
        || matches!(bytes, [b'-' | b'+', digit, ..] if digit.is_ascii_digit())
        || matches!(bytes, [b'-' | b'+', b'.'])
        || matches!(bytes, [b'-' | b'+', b'.', digit, ..] if digit.is_ascii_digit())
        || matches!(bytes, [b'-' | b'+', b'-' | b'+', digit, ..] if digit.is_ascii_digit() || *digit == b'.')
        || matches!(bytes, [b'.'])
        || matches!(bytes, [b'.', digit, ..] if digit.is_ascii_digit())
}

fn is_non_finite_spelling(text: &str) -> bool {
    matches!(
        text.to_ascii_lowercase().as_str(),
        "nan" | "inf" | "infinity" | "-nan" | "-inf" | "-infinity" | "+nan" | "+inf" | "+infinity"
    )
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::lex::lex;

    #[test]
    fn parses_call() {
        let toks = lex("+/\n1\n2\n/+\n").unwrap();
        let forms = parse_tokens(&toks).unwrap();
        assert_eq!(
            forms,
            vec![Expr::Call {
                name: "+".into(),
                args: vec![Expr::LitI64(1), Expr::LitI64(2)],
            }]
        );
    }

    #[test]
    fn parses_text_block_as_string_literal() {
        let tokens = lex("str/\nhello world\n/str\n").expect("lex");
        let forms = parse_tokens(&tokens).expect("parse");
        assert_eq!(forms, vec![Expr::LitStr("hello world".into())]);
    }

    #[test]
    fn integer_literals_preserve_complete_i64_values() {
        assert_eq!(
            atom_from_name("-9223372036854775808").ok(),
            Some(Expr::LitI64(i64::MIN))
        );
        assert_eq!(
            atom_from_name("9223372036854775807").ok(),
            Some(Expr::LitI64(i64::MAX))
        );
        assert!(atom_from_name("-9223372036854775809").is_err());
        assert!(atom_from_name("9223372036854775808").is_err());
    }

    #[test]
    fn decimal_literals_preserve_f64_identity_and_signed_zero() {
        assert_eq!(atom_from_name("2.0").ok(), Some(Expr::LitF64(2.0)));
        let value = atom_from_name("-0.0").expect("F64 literal");
        let Expr::LitF64(value) = value else {
            panic!("expected F64 literal");
        };
        assert!(value.is_sign_negative());
    }

    #[test]
    fn malformed_numeric_spellings_are_not_symbols() {
        for spelling in [
            "+1", "1e3", "1.", ".", "-.", "+.", ".5", "-.5", "+.5", "--1", "+-1", "1_000", "0x10",
            "1.2.3", "NaN", "+inf", "inf",
        ] {
            assert!(atom_from_name(spelling).is_err(), "accepted {spelling}");
        }
    }
}
