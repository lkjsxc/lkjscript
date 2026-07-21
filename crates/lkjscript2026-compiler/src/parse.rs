//! Parse tokens into expression forms.

use crate::ast::Expr;
use crate::lex::Token;
use lkjscript2026_core::{Error, Result};

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
        Some(Token::Empty(name)) => Ok((atom_from_name(name), i + 1)),
        Some(Token::Text(t)) => Ok((text_atom(t), i + 1)),
        Some(Token::Open(name)) => parse_element(tokens, i, name),
        Some(Token::Close(name)) => Err(Error::msg(format!("unexpected </{name}>"))),
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
                    "mismatched close </{other}>, expected </{name}>"
                )));
            }
            None => return Err(Error::msg(format!("unclosed <{name}>"))),
            _ => {
                let (kid, next) = parse_expr(tokens, j)?;
                kids.push(kid);
                j = next;
            }
        }
    }
}

fn element_to_expr(name: &str, kids: Vec<Expr>) -> Result<Expr> {
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
    // Open/close with zero kids is an empty call (e.g. <params></params>), not an atom.
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
        return Expr::LitNum(n);
    }
    Expr::Symbol(name.to_string())
}

fn text_atom(t: &str) -> Expr {
    if let Ok(n) = t.parse::<f64>() {
        return Expr::LitNum(n);
    }
    if t == "true" {
        return Expr::LitBool(true);
    }
    if t == "false" {
        return Expr::LitBool(false);
    }
    if t == "nil" {
        return Expr::LitNil;
    }
    Expr::LitStr(t.to_string())
}
