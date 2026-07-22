//! Expression AST lowered from canonical lkjscript source.

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    LitI64(i64),
    LitF64(f64),
    LitBool(bool),
    LitUnit,
    LitNil,
    LitStr(String),
    Symbol(String),
    Call { name: String, args: Vec<Expr> },
    List(Vec<Expr>),
}

impl Expr {
    pub fn depth(&self) -> u32 {
        match self {
            Expr::Call { args, .. } | Expr::List(args) => {
                1 + args.iter().map(Expr::depth).max().unwrap_or(0)
            }
            _ => 0,
        }
    }
}
