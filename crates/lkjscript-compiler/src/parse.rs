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
        Some(Token::Atom(name)) => Ok((atom_from_name(name), i + 1)),
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

fn atom_from_name(name: &str) -> Expr {
    if name == "nil" {
        return Expr::LitNil;
    }
    if name == "true" {
        return Expr::LitBool(true);
    }
    if name == "false" {
        return Expr::LitBool(false);
    }
    if let Ok(n) = name.parse::<f64>() {
        return Expr::LitNum {
            value: n,
            float_syntax: name.contains('.'),
        };
    }
    Expr::Symbol(name.to_string())
}

#[cfg(test)]
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
                args: vec![
                    Expr::LitNum {
                        value: 1.0,
                        float_syntax: false,
                    },
                    Expr::LitNum {
                        value: 2.0,
                        float_syntax: false,
                    },
                ],
            }]
        );
    }

    #[test]
    fn parses_text_block_as_string_literal() {
        let tokens = lex("str/\nhello world\n/str\n").expect("lex");
        let forms = parse_tokens(&tokens).expect("parse");
        assert_eq!(forms, vec![Expr::LitStr("hello world".into())]);
    }
}
